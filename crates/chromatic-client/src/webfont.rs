// SPDX-License-Identifier: GPL-3.0-only
//! Original, allocation-free static TrueType subset rasterizer, not a general font engine.
//! Only simple glyf outlines and Unicode cmap format 4; fixed limits on every loop.
pub const LIMIT: usize = 24576;
pub const ATLAS_BYTES: usize = 95 * 9 + 12; // ASCII 32..126, 6x12 1bpp + presence bits
const POINTS: usize = 256;
const CONTOURS: usize = 32;
#[derive(Clone, Copy, Default)]
struct Point {
    x: i32,
    y: i32,
    on: bool,
}
fn u16be(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(
        b.get(at..at.checked_add(2)?)?.try_into().ok()?,
    ))
}
fn i16be(b: &[u8], at: usize) -> Option<i16> {
    Some(u16be(b, at)? as i16)
}
fn u32be(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(
        b.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}
fn table<'a>(b: &'a [u8], tag: &[u8; 4]) -> Option<&'a [u8]> {
    let n = u16be(b, 4)? as usize;
    if n == 0 || n > 24 {
        return None;
    }
    for i in 0..n {
        let at = 12 + 16 * i;
        if b.get(at..at + 4)? == tag {
            let s = u32be(b, at + 8)? as usize;
            let n = u32be(b, at + 12)? as usize;
            return b.get(s..s.checked_add(n)?);
        }
    }
    None
}
struct Font<'a> {
    glyf: &'a [u8],
    loca: &'a [u8],
    cmap: &'a [u8],
    hmtx: &'a [u8],
    long: bool,
    glyphs: usize,
    metrics: usize,
    asc: i32,
    em: i32,
    span: i32,
}
impl<'a> Font<'a> {
    fn parse(b: &'a [u8]) -> Option<Self> {
        if b.len() > LIMIT || b.get(..4)? != [0, 1, 0, 0] || table(b, b"fvar").is_some() {
            return None;
        }
        let head = table(b, b"head")?;
        let maxp = table(b, b"maxp")?;
        let hhea = table(b, b"hhea")?;
        let em = i32::from(u16be(head, 18)?);
        if !(16..=16384).contains(&em) {
            return None;
        }
        let glyphs = u16be(maxp, 4)? as usize;
        let metrics = u16be(hhea, 34)? as usize;
        let format = i16be(head, 50)?;
        let asc = i32::from(i16be(hhea, 4)?);
        let desc = i32::from(i16be(hhea, 6)?);
        let span = asc - desc;
        if glyphs == 0
            || glyphs > 512
            || metrics == 0
            || metrics > glyphs
            || !(0..=1).contains(&format)
            || !(1..=32768).contains(&span)
            || asc < 0
            || desc > 0
        {
            return None;
        }
        let cmap = table(b, b"cmap")?;
        let n = u16be(cmap, 2)? as usize;
        if n > 16 {
            return None;
        }
        let mut selected = None;
        for i in 0..n {
            let at = 4 + 8 * i;
            let platform = u16be(cmap, at)?;
            let encoding = u16be(cmap, at + 2)?;
            if platform == 0 || (platform == 3 && encoding == 1) {
                let offset = u32be(cmap, at + 4)? as usize;
                let c = cmap.get(offset..)?;
                if u16be(c, 0)? == 4 {
                    let len = u16be(c, 2)? as usize;
                    let c = c.get(..len)?;
                    let segments = u16be(c, 6)? as usize / 2;
                    if segments == 0 || segments > 256 || 16 + segments * 8 > len {
                        return None;
                    }
                    selected = Some(c);
                    break;
                }
            }
        }
        let font = Self {
            glyf: table(b, b"glyf")?,
            loca: table(b, b"loca")?,
            cmap: selected?,
            hmtx: table(b, b"hmtx")?,
            long: format == 1,
            glyphs,
            metrics,
            asc,
            em,
            span,
        };
        if font.loca.len() < (glyphs + 1) * if font.long { 4 } else { 2 }
            || font.hmtx.len() < metrics * 4 + (glyphs - metrics) * 2
        {
            return None;
        }
        Some(font)
    }
    fn glyph(&self, c: u16) -> Option<usize> {
        let b = self.cmap;
        let n = u16be(b, 6)? as usize / 2;
        for i in 0..n {
            let end = u16be(b, 14 + 2 * i)?;
            let start = u16be(b, 16 + 2 * n + 2 * i)?;
            if start > end {
                return None;
            }
            if c < start || c > end {
                continue;
            }
            let delta = u16be(b, 16 + 4 * n + 2 * i)?;
            let at = 16 + 6 * n + 2 * i;
            let offset = u16be(b, at)? as usize;
            let glyph = if offset == 0 {
                c.wrapping_add(delta)
            } else {
                let v = u16be(
                    b,
                    at.checked_add(offset)?
                        .checked_add(2 * usize::from(c - start))?,
                )?;
                if v == 0 { 0 } else { v.wrapping_add(delta) }
            } as usize;
            return (glyph > 0 && glyph < self.glyphs).then_some(glyph);
        }
        None
    }
    fn outline(&self, g: usize) -> Option<&'a [u8]> {
        let offset = |i| {
            if self.long {
                u32be(self.loca, i * 4).map(|v| v as usize)
            } else {
                u16be(self.loca, i * 2).map(|v| usize::from(v) * 2)
            }
        };
        self.glyf.get(offset(g)?..offset(g + 1)?)
    }
    fn render(&self, g: usize, out: &mut [u8]) -> Option<()> {
        self.render_grid(g, out, 6, 12, false, &mut || false)
    }
    fn render_grid(
        &self,
        g: usize,
        out: &mut [u8],
        width: usize,
        height: usize,
        smooth: bool,
        cancel: &mut impl FnMut() -> bool,
    ) -> Option<()> {
        let b = self.outline(g)?;
        if b.is_empty() {
            return Some(());
        }
        let contours = i16be(b, 0)?;
        if !(0..=CONTOURS as i16).contains(&contours) {
            return None;
        }
        if contours == 0 {
            return Some(());
        }
        let n = usize::from(u16be(b, 10 + 2 * (contours as usize - 1))?) + 1;
        if n > POINTS {
            return None;
        }
        let mut ends = [0usize; CONTOURS];
        for i in 0..contours as usize {
            ends[i] = u16be(b, 10 + 2 * i)? as usize;
            if ends[i] >= n || (i > 0 && ends[i] <= ends[i - 1]) {
                return None;
            }
        }
        let mut at = 12 + 2 * contours as usize;
        let instructions = u16be(b, at - 2)? as usize;
        if instructions > 1024 {
            return None;
        }
        at = at.checked_add(instructions)?;
        let mut flags = [0u8; POINTS];
        let mut points = [Point::default(); POINTS];
        let mut i = 0;
        while i < n {
            let flag = *b.get(at)?;
            at += 1;
            let repeat = if flag & 8 != 0 {
                let r = *b.get(at)? as usize;
                at += 1;
                r + 1
            } else {
                1
            };
            if repeat > n - i {
                return None;
            }
            flags[i..i + repeat].fill(flag);
            i += repeat;
        }
        for axis in 0..2 {
            let mut value = 0i32;
            for i in 0..n {
                let short = flags[i] & (2 << axis) != 0;
                let same = flags[i] & (16 << axis) != 0;
                let delta = if short {
                    let d = i32::from(*b.get(at)?);
                    at += 1;
                    if same { d } else { -d }
                } else if same {
                    0
                } else {
                    let d = i32::from(i16be(b, at)?);
                    at += 2;
                    d
                };
                value = value.checked_add(delta)?;
                if !(-16384..=16384).contains(&value) {
                    return None;
                }
                if axis == 0 {
                    points[i].x = value;
                    points[i].on = flags[i] & 1 != 0;
                } else {
                    points[i].y = value;
                }
            }
        }
        let advance = i32::from(u16be(self.hmtx, g.min(self.metrics - 1) * 4)?);
        if !(1..=32768).contains(&advance) {
            return None;
        }
        for p in &mut points[..n] {
            if smooth {
                // Preserve outline aspect at an em square, with one-pixel side
                // bearings and a two-pixel descent in the fixed cell.
                p.x = 256 + p.x * (width as i32 - 2) * 256 / advance;
                p.y = (height as i32 - 2) * 256 - p.y * (height as i32 - 2) * 256 / self.em;
            } else {
                p.x = p.x * width as i32 * 256 / advance;
                p.y = (self.asc - p.y) * height as i32 * 256 / self.span;
            }
            if !(-32768..=32768).contains(&p.x) || !(-32768..=32768).contains(&p.y) {
                return None;
            }
        }
        let mut scan = Scan::new(out);
        let samples = if smooth { 4 } else { 1 };
        let mut coverage = [0u8; 16];
        for subrow in 0..height * samples {
            if cancel() {
                return None;
            }
            let row = subrow / samples;
            if subrow % samples == 0 {
                coverage.fill(0);
            }
            scan.y = (subrow * 256 / samples + 128 / samples) as i32;
            scan.count = 0;
            scan.failed = false;
            let mut start = 0;
            for &end in &ends[..contours as usize] {
                let first = points[start];
                let last = points[end];
                let origin = if first.on {
                    first
                } else if last.on {
                    last
                } else {
                    mid(first, last)
                };
                let mut current = origin;
                let mut control = None;
                let begin = if first.on { start + 1 } else { start };
                let finish = if !first.on && last.on { end } else { end + 1 };
                for &p in &points[begin..finish] {
                    if p.on {
                        if let Some(c) = control.take() {
                            scan.curve(current, c, p);
                        } else {
                            scan.edge(current, p);
                        }
                        current = p;
                    } else if let Some(c) = control.replace(p) {
                        let next = mid(c, p);
                        scan.curve(current, c, next);
                        current = next;
                    }
                }
                if let Some(c) = control {
                    scan.curve(current, c, origin);
                } else {
                    scan.edge(current, origin);
                }
                start = end + 1;
            }
            if scan.failed {
                return None;
            }
            if smooth {
                for (col, value) in coverage.iter_mut().enumerate().take(width) {
                    for sample in 0..2 {
                        if scan.inside((col * 256 + sample * 128 + 64) as i32) {
                            *value += 1;
                        }
                    }
                }
                if subrow % samples == samples - 1 {
                    for (col, value) in coverage.iter().enumerate().take(width) {
                        let alpha = ((*value as u16 * 3 + 4) / 8) as u8;
                        let bit = (row * width + col) * 2;
                        scan.out[bit / 8] |= alpha << (6 - bit % 8);
                    }
                }
            } else {
                scan.fill(row);
            }
        }
        Some(())
    }
}
fn mid(a: Point, b: Point) -> Point {
    Point {
        x: (a.x + b.x) / 2,
        y: (a.y + b.y) / 2,
        on: true,
    }
}
struct Scan<'a> {
    out: &'a mut [u8],
    y: i32,
    x: [i32; 64],
    direction: [i8; 64],
    count: usize,
    failed: bool,
}
impl<'a> Scan<'a> {
    fn new(out: &'a mut [u8]) -> Self {
        Self {
            out,
            y: 0,
            x: [0; 64],
            direction: [0; 64],
            count: 0,
            failed: false,
        }
    }
    fn edge(&mut self, a: Point, b: Point) {
        if (a.y <= self.y && b.y > self.y) || (b.y <= self.y && a.y > self.y) {
            if self.count == 64 {
                self.failed = true;
                return;
            }
            let x = i64::from(a.x)
                + i64::from(self.y - a.y) * i64::from(b.x - a.x) / i64::from(b.y - a.y);
            self.x[self.count] = x as i32;
            self.direction[self.count] = if b.y > a.y { 1 } else { -1 };
            self.count += 1;
        }
    }
    fn curve(&mut self, a: Point, c: Point, b: Point) {
        let mut previous = a;
        for t in 1..=8 {
            let s = 8 - t;
            let p = Point {
                x: (s * s * a.x + 2 * s * t * c.x + t * t * b.x) / 64,
                y: (s * s * a.y + 2 * s * t * c.y + t * t * b.y) / 64,
                on: true,
            };
            self.edge(previous, p);
            previous = p;
        }
    }
    fn inside(&self, x: i32) -> bool {
        let mut winding = 0;
        for i in 0..self.count {
            if self.x[i] <= x {
                winding += i32::from(self.direction[i]);
            }
        }
        winding != 0
    }
    fn fill(&mut self, row: usize) {
        for col in 0..6 {
            let x = col as i32 * 256 + 128;
            let mut winding = 0i32;
            for i in 0..self.count {
                if self.x[i] <= x {
                    winding += i32::from(self.direction[i]);
                }
            }
            if winding != 0 {
                let bit = row * 6 + col;
                self.out[bit / 8] |= 1 << (7 - bit % 8);
            }
        }
    }
}
/// Reject invalid containers; unsupported/malformed individual glyphs use the built-in font.
/// Caller publishes the atlas only after success; zero output is not a usable font.
pub fn rasterize(bytes: &[u8], atlas: &mut [u8; ATLAS_BYTES]) -> usize {
    atlas.fill(0);
    let Some(font) = Font::parse(bytes) else {
        return 0;
    };
    let mut count = 0;
    for c in 32..=126 {
        let i = c - 32;
        if let Some(g) = font.glyph(c as u16) {
            let glyph = &mut atlas[i * 9..i * 9 + 9];
            if font.render(g, glyph).is_some() {
                atlas[95 * 9 + i / 8] |= 1 << (i % 8);
                count += 1;
            } else {
                glyph.fill(0);
            }
        }
    }
    count
}

