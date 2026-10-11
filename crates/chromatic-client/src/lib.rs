// SPDX-License-Identifier: GPL-3.0-only
//! Small, allocation-free building blocks for the Chromatic firmware adapter.
//! This crate has no display, network, allocator, or operating-system dependency.
#![no_std]
#![forbid(unsafe_code)]

pub mod css;
pub mod local;
pub mod rich;
pub mod webfont;

pub const DISPLAY_WIDTH: usize = 160;
pub const DISPLAY_HEIGHT: usize = 144;
pub const TEXT_COLUMNS: usize = 24;
pub const TEXT_ROWS: usize = 7;
pub const MAX_TEXT_BYTES: usize = 48;
pub const MAX_PACKET_BYTES: usize = 512;
pub const MAX_QUERY_BYTES: usize = 96;
pub const HEADER_BYTES: usize = 20;
pub const IMAGE_WIDTH: usize = 128;
pub const IMAGE_HEIGHT: usize = 80;
pub const IMAGE_STRIPE_ROWS: usize = 8;
pub const IMAGE_STRIPE_BYTES: usize = IMAGE_WIDTH * IMAGE_STRIPE_ROWS * 2;
pub const IMAGE_HEADER_BYTES: usize = 24;
pub const MAX_IMAGE_PACKET_BYTES: usize = IMAGE_HEADER_BYTES + IMAGE_STRIPE_BYTES;
pub const MAX_IMAGE_WIRE_BYTES: usize =
    IMAGE_WIDTH * IMAGE_HEIGHT * 2 + IMAGE_HEADER_BYTES * IMAGE_HEIGHT.div_ceil(IMAGE_STRIPE_ROWS);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Result = 1,
    Reader = 2,
    Image = 3,
    Link = 4,
    Heading = 5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Size,
    Header,
    Pagination,
    Text,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    Start,
    Select,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    Previous,
    Next,
    Open(u32),
    Back,
    Search,
    Home,
}

/// Borrows one packet buffer; no text is copied or allocated during decoding.
#[derive(Debug)]
pub struct Packet<'a> {
    pub kind: Kind,
    pub document_id: u32,
    pub page_index: u16,
    pub page_count: u16,
    pub title: &'a str,
    pub lines: [&'a str; TEXT_ROWS],
    pub line_count: usize,
    pub link_target: u32,
}

/// Conservative cell sizing: ASCII uses one cell, other Unicode uses two.
/// The firmware adapter still has to map unsupported glyphs to its fallback.
pub fn cells(text: &str) -> usize {
    text.chars().map(|c| if c.is_ascii() { 1 } else { 2 }).sum()
}

fn text(bytes: &[u8]) -> Result<&str, Error> {
    if bytes.len() > MAX_TEXT_BYTES {
        return Err(Error::Text);
    }
    let value = core::str::from_utf8(bytes).map_err(|_| Error::Text)?;
    // Validate width and controls together, avoiding a second Unicode scan.
    let mut used = 0;
    for c in value.chars() {
        if c.is_control() {
            return Err(Error::Text);
        }
        used += if c.is_ascii() { 1 } else { 2 };
        if used > TEXT_COLUMNS {
            return Err(Error::Text);
        }
    }
    Ok(value)
}

impl<'a> Packet<'a> {
    pub fn decode(bytes: &'a [u8]) -> Result<Self, Error> {
        if bytes.len() < HEADER_BYTES || bytes.len() > MAX_PACKET_BYTES {
            return Err(Error::Size);
        }
        if &bytes[..4] != b"PCS1" || bytes[18..20] != [0, 0] {
            return Err(Error::Header);
        }
        let kind = match bytes[4] {
            1 => Kind::Result,
            2 => Kind::Reader,
            3 => Kind::Image,
            4 => Kind::Link,
            5 => Kind::Heading,
            _ => return Err(Error::Header),
        };
        let document_id = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let page_index = u16::from_le_bytes([bytes[12], bytes[13]]);
        let page_count = u16::from_le_bytes([bytes[14], bytes[15]]);
        if page_count == 0 || page_index >= page_count || (kind != Kind::Result && document_id == 0)
        {
            return Err(Error::Pagination);
        }
        let flags = u8::from(page_index > 0) | (u8::from(page_index + 1 < page_count) << 1);
        if bytes[5] != flags || usize::from(bytes[6]) > TEXT_ROWS {
            return Err(Error::Header);
        }
        let payload_length = usize::from(u16::from_le_bytes([bytes[16], bytes[17]]));
        if HEADER_BYTES + payload_length != bytes.len() {
            return Err(Error::Size);
        }
        let (start, link_target) = if kind == Kind::Link {
            let target = bytes.get(20..24).ok_or(Error::Size)?;
            let target = u32::from_le_bytes([target[0], target[1], target[2], target[3]]);
            if target > 256 {
                return Err(Error::Header);
            }
            (24, target)
        } else {
            (HEADER_BYTES, 0)
        };
        let title_end = start + usize::from(bytes[7]);
        let title = text(bytes.get(start..title_end).ok_or(Error::Size)?)?;
        let mut lines = [""; TEXT_ROWS];
        let line_count = usize::from(bytes[6]);
        let mut offset = title_end;
        for line in lines.iter_mut().take(line_count) {
            let length = usize::from(*bytes.get(offset).ok_or(Error::Size)?);
            offset += 1;
            let end = offset + length;
            *line = text(bytes.get(offset..end).ok_or(Error::Size)?)?;
            offset = end;
        }
        if offset != bytes.len() {
            return Err(Error::Size);
        }
        Ok(Self {
            kind,
            document_id,
            page_index,
            page_count,
            title,
            lines,
            line_count,
            link_target,
        })
    }

