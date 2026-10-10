// SPDX-License-Identifier: GPL-3.0-only
//! Bounded typography viewports; borrowed text and packed per-scalar styles.
use crate::{Error, MAX_PACKET_BYTES};

pub const ROWS: usize = 8;
pub const WIDTH: usize = 144;
pub const HEIGHT: usize = 92;
pub const GLYPHS: usize = 28;
pub const TEXT_BYTES: usize = 64;
pub const BOLD: u8 = 4;
pub const HEADING: u8 = 1;
pub const SUBHEADING: u8 = 2;
pub const CAPTION: u8 = 3;

pub fn size(style: u8) -> usize {
    [12, 16, 14, 10][usize::from(style & 3)]
}
pub fn advance(c: char, style: u8) -> usize {
    size(style) / if c.is_ascii() { 2 } else { 1 } + usize::from(style & BOLD != 0)
}
pub fn height(style: u8) -> usize {
    size(style) + 2
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Row<'a> {
    pub text: &'a str,
    pub styles: &'a [u8],
    pub y: u8,
    pub height: u8,
    pub glyphs: u8,
}
impl Row<'_> {
    pub fn style(&self, glyph: usize) -> u8 {
        (self.styles[glyph / 2] >> ((glyph % 2) * 4)) & 15
    }
}

