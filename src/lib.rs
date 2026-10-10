// SPDX-License-Identifier: GPL-3.0-only
//! Independent in-memory inverted index and BM25-style ranking.
//! Web fetching is deliberately a separate layer; this core never sends queries
//! to another search provider or records a user's search history.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub mod chromatic;
pub mod dom;
pub mod server;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Document {
    pub url: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub image_urls: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SearchHit {
    /// One-based identifier, valid only for this loaded document collection.
    pub document_id: usize,
    pub url: String,
    pub title: String,
    pub snippet: String,
    pub image_urls: Vec<String>,
    pub score: f64,
}

pub struct SearchIndex {
    documents: Vec<Document>,
    postings: BTreeMap<String, Vec<Posting>>,
    lengths: Vec<f64>,
    average_length: f64,
    dom_documents: BTreeMap<usize, dom::DomDocument>,
}

struct Posting {
    document: usize,
    frequency: f64,
}

/// English uses lowercased words. Non-ASCII runs emit characters and bigrams
/// so Japanese input works without requiring whitespace or an external service.
fn tokenize(input: &str, query: bool) -> Vec<String> {
    fn flush(run: &mut String, ascii: bool, query: bool, output: &mut Vec<String>) {
        if run.is_empty() {
            return;
        }
        if ascii {
            output.push(std::mem::take(run));
        } else {
            let chars: Vec<char> = run.chars().collect();
            if !query || chars.len() == 1 {
                output.extend(chars.iter().map(char::to_string));
            }
            output.extend(chars.windows(2).map(|pair| pair.iter().collect()));
            run.clear();
        }
    }
    let mut result = Vec::new();
    let mut run = String::new();
    let mut ascii = true;
    for c in input.to_lowercase().chars() {
        if !c.is_alphanumeric() {
            flush(&mut run, ascii, query, &mut result);
            continue;
        }
        let next_ascii = c.is_ascii();
        if !run.is_empty() && next_ascii != ascii {
            flush(&mut run, ascii, query, &mut result);
        }
        ascii = next_ascii;
        run.push(c);
    }
    flush(&mut run, ascii, query, &mut result);
    result
}

impl SearchIndex {
    pub fn new(documents: Vec<Document>) -> Self {
        let mut postings: BTreeMap<String, Vec<Posting>> = BTreeMap::new();
        let mut lengths = Vec::with_capacity(documents.len());
        for (id, doc) in documents.iter().enumerate() {
            let mut length = 0.0;
            let mut counts = BTreeMap::<String, f64>::new();
            for (value, weight) in [(&doc.title, 3.0), (&doc.body, 1.0)] {
                for token in tokenize(value, false) {
                    *counts.entry(token).or_default() += weight;
                    length += weight;
                }
            }
            for (token, frequency) in counts {
                postings.entry(token).or_default().push(Posting {
                    document: id,
                    frequency,
                });
            }
            lengths.push(length.max(1.0));
        }
        // Documents are appended in ID order, enabling compact binary search.
        for posting in postings.values_mut() {
            posting.shrink_to_fit();
        }
        let average_length = (lengths.iter().sum::<f64>() / documents.len().max(1) as f64).max(1.0);
        Self {
            documents,
            postings,
            lengths,
            average_length,
            dom_documents: BTreeMap::new(),
        }
    }

    pub fn document_count(&self) -> usize {
        self.documents.len()
    }

