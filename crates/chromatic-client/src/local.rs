// SPDX-License-Identifier: GPL-3.0-only
//! Bounded on-device crawl frontier, streaming HTML text and inverted index.
//! No allocator, HTTP, JavaScript, external search API, or bundled page content.
use crate::{HEADER_BYTES, MAX_PACKET_BYTES, TEXT_COLUMNS, TEXT_ROWS};

pub const MAX_DOCUMENTS: usize = 4;
pub const BODY_BYTES: usize = 2560;
pub const STATE_BUDGET: usize = 16384;
pub const BASE: &str = "https://www-formal.stanford.edu/jmc/whatisai/";
pub const SEED: &str = "https://www-formal.stanford.edu/jmc/whatisai/node1.html";
const URL_BYTES: usize = 128;
const WORDS: usize = 256;
const FRONTIER: usize = 8;

#[derive(Clone, Copy)]
struct Text<const N: usize> {
    bytes: [u8; N],
    len: u16,
}
impl<const N: usize> Text<N> {
    const EMPTY: Self = Self {
        bytes: [0; N],
        len: 0,
    };
    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len as usize]).unwrap_or("")
    }
    fn push(&mut self, c: char) -> bool {
        let c = if c.is_whitespace() || c.is_control() {
            ' '
        } else {
            c
        };
        let len = self.len as usize;
        if c == ' ' && (len == 0 || self.bytes[len - 1] == b' ') {
            return true;
        }
        let mut buf = [0; 4];
        let encoded = c.encode_utf8(&mut buf).as_bytes();
        if len + encoded.len() > N {
            return false;
        }
        self.bytes[len..len + encoded.len()].copy_from_slice(encoded);
        self.len += encoded.len() as u16;
        true
    }
    fn set(&mut self, s: &str) {
        self.len = 0;
        for c in s.chars() {
            if !self.push(c) {
                break;
            }
        }
    }
}
#[derive(Clone, Copy)]
struct Document {
    title: Text<96>,
    body: Text<BODY_BYTES>,
    url: Text<URL_BYTES>,
    limited: u8,
}
impl Document {
    const EMPTY: Self = Self {
        title: Text::EMPTY,
        body: Text::EMPTY,
        url: Text::EMPTY,
        limited: 0,
    };
}

#[derive(Clone, Copy)]
struct Parser {
    tag: [u8; 256],
    tag_len: u16,
    in_tag: u8,
    quote: u8,
    overflow: u8,
    skip: u8,
    head: u8,
    title: u8,
    comment: u8,
    entity: [u8; 24],
    entity_len: u8,
    in_entity: u8,
    scalar: [u8; 4],
    scalar_len: u8,
    scalar_need: u8,
}
impl Parser {
    const EMPTY: Self = Self {
        tag: [0; 256],
        tag_len: 0,
        in_tag: 0,
        quote: 0,
        overflow: 0,
        skip: 0,
        head: 0,
        title: 0,
        comment: 0,
        entity: [0; 24],
        entity_len: 0,
        in_entity: 0,
        scalar: [0; 4],
        scalar_len: 0,
        scalar_need: 0,
    };
}