#[derive(Debug)]
pub struct Packet<'a> {
    pub document_id: u32,
    pub position: u16,
    pub positions: u16,
    pub title: &'a str,
    pub rows: [Row<'a>; ROWS],
    pub row_count: usize,
}
impl<'a> Packet<'a> {
    pub fn decode(bytes: &'a [u8]) -> Result<Self, Error> {
        if !(20..=MAX_PACKET_BYTES).contains(&bytes.len()) {
            return Err(Error::Size);
        }
        if &bytes[..4] != b"PSR1" || bytes[4] != 1 || bytes[18..20] != [0, 0] {
            return Err(Error::Header);
        }
        let document_id = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        let position = u16::from_le_bytes(bytes[12..14].try_into().unwrap());
        let positions = u16::from_le_bytes(bytes[14..16].try_into().unwrap());
        if document_id == 0 || positions == 0 || position >= positions {
            return Err(Error::Pagination);
        }
        let flags = u8::from(position > 0) | (u8::from(position + 1 < positions) << 1);
        let row_count = usize::from(bytes[6]);
        if bytes[5] != flags
            || !(1..=ROWS).contains(&row_count)
            || usize::from(u16::from_le_bytes(bytes[16..18].try_into().unwrap()))
                != bytes.len() - 20
        {
            return Err(Error::Header);
        }
        let mut at = 20 + usize::from(bytes[7]);
        let title = crate::text(bytes.get(20..at).ok_or(Error::Size)?)?;
        let mut rows = [Row::default(); ROWS];
        let mut bottom = 0;
        for row in &mut rows[..row_count] {
            let d = bytes.get(at..at + 4).ok_or(Error::Size)?;
            let (y, h, n, glyphs) = (d[0], d[1], usize::from(d[2]), usize::from(d[3]));
            at += 4;
            if usize::from(y) != bottom
                || bottom + usize::from(h) > HEIGHT
                || n > TEXT_BYTES
                || glyphs > GLYPHS
            {
                return Err(Error::Text);
            }
            let text = core::str::from_utf8(bytes.get(at..at + n).ok_or(Error::Size)?)
                .map_err(|_| Error::Text)?;
            at += n;
            let style_bytes = glyphs.div_ceil(2);
            let styles = bytes.get(at..at + style_bytes).ok_or(Error::Size)?;
            at += style_bytes;
            if text.chars().count() != glyphs
                || text.chars().any(char::is_control)
                || (glyphs % 2 == 1 && styles[style_bytes - 1] & 0xf0 != 0)
            {
                return Err(Error::Text);
            }
            *row = Row {
                text,
                styles,
                y,
                height: h,
                glyphs: glyphs as u8,
            };
            let (mut width, mut tallest) = (0, 0);
            for (i, c) in text.chars().enumerate() {
                let style = row.style(i);
                if style > 7 {
                    return Err(Error::Text);
                }
                width += advance(c, style);
                tallest = tallest.max(height(style));
            }
            if width > WIDTH || usize::from(h) != if glyphs == 0 { 6 } else { tallest } {
                return Err(Error::Text);
            }
            bottom += usize::from(h);
        }
        if at != bytes.len() {
            return Err(Error::Size);
        }
        Ok(Self {
            document_id,
            position,
            positions,
            title,
            rows,
            row_count,
        })
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Line {
    text: [u8; TEXT_BYTES],
    styles: [u8; GLYPHS / 2],
    len: usize,
    glyphs: usize,
    pub height: usize,
}
impl Line {
    const EMPTY: Self = Self {
        text: [0; TEXT_BYTES],
        styles: [0; GLYPHS / 2],
        len: 0,
        glyphs: 0,
        height: 6,
    };
    fn encoded_bytes(&self) -> usize {
        4 + self.len + self.glyphs.div_ceil(2)
    }
    fn append(&mut self, c: char, style: u8) {
        let n = c
            .encode_utf8(&mut self.text[self.len..self.len + c.len_utf8()])
            .len();
        self.styles[self.glyphs / 2] |= style << ((self.glyphs % 2) * 4);
        self.glyphs += 1;
        self.len += n;
        self.height = self.height.max(height(style));
    }
    fn trim(&mut self, len: usize, glyphs: usize) {
        self.len = len;
        self.glyphs = glyphs;
        self.styles[glyphs.div_ceil(2)..].fill(0);
        if glyphs % 2 == 1 {
            self.styles[glyphs / 2] &= 15;
        }
        self.height = 6;
        for i in 0..glyphs {
            self.height = self
                .height
                .max(height((self.styles[i / 2] >> ((i % 2) * 4)) & 15));
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Run {
    pub start: u16,
    pub style: u8,
}
impl Run {
    pub const EMPTY: Self = Self { start: 0, style: 0 };
}

#[derive(Clone)]
struct Lines<'a> {
    text: &'a str,
    runs: &'a [Run],
    at: usize,
    run: usize,
    marker: u8,
}
impl<'a> Lines<'a> {
    fn new(text: &'a str, runs: &'a [Run], marker: u8) -> Self {
        Self {
            text,
            runs,
            at: 0,
            run: 0,
            marker,
        }
    }
    fn style(&mut self, at: usize) -> u8 {
        while self.run > 0 && usize::from(self.runs[self.run].start) > at {
            self.run -= 1;
        }
        while self.run + 1 < self.runs.len() && usize::from(self.runs[self.run + 1].start) <= at {
            self.run += 1;
        }
        self.runs
            .get(self.run)
            .filter(|r| usize::from(r.start) <= at)
            .map_or(0, |r| r.style)
    }
}
impl Iterator for Lines<'_> {
    type Item = Line;
    fn next(&mut self) -> Option<Line> {
        if self.at == self.text.len() {
            let label = match self.marker {
                1 | 3 => "[Text limit reached]",
                2 => "[Style limit reached]",
                _ => return None,
            };
            self.marker = if self.marker == 3 { 2 } else { 0 };
            let mut row = Line::EMPTY;
            for c in label.chars() {
                row.append(c, CAPTION);
            }
            return Some(row);
        }
        let mut row = Line::EMPTY;
        let (mut width, mut space) = (0, None);
        while self.at < self.text.len() {
            let c = self.text[self.at..].chars().next().unwrap();
            if c == '\n' {
                self.at += 1;
                break;
            }
            let style = self.style(self.at);
            if width + advance(c, style) > WIDTH
                || row.glyphs == GLYPHS
                || row.len + c.len_utf8() > TEXT_BYTES
            {
                if let Some((at, len, glyphs)) = space {
                    self.at = at;
                    row.trim(len, glyphs);
                }
                break;
            }
            if c == ' ' {
                space = Some((self.at + 1, row.len, row.glyphs));
            }
            row.append(c, style);
            width += advance(c, style);
            self.at += c.len_utf8();
        }
        if row.len > 0 && row.text[row.len - 1] == b' ' {
            row.trim(row.len - 1, row.glyphs - 1);
        }
        Some(row)
    }
}

pub(crate) fn encode(
    id: usize,
    title: &str,
    body: &str,
    runs: &[Run],
    marker: u8,
    position: usize,
    out: &mut [u8],
) -> usize {
    if out.len() < MAX_PACKET_BYTES {
        return 0;
    }
    let title = crate::local::short_title(title);
    let base = 20 + title.len();
    let mut queue = [(0, 0); ROWS];
    let (mut total, mut used, mut pixels, mut bytes) = (0, 0, 0, base);
    // The trailing window is packed by both pixel and wire budgets. Its first
    // line is the last scroll position, preserving all text without overscroll.
    for row in Lines::new(body, runs, marker) {
        let n = row.encoded_bytes();
        while used > 0
            && (used == ROWS || pixels + row.height > HEIGHT || bytes + n > MAX_PACKET_BYTES)
        {
            pixels -= queue[0].0;
            bytes -= queue[0].1;
            queue.copy_within(1..used, 0);
            used -= 1;
        }
        queue[used] = (row.height, n);
        used += 1;
        total += 1;
        pixels += row.height;
        bytes += n;
    }
    if used == 0 {
        return 0;
    }
    let positions = total - used + 1;
    if position >= positions {
        return 0;
    }
    out[..20].fill(0);
    out[..4].copy_from_slice(b"PSR1");
    out[4] = 1;
    out[5] = u8::from(position > 0) | (u8::from(position + 1 < positions) << 1);
    out[7] = title.len() as u8;
    out[8..12].copy_from_slice(&(id as u32).to_le_bytes());
    out[12..14].copy_from_slice(&(position as u16).to_le_bytes());
    out[14..16].copy_from_slice(&(positions as u16).to_le_bytes());
    out[20..base].copy_from_slice(title.as_bytes());
    let (mut at, mut y, mut count) = (base, 0, 0);
    for row in Lines::new(body, runs, marker).skip(position) {
        if count == ROWS || y + row.height > HEIGHT || at + row.encoded_bytes() > MAX_PACKET_BYTES {
            break;
        }
        out[at..at + 4].copy_from_slice(&[
            y as u8,
            row.height as u8,
            row.len as u8,
            row.glyphs as u8,
        ]);
        at += 4;
        out[at..at + row.len].copy_from_slice(&row.text[..row.len]);
        at += row.len;
        let n = row.glyphs.div_ceil(2);
        out[at..at + n].copy_from_slice(&row.styles[..n]);
        at += n;
        y += row.height;
        count += 1;
    }
    out[6] = count as u8;
    out[16..18].copy_from_slice(&((at - 20) as u16).to_le_bytes());
    at
}

/// Original diagnostic content, never inserted into the autonomous collection.
pub fn probe(out: &mut [u8]) -> usize {
    let text = "Large heading\nNormal body text.\nNormal and bold text.\n\nSmall annotation.\nA new paragraph.";
    let bold = text.find("bold").unwrap() as u16;
    let runs = [
        Run {
            start: 0,
            style: HEADING | BOLD,
        },
        Run {
            start: 14,
            style: 0,
        },
        Run {
            start: bold,
            style: BOLD,
        },
        Run {
            start: bold + 4,
            style: 0,
        },
        Run {
            start: text.find("Small").unwrap() as u16,
            style: CAPTION,
        },
        Run {
            start: text.find("A new").unwrap() as u16,
            style: 0,
        },
    ];
    encode(1, "Typography test", text, &runs, 0, 0, out)
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate std;
    use std::string::String;
    #[test]
    fn calibration_has_real_inline_bold_and_three_sizes_and_rejects_corruption() {
        let mut out = [0; 512];
        let n = probe(&mut out);
        let packet = Packet::decode(&out[..n]).unwrap();
        assert_eq!(packet.rows[0].height, 18);
        assert_eq!(packet.rows[0].style(0), HEADING | BOLD);
        assert_eq!(packet.rows[1].height, 14);
        assert_eq!(packet.rows[1].style(0), 0);
        let row = packet
            .rows
            .iter()
            .find(|r| r.text.contains("bold"))
            .unwrap();
        let bold_at = row.text.find("bold").unwrap();
        for i in 0..row.text.chars().count() {
            assert_eq!(
                row.style(i),
                if (bold_at..bold_at + 4).contains(&i) {
                    BOLD
                } else {
                    0
                }
            );
        }
        assert!(
            packet
                .rows
                .iter()
                .any(|r| r.height == 6 && r.text.is_empty())
        );
        assert!(
            packet
                .rows
                .iter()
                .any(|r| r.height == 12 && r.style(0) == CAPTION)
        );
        for end in 0..n {
            assert!(Packet::decode(&out[..end]).is_err());
        }
        for (at, value) in [(4, 2), (5, 3), (6, 9), (18, 1), (8, 0), (12, 255)] {
            let mut bad = out;
            bad[at] = value;
            assert!(Packet::decode(&bad[..n]).is_err());
        }
        let mut bad = out;
        bad[20 + usize::from(out[7])] = 1;
        assert!(Packet::decode(&bad[..n]).is_err());
        let mut bad = out;
        let styles = packet.rows[0].styles.as_ptr() as usize - out.as_ptr() as usize;
        bad[styles] = 15;
        assert!(Packet::decode(&bad[..n]).is_err());
    }
    #[test]
    fn worst_utf8_and_many_gaps_fit_both_budgets_without_dropping_the_tail() {
        let body = "😀".repeat(140);
        let title = "😀".repeat(24);
        let runs = [Run {
            start: 0,
            style: CAPTION,
        }];
        let mut out = [0; 512];
        let n = encode(1, &title, &body, &runs, 0, 0, &mut out);
        let positions = Packet::decode(&out[..n]).unwrap().positions;
        let mut seen = String::new();
        for pos in 0..positions {
            let n = encode(1, &title, &body, &runs, 0, usize::from(pos), &mut out);
            let packet = Packet::decode(&out[..n]).unwrap();
            assert!(n <= 512);
            seen.push_str(packet.rows[0].text);
            if pos + 1 == positions {
                for r in &packet.rows[1..packet.row_count] {
                    seen.push_str(r.text);
                }
            }
        }
        assert_eq!(seen, body);
        assert_eq!(
            encode(1, &title, &body, &runs, 0, usize::from(positions), &mut out),
            0
        );
        let gaps = "body\n\n".repeat(40);
        let n = encode(1, "gaps", &gaps, &[], 1, 0, &mut out);
        let positions = Packet::decode(&out[..n]).unwrap().positions;
        for pos in 0..positions {
            let n = encode(1, "gaps", &gaps, &[], 1, usize::from(pos), &mut out);
            Packet::decode(&out[..n]).unwrap();
        }
        let n = encode(
            1,
            "gaps",
            &gaps,
            &[],
            1,
            usize::from(positions - 1),
            &mut out,
        );
        assert_eq!(
            Packet::decode(&out[..n])
                .unwrap()
                .rows
                .iter()
                .rfind(|r| !r.text.is_empty())
                .unwrap()
                .text,
            "[Text limit reached]"
        );
    }
}
