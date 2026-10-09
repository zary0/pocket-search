// SPDX-License-Identifier: GPL-3.0-only
//! Host-side formatting for the bounded PCS1 text protocol.
use crate::SearchIndex;
use chromatic_client::{
    HEADER_BYTES, Kind, MAX_PACKET_BYTES, MAX_QUERY_BYTES, Packet, TEXT_COLUMNS, TEXT_ROWS, cells,
};

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    QueryTooLong,
    NotFound,
    TooManyPages,
    InvalidPacket,
}

fn wrap(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut used = 0;
    for c in text.chars() {
        if c == '\n' {
            lines.push(std::mem::take(&mut line));
            used = 0;
            continue;
        }
        let c = if c.is_control() { ' ' } else { c };
        let width = if c.is_ascii() { 1 } else { 2 };
        if used + width > TEXT_COLUMNS {
            if let Some(at) = line.rfind(' ').filter(|at| *at > 0) {
                let carry = line.split_off(at + 1);
                lines.push(std::mem::take(&mut line));
                line = carry;
                used = cells(&line);
            }
            if used + width > TEXT_COLUMNS {
                lines.push(std::mem::take(&mut line));
                used = 0;
            }
        }
        line.push(c);
        used += width;
    }
    if !line.is_empty() || lines.is_empty() {
        lines.push(line);
    }
    lines
}

fn title(text: &str) -> String {
    let clean: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    if cells(&clean) <= TEXT_COLUMNS {
        return clean;
    }
    let mut value = String::new();
    for c in clean.chars() {
        if cells(&value) + if c.is_ascii() { 1 } else { 2 } > TEXT_COLUMNS - 2 {
            break;
        }
        value.push(c);
    }
    value.push('…');
    value
}

fn encode(
    kind: Kind,
    document_id: usize,
    page_index: usize,
    page_count: usize,
    heading: &str,
    lines: &[String],
) -> Result<Vec<u8>, Error> {
    let document_id = u32::try_from(document_id).map_err(|_| Error::NotFound)?;
    let page_index = u16::try_from(page_index).map_err(|_| Error::TooManyPages)?;
    let page_count = u16::try_from(page_count).map_err(|_| Error::TooManyPages)?;
    if page_count == 0 || page_index >= page_count || lines.len() > TEXT_ROWS {
        return Err(Error::InvalidPacket);
    }
    let heading = title(heading);
    let mut bytes = vec![0; HEADER_BYTES];
    bytes[..4].copy_from_slice(b"PCS1");
    bytes[4] = kind as u8;
    bytes[5] = u8::from(page_index > 0) | (u8::from(page_index + 1 < page_count) << 1);
    bytes[6] = lines.len() as u8;
    bytes[7] = heading.len() as u8;
    bytes[8..12].copy_from_slice(&document_id.to_le_bytes());
    bytes[12..14].copy_from_slice(&page_index.to_le_bytes());
    bytes[14..16].copy_from_slice(&page_count.to_le_bytes());
    bytes.extend_from_slice(heading.as_bytes());
    for line in lines {
        bytes.push(u8::try_from(line.len()).map_err(|_| Error::InvalidPacket)?);
        bytes.extend_from_slice(line.as_bytes());
    }
    if bytes.len() > MAX_PACKET_BYTES {
        return Err(Error::InvalidPacket);
    }
    let length = (bytes.len() - HEADER_BYTES) as u16;
    bytes[16..18].copy_from_slice(&length.to_le_bytes());
    Packet::decode(&bytes).map_err(|_| Error::InvalidPacket)?;
    Ok(bytes)
}

impl SearchIndex {
    /// One search result per screen, with at most ten results per query.
    pub fn chromatic_result(&self, query: &str, result_index: usize) -> Result<Vec<u8>, Error> {
        if query.len() > MAX_QUERY_BYTES {
            return Err(Error::QueryTooLong);
        }
        let hits = self.search(query, 10);
        if hits.is_empty() && result_index == 0 {
            return encode(
                Kind::Result,
                0,
                0,
                1,
                "Pocket Search",
                &["No results".into()],
            );
        }
        let hit = hits.get(result_index).ok_or(Error::NotFound)?;
        let mut lines = wrap(&format!("{}\n{}", hit.url, hit.snippet));
        if lines.len() > TEXT_ROWS {
            lines.truncate(TEXT_ROWS);
            lines[TEXT_ROWS - 1] = "A: Read full page".into();
        }
        encode(
            Kind::Result,
            hit.document_id,
            result_index,
            hits.len(),
            &hit.title,
            &lines,
        )
    }