/// All fields have valid zero representations; the C boundary owns one aligned
/// opaque instance and serializes every access. Zero initialization is permitted.
pub struct LocalSearch {
    documents: [Document; MAX_DOCUMENTS],
    queue: [Text<URL_BYTES>; FRONTIER],
    hashes: [u64; WORDS],
    masks: [u8; WORDS],
    parser: Parser,
    count: u8,
    queued: u8,
    cursor: u8,
    active: u8,
    full_index: u8,
}
impl Default for LocalSearch {
    fn default() -> Self {
        Self::new()
    }
}
impl LocalSearch {
    pub const fn new() -> Self {
        Self {
            documents: [Document::EMPTY; MAX_DOCUMENTS],
            queue: [Text::EMPTY; FRONTIER],
            hashes: [0; WORDS],
            masks: [0; WORDS],
            parser: Parser::EMPTY,
            count: 0,
            queued: 0,
            cursor: 0,
            active: 0,
            full_index: 0,
        }
    }
    pub fn count(&self) -> usize {
        self.count as usize
    }
    pub fn seed(&mut self) {
        self.enqueue(SEED);
    }
    fn enqueue(&mut self, url: &str) {
        if !allowed_url(url) || self.queued as usize == FRONTIER {
            return;
        }
        if self.queue[..self.queued as usize]
            .iter()
            .any(|v| v.as_str() == url)
        {
            return;
        }
        self.queue[self.queued as usize].set(url);
        self.queued += 1;
    }
    pub fn next_url(&mut self, out: &mut [u8]) -> usize {
        if self.active != 0 || self.count() == MAX_DOCUMENTS || self.cursor >= self.queued {
            return 0;
        }
        let url = self.queue[self.cursor as usize].as_str();
        if out.len() <= url.len() {
            return 0;
        }
        out[..url.len()].copy_from_slice(url.as_bytes());
        out[url.len()] = 0;
        let len = url.len();
        self.documents[self.count as usize] = Document::EMPTY;
        self.documents[self.count as usize].url = self.queue[self.cursor as usize];
        self.cursor += 1;
        self.active = 1;
        self.parser = Parser::EMPTY;
        len
    }
    fn emit(&mut self, c: char) {
        if self.parser.skip != 0 || (self.parser.head != 0 && self.parser.title == 0) {
            return;
        }
        let doc = &mut self.documents[self.count as usize];
        if self.parser.title != 0 {
            let _ = doc.title.push(c);
        } else if !doc.body.push(c) {
            doc.limited = 1;
        }
    }
    fn scalar(&mut self, b: u8) {
        if b < 128 {
            if self.parser.scalar_len != 0 {
                self.emit('\u{fffd}');
                self.parser.scalar_len = 0;
            }
            self.emit(b as char);
            return;
        }
        if self.parser.scalar_len == 0 {
            let need = match b {
                0xc2..=0xdf => 2,
                0xe0..=0xef => 3,
                0xf0..=0xf4 => 4,
                _ => 0,
            };
            if need == 0 {
                self.emit('\u{fffd}');
                return;
            }
            self.parser.scalar[0] = b;
            self.parser.scalar_len = 1;
            self.parser.scalar_need = need;
        } else if b & 0xc0 != 0x80 {
            self.parser.scalar_len = 0;
            self.emit('\u{fffd}');
            self.scalar(b);
        } else {
            let at = self.parser.scalar_len as usize;
            self.parser.scalar[at] = b;
            self.parser.scalar_len += 1;
            if self.parser.scalar_len == self.parser.scalar_need {
                let c =
                    core::str::from_utf8(&self.parser.scalar[..self.parser.scalar_len as usize])
                        .ok()
                        .and_then(|s| s.chars().next())
                        .unwrap_or('\u{fffd}');
                self.parser.scalar_len = 0;
                self.emit(c);
            }
        }
    }
    fn end_tag(&mut self) {
        if self.parser.overflow != 0 {
            return;
        }
        let tag = self.parser.tag;
        let text = core::str::from_utf8(&tag[..self.parser.tag_len as usize])
            .unwrap_or("")
            .trim();
        let closing = text.starts_with('/');
        let text = text.trim_start_matches('/');
        let name = text
            .split(|c: char| c.is_ascii_whitespace() || c == '/')
            .next()
            .unwrap_or("");
        if name.eq_ignore_ascii_case("script") || name.eq_ignore_ascii_case("style") {
            if closing {
                self.parser.skip = 0;
            } else {
                self.parser.skip = 1;
            }
        } else if self.parser.skip == 0 {
            if name.eq_ignore_ascii_case("head") {
                self.parser.head = u8::from(!closing);
            }
            if name.eq_ignore_ascii_case("title") {
                self.parser.title = u8::from(!closing);
            }
            if name.eq_ignore_ascii_case("a") && !closing {
                if let Some(href) = attribute(text, "href") {
                    let href = href.split('#').next().unwrap_or("");
                    let relative = href.strip_prefix(BASE).unwrap_or(href);
                    if allowed_relative(relative) {
                        let mut url = Text::<URL_BYTES>::EMPTY;
                        url.set(BASE);
                        for c in relative.chars() {
                            let _ = url.push(c);
                        }
                        self.enqueue(url.as_str());
                    }
                }
            }
        }
        if matches_ignore_case(
            name,
            &[
                "p", "br", "div", "li", "h1", "h2", "h3", "hr", "tr", "td", "title", "a",
            ],
        ) {
            self.emit(' ');
        }
    }
    pub fn feed(&mut self, bytes: &[u8]) {
        if self.active == 0 {
            return;
        }
        for &b in bytes {
            if self.parser.comment != 0 {
                if b == b'-' {
                    self.parser.comment = (self.parser.comment + 1).min(3);
                } else if b == b'>' && self.parser.comment == 3 {
                    self.parser.comment = 0;
                    self.parser.in_tag = 0;
                } else {
                    self.parser.comment = 1;
                }
                continue;
            }
            if self.parser.in_tag != 0 {
                if b == b'>' && self.parser.quote == 0 {
                    self.end_tag();
                    self.parser.in_tag = 0;
                    continue;
                }
                if self.parser.quote == b && b != 0 {
                    self.parser.quote = 0;
                } else if self.parser.quote == 0 && (b == b'\'' || b == b'"') {
                    self.parser.quote = b;
                }
                if (self.parser.tag_len as usize) < self.parser.tag.len() {
                    self.parser.tag[self.parser.tag_len as usize] = b;
                    self.parser.tag_len += 1;
                    if self.parser.tag_len == 3 && &self.parser.tag[..3] == b"!--" {
                        self.parser.comment = 1;
                    }
                } else {
                    self.parser.overflow = 1;
                }
                continue;
            }
            if self.parser.in_entity != 0 {
                if b == b';' {
                    let entity = core::str::from_utf8(
                        &self.parser.entity[..self.parser.entity_len as usize],
                    )
                    .unwrap_or("");
                    let c = decode_entity(entity).unwrap_or(' ');
                    self.emit(c);
                    self.parser.in_entity = 0;
                    continue;
                }
                if (b.is_ascii_alphanumeric() || b == b'#')
                    && (self.parser.entity_len as usize) < self.parser.entity.len()
                {
                    self.parser.entity[self.parser.entity_len as usize] = b;
                    self.parser.entity_len += 1;
                    continue;
                }
                self.parser.in_entity = 0;
                self.emit(' ');
            }
            if b == b'<' {
                if self.parser.scalar_len != 0 {
                    self.emit('\u{fffd}');
                    self.parser.scalar_len = 0;
                }
                self.parser.in_tag = 1;
                self.parser.tag_len = 0;
                self.parser.quote = 0;
                self.parser.overflow = 0;
            } else if b == b'&' {
                self.parser.in_entity = 1;
                self.parser.entity_len = 0;
            } else {
                self.scalar(b);
            }
        }
    }
    /// Publish only a fully received valid HTML document. Failed partial text
    /// never enters the searchable index or reader.
    pub fn finish(&mut self, complete: bool) -> bool {
        if self.active == 0 {
            return false;
        }
        self.active = 0;
        if !complete || self.documents[self.count as usize].body.len == 0 {
            return false;
        }
        let id = self.count as usize;
        if self.documents[id].title.len == 0 {
            self.documents[id].title.set("Untitled page");
        }
        // Build from the retained valid text only. The URL and hidden markup
        // never become terms. A saturated table falls back to exact scanning.
        for section in 0..2 {
            let doc = self.documents[id];
            let text = if section == 0 {
                doc.title.as_str()
            } else {
                doc.body.as_str()
            };
            for word in words(text) {
                let hash = hash(word);
                let slot = self
                    .hashes
                    .iter()
                    .position(|v| *v == hash)
                    .or_else(|| self.hashes.iter().position(|v| *v == 0));
                if let Some(slot) = slot {
                    self.hashes[slot] = hash;
                    self.masks[slot] |= 1 << id;
                } else {
                    self.full_index = 1;
                }
            }
        }
        self.count += 1;
        true
    }
    fn ranking(&self, query: &str) -> ([u8; MAX_DOCUMENTS], usize) {
        let mut ids = [0; MAX_DOCUMENTS];
        let mut scores = [0; MAX_DOCUMENTS];
        let mut used = 0;
        for (id, doc) in self.documents[..self.count()].iter().enumerate() {
            let mut score = 0;
            let mut terms = 0;
            let mut matched = true;
            for term in words(query) {
                terms += 1;
                if self.full_index == 0 && term.is_ascii() {
                    let possible = self
                        .hashes
                        .iter()
                        .position(|v| *v == hash(term))
                        .is_some_and(|i| self.masks[i] & (1 << id) != 0);
                    if !possible {
                        matched = false;
                        break;
                    }
                }
                let a = occurrences(doc.title.as_str(), term);
                let b = occurrences(doc.body.as_str(), term);
                if a + b == 0 {
                    matched = false;
                    break;
                }
                score += a * 20 + b;
            }
            if matched && terms > 0 {
                let mut at = used;
                while at > 0 && scores[at - 1] < score {
                    ids[at] = ids[at - 1];
                    scores[at] = scores[at - 1];
                    at -= 1;
                }
                ids[at] = id as u8;
                scores[at] = score;
                used += 1;
            }
        }
        (ids, used)
    }
    pub fn search_packet(&self, query: &str, result: usize, out: &mut [u8]) -> usize {
        let (ids, total) = self.ranking(query);
        if total == 0 {
            return if result == 0 {
                packet(
                    1,
                    0,
                    0,
                    1,
                    "No matches",
                    &[
                        "Locally collected pages",
                        "No retained text matches",
                        "Right on home: Refresh",
                    ],
                    out,
                )
            } else {
                0
            };
        }
        if result >= total {
            return 0;
        }
        let id = ids[result] as usize;
        let doc = &self.documents[id];
        let url = doc
            .url
            .as_str()
            .strip_prefix("https://")
            .unwrap_or(doc.url.as_str());
        let mut lines = [""; TEXT_ROWS];
        let mut count = wrapped(url, &mut lines[..4], 0);
        count += wrapped(doc.body.as_str(), &mut lines[count..], 0);
        packet(
            1,
            id as u32 + 1,
            result,
            total,
            doc.title.as_str(),
            &lines[..count],
            out,
        )
    }
    /// One seven-row viewport. The PCS1 position is its top wrapped-text line,
    /// so adjacent positions overlap instead of skipping an entire screen.
    pub fn reader_packet(&self, id: usize, offset: usize, out: &mut [u8]) -> usize {
        if id == 0 || id > self.count() {
            return 0;
        }
        let doc = &self.documents[id - 1];
        let count = line_count(doc.body.as_str()) + usize::from(doc.limited != 0);
        let positions = count.saturating_sub(TEXT_ROWS) + 1;
        if offset >= positions {
            return 0;
        }
        let mut lines = [""; TEXT_ROWS];
        let n = wrapped(doc.body.as_str(), &mut lines, offset);
        if doc.limited != 0 && offset + 1 == positions && n < TEXT_ROWS {
            lines[n] = "[Text limit reached]";
            return packet(
                2,
                id as u32,
                offset,
                positions,
                doc.title.as_str(),
                &lines[..n + 1],
                out,
            );
        }
        packet(
            2,
            id as u32,
            offset,
            positions,
            doc.title.as_str(),
            &lines[..n],
            out,
        )
    }
}

