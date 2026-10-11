// SPDX-License-Identifier: GPL-3.0-only
//! Versioned retained content, not Rust memory images. Validate before publishing.
use super::*;
use crate::{
    box_css::Style,
    flow::{NODES, Node, RESOURCES, Tree},
};
pub const SNAPSHOT_BYTES: usize = 18432;
struct Writer<'a> {
    bytes: &'a mut [u8],
    at: usize,
}
impl Writer<'_> {
    fn put(&mut self, bytes: &[u8]) -> Option<()> {
        let end = self.at.checked_add(bytes.len())?;
        self.bytes.get_mut(self.at..end)?.copy_from_slice(bytes);
        self.at = end;
        Some(())
    }
    fn byte(&mut self, v: u8) -> Option<()> {
        self.put(&[v])
    }
    fn short(&mut self, v: u16) -> Option<()> {
        self.put(&v.to_le_bytes())
    }
    fn word(&mut self, v: u32) -> Option<()> {
        self.put(&v.to_le_bytes())
    }
    fn text(&mut self, s: &str) -> Option<()> {
        self.short(s.len() as u16)?;
        self.put(s.as_bytes())
    }
}
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    fn get(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.at.checked_add(n)?;
        let b = self.bytes.get(self.at..end)?;
        self.at = end;
        Some(b)
    }
    fn byte(&mut self) -> Option<u8> {
        Some(self.get(1)?[0])
    }
    fn short(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.get(2)?.try_into().ok()?))
    }
    fn word(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.get(4)?.try_into().ok()?))
    }
    fn text(&mut self, max: usize, newlines: bool) -> Option<&'a str> {
        let n = self.short()? as usize;
        if n > max {
            return None;
        }
        let s = core::str::from_utf8(self.get(n)?).ok()?;
        if s.chars()
            .any(|c| c.is_control() && !(newlines && c == '\n'))
        {
            return None;
        }
        Some(s)
    }
}
fn assign<const N: usize>(out: &mut Text<N>, s: &str) {
    out.bytes[..s.len()].copy_from_slice(s.as_bytes());
    out.len = s.len() as u16;
}
impl LocalSearch {
    pub fn snapshot(&self, out: &mut [u8]) -> Option<usize> {
        if self.active != 0 || self.updating != 0 {
            return None;
        }
        let mut w = Writer { bytes: out, at: 0 };
        w.put(b"PSC2")?;
        w.put(&[2, self.profile, self.count, self.queued, self.cursor])?;
        for d in &self.documents[..self.count()] {
            w.text(d.title.as_str())?;
            w.text(d.body.as_str())?;
            w.text(d.url.as_str())?;
            w.byte(d.limited)?;
            w.byte(d.style_limited)?;
            w.word(d.source_bytes)?;
            w.word(d.fetched_at)?;
            w.short(d.http_status)?;
            w.word(d.checked_at)?;
            w.text(d.etag.as_str())?;
            w.text(d.modified.as_str())?;
            w.byte(d.run_count)?;
            for run in &d.runs[..d.run_count as usize] {
                w.short(run.start)?;
                w.byte(run.style)?;
            }
            w.byte(d.tree.count)?;
            w.byte(d.tree.limited)?;
            w.short(d.tree.resource_len)?;
            w.put(&d.tree.resources[..d.tree.resource_len as usize])?;
            for n in &d.tree.nodes[..d.tree.count as usize] {
                w.short(n.start)?;
                w.short(n.end)?;
                w.put(&[n.parent, n.kind, n.number, n.image_height])?;
                w.short(n.style.color)?;
                w.short(n.style.background)?;
                w.short(n.style.border)?;
                w.put(&n.style.margin)?;
                w.put(&n.style.padding)?;
                w.put(&[n.style.stroke, n.style.flags])?;
                w.short(n.resource_start)?;
                w.short(n.resource_len)?;
            }
        }
        for q in &self.queue[..self.queued as usize] {
            w.text(q.as_str())?;
        }
        (w.at <= SNAPSHOT_BYTES).then_some(w.at)
    }
    /// Restore only into an empty initialized collection. Corruption leaves it empty.
    pub fn restore_snapshot(&mut self, bytes: &[u8]) -> bool {
        if self.active != 0 || self.count != 0 || parse(bytes, None).is_none() {
            return false;
        }
        if parse(bytes, Some(self)).is_none() {
            self.set_profile(self.profile);
            return false;
        }
        self.hashes.fill(0);
        self.masks.fill(0);
        self.full_index = 0;
        for id in 0..self.count() {
            self.documents[id].flow_positions =
                crate::flow::positions(self.documents[id].content()) as u16;
            self.index_document(id);
        }
        true
    }
    pub fn pending(&self) -> bool {
        self.count() < MAX_DOCUMENTS && self.cursor < self.queued
    }
}
fn parse(bytes: &[u8], mut out: Option<&mut LocalSearch>) -> Option<()> {
    if bytes.len() > SNAPSHOT_BYTES {
        return None;
    }
    let mut r = Reader { bytes, at: 0 };
    if r.get(4)? != b"PSC2" || r.byte()? != 2 {
        return None;
    }
    let profile = r.byte()?;
    let count = r.byte()? as usize;
    let queued = r.byte()? as usize;
    let cursor = r.byte()? as usize;
    if profile as usize >= PROFILES.len()
        || count > MAX_DOCUMENTS
        || queued > FRONTIER
        || cursor > queued
        || count > cursor
    {
        return None;
    }
    if let Some(state) = out.as_deref_mut() {
        state.set_profile(profile);
    }
    for id in 0..count {
        let title = r.text(96, false)?;
        let body = r.text(BODY_BYTES, true)?;
        let url = r.text(URL_BYTES - 1, false)?;
        if body.is_empty() || !allowed_profile_url(profile, url) {
            return None;
        }
        let limited = r.byte()?;
        let style_limited = r.byte()?;
        let source_bytes = r.word()?;
        let fetched_at = r.word()?;
        let http_status = r.short()?;
        let checked_at = r.word()?;
        let etag = r.text(95, false)?;
        let modified = r.text(63, false)?;
        if limited > 1
            || style_limited > 1
            || source_bytes > 65536
            || !matches!(http_status, 0 | 200)
            || (http_status == 200 && fetched_at < 1609459200)
            || checked_at < fetched_at
            || !etag.is_ascii()
            || !modified.is_ascii()
        {
            return None;
        }
        if let Some(state) = out.as_deref_mut() {
            let d = &mut state.documents[id];
            *d = Document::EMPTY;
            assign(&mut d.title, title);
            assign(&mut d.body, body);
            assign(&mut d.url, url);
            d.limited = limited;
            d.style_limited = style_limited;
            d.source_bytes = source_bytes;
            d.fetched_at = fetched_at;
            d.http_status = http_status;
            d.checked_at = checked_at;
            assign(&mut d.etag, etag);
            assign(&mut d.modified, modified);
        }
        let runs = r.byte()? as usize;
        if runs > STYLE_RUNS {
            return None;
        }
        let mut previous = None;
        for i in 0..runs {
            let start = r.short()?;
            let style = r.byte()?;
            if start as usize >= body.len()
                || !body.is_char_boundary(start as usize)
                || style > 15
                || previous.is_some_and(|p| start <= p)
            {
                return None;
            }
            previous = Some(start);
            if let Some(state) = out.as_deref_mut() {
                state.documents[id].runs[i] = crate::rich::Run { start, style };
            }
        }
        let nodes = r.byte()? as usize;
        let tree_limited = r.byte()?;
        let resource_len = r.short()? as usize;
        if nodes > NODES || tree_limited > 1 || resource_len > RESOURCES {
            return None;
        }
        let resources = core::str::from_utf8(r.get(resource_len)?).ok()?;
        if resources.chars().any(char::is_control) {
            return None;
        }
        if let Some(state) = out.as_deref_mut() {
            let d = &mut state.documents[id];
            d.run_count = runs as u8;
            d.tree = Tree::EMPTY;
            d.tree.resources[..resource_len].copy_from_slice(resources.as_bytes());
            d.tree.resource_len = resource_len as u16;
            d.tree.count = nodes as u8;
            d.tree.limited = tree_limited;
        }
        for i in 0..nodes {
            let start = r.short()?;
            let end = r.short()?;
            let parent = r.byte()?;
            let kind = r.byte()?;
            let number = r.byte()?;
            let image_height = r.byte()?;
            let color = r.short()?;
            let background = r.short()?;
            let border = r.short()?;
            let margin: [u8; 4] = r.get(4)?.try_into().ok()?;
            let padding: [u8; 4] = r.get(4)?.try_into().ok()?;
            let stroke = r.byte()?;
            let flags = r.byte()?;
            let resource_start = r.short()?;
            let resource_len = r.short()?;
            let resource_end = resource_start as usize + resource_len as usize;
            if start > end
                || end as usize > body.len()
                || !body.is_char_boundary(start as usize)
                || !body.is_char_boundary(end as usize)
                || (parent != 255 && parent as usize >= i)
                || !(1..=28).contains(&kind)
                || image_height > 64
                || margin.iter().chain(padding.iter()).any(|v| *v > 12)
                || stroke > 3
                || flags > 3
                || resource_end > resources.len()
                || !resources.is_char_boundary(resource_start as usize)
                || !resources.is_char_boundary(resource_end)
            {
                return None;
            }
            if let Some(state) = out.as_deref_mut() {
                state.documents[id].tree.nodes[i] = Node {
                    start,
                    end,
                    parent,
                    kind,
                    number,
                    image_height,
                    style: Style {
                        color,
                        background,
                        border,
                        margin,
                        padding,
                        stroke,
                        flags,
                    },
                    resource_start,
                    resource_len,
                };
            }
        }
    }
    let mut queue_refs = [""; FRONTIER];
    for i in 0..queued {
        let url = r.text(URL_BYTES - 1, false)?;
        if !allowed_profile_url(profile, url) || queue_refs[..i].contains(&url) {
            return None;
        }
        queue_refs[i] = url;
        if let Some(state) = out.as_deref_mut() {
            assign(&mut state.queue[i], url);
        }
    }
    if r.at != bytes.len() {
        return None;
    }
    if let Some(state) = out {
        state.count = count as u8;
        state.queued = queued as u8;
        state.cursor = cursor as u8;
    }
    Some(())
}
#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    fn source() -> LocalSearch {
        let mut s = LocalSearch::new();
        s.set_profile(1);
        let mut url = [0; 128];
        assert!(s.next_url(&mut url) > 0);
        s.feed(b"<title>Saved</title><h1>Web</h1><p style='color:red'>persisted text</p><img src='photo.jpg' alt='source'><a href='next.html'>More</a>");
        assert!(s.metadata(1000, 1791680000, 200));
        assert!(s.finish(true));
        s
    }
    #[test]
    fn roundtrip_search_layout_provenance_and_frontier() {
        let s = source();
        let mut bytes = [0; SNAPSHOT_BYTES];
        let n = s.snapshot(&mut bytes).unwrap();
        let mut loaded = LocalSearch::new();
        assert!(loaded.restore_snapshot(&bytes[..n]));
        assert_eq!(loaded.count(), 1);
        assert_eq!(loaded.profile(), 1);
        assert!(loaded.pending());
        let mut a = [0; 512];
        let mut b = [0; 512];
        assert_eq!(
            s.search_packet("persisted", 0, &mut a),
            loaded.search_packet("persisted", 0, &mut b)
        );
        assert_eq!(a, b);
        assert_eq!(
            s.info_packet(1, 0, &mut a),
            loaded.info_packet(1, 0, &mut b)
        );
        assert_eq!(a, b);
        assert_eq!(
            s.flow_packet(1, 0, &mut a),
            loaded.flow_packet(1, 0, &mut b)
        );
        assert_eq!(a, b);
        let mut url = [0; 128];
        assert!(loaded.next_url(&mut url) > 0);
        assert!(core::str::from_utf8(&url).unwrap().contains("next.html"));
    }
    #[test]
    fn every_truncation_and_structural_mutations_reject_without_publishing() {
        let s = source();
        let mut bytes = [0; SNAPSHOT_BYTES];
        let n = s.snapshot(&mut bytes).unwrap();
        for end in 0..n {
            let mut out = LocalSearch::new();
            assert!(!out.restore_snapshot(&bytes[..end]));
            assert_eq!(out.count(), 0);
        }
        for i in [0, 4, 5, 6, 7, 8, 9, 10] {
            let mut copy = bytes;
            copy[i] = 255;
            let mut out = LocalSearch::new();
            assert!(!out.restore_snapshot(&copy[..n]));
            assert_eq!(out.count(), 0);
        }
        let mut out = LocalSearch::new();
        assert!(out.restore_snapshot(&bytes[..n]));
        assert!(!out.restore_snapshot(&bytes[..n]));
    }
    #[test]
    fn differential_update_preserves_old_on_failure_and_rebuilds_terms_on_success() {
        let mut s = source();
        let mut saved = SavedDocument(Document::EMPTY);
        let mut url = [0; 128];
        let mut packet = [0; 512];
        assert!(s.begin_update(1, &mut saved, &mut url) > 0);
        s.feed(b"<p>incomplete replacement</p>");
        assert!(s.restore_update(&saved, false, 0));
        assert!(s.search_packet("persisted", 0, &mut packet) > 0);
        let n = s.search_packet("persisted", 0, &mut packet);
        assert_eq!(crate::Packet::decode(&packet[..n]).unwrap().document_id, 1);
        assert!(s.begin_update(1, &mut saved, &mut url) > 0);
        assert!(s.restore_update(&saved, true, 1791680100));
        assert_eq!(s.documents[0].checked_at, 1791680100);
        assert_eq!(s.documents[0].fetched_at, 1791680000);
        assert!(s.begin_update(1, &mut saved, &mut url) > 0);
        s.feed(b"<title>Updated</title><p>newunique content</p>");
        assert!(s.validators("\"abc\"", "Sun, 11 Oct 2026 10:00:00 GMT"));
        assert!(s.metadata(2000, 1791680200, 200));
        assert!(s.finish(true));
        assert_eq!(s.count(), 1);
        let n = s.search_packet("newunique", 0, &mut packet);
        assert_eq!(crate::Packet::decode(&packet[..n]).unwrap().document_id, 1);
        let n = s.search_packet("persisted", 0, &mut packet);
        assert_eq!(crate::Packet::decode(&packet[..n]).unwrap().document_id, 0);
        let mut bytes = [0; SNAPSHOT_BYTES];
        let n = s.snapshot(&mut bytes).unwrap();
        let mut loaded = LocalSearch::new();
        assert!(loaded.restore_snapshot(&bytes[..n]));
        assert_eq!(loaded.documents[0].etag.as_str(), "\"abc\"");
        assert!(loaded.begin_update(1, &mut saved, &mut url) > 0);
        let mut header = [0; 96];
        assert_eq!(LocalSearch::update_header(&saved, false, &mut header), 5);
        assert!(loaded.restore_update(&saved, false, 0));
    }
    #[test]
    fn cancellation_requeues_unpublished_page_and_snapshot_keeps_resume_position() {
        let mut s = source();
        let mut url = [0; 128];
        assert!(s.next_url(&mut url) > 0);
        let expected = url;
        s.feed(b"<p>incomplete</p>");
        s.cancel_transfer();
        let mut bytes = [0; SNAPSHOT_BYTES];
        let n = s.snapshot(&mut bytes).unwrap();
        let mut loaded = LocalSearch::new();
        assert!(loaded.restore_snapshot(&bytes[..n]));
        assert!(loaded.next_url(&mut url) > 0);
        assert_eq!(url, expected);
    }
    #[test]
    fn maximum_retained_fields_fit_snapshot_budget() {
        let mut s = LocalSearch::new();
        s.set_profile(0);
        s.count = 4;
        s.cursor = 4;
        s.queued = 4;
        for i in 0..4 {
            let url = [
                SEED,
                "https://www-formal.stanford.edu/jmc/whatisai/node2.html",
                "https://www-formal.stanford.edu/jmc/whatisai/node3.html",
                "https://www-formal.stanford.edu/jmc/whatisai/node4.html",
            ][i];
            s.queue[i].set(url);
            let d = &mut s.documents[i];
            d.url.set(url);
            d.title.bytes.fill(b'T');
            d.title.len = 96;
            d.body.bytes.fill(b'x');
            d.body.len = BODY_BYTES as u16;
            d.etag.bytes.fill(b'e');
            d.etag.len = 95;
            d.modified.bytes.fill(b'm');
            d.modified.len = 63;
            d.run_count = 48;
            for j in 0..48 {
                d.runs[j] = crate::rich::Run {
                    start: j as u16,
                    style: 15,
                };
            }
            d.tree.count = NODES as u8;
            d.tree.resource_len = RESOURCES as u16;
            d.tree.resources.fill(b'r');
            for j in 0..NODES {
                d.tree.nodes[j] = Node {
                    start: 0,
                    end: BODY_BYTES as u16,
                    parent: 255,
                    kind: 17,
                    number: 0,
                    image_height: 0,
                    style: Style::DEFAULT,
                    resource_start: 0,
                    resource_len: RESOURCES as u16,
                };
            }
        }
        let mut bytes = [0; SNAPSHOT_BYTES];
        let n = s.snapshot(&mut bytes).unwrap();
        assert!(n < SNAPSHOT_BYTES);
        let mut loaded = LocalSearch::new();
        assert!(loaded.restore_snapshot(&bytes[..n]));
        assert_eq!(loaded.count(), 4);
    }
    #[test]
    fn active_transfer_is_never_exported() {
        let mut s = source();
        let mut bytes = [0; SNAPSHOT_BYTES];
        let mut url = [0; 128];
        assert!(s.next_url(&mut url) > 0);
        assert!(s.snapshot(&mut bytes).is_none());
    }
}