/// One immutable, fixed-size face. Slots are ordered by Unicode scalar; 16x16, 2bpp.
pub const UNICODE_SLOTS: usize = 128;
#[repr(C)]
pub struct UnicodeAtlas {
    pub count: u32,
    pub codes: [u32; UNICODE_SLOTS],
    pub pixels: [[u8; 64]; UNICODE_SLOTS],
}
impl UnicodeAtlas {
    pub const EMPTY: Self = Self {
        count: 0,
        codes: [0; UNICODE_SLOTS],
        pixels: [[0; 64]; UNICODE_SLOTS],
    };
}
/// BMP cmap4, simple outlines only. Unsupported glyphs keep the built-in fallback.
/// Cancellation discards the whole unpublished atlas; no heap is used.
#[derive(Debug, PartialEq, Eq)]
pub enum RasterError {
    Cancelled,
}
pub fn rasterize_unicode(
    bytes: &[u8],
    atlas: &mut UnicodeAtlas,
    cancel: impl FnMut() -> bool,
) -> Result<usize, RasterError> {
    let mut latched = false;
    let mut original = cancel;
    let mut cancel = || {
        latched |= original();
        latched
    };
    atlas.count = 0;
    atlas.codes.fill(0);
    atlas.pixels.fill([0; 64]);
    let Some(font) = Font::parse(bytes) else {
        return Ok(0);
    };
    let segments = usize::from(u16be(font.cmap, 6).unwrap_or(0)) / 2;
    let mut scanned = 0usize;
    let mut previous = 0u16;
    for i in 0..segments {
        if cancel() {
            atlas.count = 0;
            return Err(RasterError::Cancelled);
        }
        let Some(start) = u16be(font.cmap, 16 + 2 * segments + 2 * i) else {
            return Ok(0);
        };
        let Some(end) = u16be(font.cmap, 14 + 2 * i) else {
            return Ok(0);
        };
        if start > end || (i > 0 && start <= previous) {
            atlas.count = 0;
            return Ok(0);
        }
        previous = end;
        // Sentinel is not a glyph; broad cmaps are not expanded or sampled silently.
        if start == 65535 {
            continue;
        }
        scanned += usize::from(end - start) + 1;
        if scanned > 4096 {
            atlas.count = 0;
            return Ok(0);
        }
        for code in start..=end {
            if code < 32 {
                continue;
            }
            if cancel() {
                atlas.count = 0;
                return Err(RasterError::Cancelled);
            }
            let slot = atlas.count as usize;
            if slot == UNICODE_SLOTS {
                return Ok(slot);
            }
            let Some(glyph) = font.glyph(code) else {
                continue;
            };
            if font
                .render_grid(glyph, &mut atlas.pixels[slot], 16, 16, true, &mut cancel)
                .is_some()
            {
                atlas.codes[slot] = u32::from(code);
                atlas.count += 1;
            } else {
                atlas.pixels[slot].fill(0);
            }
        }
    }
    if cancel() {
        atlas.count = 0;
        return Err(RasterError::Cancelled);
    }
    Ok(atlas.count as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate std;
    use std::vec::Vec;
    fn put(b: &mut [u8], at: usize, n: u16) {
        b[at..at + 2].copy_from_slice(&n.to_be_bytes());
    }
    // Original mathematical outlines; no third-party font file is bundled.
    fn fixture(curve: bool) -> Vec<u8> {
        let mut head = std::vec![0;54];
        put(&mut head, 50, 0);
        put(&mut head, 18, 1200);
        let mut maxp = std::vec![0;6];
        put(&mut maxp, 4, 2);
        let mut hhea = std::vec![0;36];
        put(&mut hhea, 4, 1200);
        put(&mut hhea, 34, 2);
        let mut hmtx = std::vec![0;8];
        put(&mut hmtx, 4, 600);
        let coords: &[(i16, i16, bool)] = if curve {
            &[(0, 0, true), (300, 1200, false), (600, 0, true)]
        } else {
            &[
                (0, 0, true),
                (600, 0, true),
                (600, 1200, true),
                (0, 1200, true),
                (200, 400, true),
                (200, 800, true),
                (400, 800, true),
                (400, 400, true),
            ]
        };
        let mut glyf = std::vec![0;10];
        put(&mut glyf, 0, if curve { 1 } else { 2 });
        if !curve {
            glyf.extend(3u16.to_be_bytes());
        }
        glyf.extend(((coords.len() - 1) as u16).to_be_bytes());
        glyf.extend([0, 0]);
        for &(_, _, on) in coords {
            glyf.push(u8::from(on));
        }
        for axis in 0..2 {
            let mut last = 0i16;
            for &(x, y, _) in coords {
                let value = if axis == 0 { x } else { y };
                glyf.extend((value - last).to_be_bytes());
                last = value;
            }
        }
        if glyf.len() % 2 != 0 {
            glyf.push(0);
        }
        let mut loca = std::vec![0;6];
        put(&mut loca, 4, (glyf.len() / 2) as u16);
        let mut cmap = std::vec![0;12];
        put(&mut cmap, 2, 1);
        put(&mut cmap, 4, 3);
        put(&mut cmap, 6, 1);
        cmap[8..12].copy_from_slice(&12u32.to_be_bytes());
        let mut sub = std::vec![0;32];
        put(&mut sub, 0, 4);
        put(&mut sub, 2, 32);
        put(&mut sub, 6, 4);
        put(&mut sub, 14, 65);
        put(&mut sub, 16, 65535);
        put(&mut sub, 20, 65);
        put(&mut sub, 22, 65535);
        put(&mut sub, 24, 1u16.wrapping_sub(65));
        put(&mut sub, 26, 1);
        cmap.extend(sub);
        let tables = [
            (b"head", head),
            (b"maxp", maxp),
            (b"hhea", hhea),
            (b"hmtx", hmtx),
            (b"loca", loca),
            (b"glyf", glyf),
            (b"cmap", cmap),
        ];
        let mut bytes = std::vec![0;12+16*tables.len()];
        bytes[..4].copy_from_slice(&[0, 1, 0, 0]);
        put(&mut bytes, 4, tables.len() as u16);
        for (i, (tag, data)) in tables.iter().enumerate() {
            let at = 12 + 16 * i;
            bytes[at..at + 4].copy_from_slice(*tag);
            let offset = bytes.len() as u32;
            bytes[at + 8..at + 12].copy_from_slice(&offset.to_be_bytes());
            bytes[at + 12..at + 16].copy_from_slice(&(data.len() as u32).to_be_bytes());
            bytes.extend(data);
        }
        bytes
    }
    #[test]
    fn japanese_reference_uses_bounded_grayscale_and_cancels_before_publication() {
        let font = include_bytes!("../../../tests/fixtures/webfonts/noto-jp-reference.ttf");
        let mut atlas = UnicodeAtlas::EMPTY;
        let n = rasterize_unicode(font, &mut atlas, || false).unwrap();
        for code in "日本語検索結果がぱ".chars() {
            assert!(atlas.codes[..n].contains(&(code as u32)), "missing {code}");
        }
        assert!(
            atlas.pixels[..n]
                .iter()
                .flatten()
                .any(|b| (0..4).any(|i| matches!((b >> (i * 2)) & 3, 1 | 2)))
        );
        assert!(atlas.codes[..n].windows(2).all(|w| w[0] < w[1]));
        for stop in [1, 3, 20, 200, 500] {
            let mut checks = 0;
            assert_eq!(
                rasterize_unicode(font, &mut atlas, || {
                    checks += 1;
                    checks >= stop
                }),
                Err(RasterError::Cancelled)
            );
            assert_eq!(atlas.count, 0);
        }
        assert!(rasterize_unicode(font, &mut atlas, || false).unwrap() > 0);
        if let Ok(path) = std::env::var("POCKET_FONT_REFERENCE") {
            let mut bytes = Vec::new();
            bytes.extend(atlas.count.to_le_bytes());
            for c in atlas.codes {
                bytes.extend(c.to_le_bytes());
            }
            for pixels in atlas.pixels {
                bytes.extend(pixels);
            }
            std::fs::write(path, bytes).unwrap();
        }
    }
    #[test]
    fn mathematical_hole_and_quadratic_outline_have_expected_pixels() {
        let mut atlas = [0; ATLAS_BYTES];
        assert_eq!(rasterize(&fixture(false), &mut atlas), 1);
        let i = usize::from(b'A' - 32);
        assert_ne!(atlas[855 + i / 8] & (1 << (i % 8)), 0);
        for y in 0..12 {
            for x in 0..6 {
                let bit = y * 6 + x;
                assert_eq!(
                    atlas[i * 9 + bit / 8] & (1 << (7 - bit % 8)) != 0,
                    !((2..4).contains(&x) && (4..8).contains(&y))
                );
            }
        }
        assert_eq!(rasterize(&fixture(true), &mut atlas), 1);
        assert_eq!(atlas[i * 9], 0);
        assert!(atlas[i * 9 + 7] != 0);
    }
    #[test]
    fn truncations_unsupported_formats_composites_and_mutations_stay_bounded() {
        let font = fixture(false);
        let mut atlas = [0; ATLAS_BYTES];
        for end in 0..font.len() {
            let _ = rasterize(&font[..end], &mut atlas);
        }
        for at in 0..font.len() {
            for byte in [0, 0xff, 0x7f] {
                let mut bad = font.clone();
                bad[at] = byte;
                let _ = rasterize(&bad, &mut atlas);
            }
        }
        for tag in [b"wOF2", b"wOFF", b"OTTO"] {
            let mut bad = font.clone();
            bad[..4].copy_from_slice(tag);
            assert_eq!(rasterize(&bad, &mut atlas), 0);
        }
        let f = Font::parse(&font).unwrap();
        let at = f.glyf.as_ptr() as usize - font.as_ptr() as usize;
        let mut bad = font.clone();
        bad[at..at + 2].copy_from_slice(&(-1i16).to_be_bytes());
        assert_eq!(rasterize(&bad, &mut atlas), 0);
        let mut oversized = font;
        oversized.resize(LIMIT + 1, 0);
        assert_eq!(rasterize(&oversized, &mut atlas), 0);
    }
}

#[cfg(test)]
mod native_acceptance_reference {
    use super::*;
    extern crate std;
    #[test]
    fn small_japanese_native_reference() {
        let bytes = include_bytes!("../../../tests/fixtures/webfonts/noto-jp-native-reference.ttf");
        let mut atlas = UnicodeAtlas::EMPTY;
        let n = rasterize_unicode(bytes, &mut atlas, || false).unwrap();
        for c in "日本語検索結果".chars() {
            assert!(atlas.codes[..n].contains(&(c as u32)));
        }
        if let Ok(path) = std::env::var("POCKET_NATIVE_FONT_REFERENCE") {
            let mut bytes = std::vec::Vec::new();
            bytes.extend(atlas.count.to_le_bytes());
            for c in atlas.codes {
                bytes.extend(c.to_le_bytes());
            }
            for pixels in atlas.pixels {
                bytes.extend(pixels);
            }
            std::fs::write(path, bytes).unwrap();
        }
    }
}
