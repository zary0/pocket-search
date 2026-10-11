// SPDX-License-Identifier: GPL-3.0-only
//! Bounded on-device crawl frontier, streaming HTML text and inverted index.
//! No allocator, HTTP, JavaScript, external search API, or bundled page content.
mod storage;
use crate::{HEADER_BYTES, MAX_PACKET_BYTES, TEXT_COLUMNS, TEXT_ROWS};
pub use storage::SNAPSHOT_BYTES;

pub const MAX_DOCUMENTS: usize = 4;
pub const BODY_BYTES: usize = 2560;
pub const STATE_BUDGET: usize = 22272;
pub const BASE: &str = "https://www-formal.stanford.edu/jmc/whatisai/";
pub const SEED: &str = "https://www-formal.stanford.edu/jmc/whatisai/node1.html";
include!("sources.rs");
const URL_BYTES: usize = 128;
const WORDS: usize = 256;
const FRONTIER: usize = 8;
const STYLE_RUNS: usize = 48;

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
struct InfoText {
    bytes: [u8; 384],
    len: usize,
}
impl core::fmt::Write for InfoText {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        if s.len() > self.bytes.len() - self.len {
            return Err(core::fmt::Error);
        }
        self.bytes[self.len..self.len + s.len()].copy_from_slice(s.as_bytes());
        self.len += s.len();
        Ok(())
    }
}
fn utc(timestamp: u32) -> (u32, u32, u32, u32, u32, u32) {
    fn leap(year: u32) -> bool {
        year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
    }
    let mut days = timestamp / 86400;
    let mut year = 1970;
    loop {
        let n = if leap(year) { 366 } else { 365 };
        if days < n {
            break;
        }
        days -= n;
        year += 1;
    }
    let months = [
        31,
        if leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 0;
    while days >= months[month] {
        days -= months[month];
        month += 1;
    }
    let seconds = timestamp % 86400;
    (
        year,
        month as u32 + 1,
        days + 1,
        seconds / 3600,
        (seconds / 60) % 60,
        seconds % 60,
    )
}

#[derive(Clone, Copy)]
struct Document {
    title: Text<96>,
    body: Text<BODY_BYTES>,
    url: Text<URL_BYTES>,
    limited: u8,
    runs: [crate::rich::Run; STYLE_RUNS],
    run_count: u8,
    style_limited: u8,
    tree: crate::flow::Tree,
    source_bytes: u32,
    fetched_at: u32,
    http_status: u16,
    checked_at: u32,
    flow_positions: u16,
    etag: Text<96>,
    modified: Text<64>,
}
impl Document {
    fn content(&self) -> crate::flow::Content<'_> {
        crate::flow::Content {
            title: self.title.as_str(),
            body: self.body.as_str(),
            runs: &self.runs[..self.run_count as usize],
            tree: &self.tree,
            marker: u8::from(self.limited != 0) | (u8::from(self.style_limited != 0) << 1),
        }
    }
    const EMPTY: Self = Self {
        title: Text::EMPTY,
        body: Text::EMPTY,
        url: Text::EMPTY,
        limited: 0,
        runs: [crate::rich::Run::EMPTY; STYLE_RUNS],
        run_count: 0,
        style_limited: 0,
        tree: crate::flow::Tree::EMPTY,
        source_bytes: 0,
        fetched_at: 0,
        http_status: 0,
        checked_at: 0,
        flow_positions: 0,
        etag: Text::EMPTY,
        modified: Text::EMPTY,
    };
}