    /// Body pages reference the same document collection used for the results.
    pub fn chromatic_page(&self, document_id: usize, page_index: usize) -> Result<Vec<u8>, Error> {
        let index = document_id.checked_sub(1).ok_or(Error::NotFound)?;
        let document = self.documents.get(index).ok_or(Error::NotFound)?;
        let lines = wrap(&document.body);
        let page_count = lines.len().div_ceil(TEXT_ROWS);
        if page_count > u16::MAX as usize {
            return Err(Error::TooManyPages);
        }
        let page = lines
            .chunks(TEXT_ROWS)
            .nth(page_index)
            .ok_or(Error::NotFound)?;
        encode(
            Kind::Reader,
            document_id,
            page_index,
            page_count,
            &document.title,
            page,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Document;
    use chromatic_client::{Action, Button, Error as DecodeError};

    fn index(body: &str) -> SearchIndex {
        SearchIndex::new(vec![Document {
            url: "https://example.invalid/chromatic".into(),
            title: "Chromatic 日本語 reader guide".into(),
            body: body.into(),
            image_urls: vec![],
        }])
    }

    #[test]
    fn japanese_reader_pages_reconstruct_body_with_bounded_lines() {
        let body = "日本語の文章を小さな画面で読む。".repeat(40);
        let index = index(&body);
        let first = index.chromatic_page(1, 0).unwrap();
        let count = Packet::decode(&first).unwrap().page_count;
        assert!(count > 1);
        let mut reconstructed = String::new();
        for page_index in 0..count {
            let bytes = index.chromatic_page(1, usize::from(page_index)).unwrap();
            assert!(bytes.len() <= MAX_PACKET_BYTES);
            let page = Packet::decode(&bytes).unwrap();
            for line in &page.lines[..page.line_count] {
                assert!(cells(line) <= TEXT_COLUMNS);
                reconstructed.push_str(line);
            }
        }
        assert_eq!(reconstructed, body);
    }

    #[test]
    fn selection_opens_the_ranked_document_and_empty_results_do_not_open() {
        let index = index("Text");
        let result = index.chromatic_result("日本語", 0).unwrap();
        let packet = Packet::decode(&result).unwrap();
        assert_eq!(packet.action(Button::A), Action::Open(1));
        assert_eq!(packet.action(Button::Up), Action::None);
        assert_eq!(packet.action(Button::Down), Action::None);
        assert_eq!(packet.action(Button::B), Action::Back);
        let empty = index.chromatic_result("missing", 0).unwrap();
        assert_eq!(
            Packet::decode(&empty).unwrap().action(Button::A),
            Action::None
        );
        assert_eq!(index.chromatic_page(0, 0), Err(Error::NotFound));
        assert_eq!(index.chromatic_result("x", 1), Err(Error::NotFound));
        assert_eq!(
            index.chromatic_result(&"あ".repeat(33), 0),
            Err(Error::QueryTooLong)
        );
    }

    #[test]
    fn reader_navigation_stops_at_both_ends() {
        let index = index(&"Reading ".repeat(60));
        let bytes = index.chromatic_page(1, 0).unwrap();
        let first = Packet::decode(&bytes).unwrap();
        assert_eq!(first.action(Button::Left), Action::None);
        assert_eq!(first.action(Button::Right), Action::Next);
        let bytes = index
            .chromatic_page(1, usize::from(first.page_count - 1))
            .unwrap();
        let last = Packet::decode(&bytes).unwrap();
        assert_eq!(last.action(Button::Right), Action::None);
        assert_eq!(last.action(Button::Left), Action::Previous);
        assert_eq!(last.action(Button::Select), Action::Home);
    }

    #[test]
    fn ordinary_english_words_stay_together_without_losing_body_text() {
        let body =
            "Original fixture for a handheld reader. Search terms come from the independent index.";
        let index = index(body);
        let bytes = index.chromatic_page(1, 0).unwrap();
        let packet = Packet::decode(&bytes).unwrap();
        let lines = &packet.lines[..packet.line_count];
        assert_eq!(lines.concat(), body);
        for word in body.split_whitespace() {
            assert!(lines.iter().any(|line| line.contains(word)));
        }
    }

    #[test]
    fn malformed_or_truncated_packets_never_decode() {
        let good = index("Text").chromatic_page(1, 0).unwrap();
        for end in 0..good.len() {
            assert!(Packet::decode(&good[..end]).is_err());
        }
        for (offset, value) in [
            (0, 0),
            (4, 99),
            (5, 128),
            (6, 8),
            (7, 255),
            (14, 0),
            (18, 1),
        ] {
            let mut corrupt = good.clone();
            corrupt[offset] = value;
            assert!(Packet::decode(&corrupt).is_err());
        }
        let mut invalid_utf8 = good.clone();
        invalid_utf8[HEADER_BYTES] = 255;
        assert_eq!(
            Packet::decode(&invalid_utf8).unwrap_err(),
            DecodeError::Text
        );
        let mut control = good.clone();
        control[HEADER_BYTES] = 27;
        assert_eq!(Packet::decode(&control).unwrap_err(), DecodeError::Text);
        let mut trailing = good.clone();
        trailing.push(0);
        assert!(Packet::decode(&trailing).is_err());
        assert_eq!(
            Packet::decode(&[0; MAX_PACKET_BYTES + 1]).unwrap_err(),
            DecodeError::Size
        );
    }

    #[test]
    fn non_bmp_text_and_controls_fit_without_splitting_utf8() {
        let index = index(&format!("{}\n\tend", "🦀".repeat(80)));
        let bytes = index.chromatic_page(1, 0).unwrap();
        let packet = Packet::decode(&bytes).unwrap();
        assert!(packet.lines.iter().all(|line| line.len() <= 48));
        assert!(
            packet
                .lines
                .iter()
                .all(|line| !line.chars().any(char::is_control))
        );
        assert!(packet.title.ends_with('…'));
    }
}
