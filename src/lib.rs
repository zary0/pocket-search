//! Independent in-memory inverted index and BM25-style ranking.
//! Web fetching is deliberately a separate layer; this core never sends queries
//! to another search provider or records a user's search history.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub mod chromatic;

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
    postings: BTreeMap<String, BTreeMap<usize, f64>>,
    lengths: Vec<f64>,
    average_length: f64,
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
        let mut postings: BTreeMap<String, BTreeMap<usize, f64>> = BTreeMap::new();
        let mut lengths = Vec::with_capacity(documents.len());
        for (id, doc) in documents.iter().enumerate() {
            let mut length = 0.0;
            for (value, weight) in [(&doc.title, 3.0), (&doc.body, 1.0)] {
                for token in tokenize(value, false) {
                    *postings.entry(token).or_default().entry(id).or_default() += weight;
                    length += weight;
                }
            }
            lengths.push(length.max(1.0));
        }
        let average_length = (lengths.iter().sum::<f64>() / documents.len().max(1) as f64).max(1.0);
        Self {
            documents,
            postings,
            lengths,
            average_length,
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
        let mut candidates: Option<BTreeSet<usize>> = None;
        for term in &terms {
            let Some(posting) = self.postings.get(term) else {
                return Vec::new();
            };
            let ids: BTreeSet<usize> = posting.keys().copied().collect();
            candidates = Some(match candidates {
                None => ids,
                Some(previous) => previous.intersection(&ids).copied().collect(),
            });
        }
        let n = self.documents.len() as f64;
        let mut hits: Vec<SearchHit> = candidates
            .unwrap_or_default()
            .into_iter()
            .map(|id| {
                let doc = &self.documents[id];
                let mut score = 0.0;
                for term in &terms {
                    let posting = &self.postings[term];
                    let df = posting.len() as f64;
                    let idf = (1.0 + (n - df + 0.5) / (df + 0.5)).ln();
                    let tf = posting[&id];
                    let norm = 1.2 * (0.25 + 0.75 * self.lengths[id] / self.average_length);
                    score += idf * tf * 2.2 / (tf + norm);
                }
                if doc.title.to_lowercase().contains(&normalized) {
                    score += 2.0;
                }
                SearchHit {
                    document_id: id + 1,
                    url: doc.url.clone(),
                    title: doc.title.clone(),
                    snippet: make_snippet(&doc.body, &normalized, 120),
                    image_urls: doc.image_urls.clone(),
                    score,
                }
            })
            .collect();
        hits.sort_by(|a, b| b.score.total_cmp(&a.score).then_with(|| a.url.cmp(&b.url)));
        hits.truncate(limit.min(50));
        hits
    }
}

fn make_snippet(body: &str, query: &str, length: usize) -> String {
    let lower = body.to_lowercase();
    let offset = lower
        .find(query)
        .map(|at| lower[..at].chars().count())
        .unwrap_or(0);
    let chars: Vec<char> = body.chars().collect();
    let start = offset.saturating_sub(25).min(chars.len());
    let end = (start + length).min(chars.len());
    let content: String = chars[start..end].iter().collect();
    format!(
        "{}{}{}",
        if start > 0 { "…" } else { "" },
        content,
        if end < chars.len() { "…" } else { "" }
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
