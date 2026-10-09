// SPDX-License-Identifier: GPL-3.0-only
//! Small, allocation-free building blocks for the Chromatic firmware adapter.
//! This crate has no display, network, allocator, or operating-system dependency.
#![no_std]
#![forbid(unsafe_code)]

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Result = 1,
    Reader = 2,
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
            _ => return Err(Error::Header),
        };
        let document_id = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let page_index = u16::from_le_bytes([bytes[12], bytes[13]]);
        let page_count = u16::from_le_bytes([bytes[14], bytes[15]]);
        if page_count == 0 || page_index >= page_count || (kind == Kind::Reader && document_id == 0)
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
        let title_end = HEADER_BYTES + usize::from(bytes[7]);
        let title = text(bytes.get(HEADER_BYTES..title_end).ok_or(Error::Size)?)?;
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
            Button::Up if self.kind == Kind::Result && self.page_index > 0 => Action::Previous,
            Button::Down
                if self.kind == Kind::Result
                    && self.page_index.saturating_add(1) < self.page_count =>
            {
                Action::Next
            }
            Button::Left if self.kind == Kind::Reader && self.page_index > 0 => Action::Previous,
            Button::Right
                if self.kind == Kind::Reader
                    && self.page_index.saturating_add(1) < self.page_count =>
            {
                Action::Next
            }
            _ => Action::None,
        }
    }
}