fn matches_ignore_case(value: &str, values: &[&str]) -> bool {
    values.iter().any(|s| value.eq_ignore_ascii_case(s))
}
fn allowed_relative(value: &str) -> bool {
    value
        .strip_prefix("node")
        .and_then(|s| s.strip_suffix(".html"))
        .is_some_and(|n| !n.is_empty() && n.len() <= 2 && n.bytes().all(|c| c.is_ascii_digit()))
}
pub fn allowed_url(url: &str) -> bool {
    url.strip_prefix(BASE).is_some_and(allowed_relative)
}
fn attribute<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let bytes = text.as_bytes();
    let mut at = text.find(char::is_whitespace)?;
    while at < bytes.len() {
        while at < bytes.len() && (bytes[at].is_ascii_whitespace() || bytes[at] == b'/') {
            at += 1;
        }
        let start = at;
        while at < bytes.len() && !bytes[at].is_ascii_whitespace() && bytes[at] != b'=' {
            at += 1;
        }
        let name = text.get(start..at)?;
        while at < bytes.len() && bytes[at].is_ascii_whitespace() {
            at += 1;
        }
        if bytes.get(at) != Some(&b'=') {
            if at == start {
                at += 1;
            }
            continue;
        }
        at += 1;
        while at < bytes.len() && bytes[at].is_ascii_whitespace() {
            at += 1;
        }
        let quote = bytes.get(at).copied()?;
        let quoted = quote == b'\'' || quote == b'"';
        if quoted {
            at += 1;
        }
        let start = at;
        while at < bytes.len()
            && (if quoted {
                bytes[at] != quote
            } else {
                !bytes[at].is_ascii_whitespace()
            })
        {
            at += 1;
        }
        if name.eq_ignore_ascii_case(key) {
            return text.get(start..at);
        }
        if quoted {
            at += 1;
        }
    }
    None
}
fn decode_entity(entity: &str) -> Option<char> {
    match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some(' '),
        _ => entity
            .strip_prefix("#x")
            .or_else(|| entity.strip_prefix("#X"))
            .and_then(|n| u32::from_str_radix(n, 16).ok())
            .or_else(|| entity.strip_prefix('#').and_then(|n| n.parse().ok()))
            .and_then(char::from_u32),
    }
}
fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| c.is_ascii() && !c.is_ascii_alphanumeric() && c != '_')
        .filter(|s| !s.is_empty())
}
fn hash(word: &str) -> u64 {
    word.bytes()
        .fold(14695981039346656037u64, |n, b| {
            (n ^ u64::from(b.to_ascii_lowercase())).wrapping_mul(1099511628211)
        })
        .max(1)
}
fn occurrences(text: &str, term: &str) -> usize {
    if term.is_ascii() {
        words(text)
            .filter(|word| word.eq_ignore_ascii_case(term))
            .count()
    } else {
        text.match_indices(term).count()
    }
}
fn take_line(text: &str) -> (&str, &str) {
    let mut columns = 0;
    let mut end = 0;
    let mut space = None;
    for (at, c) in text.char_indices() {
        let width = if c.is_ascii() { 1 } else { 2 };
        if columns + width > TEXT_COLUMNS {
            break;
        }
        columns += width;
        end = at + c.len_utf8();
        if c == ' ' {
            space = Some(at);
        }
    }
    if end < text.len() {
        if let Some(at) = space.filter(|v| *v > 0) {
            end = at;
        }
    }
    (&text[..end], text[end..].trim_start())
}
fn line_count(mut text: &str) -> usize {
    let mut n = 0;
    while !text.is_empty() {
        text = take_line(text).1;
        n += 1;
    }
    n
}
fn wrapped<'a>(mut text: &'a str, out: &mut [&'a str], mut skip: usize) -> usize {
    while skip > 0 && !text.is_empty() {
        text = take_line(text).1;
        skip -= 1;
    }
    let mut n = 0;
    for line in out {
        if text.is_empty() {
            break;
        }
        let (part, rest) = take_line(text);
        *line = part;
        text = rest;
        n += 1;
    }
    n
}
fn packet(
    kind: u8,
    id: u32,
    page: usize,
    count: usize,
    title: &str,
    lines: &[&str],
    out: &mut [u8],
) -> usize {
    if out.len() < MAX_PACKET_BYTES || lines.len() > TEXT_ROWS || count == 0 || page >= count {
        return 0;
    }
    let title = take_line(title).0;
    out[..HEADER_BYTES].fill(0);
    out[..4].copy_from_slice(b"PCS1");
    out[4] = kind;
    out[5] = u8::from(page > 0) | (u8::from(page + 1 < count) << 1);
    out[6] = lines.len() as u8;
    out[7] = title.len() as u8;
    out[8..12].copy_from_slice(&id.to_le_bytes());
    out[12..14].copy_from_slice(&(page as u16).to_le_bytes());
    out[14..16].copy_from_slice(&(count as u16).to_le_bytes());
    let mut at = HEADER_BYTES;
    out[at..at + title.len()].copy_from_slice(title.as_bytes());
    at += title.len();
    for line in lines {
        let line = take_line(line).0;
        out[at] = line.len() as u8;
        at += 1;
        out[at..at + line.len()].copy_from_slice(line.as_bytes());
        at += line.len();
    }
    out[16..18].copy_from_slice(&((at - HEADER_BYTES) as u16).to_le_bytes());
    at
}