/// Temporary, integer-only snapshot owned by the adapter during one CSS reparse.
/// Not part of the retained index. Zero is a valid empty snapshot representation.
pub struct SavedDocument(Document);

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
    styles: [(u8, u8); 16],
    depth: u8,
    style: u8,
    css_depth: u8,
    css_quote: u8,
    css_overflow: u8,
    css_comment: u8,
    css_prev: u8,
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
        styles: [(0, 0); 16],
        depth: 0,
        style: 0,
        css_depth: 0,
        css_quote: 0,
        css_overflow: 0,
        css_comment: 0,
        css_prev: 0,
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
    sheet: crate::css::Sheet,
    count: u8,
    queued: u8,
    cursor: u8,
    active: u8,
    target: u8,
    updating: u8,
    full_index: u8,
    profile: u8,
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
            sheet: crate::css::Sheet::EMPTY,
            count: 0,
            queued: 0,
            cursor: 0,
            active: 0,
            target: 0,
            updating: 0,
            full_index: 0,
            profile: 0,
        }
    }
    pub fn count(&self) -> usize {
        self.count as usize
    }
    pub fn seed(&mut self) {
        self.enqueue(PROFILES[self.profile as usize].2);
    }
    pub fn set_profile(&mut self, profile: u8) -> bool {
        if profile as usize >= PROFILES.len() {
            return false;
        }
        // Reset the reachable collection and index without a whole-state copy.
        // next_url clears each reused document before it can be published.
        self.count = 0;
        self.queued = 0;
        self.cursor = 0;
        self.active = 0;
        self.target = 0;
        self.updating = 0;
        self.full_index = 0;
        self.hashes.fill(0);
        self.masks.fill(0);
        self.parser = Parser::EMPTY;
        self.sheet = crate::css::Sheet::EMPTY;
        self.profile = profile;
        self.seed();
        true
    }
    pub fn profile(&self) -> u8 {
        self.profile
    }
    fn enqueue(&mut self, url: &str) {
        if !allowed_profile_url(self.profile, url)
            || url.len() >= URL_BYTES
            || self.queued as usize == FRONTIER
        {
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
    pub fn cancel_transfer(&mut self) {
        if self.active != 0 && self.updating == 0 {
            self.active = 0;
            self.cursor = self.cursor.saturating_sub(1);
        }
    }
    pub fn next_url(&mut self, out: &mut [u8]) -> usize {
        if self.active != 0 || self.count() == MAX_DOCUMENTS || self.cursor >= self.queued {
            return 0;
        }
        self.target = self.count;
        let url = self.queue[self.cursor as usize].as_str();
        if out.len() <= url.len() {
            return 0;
        }
        out[..url.len()].copy_from_slice(url.as_bytes());
        out[url.len()] = 0;
        let len = url.len();
        self.documents[self.target as usize] = Document::EMPTY;
        self.documents[self.target as usize].url = self.queue[self.cursor as usize];
        self.cursor += 1;
        self.active = 1;
        self.parser = Parser::EMPTY;
        self.sheet.begin_page();
        len
    }
    fn emit(&mut self, c: char) {
        if self.parser.skip != 0 || (self.parser.head != 0 && self.parser.title == 0) {
            return;
        }
        let doc = &mut self.documents[self.target as usize];
        if self.parser.title != 0 {
            let _ = doc.title.push(c);
        } else {
            if c.is_whitespace() && doc.body.as_str().ends_with('\n') {
                return;
            }
            let before = doc.body.len;
            if !doc.body.push(c) {
                doc.limited = 1;
            }
            if doc.body.len > before {
                let mut style = if doc.style_limited != 0 {
                    0
                } else {
                    self.parser.style
                };
                let previous = doc
                    .runs
                    .get(usize::from(doc.run_count).wrapping_sub(1))
                    .map_or(0, |r| r.style);
                if style != previous {
                    if usize::from(doc.run_count) >= STYLE_RUNS - 1 {
                        doc.style_limited = 1;
                        style = 0;
                    }
                    if style != previous && usize::from(doc.run_count) < STYLE_RUNS {
                        doc.runs[usize::from(doc.run_count)] = crate::rich::Run {
                            start: before,
                            style,
                        };
                        doc.run_count += 1;
                    }
                }
            }
        }
    }
    fn block_break(&mut self, wanted: usize) {
        let doc = &mut self.documents[self.target as usize];
        while doc.body.as_str().ends_with(' ') {
            doc.body.len -= 1;
        }
        while doc.run_count > 0 && doc.runs[usize::from(doc.run_count - 1)].start >= doc.body.len {
            doc.run_count -= 1;
        }
        if doc.body.len == 0 {
            return;
        }
        let present = doc
            .body
            .as_str()
            .bytes()
            .rev()
            .take_while(|b| *b == b'\n')
            .count();
        for _ in present..wanted {
            let n = usize::from(doc.body.len);
            if n == BODY_BYTES {
                doc.limited = 1;
                break;
            }
            doc.body.bytes[n] = b'\n';
            doc.body.len += 1;
        }
    }
    fn style_tag(&mut self, name: &str, closing: bool, attributes: &str) {
        let tags = [
            "h1",
            "h2",
            "h3",
            "h4",
            "h5",
            "h6",
            "b",
            "strong",
            "small",
            "sub",
            "sup",
            "figcaption",
            "aside",
            "footer",
            "font",
            "span",
            "p",
            "div",
            "blockquote",
            "caption",
            "body",
            "a",
            "li",
            "big",
        ];
        let Some(tag) = tags
            .iter()
            .position(|t| name.eq_ignore_ascii_case(t))
            .map(|n| n as u8 + 1)
        else {
            return;
        };
        let doc = &mut self.documents[self.target as usize];
        if doc.style_limited != 0 {
            self.parser.style = 0;
            return;
        }
        if closing {
            if let Some(n) = self.parser.styles[..usize::from(self.parser.depth)]
                .iter()
                .rposition(|s| s.0 == tag)
            {
                self.parser.style = self.parser.styles[n].1;
                self.parser.depth = n as u8;
            }
            return;
        }
        // Legacy pages may omit </p>; a new block ends the prior paragraph.
        if (1..=6).contains(&tag) || tag == 17 || tag == 18 {
            if let Some(n) = self.parser.styles[..usize::from(self.parser.depth)]
                .iter()
                .rposition(|s| s.0 == 17)
            {
                self.parser.style = self.parser.styles[n].1;
                self.parser.depth = n as u8;
            }
        }
        if usize::from(self.parser.depth) == self.parser.styles.len() {
            doc.style_limited = 1;
            self.parser.style = 0;
            self.parser.depth = 0;
            return;
        }
        self.parser.styles[usize::from(self.parser.depth)] = (tag, self.parser.style);
        self.parser.depth += 1;
        use crate::rich::{BOLD, CAPTION, HEADING, SUBHEADING};
        if tag == 1 {
            self.parser.style = (self.parser.style & crate::rich::WEBFONT) | HEADING | BOLD;
        } else if tag <= 3 {
            self.parser.style = (self.parser.style & crate::rich::WEBFONT) | SUBHEADING | BOLD;
        } else if tag <= 6 {
            self.parser.style = (self.parser.style & crate::rich::WEBFONT) | BOLD;
        } else if tag <= 8 {
            self.parser.style |= BOLD;
        } else if tag <= 14 || tag == 20 {
            self.parser.style = (self.parser.style & (BOLD | crate::rich::WEBFONT)) | CAPTION;
        }
        if tag == 24 {
            self.parser.style = (self.parser.style & (BOLD | crate::rich::WEBFONT)) | SUBHEADING;
        }
        if let Some(value) = attribute(attributes, "size").filter(|_| tag == 15) {
            self.parser.style = (self.parser.style & (BOLD | crate::rich::WEBFONT))
                | match value {
                    "1" | "-1" | "-2" => CAPTION,
                    "4" | "+1" => SUBHEADING,
                    "5" | "6" | "7" | "+2" | "+3" | "+4" => HEADING,
                    _ => 0,
                };
        }
        self.parser.style = self.sheet.apply(
            name,
            attribute(attributes, "id").unwrap_or(""),
            attribute(attributes, "class").unwrap_or(""),
            attribute(attributes, "style").unwrap_or(""),
            self.parser.style,
        );
    }
    pub fn resource(&self, font: bool) -> &str {
        if font {
            self.sheet.font_url()
        } else {
            self.sheet.href()
        }
    }
    pub fn stylesheet(&mut self, bytes: &[u8], base: &str) {
        self.sheet.parse(bytes, base);
    }
    pub fn font_ready(&mut self) {
        self.sheet.font_ready = 1;
    }
    pub fn css_cached(&self) -> bool {
        self.sheet.cached()
    }
    pub fn css_count(&self) -> usize {
        self.sheet.count()
    }
    /// Reparse the active page after resource loading; published documents remain untouched.
    pub fn reparse(&mut self) {
        if self.active != 0 {
            let url = self.documents[self.target as usize].url;
            self.documents[self.target as usize] = Document::EMPTY;
            self.documents[self.target as usize].url = url;
            self.parser = Parser::EMPTY;
            self.sheet.enable();
        }
    }
    pub fn save_active(&self, out: &mut SavedDocument) -> bool {
        if self.active == 0 {
            return false;
        }
        out.0 = self.documents[self.target as usize];
        true
    }
    pub fn restore_active(&mut self, saved: &SavedDocument) {
        if self.active != 0 {
            self.documents[self.target as usize] = saved.0;
            self.parser = Parser::EMPTY;
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
            self.documents[self.target as usize].style_limited = 1;
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
                self.parser.skip = if name.eq_ignore_ascii_case("style")
                    && attribute(text, "media").is_none_or(|v| {
                        v.eq_ignore_ascii_case("all") || v.eq_ignore_ascii_case("screen")
                    }) {
                    2
                } else {
                    1
                };
                self.parser.tag_len = 0;
                self.parser.css_depth = 0;
                self.parser.css_quote = 0;
                self.parser.css_overflow = 0;
                self.parser.css_comment = 0;
                self.parser.css_prev = 0;
            }
        } else if self.parser.skip == 0 {
            if name.eq_ignore_ascii_case("head") {
                self.parser.head = u8::from(!closing);
            }
            if name.eq_ignore_ascii_case("title") {
                self.parser.title = u8::from(!closing);
            }
            if name.eq_ignore_ascii_case("link")
                && !closing
                && self.parser.head != 0
                && attribute(text, "rel").is_some_and(|v| {
                    v.split_ascii_whitespace()
                        .any(|v| v.eq_ignore_ascii_case("stylesheet"))
                })
                && attribute(text, "media").is_none_or(|v| {
                    v.eq_ignore_ascii_case("all") || v.eq_ignore_ascii_case("screen")
                })
            {
                if let Some(href) = attribute(text, "href") {
                    if self.sheet.href().is_empty() {
                        self.sheet
                            .discover(self.documents[self.target as usize].url.as_str(), href);
                    }
                }
            }
            if name.eq_ignore_ascii_case("a") && !closing {
                if let Some(href) = attribute(text, "href") {
                    let href = href.split('#').next().unwrap_or("");
                    let mut url = [0; crate::css::URL];
                    let n = crate::css::resolve(
                        self.documents[self.target as usize].url.as_str(),
                        href,
                        &mut url,
                    );
                    if let Ok(url) = core::str::from_utf8(&url[..n]) {
                        self.enqueue(url);
                    }
                }
            }
        }
        if self.parser.skip == 0 && self.parser.head == 0 && self.parser.title == 0 {
            if closing {
                self.documents[self.target as usize]
                    .tree
                    .close(name, self.documents[self.target as usize].body.len);
            }
            if matches_ignore_case(
                name,
                &[
                    "p",
                    "h1",
                    "h2",
                    "h3",
                    "h4",
                    "h5",
                    "h6",
                    "blockquote",
                    "figcaption",
                    "aside",
                    "footer",
                    "caption",
                ],
            ) {
                self.block_break(2);
            } else if matches_ignore_case(
                name,
                &[
                    "br", "div", "li", "hr", "tr", "td", "dt", "dd", "dl", "ul", "ol", "figure",
                    "img",
                ],
            ) {
                self.block_break(1);
            }
            self.style_tag(name, closing, text);
            if !closing {
                let at = self.documents[self.target as usize].body.len;
                let opened =
                    self.documents[self.target as usize]
                        .tree
                        .open(name, text, at, &self.sheet);
                if name.eq_ignore_ascii_case("li") {
                    if let Some(i) = opened {
                        let number =
                            self.documents[self.target as usize].tree.nodes[i as usize].number;
                        if number > 0 {
                            if number >= 10 {
                                self.emit(char::from(b'0' + (number / 10).min(9)));
                            }
                            self.emit(char::from(b'0' + number % 10));
                            self.emit('.');
                        } else {
                            self.emit('-');
                        }
                        self.emit(' ');
                    }
                }
                if name.eq_ignore_ascii_case("img") {
                    for c in "[Image: ".chars() {
                        self.emit(c);
                    }
                    for c in attribute(text, "alt").unwrap_or("no description").chars() {
                        self.emit(c);
                    }
                    self.emit(']');
                    let at = self.documents[self.target as usize].body.len;
                    self.documents[self.target as usize].tree.close("img", at);
                    self.block_break(1);
                }
            }
        }
    }
    fn embedded_byte(&mut self, b: u8) {
        let p = &mut self.parser;
        if p.css_comment != 0 {
            if p.css_prev == b'*' && b == b'/' {
                p.css_comment = 0;
            }
            p.css_prev = b;
            return;
        }
        if p.css_quote == 0 && p.css_prev == b'/' && b == b'*' {
            p.css_comment = 1;
            p.tag_len = p.tag_len.saturating_sub(1);
            p.css_prev = b;
            return;
        }
        if p.css_quote != 0 {
            if b == p.css_quote {
                p.css_quote = 0;
            }
        } else if b == b'\'' || b == b'"' {
            p.css_quote = b;
        } else if b == b'{' {
            p.css_depth = p.css_depth.saturating_add(1);
            if p.css_depth > 1 {
                p.css_overflow = 1;
            }
        } else if b == b'}' {
            p.css_depth = p.css_depth.saturating_sub(1);
        }
        if b == b'\\' {
            p.css_overflow = 1;
        }
        if (p.tag_len as usize) < p.tag.len() {
            p.tag[p.tag_len as usize] = b;
            p.tag_len += 1;
        } else {
            p.css_overflow = 1;
        }
        if b == b'}' && p.css_quote == 0 && p.css_depth == 0 {
            if p.css_overflow == 0 {
                self.sheet.embedded(&p.tag[..p.tag_len as usize]);
            } else {
                self.sheet.limited = 1;
            }
            p.tag_len = 0;
            p.css_overflow = 0;
        }
        p.css_prev = b;
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
            if self.parser.skip == 2 && b != b'<' {
                self.embedded_byte(b);
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
        let doc = &mut self.documents[self.target as usize];
        doc.body.len = doc.body.as_str().trim_end().len() as u16;
        doc.tree.finish(doc.body.len);
        while doc.run_count > 0 && doc.runs[usize::from(doc.run_count - 1)].start >= doc.body.len {
            doc.run_count -= 1;
        }
        if !complete || self.documents[self.target as usize].body.len == 0 {
            return false;
        }
        let id = self.target as usize;
        if self.documents[id].title.len == 0 {
            self.documents[id].title.set("Untitled page");
        }
        self.documents[id].flow_positions =
            crate::flow::positions(self.documents[id].content()) as u16;
        if self.updating != 0 {
            self.hashes.fill(0);
            self.masks.fill(0);
            self.full_index = 0;
            for index in 0..self.count() {
                self.index_document(index);
            }
            self.updating = 0;
        } else {
            self.index_document(id);
            self.count += 1;
        }
        true
    }
    fn index_document(&mut self, id: usize) {
        // Build from the retained valid text only. The URL and hidden markup
        // never become terms. A saturated table falls back to exact scanning.
        for section in 0..2 {
            let doc = &self.documents[id];
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
    pub fn metadata(&mut self, bytes: u32, fetched_at: u32, status: u16) -> bool {
        if self.active == 0 || bytes > 65536 || status != 200 || fetched_at < 1609459200 {
            return false;
        }
        let doc = &mut self.documents[self.target as usize];
        doc.source_bytes = bytes;
        doc.fetched_at = fetched_at;
        doc.http_status = status;
        doc.checked_at = fetched_at;
        true
    }
    pub fn validators(&mut self, etag: &str, modified: &str) -> bool {
        if self.active == 0
            || etag.len() > 95
            || modified.len() > 63
            || etag
                .bytes()
                .chain(modified.bytes())
                .any(|b| !(32..=126).contains(&b))
        {
            return false;
        }
        let d = &mut self.documents[self.target as usize];
        d.etag.bytes[..etag.len()].copy_from_slice(etag.as_bytes());
        d.etag.len = etag.len() as u16;
        d.modified.bytes[..modified.len()].copy_from_slice(modified.as_bytes());
        d.modified.len = modified.len() as u16;
        true
    }
    pub fn begin_update(&mut self, id: usize, saved: &mut SavedDocument, out: &mut [u8]) -> usize {
        if self.active != 0 || id == 0 || id > self.count() {
            return 0;
        }
        let d = &self.documents[id - 1];
        let url = d.url.as_str();
        if out.len() <= url.len() {
            return 0;
        }
        out[..url.len()].copy_from_slice(url.as_bytes());
        out[url.len()] = 0;
        let n = url.len();
        saved.0 = *d;
        self.target = (id - 1) as u8;
        self.updating = 1;
        self.active = 1;
        self.documents[id - 1] = Document::EMPTY;
        self.documents[id - 1].url = saved.0.url;
        self.parser = Parser::EMPTY;
        self.sheet.begin_page();
        n
    }
    pub fn update_header(saved: &SavedDocument, modified: bool, out: &mut [u8]) -> usize {
        let value = if modified {
            saved.0.modified.as_str()
        } else {
            saved.0.etag.as_str()
        };
        if out.len() <= value.len() {
            return 0;
        }
        out[..value.len()].copy_from_slice(value.as_bytes());
        out[value.len()] = 0;
        value.len()
    }
    pub fn restore_update(&mut self, saved: &SavedDocument, unchanged: bool, checked: u32) -> bool {
        if self.updating == 0 {
            return false;
        }
        self.documents[self.target as usize] = saved.0;
        if unchanged && checked >= 1609459200 {
            self.documents[self.target as usize].checked_at = checked;
        }
        self.active = 0;
        self.updating = 0;
        true
    }
    /// Full admitted URL and acquisition metadata; no query or browsing history.
    pub fn info_packet(&self, id: usize, page: usize, out: &mut [u8]) -> usize {
        use core::fmt::Write;
        if id == 0 || id > self.count() {
            return 0;
        }
        let doc = &self.documents[id - 1];
        let mut text = InfoText {
            bytes: [0; 384],
            len: 0,
        };
        if writeln!(
            text,
            "{}\n{}",
            PROFILES[self.profile as usize].0,
            doc.url.as_str()
        )
        .is_err()
        {
            return 0;
        }
        if doc.http_status == 200 {
            let (year, month, day, hour, minute, second) = utc(doc.fetched_at);
            if writeln!(text,"HTTPS verified\nHTTP {}\nFetched UTC\n{year:04}-{month:02}-{day:02}\n{hour:02}:{minute:02}:{second:02}\nReceived {} B",doc.http_status,doc.source_bytes).is_err(){return 0;}
        } else if writeln!(text, "Acquisition not recorded").is_err() {
            return 0;
        }
        if doc.checked_at > doc.fetched_at {
            let (y, m, d, h, min, sec) = utc(doc.checked_at);
            if writeln!(
                text,
                "Unchanged check UTC\n{y:04}-{m:02}-{d:02}\n{h:02}:{min:02}:{sec:02}"
            )
            .is_err()
            {
                return 0;
            }
        }
        if writeln!(
            text,
            "Retained {} B\n{}",
            doc.body.len,
            if doc.limited != 0 {
                "Text limit reached"
            } else {
                "Complete retained text"
            }
        )
        .is_err()
        {
            return 0;
        }
        let mut rows = [""; 32];
        let count = wrapped(
            core::str::from_utf8(&text.bytes[..text.len]).unwrap_or(""),
            &mut rows,
            0,
        );
        let pages = count.div_ceil(TEXT_ROWS);
        if page >= pages {
            return 0;
        }
        let begin = page * TEXT_ROWS;
        packet(
            5,
            id as u32,
            page,
            pages,
            "Source information",
            &rows[begin..(begin + TEXT_ROWS).min(count)],
            out,
        )
    }
    /// Resolve the first image slot visible in a reading viewport.
    pub fn image_url(&self, id: usize, offset: usize, out: &mut [u8]) -> usize {
        if id == 0 || id > self.count() {
            return 0;
        }
        let doc = &self.documents[id - 1];
        let content = crate::flow::Content {
            title: doc.title.as_str(),
            body: doc.body.as_str(),
            runs: &doc.runs[..doc.run_count as usize],
            tree: &doc.tree,
            marker: 0,
        };
        let Some(image) = crate::flow::visible_image(content, offset) else {
            return 0;
        };
        crate::css::resolve(doc.url.as_str(), doc.tree.resource(image), out)
    }
    /// Semantic 160px viewport; source images retain their slot/alt/source only.
    pub fn flow_packet_reference(&self, id: usize, offset: usize, out: &mut [u8]) -> usize {
        self.flow_packet_impl(id, offset, out, false)
    }
    pub fn flow_packet(&self, id: usize, offset: usize, out: &mut [u8]) -> usize {
        self.flow_packet_impl(id, offset, out, true)
    }
    fn flow_packet_impl(&self, id: usize, offset: usize, out: &mut [u8], cached: bool) -> usize {
        if id == 0 || id > self.count() {
            return 0;
        }
        let doc = &self.documents[id - 1];
        crate::flow::encode_cached(
            id,
            doc.content(),
            offset,
            if cached {
                doc.flow_positions as usize
            } else {
                crate::flow::positions(doc.content())
            },
            out,
            |link| {
                let target = doc.tree.resource(link);
                let mut resolved = [0; crate::css::URL];
                let n = crate::css::resolve(doc.url.as_str(), target, &mut resolved);
                self.documents[..self.count()]
                    .iter()
                    .position(|d| d.url.as_str().as_bytes() == &resolved[..n] && n > 0)
                    .map_or(0, |i| i as u32 + 1)
            },
        )
    }
    /// One styled viewport. Its position is a visual line, with both pixel and
    /// packet budgets determining the last valid viewport.
    pub fn reader_packet(&self, id: usize, offset: usize, out: &mut [u8]) -> usize {
        if id == 0 || id > self.count() {
            return 0;
        }
        let doc = &self.documents[id - 1];
        crate::rich::encode(
            id,
            doc.title.as_str(),
            doc.body.as_str(),
            &doc.runs[..usize::from(doc.run_count)],
            u8::from(doc.limited != 0) | (u8::from(doc.style_limited != 0) << 1),
            offset,
            out,
        )
    }
}

fn matches_ignore_case(value: &str, values: &[&str]) -> bool {
    values.iter().any(|s| value.eq_ignore_ascii_case(s))
}

#[cfg(test)]
mod flow_tests {
    use super::*;
    use crate::{box_css, flow};
    extern crate std;
    use std::{boxed::Box, string::String};
    const HTML: &[u8] = include_bytes!("../../../examples/fixtures/flow.html");
    #[test]
    fn native_lab_fixture_retains_complete_mixed_text_and_image_within_budgets() {
        let html = include_bytes!("../../../examples/fixtures/native-lab.html");
        for chunk in [1, 17, 512, html.len()] {
            let mut state = Box::new(LocalSearch::new());
            state.seed();
            assert!(state.next_url(&mut [0; 128]) > 0);
            for part in html.chunks(chunk) {
                state.feed(part);
            }
            assert!(state.finish(true));
            let doc = &state.documents[0];
            assert_eq!(
                (doc.limited, doc.style_limited, doc.tree.limited),
                (0, 0, 0)
            );
            assert!(doc.body.as_str().contains("日本語とEnglishを読む。"));
            assert!(doc.body.as_str().contains("End of document."));
            assert!(doc.tree.nodes[..doc.tree.count as usize]
                .iter()
                .enumerate()
                .any(|(i, node)| node.kind == flow::IMAGE && doc.tree.resource(i) == "bars.jpg"));
            let mut bytes = [0; 512];
            let mut tail = false;
            for position in 0..doc.flow_positions {
                let n = state.flow_packet(1, position as usize, &mut bytes);
                let packet = flow::Packet::decode(&bytes[..n]).unwrap();
                assert_eq!(packet.positions, doc.flow_positions);
                tail |= packet.rows[..packet.row_count]
                    .iter()
                    .any(|r| r.text.contains("End of document."));
            }
            assert!(tail);
            assert_eq!(
                state.flow_packet(1, doc.flow_positions as usize, &mut bytes),
                0
            );
        }
    }
    #[test]
    fn closing_nested_same_kind_keeps_outer_ranges_and_paint() {
        let html=b"<div style='color:red'><div style='color:blue'>inner</div>outer tail<ul><li>first<ul><li>inner item</li></ul>outer item tail</li><li>second item</li></ul></div>";
        for split in 0..=html.len() {
            let mut state = Box::new(LocalSearch::new());
            state.seed();
            assert!(state.next_url(&mut [0; 128]) > 0);
            state.feed(&html[..split]);
            state.feed(&html[split..]);
            assert!(state.finish(true));
            let doc = &state.documents[0];
            let at = doc.body.as_str().find("outer tail").unwrap();
            let outer = &doc.tree.nodes[0];
            assert!(outer.end as usize > at);
            assert_eq!(outer.style.color, box_css::color("red").unwrap());
            let nested = &doc.tree.nodes[1];
            assert!((nested.end as usize) <= at);
            let at = doc.body.as_str().find("outer item tail").unwrap();
            let parent = doc.tree.nodes[..doc.tree.count as usize]
                .iter()
                .find(|n| {
                    n.kind == flow::ITEM
                        && doc.body.as_str()[n.start as usize..n.end as usize].contains("first")
                })
                .unwrap();
            assert!(parent.end as usize > at);
            let mut packet = [0; 512];
            let n = state.flow_packet(1, 0, &mut packet);
            flow::Packet::decode(&packet[..n]).unwrap();
        }
    }
    fn collect(split: usize) -> Box<LocalSearch> {
        let mut state = Box::new(LocalSearch::new());
        state.seed();
        let mut url = [0; 128];
        assert!(state.next_url(&mut url) > 0);
        state.feed(&HTML[..split]);
        state.feed(&HTML[split..]);
        assert!(state.finish(true));
        state
    }
    #[test]
    fn semantic_tree_and_css_survive_every_stream_boundary() {
        let baseline = collect(0);
        let mut expected = [0; 512];
        let n = baseline.flow_packet(1, 0, &mut expected);
        flow::Packet::decode(&expected[..n]).unwrap();
        let doc = &baseline.documents[0];
        assert!(doc.body.as_str().contains("1. Keep the structure."));
        assert!(doc.body.as_str().contains("2. Reflow to 160 pixels."));
        assert!(doc.body.as_str().contains("- Read by scrolling."));
        let link = doc.tree.nodes[..doc.tree.count as usize]
            .iter()
            .position(|n| n.kind == flow::LINK)
            .unwrap();
        assert_eq!(doc.tree.resource(link), "node2.html");
        assert_eq!(
            doc.tree.nodes[link].style.color,
            box_css::color("#1a73e8").unwrap_or(0x1b3d)
        );
        let image = doc.tree.nodes[..doc.tree.count as usize]
            .iter()
            .position(|n| n.kind == flow::IMAGE)
            .unwrap();
        assert_eq!(doc.tree.resource(image), "article.jpg");
        assert!(
            doc.body.as_str()
                [doc.tree.nodes[image].start as usize..doc.tree.nodes[image].end as usize]
                .contains("Source illustration")
        );
        let item = doc.tree.nodes[..doc.tree.count as usize]
            .iter()
            .find(|n| n.kind == flow::ITEM)
            .unwrap();
        assert_eq!(doc.tree.nodes[item.parent as usize].kind, 26);
        for split in 0..=HTML.len() {
            let state = collect(split);
            let mut actual = [0; 512];
            let len = state.flow_packet(1, 0, &mut actual);
            assert_eq!(len, n, "split {split}");
            assert_eq!(actual[..len], expected[..n], "split {split}");
        }
    }
    #[test]
    fn all_viewports_fit_and_preserve_tail_links_images_and_color_boxes() {
        let mut state = collect(0);
        let mut url = [0; 128];
        assert!(state.next_url(&mut url) > 0);
        state.feed(b"<title>Linked source</title><p>Actual linked content</p>");
        assert!(state.finish(true));
        let mut bytes = [0; 512];
        let n = state.flow_packet(1, 0, &mut bytes);
        let positions = flow::Packet::decode(&bytes[..n]).unwrap().positions;
        let (mut link, mut image, mut border, mut red, mut tail) =
            (false, false, false, false, false);
        for position in 0..positions {
            let n = state.flow_packet(1, position as usize, &mut bytes);
            assert!(n > 0 && n <= 512);
            let packet = flow::Packet::decode(&bytes[..n]).unwrap();
            link |= packet.link_target == 2;
            for row in &packet.rows[..packet.row_count] {
                image |= row.flags & 4 != 0;
                border |= row.stroke > 0;
                tail |= row.text.contains("retained page.");
                for code in row.codes {
                    let p = &packet.palette[usize::from(code >> 4) * 5..];
                    red |= u16::from_le_bytes([p[0], p[1]])
                        == box_css::color("rgb(120,40,70)").unwrap();
                }
            }
        }
        assert!(link && image && border && red && tail);
        let mut reference = [0; 512];
        for p in 0..positions {
            let n = state.flow_packet(1, p as usize, &mut bytes);
            let m = state.flow_packet_reference(1, p as usize, &mut reference);
            assert_eq!(n, m);
            assert_eq!(&bytes[..n], &reference[..m]);
        }
        assert_eq!(state.flow_packet(1, positions as usize, &mut bytes), 0);
        let n = state.flow_packet(1, positions as usize - 1, &mut bytes);
        let last = flow::Packet::decode(&bytes[..n]).unwrap();
        assert!(
            last.rows[..last.row_count]
                .iter()
                .any(|r| r.text.contains("retained page."))
        );
    }
    #[test]
    fn structural_overflow_keeps_searchable_text_and_a_visible_limit() {
        let mut state = Box::new(LocalSearch::new());
        state.seed();
        assert!(state.next_url(&mut [0; 128]) > 0);
        for _ in 0..80 {
            state.feed(b"<p><span style='color:red'>AI structure</span></p>");
        }
        state.feed(b"<p>Tail sentinel survives.</p>");
        assert!(state.finish(true));
        assert_eq!(state.documents[0].tree.limited, 1);
        let mut bytes = [0; 512];
        let n = state.flow_packet(1, 0, &mut bytes);
        let positions = flow::Packet::decode(&bytes[..n]).unwrap().positions;
        let mut seen = String::new();
        for p in 0..positions {
            let n = state.flow_packet(1, p as usize, &mut bytes);
            let v = flow::Packet::decode(&bytes[..n]).unwrap();
            for row in &v.rows[..v.row_count] {
                seen.push_str(row.text);
            }
        }
        assert!(seen.contains("Tail sentinel survives."));
        assert!(seen.contains("[Structure limit reached]"));
        assert!(core::mem::size_of::<LocalSearch>() <= STATE_BUDGET);
    }
    #[test]
    fn packet_truncations_mutations_and_utf8_never_panic() {
        let state = collect(0);
        let mut bytes = [0; 512];
        let n = state.flow_packet(1, 0, &mut bytes);
        for length in 0..n {
            assert!(flow::Packet::decode(&bytes[..length]).is_err());
        }
        for at in 0..n {
            for b in [0, 31, 127, 255] {
                let mut altered = bytes;
                altered[at] = b;
                let _ = flow::Packet::decode(&altered[..n]);
            }
        }
        let mut state = Box::new(LocalSearch::new());
        state.seed();
        assert!(state.next_url(&mut [0; 128]) > 0);
        state.feed("<h1>日本語</h1><p style='padding:999px;border:999px solid red'>日本語の本文🙂が続く。</p>".as_bytes());
        assert!(state.finish(true));
        let n = state.flow_packet(1, 0, &mut bytes);
        flow::Packet::decode(&bytes[..n]).unwrap();
    }
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

pub fn allowed_profile_url(profile: u8, url: &str) -> bool {
    let Some(profile_data) = PROFILES.get(profile as usize) else {
        return false;
    };
    url.strip_prefix(profile_data.1).is_some_and(|path| {
        if NUMBERED_SCOPE[profile as usize] {
            return allowed_relative(path);
        }
        !path.starts_with('/')
            && !path.contains("..")
            && !path.contains("//")
            && path
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"/-_.".contains(&b))
            && (path.is_empty() || path.ends_with('/') || path.ends_with(".html"))
    })
}

#[cfg(test)]
mod profile_tests {
    extern crate std;
    use super::*;

    #[test]
    fn embedded_styles_survive_all_chunks_without_leaking_to_next_page() {
        let html=b"<head><style>/* note */ h1{font-size:16px;color:red} .card{background:#00ff00;padding:2px;border:1px solid blue}</style></head><h1>Heading</h1><p class='card'>Body words</p><a href='node2.html'>Next</a>";
        for split in 1..=html.len() {
            let mut s = LocalSearch::new();
            s.seed();
            assert!(s.next_url(&mut [0; 128]) > 0);
            for chunk in html.chunks(split) {
                s.feed(chunk);
            }
            assert!(s.finish(true));
            assert!(
                s.documents[0]
                    .runs
                    .iter()
                    .any(|r| r.style & 3 == crate::rich::HEADING)
            );
            let p = s.documents[0]
                .tree
                .nodes
                .iter()
                .find(|n| n.kind == 17)
                .unwrap();
            assert_eq!(p.style.background, 0x07e0);
            assert_eq!(p.style.padding, [2; 4]);
            assert!(s.next_url(&mut [0; 128]) > 0);
            s.feed(b"<p class='card'>No leaked rule</p>");
            assert!(s.finish(true));
            let p = s.documents[1]
                .tree
                .nodes
                .iter()
                .find(|n| n.kind == 17)
                .unwrap();
            assert_eq!(p.style.flags & crate::box_css::BACKGROUND, 0);
        }
    }
    #[test]
    fn nested_oversized_and_print_styles_do_not_apply_inner_rules() {
        let mut s = LocalSearch::new();
        s.seed();
        s.next_url(&mut [0; 128]);
        s.feed(b"<head><style media='print'>p{color:red}</style><style>@media screen{p{color:blue}}</style></head><p>Readable</p>");
        assert!(s.finish(true));
        assert_eq!(
            s.documents[0].tree.nodes[0].style.color,
            crate::box_css::Style::DEFAULT.color
        );
        assert!(core::mem::size_of::<LocalSearch>() <= STATE_BUDGET);
    }
    #[test]
    fn provenance_keeps_complete_url_date_limits_and_valid_packet_pages() {
        let mut state = LocalSearch::new();
        state.seed();
        state.next_url(&mut [0; 128]);
        state.feed(b"<title>Original metadata test</title><p>AI body</p>");
        assert!(!state.metadata(1, 0, 200));
        assert!(!state.metadata(65537, 1791725950, 200));
        assert!(!state.metadata(123, 1791725950, 404));
        assert!(state.metadata(1234, 1791725950, 200));
        assert!(state.finish(true));
        let mut bytes = [0; 512];
        let n = state.info_packet(1, 0, &mut bytes);
        let view = crate::Packet::decode(&bytes[..n]).unwrap();
        let mut joined = std::string::String::new();
        for page in 0..view.page_count {
            let n = state.info_packet(1, page as usize, &mut bytes);
            let view = crate::Packet::decode(&bytes[..n]).unwrap();
            assert_eq!(view.kind as u8, 5);
            for line in view.lines.iter().take(view.line_count) {
                joined.push_str(line);
            }
        }
        assert!(joined.contains(SEED));
        assert!(joined.contains("2026-10-11"));
        assert!(joined.contains("1234 B"));
        assert_eq!(state.info_packet(0, 0, &mut bytes), 0);
        assert_eq!(state.info_packet(1, 99, &mut bytes), 0);
        assert_eq!(utc(951782400), (2000, 2, 29, 0, 0, 0));
        assert_eq!(utc(4107542400), (2100, 3, 1, 0, 0, 0));
    }

    #[test]
    fn image_sources_match_only_visible_slots_and_keep_full_resolved_urls() {
        let mut state = LocalSearch::new();
        assert!(state.set_profile(1));
        state.next_url(&mut [0; 128]);
        state.feed(b"<title>Web images</title><p>Introductory text</p><img src='/Press/Stock/original.jpg' alt='Original test' height='40'><p>Article after the picture.</p>");
        assert!(state.finish(true));
        let mut packet = [0; 512];
        let n = state.flow_packet(1, 0, &mut packet);
        let positions = crate::flow::Packet::decode(&packet[..n]).unwrap().positions;
        let mut visible = 0;
        for position in 0..positions {
            let n = state.flow_packet(1, position as usize, &mut packet);
            let view = crate::flow::Packet::decode(&packet[..n]).unwrap();
            let slot = view.rows[..view.row_count].iter().any(|r| r.flags & 4 != 0);
            let mut url = [0; 256];
            let n = state.image_url(1, position as usize, &mut url);
            assert_eq!(n > 0, slot);
            if slot {
                assert_eq!(&url[..n], b"https://www.w3.org/Press/Stock/original.jpg");
                visible += 1;
            }
        }
        assert!(visible > 0);
        assert_eq!(state.image_url(0, 0, &mut [0; 256]), 0);
        assert_eq!(state.image_url(1, 65535, &mut [0; 256]), 0);
    }

    #[test]
    fn profile_selection_is_explicit_invalid_choices_preserve_content() {
        let mut state = LocalSearch::new();
        state.seed();
        assert!(state.next_url(&mut [0; 128]) > 0);
        state.feed(b"<title>AI</title><p>Original retained article</p>");
        assert!(state.finish(true));
        assert!(!state.set_profile(255));
        assert_eq!(state.count(), 1);
        assert!(state.set_profile(1));
        assert_eq!(state.count(), 0);
        let mut url = [0; 128];
        let n = state.next_url(&mut url);
        assert_eq!(&url[..n], PROFILES[1].2.as_bytes());
    }

    #[test]
    fn w3c_frontier_keeps_only_bounded_admitted_same_profile_links() {
        let mut state = LocalSearch::new();
        assert!(state.set_profile(1));
        assert!(state.next_url(&mut [0; 128]) > 0);
        state.feed(b"<title>Web</title><p>World Wide Web</p><a href='Biography.html'>Biography</a><a href='/People/Berners-Lee/FAQ.html'>FAQ</a><a href='https://www.w3.org@localhost/People/Berners-Lee/x.html'>bad</a><a href='//localhost/x.html'>bad</a><a href='../outside.html'>bad</a><a href='%2e%2e/escape.html'>bad</a><a href='https://other.example/page.html'>bad</a>");
        assert!(state.finish(true));
        assert_eq!(state.queued, 3);
        assert!(
            state.queue[..3]
                .iter()
                .all(|u| allowed_profile_url(1, u.as_str()))
        );
        assert_eq!(
            state.queue[1].as_str(),
            "https://www.w3.org/People/Berners-Lee/Biography.html"
        );
    }

    #[test]
    fn source_scope_rejects_private_authorities_ports_and_escaped_paths() {
        for url in [
            "http://www.w3.org/People/Berners-Lee/",
            "https://www.w3.org:443/People/Berners-Lee/",
            "https://www.w3.org@127.0.0.1/People/Berners-Lee/",
            "https://www.w3.org/People/Berners-Lee/../outside.html",
            "https://www.w3.org/People/Berners-Lee/%2e%2e/a.html",
            "https://www.w3.org/People/Berners-Lee/x.html?query=1",
            "https://www.w3.org/People/Berners-Lee//x.html",
        ] {
            assert!(!allowed_profile_url(1, url), "{url}");
        }
        assert!(!allowed_profile_url(1, SEED));
        assert!(!allowed_profile_url(0, PROFILES[1].2));
    }
}
pub(crate) fn attribute<'a>(text: &'a str, key: &str) -> Option<&'a str> {
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
        if c == '\n' {
            return (&text[..at], text[at..].trim_start());
        }
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
pub(crate) fn short_title(text: &str) -> &str {
    take_line(text).0
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
    if body.len() > 4096 || body.contains('%') {
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
            assert_eq!(index.documents[0].body.as_str(), "Real 日本 AI 語\n\nNext");
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
        assert_eq!(
            crate::rich::Packet::decode(&out[..n]).unwrap().document_id,
            1
        );
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
        let n = index.reader_packet(1, 0, &mut out);
        let positions = usize::from(crate::rich::Packet::decode(&out[..n]).unwrap().positions);
        for p in 0..positions {
            let n = index.reader_packet(1, p, &mut out);
            let packet = crate::rich::Packet::decode(&out[..n]).unwrap();
            assert_eq!(packet.positions as usize, positions);
            if p + 1 == positions {
                assert!(packet.rows.iter().any(|r| r.text == "[Text limit reached]"));
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
        let first_view = crate::rich::Packet::decode(&first[..n]).unwrap();
        assert!(first_view.row_count >= 6);
        assert!(first_view.positions > 1);
        let positions = first_view.positions as usize;
        for offset in 1..positions {
            let a = index.reader_packet(1, offset - 1, &mut first);
            let b = index.reader_packet(1, offset, &mut next);
            let previous = crate::rich::Packet::decode(&first[..a]).unwrap();
            let current = crate::rich::Packet::decode(&next[..b]).unwrap();
            for i in 1..previous.row_count {
                assert_eq!(previous.rows[i].text, current.rows[i - 1].text);
            }
            assert_eq!(current.position as usize, offset);
            if offset + 1 == positions {
                assert!(current.rows[current.row_count - 1].text.contains("here."));
            }
        }
        assert_eq!(index.reader_packet(1, positions, &mut next), 0);
    }
    #[test]
    fn streaming_typography_preserves_nested_styles_and_paragraphs_at_every_split() {
        let html=b"<h1>Heading title</h1><p>Normal <strong>bold <small>note</small></strong> tail.</p><p style='font-size:10px;font-weight:700'>Caption.</p><h2>Subhead</h2><p>Final paragraph.</p>";
        for split in 1..=html.len() {
            let mut index = LocalSearch::new();
            index.seed();
            collect(&mut index, html, split);
            let doc = &index.documents[0];
            assert!(doc.body.as_str().contains("\n\n"));
            assert_eq!(doc.style_limited, 0);
            let style_at = |word: &str| {
                let at = doc.body.as_str().find(word).unwrap();
                doc.runs[..usize::from(doc.run_count)]
                    .iter()
                    .rev()
                    .find(|r| usize::from(r.start) <= at)
                    .map_or(0, |r| r.style)
            };
            assert_eq!(
                style_at("Heading"),
                crate::rich::HEADING | crate::rich::BOLD
            );
            assert_eq!(style_at("bold"), crate::rich::BOLD);
            assert_eq!(style_at("note"), crate::rich::CAPTION | crate::rich::BOLD);
            assert_eq!(style_at("tail"), 0);
            assert_eq!(
                style_at("Caption"),
                crate::rich::CAPTION | crate::rich::BOLD
            );
            assert_eq!(
                style_at("Subhead"),
                crate::rich::SUBHEADING | crate::rich::BOLD
            );
            assert_eq!(style_at("Final"), 0);
            assert_eq!(index.ranking("bold").1, 1);
            let mut out = [0; 512];
            let n = index.reader_packet(1, 0, &mut out);
            let first = crate::rich::Packet::decode(&out[..n]).unwrap();
            assert_eq!(
                first.rows[0].style(0),
                crate::rich::HEADING | crate::rich::BOLD
            );
            assert_eq!(first.rows[0].height, 18);
            assert!(
                first.rows[..first.row_count]
                    .iter()
                    .any(|r| r.text.is_empty() && r.height == 6)
            );
            for position in 0..first.positions {
                let n = index.reader_packet(1, usize::from(position), &mut out);
                crate::rich::Packet::decode(&out[..n]).unwrap();
            }
        }
    }
    #[test]
    fn typography_limits_keep_all_text_readable_and_mark_the_fallback() {
        let mut index = LocalSearch::new();
        index.seed();
        index.next_url(&mut [0; 128]);
        for _ in 0..60 {
            index.feed(b"<b>x</b>y ");
        }
        assert!(index.finish(true));
        let doc = &index.documents[0];
        assert_eq!(doc.style_limited, 1);
        assert!(usize::from(doc.run_count) <= STYLE_RUNS);
        assert_eq!(doc.body.as_str().matches("xy").count(), 60);
        assert_eq!(index.ranking("xy").1, 1);
        let mut out = [0; 512];
        let n = index.reader_packet(1, 0, &mut out);
        let positions = crate::rich::Packet::decode(&out[..n]).unwrap().positions;
        let n = index.reader_packet(1, usize::from(positions - 1), &mut out);
        assert!(
            crate::rich::Packet::decode(&out[..n])
                .unwrap()
                .rows
                .iter()
                .any(|r| r.text == "[Style limit reached]")
        );
        let mut deep = LocalSearch::new();
        deep.seed();
        deep.next_url(&mut [0; 128]);
        for _ in 0..17 {
            deep.feed(b"<b>");
        }
        deep.feed(b"safe");
        for _ in 0..17 {
            deep.feed(b"</b>");
        }
        assert!(deep.finish(true));
        assert_eq!(deep.documents[0].style_limited, 1);
        assert_eq!(deep.ranking("safe").1, 1);
    }
    #[test]
    fn implicit_paragraph_closing_restores_normal_style_without_stack_growth() {
        let mut index = LocalSearch::new();
        index.seed();
        index.next_url(&mut [0; 128]);
        index.feed(b"<p style='font-weight:bold'>first<p>second");
        for _ in 0..40 {
            index.feed(b"<p>next");
        }
        assert!(index.finish(true));
        let doc = &index.documents[0];
        assert_eq!(doc.style_limited, 0);
        assert!(doc.body.as_str().starts_with("first\n\nsecond"));
        let at = doc.body.as_str().find("second").unwrap();
        assert_eq!(
            doc.runs[..usize::from(doc.run_count)]
                .iter()
                .rev()
                .find(|r| usize::from(r.start) <= at)
                .unwrap()
                .style,
            0
        );
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
        assert!(!robots_allowed(&[b' '; 4097], "/"));
        assert!(robots_allowed(
            b"User-agent: *\nDisallow: /jmc/\nAllow: /jmc/",
            "/jmc/"
        ));
    }
}

#[cfg(test)]
mod css_pipeline_tests {
    use super::*;
    #[test]
    fn every_html_chunk_boundary_discovers_css_reparses_and_keeps_font_inheritance() {
        let html=b"<head><link rel='stylesheet' href='/site.css'><title>AI</title></head><body><h1>Heading</h1><p class='note'>AI body <strong>bold</strong>.</p></body>";
        let css=b"body{font-family:Test}.note{font-size:10px}h1{font-size:14px;font-weight:normal}@font-face{font-family:Test;src:url(font.ttf)}";
        for chunk in 1..=html.len() {
            let mut index = LocalSearch::new();
            index.seed();
            let mut url = [0; 128];
            assert!(index.next_url(&mut url) > 0);
            for part in html.chunks(chunk) {
                index.feed(part);
            }
            assert_eq!(
                index.resource(false),
                "https://www-formal.stanford.edu/site.css"
            );
            index.stylesheet(css, "https://www-formal.stanford.edu/site.css");
            index.font_ready();
            index.reparse();
            for part in html.chunks(chunk) {
                index.feed(part);
            }
            assert!(index.finish(true));
            let mut packet = [0; 512];
            let n = index.reader_packet(1, 0, &mut packet);
            assert_eq!(packet[4], 2);
            let decoded = crate::rich::Packet::decode(&packet[..n]).unwrap();
            assert_eq!(
                decoded.rows[0].style(0),
                crate::rich::SUBHEADING | crate::rich::WEBFONT
            );
            assert!(decoded.rows.iter().any(|r| r.text.contains("AI body")
                && r.style(0) == crate::rich::CAPTION | crate::rich::WEBFONT));
            assert!(decoded.rows.iter().any(|r| {
                r.text.chars().enumerate().any(|(i, _)| {
                    r.style(i) == crate::rich::CAPTION | crate::rich::WEBFONT | crate::rich::BOLD
                })
            }));
            assert_eq!(index.count(), 1);
        }
    }
    #[test]
    fn failed_second_fetch_restores_only_the_complete_original_document() {
        let mut index = LocalSearch::new();
        index.seed();
        let mut url = [0; 128];
        index.next_url(&mut url);
        index.feed(b"<p>AI complete original</p>");
        let mut saved = SavedDocument(Document::EMPTY);
        assert!(index.save_active(&mut saved));
        index.stylesheet(b"p{font-size:16px}", BASE);
        index.reparse();
        index.feed(b"<p>partial replaced");
        index.restore_active(&saved);
        assert!(index.finish(true));
        assert_eq!(index.documents[0].body.as_str(), "AI complete original");
        assert!(!index.save_active(&mut saved));
        assert!(core::mem::size_of::<SavedDocument>() <= 4608);
    }
}
