// SPDX-License-Identifier: GPL-3.0-only
//! Fixed-capacity basic paint and box declarations. Pixel lengths only.
pub const PROPERTIES: usize = 13;
pub const BACKGROUND: u8 = 1;
pub const UNDERLINE: u8 = 2;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style {
    pub color: u16,
    pub background: u16,
    pub border: u16,
    pub margin: [u8; 4],
    pub padding: [u8; 4],
    pub stroke: u8,
    pub flags: u8,
}
impl Style {
    pub const DEFAULT: Self = Self {
        color: 0x2104,
        background: 0xffff,
        border: 0x2104,
        margin: [0; 4],
        padding: [0; 4],
        stroke: 0,
        flags: 0,
    };
    pub fn inherit(parent: Self) -> Self {
        Self {
            color: parent.color,
            flags: parent.flags & UNDERLINE,
            ..Self::DEFAULT
        }
    }
    pub fn set(&mut self, i: usize, value: u16) {
        match i {
            0 => self.color = value,
            1 => {
                self.background = value;
                self.flags |= BACKGROUND;
            }
            2 => self.border = value,
            3 => self.stroke = value as u8,
            4..=7 => self.margin[i - 4] = value as u8,
            8..=11 => self.padding[i - 8] = value as u8,
            12 => self.flags = (self.flags & !UNDERLINE) | value as u8,
            _ => {}
        }
    }
}
#[derive(Clone, Copy)]
pub(crate) struct Declarations {
    pub values: [u16; PROPERTIES],
    pub mask: u16,
    pub important: u16,
}
impl Declarations {
    pub const EMPTY: Self = Self {
        values: [0; PROPERTIES],
        mask: 0,
        important: 0,
    };
    fn put(&mut self, i: usize, v: u16, important: bool) {
        let bit = 1 << i;
        if important || self.important & bit == 0 {
            self.values[i] = v;
            self.mask |= bit;
            if important {
                self.important |= bit;
            }
        }
    }
    pub fn parse(&mut self, body: &str) {
        // As with the typography parser, unsupported comment grammar is skipped.
        if body.contains("/*") {
            return;
        }
        for declaration in body.split(';') {
            let Some((key, raw)) = declaration.split_once(':') else {
                continue;
            };
            let raw = raw.trim();
            let suffix = raw.len().saturating_sub(10);
            let important = raw
                .get(suffix..)
                .is_some_and(|s| s.eq_ignore_ascii_case("!important"));
            let value = if important { raw[..suffix].trim() } else { raw };
            let key = key.trim();
            for (i, name) in ["color", "background-color", "border-color"]
                .iter()
                .enumerate()
            {
                if key.eq_ignore_ascii_case(name)
                    || (i == 1 && key.eq_ignore_ascii_case("background"))
                {
                    if let Some(v) = color(value) {
                        self.put(i, v, important);
                    }
                }
            }
            if key.eq_ignore_ascii_case("border") {
                let mut parts = value.split_ascii_whitespace();
                if let (Some(w), Some(kind), Some(c), None) =
                    (parts.next(), parts.next(), parts.next(), parts.next())
                {
                    if kind.eq_ignore_ascii_case("solid") {
                        if let (Some(w), Some(c)) = (pixels(w, 3), color(c)) {
                            self.put(3, u16::from(w), important);
                            self.put(2, c, important);
                        }
                    }
                } else if value.eq_ignore_ascii_case("none") {
                    self.put(3, 0, important);
                }
            }
            if key.eq_ignore_ascii_case("border-width") {
                if let Some(v) = pixels(value, 3) {
                    self.put(3, u16::from(v), important);
                }
            }
            if key.eq_ignore_ascii_case("text-decoration") {
                if value.eq_ignore_ascii_case("underline") {
                    self.put(12, u16::from(UNDERLINE), important);
                } else if value.eq_ignore_ascii_case("none") {
                    self.put(12, 0, important);
                }
            }
            for (prefix, base) in [("margin", 4), ("padding", 8)] {
                if key.eq_ignore_ascii_case(prefix) {
                    let mut values = [0; 4];
                    let mut n = 0;
                    let mut valid = true;
                    for part in value.split_ascii_whitespace() {
                        if n == 4 {
                            valid = false;
                            break;
                        }
                        if let Some(v) = pixels(part, 12) {
                            values[n] = v;
                            n += 1;
                        } else {
                            valid = false;
                            break;
                        }
                    }
                    if valid && n > 0 {
                        let four = match n {
                            1 => [values[0]; 4],
                            2 => [values[0], values[1], values[0], values[1]],
                            3 => [values[0], values[1], values[2], values[1]],
                            _ => values,
                        };
                        for (side, v) in four.into_iter().enumerate() {
                            self.put(base + side, u16::from(v), important);
                        }
                    }
                }
                for (side, suffix) in ["top", "right", "bottom", "left"].iter().enumerate() {
                    if key.len() == prefix.len() + 1 + suffix.len()
                        && key
                            .get(..prefix.len())
                            .is_some_and(|s| s.eq_ignore_ascii_case(prefix))
                        && key.as_bytes()[prefix.len()] == b'-'
                        && key
                            .get(prefix.len() + 1..)
                            .is_some_and(|s| s.eq_ignore_ascii_case(suffix))
                    {
                        if let Some(v) = pixels(value, 12) {
                            self.put(base + side, u16::from(v), important);
                        }
                    }
                }
            }
        }
    }
    pub fn cascade(&self, score: u16, priorities: &mut [u16; PROPERTIES], style: &mut Style) {
        if score == 0 {
            return;
        }
        for (i, priority) in priorities.iter_mut().enumerate() {
            let p = score
                + if self.important & (1 << i) != 0 {
                    10000
                } else {
                    0
                };
            if self.mask & (1 << i) != 0 && p >= *priority {
                *priority = p;
                style.set(i, self.values[i]);
            }
        }
    }
}
fn pixels(value: &str, cap: u8) -> Option<u8> {
    let value = value
        .strip_suffix("px")
        .or_else(|| (value == "0").then_some(value))?;
    value
        .parse::<u16>()
        .ok()
        .map(|n| n.min(u16::from(cap)) as u8)
}
pub fn color(value: &str) -> Option<u16> {
    let (r, g, b) = if let Some(hex) = value.strip_prefix('#') {
        if hex.len() == 3 {
            let n = u16::from_str_radix(hex, 16).ok()?;
            (((n >> 8) & 15) * 17, ((n >> 4) & 15) * 17, (n & 15) * 17)
        } else if hex.len() == 6 {
            let n = u32::from_str_radix(hex, 16).ok()?;
            (
                ((n >> 16) & 255) as u16,
                ((n >> 8) & 255) as u16,
                (n & 255) as u16,
            )
        } else {
            return None;
        }
    } else if let Some(inner) = value.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')')) {
        let mut parts = inner.split(',');
        let r = parts.next()?.trim().parse::<u8>().ok()?;
        let g = parts.next()?.trim().parse::<u8>().ok()?;
        let b = parts.next()?.trim().parse::<u8>().ok()?;
        if parts.next().is_some() {
            return None;
        }
        (u16::from(r), u16::from(g), u16::from(b))
    } else {
        let names = [
            ("black", 0, 0, 0),
            ("white", 255, 255, 255),
            ("red", 255, 0, 0),
            ("green", 0, 128, 0),
            ("blue", 0, 0, 255),
            ("yellow", 255, 255, 0),
            ("gray", 128, 128, 128),
            ("grey", 128, 128, 128),
            ("silver", 192, 192, 192),
            ("navy", 0, 0, 128),
            ("teal", 0, 128, 128),
            ("purple", 128, 0, 128),
            ("maroon", 128, 0, 0),
            ("lime", 0, 255, 0),
            ("aqua", 0, 255, 255),
            ("fuchsia", 255, 0, 255),
        ];
        let (_, r, g, b) = names
            .into_iter()
            .find(|(name, _, _, _)| value.eq_ignore_ascii_case(name))?;
        (r, g, b)
    };
    Some(((r >> 3) << 11) | ((g >> 2) << 5) | (b >> 3))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn basic_colors_lengths_shorthands_and_cascade_are_bounded() {
        let mut sheet = crate::css::Sheet::EMPTY;
        sheet.parse(b"p {color:red !important; background:#fff; margin:2px 4px; padding:1px 2px 3px; border:2px solid blue} #intro{color:green;padding-left:5px}","https://example.org/style.css");
        let style = sheet.paint(
            "p",
            "intro",
            "",
            "color:black;margin-top:9px",
            Style::DEFAULT,
        );
        assert_eq!(style.color, 0xf800);
        assert_eq!(style.background, 0xffff);
        assert_eq!(style.border, 31);
        assert_eq!(style.stroke, 2);
        assert_eq!(style.margin, [9, 4, 2, 4]);
        assert_eq!(style.padding, [1, 2, 3, 5]);
        let inherited = Style::inherit(style);
        assert_eq!(inherited.color, 0xf800);
        assert_eq!(inherited.padding, [0; 4]);
        assert_eq!(inherited.flags & BACKGROUND, 0);
        let capped = sheet.paint(
            "span",
            "",
            "",
            "padding:999px; border:999px solid #abc; margin:-2px; color:javascript:bad; 色色:1px",
            Style::DEFAULT,
        );
        assert_eq!(capped.padding, [12; 4]);
        assert_eq!(capped.stroke, 3);
        assert_eq!(capped.margin, [0; 4]);
        assert_eq!(capped.color, Style::DEFAULT.color);
        assert_eq!(color("#ff0000"), color("red"));
        assert_eq!(color("#fff"), color("rgb(255,255,255)"));
        assert_eq!(color("rgb(256,0,0)"), None);
    }
}