/// Conservative robots evaluator for the fixed ASCII path frontier. Oversize,
/// non-UTF-8 or percent-encoded rule files fail closed rather than approximating.
pub fn robots_allowed(body: &[u8], path: &str) -> bool {
    let Ok(body) = core::str::from_utf8(body) else {
        return false;
    };
    if body.len() > 2048 || body.contains('%') {
        return false;
    }
    let specific = body.lines().filter_map(rule).any(|(key, value)| {
        key.eq_ignore_ascii_case("user-agent") && value.eq_ignore_ascii_case("PocketSearchBot")
    });
    let mut applies = false;
    let mut had_rule = false;
    let mut best = 0;
    let mut allowed = true;
    for (key, value) in body.lines().filter_map(rule) {
        if key.eq_ignore_ascii_case("user-agent") {
            if had_rule {
                applies = false;
                had_rule = false;
            }
            applies |= if specific {
                value.eq_ignore_ascii_case("PocketSearchBot")
            } else {
                value == "*"
            };
        } else if key.eq_ignore_ascii_case("allow") || key.eq_ignore_ascii_case("disallow") {
            had_rule = true;
            if applies && !value.is_empty() && pattern_match(value.as_bytes(), path.as_bytes()) {
                let specificity = value.bytes().filter(|b| *b != b'*' && *b != b'$').count();
                let allow = key.eq_ignore_ascii_case("allow");
                if specificity > best || (specificity == best && allow) {
                    best = specificity;
                    allowed = allow;
                }
            }
        }
    }
    allowed
}
fn rule(line: &str) -> Option<(&str, &str)> {
    let line = line.split('#').next()?.trim();
    let (key, value) = line.split_once(':')?;
    Some((key.trim(), value.trim()))
}
fn pattern_match(pattern: &[u8], text: &[u8]) -> bool {
    let anchored = pattern.last() == Some(&b'$');
    let pattern = if anchored {
        &pattern[..pattern.len() - 1]
    } else {
        pattern
    };
    let (mut p, mut t, mut star, mut retry) = (0, 0, None, 0);
    while t < text.len() {
        if p == pattern.len() && !anchored {
            return true;
        }
        if pattern.get(p) == Some(&b'*') {
            star = Some(p);
            p += 1;
            retry = t;
        } else if pattern.get(p) == Some(&text[t]) {
            p += 1;
            t += 1;
        } else if let Some(s) = star {
            retry += 1;
            t = retry;
            p = s + 1;
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == b'*' {
        p += 1;
    }
    p == pattern.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Packet;
    fn collect(index: &mut LocalSearch, html: &[u8], split: usize) {
        let mut url = [0; 128];
        assert!(index.next_url(&mut url) > 0);
        for chunk in html.chunks(split) {
            index.feed(chunk);
        }
        assert!(index.finish(true));
    }
    #[test]
    fn every_stream_boundary_is_equivalent_and_no_active_content_is_indexed() {
        let html = "<head><title>AI &amp; learning</title><style>secret</style></head><!--noise--><p>Real &#x65E5;&#26412; AI 語</p><script>virus</script><a href='node2.html'>Next</a>".as_bytes();
        for split in 1..=html.len() {
            let mut index = LocalSearch::new();
            index.seed();
            collect(&mut index, html, split);
            assert_eq!(index.documents[0].title.as_str(), "AI & learning");
            assert_eq!(index.documents[0].body.as_str(), "Real 日本 AI 語 Next ");
            assert_eq!(index.ranking("virus").1, 0);
            assert_eq!(index.ranking("noise").1, 0);
            assert_eq!(index.ranking("日本").1, 1);
            assert_eq!(index.ranking("AI").1, 1);
            let mut url = [0; 128];
            let n = index.next_url(&mut url);
            assert_eq!(&url[..n], format_url("node2.html").as_bytes());
        }
    }
    fn format_url(name: &str) -> Text<128> {
        let mut t = Text::EMPTY;
        t.set(BASE);
        for c in name.chars() {
            t.push(c);
        }
        t
    }
    impl<const N: usize> Text<N> {
        fn as_bytes(&self) -> &[u8] {
            &self.bytes[..self.len as usize]
        }
    }
    #[test]
    fn limits_failed_documents_frontier_and_packets() {
        assert!(core::mem::size_of::<LocalSearch>() <= STATE_BUDGET);
        let mut index = LocalSearch::new();
        index.seed();
        for _ in 0..4 {
            collect(&mut index, b"<title>AI</title><p>AI local search works.</p><a href='node2.html'>Next</a><a href='node3.html'>Third</a><a href='node4.html'>Fourth</a><a href='https://localhost/'>bad</a>", 1);
        }
        assert_eq!(index.count(), 4);
        assert_eq!(index.next_url(&mut [0; 128]), 0);
        let mut out = [0; 512];
        for result in 0..4 {
            let n = index.search_packet("AI", result, &mut out);
            let p = Packet::decode(&out[..n]).unwrap();
            assert_eq!(p.page_count, 4);
            assert_eq!(p.document_id, result as u32 + 1);
        }
        assert_eq!(index.search_packet("AI", 4, &mut out), 0);
        let n = index.reader_packet(1, 0, &mut out);
        assert_eq!(Packet::decode(&out[..n]).unwrap().kind, crate::Kind::Reader);
        assert_eq!(index.reader_packet(1, 99, &mut out), 0);
        assert_eq!(index.ranking("stanford").1, 0);
        assert_eq!(index.ranking("AI nonexistent").1, 0);
        let mut broken = LocalSearch::new();
        broken.seed();
        broken.next_url(&mut [0; 128]);
        broken.feed(b"<p>partial AI");
        assert!(!broken.finish(false));
        assert_eq!(broken.count(), 0);
        assert!(!allowed_url(
            "https://www-formal.stanford.edu@localhost/node1.html"
        ));
        assert!(!allowed_url(
            "https://www-formal.stanford.edu/jmc/whatisai/../node1.html"
        ));
    }
    #[test]
    fn utf8_body_limit_preserves_safe_reader_and_visible_truncation() {
        let mut index = LocalSearch::new();
        index.seed();
        index.next_url(&mut [0; 128]);
        index.feed(b"<p>");
        for _ in 0..BODY_BYTES {
            index.feed("語".as_bytes());
        }
        index.feed(b"</p>");
        assert!(index.finish(true));
        assert!(index.documents[0].limited != 0);
        assert!(index.documents[0].body.len as usize <= BODY_BYTES);
        assert_eq!(index.ranking("語").1, 1);
        let mut out = [0; 512];
        let positions =
            (line_count(index.documents[0].body.as_str()) + 1).saturating_sub(TEXT_ROWS) + 1;
        for p in 0..positions {
            let n = index.reader_packet(1, p, &mut out);
            let packet = Packet::decode(&out[..n]).unwrap();
            assert_eq!(packet.page_count as usize, positions);
            if p + 1 == positions {
                assert!(packet.lines.contains(&"[Text limit reached]"));
            }
        }
        assert_eq!(index.reader_packet(1, positions, &mut out), 0);
    }
    #[test]
    fn adjacent_reader_windows_overlap_without_losing_text_or_leaving_the_end() {
        let mut index = LocalSearch::new();
        index.seed();
        collect(&mut index, "<p>First sentence 日本語 starts the article. Second sentence moves below it. Third sentence keeps more content. Fourth sentence reaches another viewport. Last sentence ends here.</p>".as_bytes(), 1);
        let mut first = [0; 512];
        let mut next = [0; 512];
        let n = index.reader_packet(1, 0, &mut first);
        let first_view = Packet::decode(&first[..n]).unwrap();
        assert_eq!(first_view.line_count, TEXT_ROWS);
        assert!(first_view.page_count > 1);
        assert_eq!(first_view.action(crate::Button::Up), crate::Action::None);
        assert_eq!(first_view.action(crate::Button::Down), crate::Action::Next);
        let positions = first_view.page_count as usize;
        for offset in 1..positions {
            let a = index.reader_packet(1, offset - 1, &mut first);
            let b = index.reader_packet(1, offset, &mut next);
            let previous = Packet::decode(&first[..a]).unwrap();
            let current = Packet::decode(&next[..b]).unwrap();
            assert_eq!(previous.lines[1..], current.lines[..TEXT_ROWS - 1]);
            assert_eq!(current.page_index as usize, offset);
            assert_eq!(current.action(crate::Button::Up), crate::Action::Previous);
            if offset + 1 == positions {
                assert_eq!(current.action(crate::Button::Down), crate::Action::None);
                assert!(current.lines[TEXT_ROWS - 1].contains("here."));
            }
        }
        assert_eq!(index.reader_packet(1, positions, &mut next), 0);
    }
    #[test]
    fn saturated_terms_fall_back_and_hash_candidates_require_exact_text() {
        let mut index = LocalSearch::new();
        index.seed();
        index.next_url(&mut [0; 128]);
        index.feed(b"<title>Terms</title><p>");
        for n in 0..WORDS + 32 {
            index.feed(&[b't', b'a' + (n / 26) as u8, b'a' + (n % 26) as u8, b' ']);
        }
        index.feed(b"tailkeyword</p>");
        assert!(index.finish(true));
        assert_eq!(index.full_index, 1);
        assert_eq!(index.documents[0].limited, 0);
        assert!(!index.hashes.contains(&hash("tailkeyword")));
        assert_eq!(index.ranking("tailkeyword").1, 1);
        assert_eq!(index.ranking("absentkeyword").1, 0);

        let mut collision = LocalSearch::new();
        collision.seed();
        collect(&mut collision, b"<p>presentkeyword</p>", 1);
        collision.hashes[0] = hash("absentkeyword");
        collision.masks[0] = 1;
        assert_eq!(collision.full_index, 0);
        assert_eq!(collision.ranking("absentkeyword").1, 0);
    }
    #[test]
    fn robots_specific_groups_merge_allow_ties_wildcards_and_fail_closed() {
        let rules = b"User-agent: *\nDisallow: /\nUser-agent: PocketSearchBot\nDisallow: /jmc/*\nAllow: /jmc/whatisai/\nUser-agent: PocketSearchBot\nDisallow: /jmc/whatisai/*node4.html$\n";
        assert!(robots_allowed(rules, "/jmc/whatisai/node1.html"));
        assert!(!robots_allowed(rules, "/jmc/whatisai/node4.html"));
        assert!(!robots_allowed(
            b"User-agent: *\nDisallow: /jmc/\n",
            "/jmc/whatisai/node1.html"
        ));
        assert!(!robots_allowed(&[255], "/"));
        assert!(!robots_allowed(&[b' '; 2049], "/"));
        assert!(robots_allowed(
            b"User-agent: *\nDisallow: /jmc/\nAllow: /jmc/",
            "/jmc/"
        ));
    }
}