    /// All normalized query terms must match. Title occurrences are weighted
    /// three times, and an exact title substring adds a small relevance bonus.
    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchHit> {
        let normalized = query.trim().to_lowercase();
        let terms: BTreeSet<String> = tokenize(&normalized, true).into_iter().collect();
        if terms.is_empty() || limit == 0 {
            return Vec::new();
        }
        let n = self.documents.len() as f64;
        let mut term_postings = Vec::with_capacity(terms.len());
        for term in &terms {
            let Some(posting) = self.postings.get(term) else {
                return Vec::new();
            };
            let df = posting.len() as f64;
            let idf = (1.0 + (n - df + 0.5) / (df + 0.5)).ln();
            term_postings.push((posting.as_slice(), idf));
        }
        let seed = term_postings
            .iter()
            .min_by_key(|(posting, _)| posting.len())
            .unwrap()
            .0;
        let limit = limit.min(50);
        let mut best: Vec<(usize, f64)> = Vec::with_capacity(limit);
        let rank = |a: &(usize, f64), b: &(usize, f64)| {
            b.1.total_cmp(&a.1)
                .then_with(|| self.documents[a.0].url.cmp(&self.documents[b.0].url))
                .then_with(|| a.0.cmp(&b.0))
        };
        for candidate in seed {
            let id = candidate.document;
            let norm = 1.2 * (0.25 + 0.75 * self.lengths[id] / self.average_length);
            let mut score = 0.0;
            let mut matched = true;
            for (posting, idf) in &term_postings {
                let Ok(position) = posting.binary_search_by_key(&id, |entry| entry.document) else {
                    matched = false;
                    break;
                };
                let tf = posting[position].frequency;
                score += idf * tf * 2.2 / (tf + norm);
            }
            if !matched {
                continue;
            }
            if contains_normalized(&self.documents[id].title, &normalized) {
                score += 2.0;
            }
            let result = (id, score);
            let position = best
                .binary_search_by(|entry| rank(entry, &result))
                .unwrap_or_else(|at| at);
            if position < limit {
                if best.len() == limit {
                    best.pop();
                }
                best.insert(position, result);
            }
        }
        // Only the selected documents allocate output strings and snippets.
        best.into_iter()
            .map(|(id, score)| {
                let doc = &self.documents[id];
                SearchHit {
                    document_id: id + 1,
                    url: doc.url.clone(),
                    title: doc.title.clone(),
                    snippet: make_snippet(&doc.body, &normalized, 120),
                    image_urls: doc.image_urls.clone(),
                    score,
                }
            })
            .collect()
    }
}

fn contains_normalized(value: &str, normalized: &str) -> bool {
    if value.is_ascii() {
        return normalized.is_ascii()
            && value
                .as_bytes()
                .windows(normalized.len())
                .any(|part| part.eq_ignore_ascii_case(normalized.as_bytes()));
    }
    if value.contains(normalized) {
        return true;
    }
    value.to_lowercase().contains(normalized)
}

fn make_snippet(body: &str, query: &str, length: usize) -> String {
    let lower = body.to_lowercase();
    let offset = lower
        .find(query)
        .map(|at| lower[..at].chars().count())
        .unwrap_or(0);
    let start = offset.saturating_sub(25).min(body.chars().count());
    let mut chars = body.chars().skip(start);
    let content: String = chars.by_ref().take(length).collect();
    let more = chars.next().is_some();
    format!(
        "{}{}{}",
        if start > 0 { "…" } else { "" },
        content,
        if more { "…" } else { "" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn doc(url: &str, title: &str, body: &str) -> Document {
        Document {
            url: url.into(),
            title: title.into(),
            body: body.into(),
            image_urls: vec![],
        }
    }
    #[test]
    fn japanese_query_matches_without_spaces() {
        let index = SearchIndex::new(vec![
            doc("a", "ゲームボーイの本", "小さな画面で遊ぶ"),
            doc("b", "読書", "電子書籍を読む"),
        ]);
        let hits = index.search("ゲームボーイ", 10);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].url, "a");
    }
    #[test]
    fn english_is_case_insensitive_and_all_terms_required() {
        let index = SearchIndex::new(vec![
            doc("a", "Chromatic WiFi", "Network settings"),
            doc("b", "Chromatic", "A handheld"),
        ]);
        let hits = index.search("CHROMATIC wifi", 10);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].url, "a");
    }
    #[test]
    fn relevant_title_outranks_body_repetition() {
        let index = SearchIndex::new(vec![
            doc("a", "Chromatic", "A handheld"),
            doc("b", "Shopping", &"chromatic ".repeat(100)),
        ]);
        assert_eq!(index.search("chromatic", 10)[0].url, "a");
    }
    #[test]
    fn no_match_and_blank_queries_do_not_return_fabricated_results() {
        let index = SearchIndex::new(vec![doc("a", "Reading", "An original document")]);
        assert!(index.search("missing", 10).is_empty());
        assert!(index.search("   ", 10).is_empty());
        assert!(index.search("!!!", 10).is_empty());
        assert!(SearchIndex::new(vec![]).search("reading", 10).is_empty());
    }
    #[test]
    fn results_are_limited_and_ties_are_deterministic() {
        let index = SearchIndex::new(vec![
            doc("z", "Reading", "Books"),
            doc("a", "Reading", "Books"),
        ]);
        assert_eq!(index.search("reading", 1)[0].url, "a");
        assert!(index.search("reading", 0).is_empty());
    }

    #[test]
    fn late_candidates_can_displace_earlier_ties_and_output_is_capped() {
        let documents = (0..60)
            .rev()
            .map(|id| doc(&format!("{id:02}"), "Reading", "Books"))
            .collect();
        let index = SearchIndex::new(documents);
        let hits = index.search("reading", 3);
        assert_eq!(
            hits.iter().map(|hit| hit.url.as_str()).collect::<Vec<_>>(),
            ["00", "01", "02"]
        );
        assert_eq!(hits[0].document_id, 60);
        assert_eq!(index.search("reading", 100).len(), 50);
    }

    #[test]
    fn normalized_title_matching_preserves_unicode_case_mapping() {
        assert!(contains_normalized("ǅ", "ǆ"));
        assert!(contains_normalized("CHROMATIC", "chromatic"));
        assert!(contains_normalized("小さな日本語の本", "日本語"));
        assert!(!contains_normalized("Chromatic", "日本語"));
    }
    #[test]
    fn snippets_are_unicode_safe_and_images_keep_source_urls() {
        let mut page = doc(
            "a",
            "画像",
            &format!("{}画像{}", "あ".repeat(60), "本".repeat(120)),
        );
        page.image_urls
            .push("https://example.invalid/actual-page-photo.jpg".into());
        let index = SearchIndex::new(vec![page]);
        let hit = &index.search("画像", 10)[0];
        assert!(hit.snippet.contains("画像"));
        assert_eq!(hit.image_urls.len(), 1);
        assert!(hit.snippet.chars().count() <= 122);
    }
}