    pub fn action(&self, button: Button) -> Action {
        match button {
            Button::B => Action::Back,
            Button::Select => Action::Home,
            Button::Start => Action::Search,
            Button::A if self.kind == Kind::Result && self.document_id != 0 => {
                Action::Open(self.document_id)
            }
            Button::A if self.kind == Kind::Link && self.link_target != 0 => {
                Action::Open(self.link_target)
            }
            Button::Up if self.page_index > 0 => Action::Previous,
            Button::Down if self.page_index.saturating_add(1) < self.page_count => Action::Next,
            Button::Left if self.kind != Kind::Result && self.page_index > 0 => Action::Previous,
            Button::Right
                if self.kind != Kind::Result
                    && self.page_index.saturating_add(1) < self.page_count =>
            {
                Action::Next
            }
            _ => Action::None,
        }
    }
}

/// IEEE CRC-32 with a 64-byte read-only nibble table; no allocation or float work.
pub fn image_crc(bytes: &[u8]) -> u32 {
    const TABLE: [u32; 16] = [
        0, 0x1db71064, 0x3b6e20c8, 0x26d930ac, 0x76dc4190, 0x6b6b51f4, 0x4db26158, 0x5005713c,
        0xedb88320, 0xf00f9344, 0xd6d6a3e8, 0xcb61b38c, 0x9b64c2b0, 0x86d3d2d4, 0xa00ae278,
        0xbdbdf21c,
    ];
    let mut crc = !0u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        crc = (crc >> 4) ^ TABLE[(crc & 15) as usize];
        crc = (crc >> 4) ^ TABLE[(crc & 15) as usize];
    }
    !crc
}

/// A complete RGB565 little-endian stripe. Transport checks sequence and order.
pub struct ImageStripe<'a> {
    pub document_id: u32,
    pub page_index: u16,
    pub width: u16,
    pub height: u16,
    pub y: u16,
    pub rows: u16,
    pub pixels: &'a [u8],
}
impl<'a> ImageStripe<'a> {
    pub fn decode(bytes: &'a [u8]) -> Result<Self, Error> {
        if !(IMAGE_HEADER_BYTES..=MAX_IMAGE_PACKET_BYTES).contains(&bytes.len()) {
            return Err(Error::Size);
        }
        if &bytes[..4] != b"PCI1" || bytes[4] != 1 || bytes[5] != 1 {
            return Err(Error::Header);
        }
        let word = |at| u16::from_le_bytes([bytes[at], bytes[at + 1]]);
        let document_id = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let (width, rows, height, y) =
            (u16::from(bytes[6]), u16::from(bytes[7]), word(16), word(14));
        if document_id == 0
            || width == 0
            || usize::from(width) > IMAGE_WIDTH
            || height == 0
            || usize::from(height) > IMAGE_HEIGHT
            || y >= height
            || usize::from(y) % IMAGE_STRIPE_ROWS != 0
            || rows != (height - y).min(IMAGE_STRIPE_ROWS as u16)
        {
            return Err(Error::Header);
        }
        let pixels = &bytes[IMAGE_HEADER_BYTES..];
        if pixels.len() != usize::from(width) * usize::from(rows) * 2
            || usize::from(word(18)) != pixels.len()
        {
            return Err(Error::Size);
        }
        let checksum = u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
        if image_crc(pixels) != checksum {
            return Err(Error::Header);
        }
        Ok(Self {
            document_id,
            page_index: word(12),
            width,
            height,
            y,
            rows,
            pixels,
        })
    }
}

#[cfg(test)]
mod image_tests {
    use super::*;
    #[test]
    fn crc_and_each_truncation_corruption_and_header_bound() {
        assert_eq!(image_crc(b"123456789"), 0xcbf43926);
        let mut bytes = [0u8; 26];
        bytes[..8].copy_from_slice(&[80, 67, 73, 49, 1, 1, 1, 1]);
        bytes[8] = 7;
        bytes[16] = 1;
        bytes[18] = 2;
        bytes[24] = 0;
        bytes[25] = 248;
        bytes[20..24].copy_from_slice(&image_crc(&[0, 248]).to_le_bytes());
        let image = ImageStripe::decode(&bytes).unwrap();
        assert_eq!(image.pixels, &[0, 248]);
        for n in 0..bytes.len() {
            assert!(ImageStripe::decode(&bytes[..n]).is_err());
        }
        for (at, value) in [
            (4, 2),
            (5, 2),
            (6, 129),
            (7, 0),
            (8, 0),
            (14, 1),
            (16, 81),
            (18, 0),
            (25, 0),
        ] {
            let mut invalid = bytes;
            invalid[at] = value;
            assert!(ImageStripe::decode(&invalid).is_err());
        }
    }
}
