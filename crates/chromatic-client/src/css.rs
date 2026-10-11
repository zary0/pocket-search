// SPDX-License-Identifier: GPL-3.0-only
//! Deliberately small typography cascade. No DOM, allocation, imports or URLs fetched here.
use crate::rich::{BOLD, CAPTION, HEADING, SUBHEADING, WEBFONT};

pub const RULES: usize = 8;
pub const URL: usize = 256;
pub const BYTES: usize = 3072;
#[derive(Clone, Copy)]
struct Rule {
    selector: [u8; 24],
    len: u8,
    values: [u8; 3],
    mask: u8,
    important: u8,
}
impl Rule {
    const EMPTY: Self = Self {
        selector: [0; 24],
        len: 0,
        values: [0; 3],
        mask: 0,
        important: 0,
    };
}
/// Integer-only fields allow the native opaque state to be zero-initialized.
pub struct Sheet {
    rules: [Rule; RULES],
    count: u8,
    pub limited: u8,
    href: [u8; URL],
    href_len: u16,
    font: [u8; URL],
    font_len: u16,
    family: [u8; 24],
    family_len: u8,
    pub font_ready: u8,
    enabled: u8,
    cached: u8,
    loaded_href_len: u16,
}
impl Sheet {
    pub const EMPTY: Self = Self {
        rules: [Rule::EMPTY; RULES],
        count: 0,
        limited: 0,
        href: [0; URL],
        href_len: 0,
        font: [0; URL],
        font_len: 0,
        family: [0; 24],
        family_len: 0,
        font_ready: 0,
        enabled: 0,
        cached: 0,
        loaded_href_len: 0,
    };
    pub fn href(&self) -> &str {
        core::str::from_utf8(&self.href[..self.href_len as usize]).unwrap_or("")
    }
    pub fn font_url(&self) -> &str {
        core::str::from_utf8(&self.font[..self.font_len as usize]).unwrap_or("")
    }
    pub fn begin_page(&mut self) {
        self.enabled = 0;
        self.cached = 0;
        self.href_len = 0;
    }
    pub fn enable(&mut self) {
        self.enabled = 1;
    }
    pub fn cached(&self) -> bool {
        self.cached != 0
    }
    pub fn discover(&mut self, base: &str, href: &str) {
        if self.href_len == 0 {
            let mut resolved = [0; URL];
            let n = resolve(base, href, &mut resolved);
            self.cached = u8::from(
                n > 0 && n == self.loaded_href_len as usize && resolved[..n] == self.href[..n],
            );
            if self.cached == 0 {
                self.loaded_href_len = 0;
            }
            self.href = resolved;
            self.href_len = n as u16;
        } else {
            self.limited = 1;
        }
    }
    fn family(&self) -> &str {
        core::str::from_utf8(&self.family[..self.family_len as usize]).unwrap_or("")
    }
    pub fn parse(&mut self, bytes: &[u8], base: &str) {
        self.enabled = 0;
        self.cached = 0;
        self.loaded_href_len = 0;
        self.count = 0;
        self.font_len = 0;
        self.family_len = 0;
        self.font_ready = 0;
        if bytes.len() > BYTES {
            self.limited = 1;
            return;
        }
        let Ok(css) = core::str::from_utf8(bytes) else {
            self.limited = 1;
            return;
        };
        self.href_len = resolve(base, base, &mut self.href) as u16;
        self.loaded_href_len = self.href_len;
        self.enabled = 1;
        self.cached = 1;
        // Find the face first, so family rules work before and after @font-face.
        for pass in 0..2 {
            let mut rest = css;
            while !rest.trim().is_empty() {
                rest = rest.trim_start();
                if rest.starts_with("/*") {
                    let Some(end) = rest.find("*/") else {
                        self.limited = 1;
                        break;
                    };
                    rest = &rest[end + 2..];
                    continue;
                }
                let Some(open) = boundary(rest, b'{') else {
                    self.limited = 1;
                    break;
                };
                let selector = rest[..open].trim();
                rest = &rest[open + 1..];
                let Some(close) = block_end(rest) else {
                    self.limited = 1;
                    break;
                };
                let body = &rest[..close];
                rest = &rest[close + 1..];
                if selector.eq_ignore_ascii_case("@font-face") && pass == 1 {
                    continue;
                }
                if pass == 0 && !selector.eq_ignore_ascii_case("@font-face") {
                    continue;
                }
                if selector.eq_ignore_ascii_case("@font-face") {
                    if self.font_len != 0 {
                        self.limited = 1;
                        continue;
                    }
                    let mut source = "";
                    let mut family = "";
                    for d in body.split(';') {
                        if let Some((k, v)) = d.split_once(':') {
                            if k.trim().eq_ignore_ascii_case("font-family") {
                                family = unquote(v.trim());
                            }
                            if k.trim().eq_ignore_ascii_case("src") {
                                source = v.trim();
                            }
                        }
                    }
                    // Only a single url(...), no local(), multiple sources, escapes, or formats other than truetype.
                    if family.is_empty()
                        || family.len() > 24
                        || !family.is_ascii()
                        || !source.starts_with("url(")
                    {
                        self.limited = 1;
                        continue;
                    }
                    let Some(end) = source.find(')') else {
                        self.limited = 1;
                        continue;
                    };
                    let tail = source[end + 1..].trim();
                    if !tail.is_empty()
                        && tail != "format('truetype')"
                        && tail != "format(\"truetype\")"
                    {
                        self.limited = 1;
                        continue;
                    }
                    self.font_len =
                        resolve(base, unquote(source[4..end].trim()), &mut self.font) as u16;
                    if self.font_len == 0 {
                        self.limited = 1;
                        continue;
                    }
                    self.family[..family.len()].copy_from_slice(family.as_bytes());
                    self.family_len = family.len() as u8;
                    continue;
                }
                if selector.starts_with('@') || body.contains('{') || body.contains("/*") {
                    self.limited = 1;
                    continue;
                }
                for sel in selector.split(',') {
                    let sel = sel.trim();
                    if self.count as usize == RULES || !simple(sel) {
                        self.limited = 1;
                        continue;
                    }
                    let mut rule = Rule::EMPTY;
                    rule.selector[..sel.len()].copy_from_slice(sel.as_bytes());
                    rule.len = sel.len() as u8;
                    declarations(body, self.family(), &mut rule);
                    if rule.mask == 0 {
                        continue;
                    }
                    self.rules[self.count as usize] = rule;
                    self.count += 1;
                }
            }
        }
    }
    pub fn apply(&self, name: &str, id: &str, classes: &str, inline: &str, mut style: u8) -> u8 {
        let mut values = [0; 3];
        let mut priorities = [0u16; 3];
        for rule in &self.rules[..if self.enabled != 0 {
            self.count as usize
        } else {
            0
        }] {
            let sel = core::str::from_utf8(&rule.selector[..rule.len as usize]).unwrap_or("");
            let score = if let Some(s) = sel.strip_prefix('#') {
                if s == id { 3 } else { 0 }
            } else if let Some(s) = sel.strip_prefix('.') {
                if classes.split_ascii_whitespace().any(|c| c == s) {
                    2
                } else {
                    0
                }
            } else if let Some((tag, class)) = sel.split_once('.') {
                if tag.eq_ignore_ascii_case(name)
                    && classes.split_ascii_whitespace().any(|c| c == class)
                {
                    202
                } else {
                    0
                }
            } else if sel == "*" {
                1
            } else if sel.eq_ignore_ascii_case(name) {
                2
            } else {
                0
            };
            // Tags have lower specificity than classes, even when their spelling matches.
            let score = if sel.starts_with('.') {
                score * 100
            } else if sel.starts_with('#') {
                score * 1000
            } else {
                score
            };
            cascade(rule, score, &mut priorities, &mut values);
        }
        let mut rule = Rule::EMPTY;
        declarations(inline, self.family(), &mut rule);
        cascade(&rule, 4000, &mut priorities, &mut values);
        if priorities[0] > 0 {
            style = (style & !3) | values[0];
        }
        if priorities[1] > 0 {
            style = (style & !BOLD) | values[1];
        }
        if priorities[2] > 0 {
            style = (style & !WEBFONT)
                | if self.font_ready != 0 && self.enabled != 0 {
                    values[2]
                } else {
                    0
                };
        }
        style
    }
    pub fn count(&self) -> usize {
        self.count as usize
    }
}
fn cascade(rule: &Rule, score: u16, priority: &mut [u16; 3], values: &mut [u8; 3]) {
    if score == 0 {
        return;
    }
    for i in 0..3 {
        let p = score
            + if rule.important & (1 << i) != 0 {
                10000
            } else {
                0
            };
        if rule.mask & (1 << i) != 0 && p >= priority[i] {
            priority[i] = p;
            values[i] = rule.values[i];
        }
    }
}
fn declarations(body: &str, family: &str, rule: &mut Rule) {
    for d in body.split(';') {
        let Some((key, raw)) = d.split_once(':') else {
            continue;
        };
        let (value, important) = if let Some(v) = raw.trim().strip_suffix("!important") {
            (v.trim(), true)
        } else {
            (raw.trim(), false)
        };
        let property = if key.trim().eq_ignore_ascii_case("font-size") {
            0
        } else if key.trim().eq_ignore_ascii_case("font-weight") {
            1
        } else if key.trim().eq_ignore_ascii_case("font-family") {
            2
        } else {
            continue;
        };
        let v = match property {
            0 => {
                if value.eq_ignore_ascii_case("small")
                    || value.eq_ignore_ascii_case("smaller")
                    || value.eq_ignore_ascii_case("x-small")
                    || value.eq_ignore_ascii_case("xx-small")
                {
                    Some(CAPTION)
                } else if value.eq_ignore_ascii_case("large")
                    || value.eq_ignore_ascii_case("larger")
                    || value.eq_ignore_ascii_case("x-large")
                    || value.eq_ignore_ascii_case("xx-large")
                {
                    Some(HEADING)
                } else if value.eq_ignore_ascii_case("medium") {
                    Some(0)
                } else {
                    value
                        .strip_suffix("px")
                        .and_then(|v| v.trim().parse::<u16>().ok())
                        .map(|n| match n {
                            0..=11 => CAPTION,
                            12..=13 => 0,
                            14..=15 => SUBHEADING,
                            _ => HEADING,
                        })
                }
            }
            1 => {
                if value.eq_ignore_ascii_case("bold")
                    || value.eq_ignore_ascii_case("bolder")
                    || value.parse::<u16>().is_ok_and(|n| (600..=900).contains(&n))
                {
                    Some(BOLD)
                } else if value.eq_ignore_ascii_case("normal")
                    || value.parse::<u16>().is_ok_and(|n| (100..=500).contains(&n))
                {
                    Some(0)
                } else {
                    None
                }
            }
            _ => Some(
                if !family.is_empty()
                    && value
                        .split(',')
                        .any(|f| unquote(f.trim()).eq_ignore_ascii_case(family))
                {
                    WEBFONT
                } else {
                    0
                },
            ),
        };
        if let Some(v) = v {
            let bit = 1 << property;
            if important || rule.important & bit == 0 {
                rule.mask |= bit;
                rule.values[property] = v;
                if important {
                    rule.important |= bit;
                }
            }
        }
    }
}
fn unquote(v: &str) -> &str {
    if v.len() >= 2
        && ((v.starts_with('\'') && v.ends_with('\'')) || (v.starts_with('"') && v.ends_with('"')))
    {
        &v[1..v.len() - 1]
    } else {
        v
    }
}
fn identifier(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
fn simple(s: &str) -> bool {
    if s.len() > 24 {
        return false;
    }
    if s == "*" {
        return true;
    }
    if let Some(rest) = s.strip_prefix(['.', '#']) {
        return identifier(rest);
    }
    if let Some((tag, class)) = s.split_once('.') {
        return identifier(tag) && identifier(class);
    }
    identifier(s)
}
fn boundary(s: &str, target: u8) -> Option<usize> {
    let b = s.as_bytes();
    let mut at = 0;
    let mut quote = 0;
    while at < b.len() {
        if quote != 0 {
            if b[at] == b'\\' {
                at += 2;
                continue;
            }
            if b[at] == quote {
                quote = 0;
            }
        } else if b[at] == b'\'' || b[at] == b'"' {
            quote = b[at];
        } else if b.get(at..at + 2) == Some(b"/*") {
            let end = s[at + 2..].find("*/")?;
            at += end + 4;
            continue;
        } else if b[at] == target {
            return Some(at);
        }
        at += 1;
    }
    None
}
/// A small HTTPS resolver. Native transport additionally enforces exact public hosts/paths.
pub fn resolve(base: &str, href: &str, out: &mut [u8]) -> usize {
    if href.is_empty()
        || !href.is_ascii()
        || href
            .bytes()
            .any(|b| b <= 32 || matches!(b, b'\\' | b'#' | b'\'' | b'"'))
    {
        return 0;
    }
    let (prefix, tail) = if href.starts_with("https://") {
        ("", href)
    } else if href.starts_with('/') && !href.starts_with("//") {
        let Some(at) = base.strip_prefix("https://").and_then(|b| b.find('/')) else {
            return 0;
        };
        (&base[..at + 8], href)
    } else if !href.contains(':')
        && !href.starts_with("//")
        && !href.split('/').any(|p| p == ".." || p == ".")
    {
        let Some(at) = base.rfind('/') else {
            return 0;
        };
        (&base[..at + 1], href)
    } else {
        return 0;
    };
    let n = prefix.len() + tail.len();
    if n >= out.len() {
        return 0;
    }
    out[..prefix.len()].copy_from_slice(prefix.as_bytes());
    out[prefix.len()..n].copy_from_slice(tail.as_bytes());
    out[n] = 0;
    n
}

fn block_end(s: &str) -> Option<usize> {
    let mut rest = s;
    let mut at = 0;
    let mut depth = 1;
    while let Some(n) = next_brace(rest) {
        if rest.as_bytes()[n] == b'{' {
            depth += 1;
        } else {
            depth -= 1;
            if depth == 0 {
                return Some(at + n);
            }
        }
        at += n + 1;
        rest = &rest[n + 1..];
    }
    None
}
fn next_brace(s: &str) -> Option<usize> {
    match (boundary(s, b'{'), boundary(s, b'}')) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tag_class_keywords_and_cache_require_same_url_and_explicit_reparse() {
        let mut sheet = Sheet::EMPTY;
        sheet.parse(
            b".TINY{font-size:16px}SMALL.TINY{font-size:xx-small}BIG.LARGE{font-size:x-large}",
            "https://example.org/site.css",
        );
        assert_eq!(sheet.apply("small", "", "TINY", "", 0), CAPTION);
        assert_eq!(sheet.apply("big", "", "LARGE", "", 0), HEADING);
        sheet.begin_page();
        sheet.discover("https://example.org/new/page.html", "/site.css");
        assert!(sheet.cached());
        assert_eq!(sheet.apply("small", "", "TINY", "", 0), 0);
        sheet.enable();
        assert_eq!(sheet.apply("small", "", "TINY", "", 0), CAPTION);
        sheet.begin_page();
        sheet.discover("https://example.org/new/page.html", "/other.css");
        assert!(!sheet.cached());
        assert_eq!(sheet.apply("small", "", "TINY", "", 0), 0);
        sheet.begin_page();
        sheet.discover("https://example.org/new/page.html", "/other.css");
        assert!(!sheet.cached());
        sheet.parse(b"p{color:red}", "https://example.org/other.css");
        assert_eq!(sheet.count(), 0);
        let oversized = [b' '; BYTES + 1];
        sheet.parse(&oversized, "https://example.org/x");
        assert!(!sheet.cached());
        assert_eq!(sheet.count(), 0);
    }
    #[test]
    fn specificity_order_inheritance_important_and_face_after_its_users() {
        let mut sheet = Sheet::EMPTY;
        sheet.parse(b"p {font-size:16px;font-weight:bold}.note{font-size:10px} #note{font-size:14px} p{font-weight:normal!important;font-family:Test} @font-face{font-family:Test;src:url(font.ttf) format('truetype')}","https://example.org/style/site.css");
        assert_eq!(sheet.font_url(), "https://example.org/style/font.ttf");
        assert_eq!(
            sheet.apply("p", "note", "note", "font-size:12px;font-weight:bold", 0),
            0
        );
        sheet.font_ready = 1;
        assert_eq!(
            sheet.apply("p", "note", "note", "font-size:12px;font-weight:bold", 0),
            WEBFONT
        );
        assert_eq!(
            sheet.apply("p", "note", "note", "font-weight:bold!important", 0),
            SUBHEADING | BOLD | WEBFONT
        );
        assert_eq!(
            sheet.apply("span", "", "", "", CAPTION | WEBFONT),
            CAPTION | WEBFONT
        );
        assert_eq!(sheet.apply("p", "", "note", "", 0), CAPTION | WEBFONT);
    }
    #[test]
    fn unsupported_blocks_comments_and_limits_do_not_change_unrelated_rules() {
        let mut sheet = Sheet::EMPTY;
        sheet.parse(b"/* } { */ @media screen {p{font-size:16px}} p{font-size:10px} p:hover{font-weight:bold} a b{font-size:14px}","https://example.org/site.css");
        assert_eq!(sheet.count(), 1);
        assert_eq!(sheet.limited, 1);
        assert_eq!(sheet.apply("p", "", "", "", 0), CAPTION);
        let repeated=b"p{font-size:10px}p{font-size:10px}p{font-size:10px}p{font-size:10px}p{font-size:10px}p{font-size:10px}p{font-size:10px}p{font-size:10px}p{font-size:16px}";
        sheet.parse(repeated, "https://example.org/x.css");
        assert_eq!(sheet.count(), RULES);
        assert_eq!(sheet.apply("p", "", "", "", 0), CAPTION);
        for data in [
            b"p{font-size:16px".as_slice(),
            b"\xff",
            b"@font-face{font-family:X;src:local(X)}",
            b"@font-face{font-family:X;src:url(x.woff2) format('woff2')}",
        ] {
            sheet.parse(data, "https://example.org/x.css");
            assert!(sheet.limited != 0);
            assert!(sheet.font_url().is_empty());
        }
    }
    #[test]
    fn only_bounded_https_urls_resolve() {
        let mut out = [0; URL];
        assert_eq!(
            resolve("https://example.org/path/page.html", "../secret", &mut out),
            0
        );
        for bad in ["//evil/x", "http://evil/x", "data:x", "a#x", "a\\b", "a\nx"] {
            assert_eq!(resolve("https://example.org/p", bad, &mut out), 0);
        }
        let n = resolve("https://example.org/path/page.html", "/site.css", &mut out);
        assert_eq!(&out[..n], b"https://example.org/site.css");
    }
}
