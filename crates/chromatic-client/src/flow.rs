// SPDX-License-Identifier: GPL-3.0-only
//! A small retained semantic tree and allocation-free 160px reader reflow.
use crate::{
    Error, MAX_PACKET_BYTES,
    box_css::{BACKGROUND, Style, UNDERLINE},
    rich,
};
pub const NODES: usize = 24;
pub const RESOURCES: usize = 384;
pub const WIDTH: usize = 160;
pub const HEIGHT: usize = 92;
pub const ROWS: usize = 8;
pub const PAINTS: usize = 8;
const NONE: u8 = 255;
pub const LINK: u8 = 22;
pub const ITEM: u8 = 23;
pub const IMAGE: u8 = 28;
fn tag(name: &str) -> u8 {
    [
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
        "ul",
        "ol",
        "figure",
        "img",
    ]
    .iter()
    .position(|s| name.eq_ignore_ascii_case(s))
    .map_or(0, |n| n as u8 + 1)
}
fn block(tag: u8) -> bool {
    matches!(tag,1..=6|12..=14|17..=21|23|25..=28)
}
#[derive(Clone, Copy)]
pub struct Node {
    pub start: u16,
    pub end: u16,
    pub parent: u8,
    pub kind: u8,
    pub number: u8,
    pub image_height: u8,
    pub style: Style,
    pub resource_start: u16,
    pub resource_len: u16,
}
impl Node {
    const EMPTY: Self = Self {
        start: 0,
        end: 0,
        parent: NONE,
        kind: 0,
        number: 0,
        image_height: 0,
        style: Style::DEFAULT,
        resource_start: 0,
        resource_len: 0,
    };
}
#[derive(Clone, Copy)]
pub struct Tree {
    pub nodes: [Node; NODES],
    pub count: u8,
    pub limited: u8,
    pub(crate) resources: [u8; RESOURCES],
    pub(crate) resource_len: u16,
    stack: [u8; 16],
    depth: u8,
    ignored: u8,
}
impl Tree {
    pub const EMPTY: Self = Self {
        nodes: [Node::EMPTY; NODES],
        count: 0,
        limited: 0,
        resources: [0; RESOURCES],
        resource_len: 0,
        stack: [0; 16],
        depth: 0,
        ignored: 0,
    };
    pub fn resource(&self, index: usize) -> &str {
        let Some(node) = self
            .nodes
            .get(index)
            .filter(|_| index < self.count as usize)
        else {
            return "";
        };
        core::str::from_utf8(
            &self.resources
                [node.resource_start as usize..(node.resource_start + node.resource_len) as usize],
        )
        .unwrap_or("")
    }
    fn close_from(&mut self, depth: usize, at: u16) {
        for &i in &self.stack[depth..self.depth as usize] {
            self.nodes[i as usize].end = at;
        }
        self.depth = depth as u8;
    }
    pub fn close(&mut self, name: &str, at: u16) {
        let kind = tag(name);
        if kind == 0 {
            return;
        }
        if self.ignored > 0 {
            self.ignored -= 1;
            return;
        }
        if let Some(depth) = self.stack[..self.depth as usize]
            .iter()
            .rposition(|&i| self.nodes[i as usize].kind == kind)
        {
            self.close_from(depth, at);
        }
    }
    pub fn open(
        &mut self,
        name: &str,
        attributes: &str,
        at: u16,
        sheet: &crate::css::Sheet,
    ) -> Option<u8> {
        let kind = tag(name);
        if kind == 0 {
            return None;
        }
        // Common optional HTML end tags; a child list's item cannot close its parent item.
        if block(kind) && kind != IMAGE {
            if let Some(depth) = self.stack[..self.depth as usize]
                .iter()
                .rposition(|&i| self.nodes[i as usize].kind == 17)
            {
                self.close_from(depth, at);
            }
        }
        if kind == ITEM {
            let list = self.stack[..self.depth as usize]
                .iter()
                .rposition(|&i| matches!(self.nodes[i as usize].kind, 25 | 26));
            if let Some(depth) = self.stack[..self.depth as usize]
                .iter()
                .enumerate()
                .rfind(|(d, i)| {
                    self.nodes[**i as usize].kind == ITEM && list.is_none_or(|l| *d > l)
                })
                .map(|(d, _)| d)
            {
                self.close_from(depth, at);
            }
        }
        if self.ignored > 0
            || self.depth as usize == self.stack.len()
            || self.count as usize == NODES
        {
            self.limited = 1;
            if kind != IMAGE {
                self.ignored = self.ignored.saturating_add(1);
            }
            return None;
        }
        let parent = self.stack[..self.depth as usize]
            .last()
            .copied()
            .unwrap_or(NONE);
        let mut style = Style::inherit(if parent == NONE {
            Style::DEFAULT
        } else {
            self.nodes[parent as usize].style
        });
        if matches!(kind, 1..=6 | 17 | 19 | 27) {
            style.margin[0] = 3;
            style.margin[2] = 3;
        }
        if kind == LINK {
            style.color = 0x1b9d;
            style.flags |= UNDERLINE;
        }
        if kind == IMAGE {
            style.stroke = 1;
            style.border = 0x9cf3;
            style.padding = [2; 4];
        }
        style = sheet.paint(
            name,
            crate::local::attribute(attributes, "id").unwrap_or(""),
            crate::local::attribute(attributes, "class").unwrap_or(""),
            crate::local::attribute(attributes, "style").unwrap_or(""),
            style,
        );
        let mut node = Node {
            start: at,
            end: u16::MAX,
            parent,
            kind,
            style,
            ..Node::EMPTY
        };
        if kind == IMAGE {
            node.image_height = crate::local::attribute(attributes, "height")
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or(32)
                .clamp(18, 48) as u8;
        }
        if kind == ITEM {
            if let Some(&list) = self.stack[..self.depth as usize]
                .iter()
                .rev()
                .find(|&&i| matches!(self.nodes[i as usize].kind, 25 | 26))
            {
                if self.nodes[list as usize].kind == 26 {
                    self.nodes[list as usize].number =
                        self.nodes[list as usize].number.saturating_add(1);
                    node.number = self.nodes[list as usize].number;
                }
            }
        }
        if matches!(kind, LINK | IMAGE) {
            let key = if kind == LINK { "href" } else { "src" };
            if let Some(value) = crate::local::attribute(attributes, key) {
                let n = value.len();
                let start = self.resource_len as usize;
                if n <= 255 && start + n <= RESOURCES && !value.chars().any(char::is_control) {
                    self.resources[start..start + n].copy_from_slice(value.as_bytes());
                    node.resource_start = start as u16;
                    node.resource_len = n as u16;
                    self.resource_len += n as u16;
                } else {
                    self.limited = 1;
                }
            }
        }
        let index = self.count;
        self.nodes[index as usize] = node;
        self.count += 1;
        self.stack[self.depth as usize] = index;
        self.depth += 1;
        Some(index)
    }
    pub fn finish(&mut self, at: u16) {
        self.close_from(0, at);
        self.ignored = 0;
        for node in &mut self.nodes[..self.count as usize] {
            node.start = node.start.min(at);
            node.end = node.end.min(at).max(node.start);
        }
    }
    fn deepest(&self, at: usize, blocks: bool) -> Option<usize> {
        self.nodes[..self.count as usize]
            .iter()
            .enumerate()
            .rev()
            .find(|(_, n)| {
                n.start as usize <= at && at < (n.end as usize) && (!blocks || block(n.kind))
            })
            .map(|(i, _)| i)
    }
    fn paint_at(&self, at: usize) -> Style {
        let mut index = self.deepest(at, false);
        let mut style = index.map_or(Style::DEFAULT, |i| self.nodes[i].style);
        for _ in 0..NODES {
            let Some(i) = index else { break };
            let node = &self.nodes[i];
            if node.style.flags & BACKGROUND != 0 {
                style.background = node.style.background;
                style.flags |= BACKGROUND;
                break;
            }
            index = (node.parent != NONE).then_some(node.parent as usize);
        }
        style
    }
    pub fn first_link(&self, at: usize) -> Option<usize> {
        self.nodes[..self.count as usize]
            .iter()
            .enumerate()
            .find(|(_, n)| n.kind == LINK && n.start as usize <= at && at < (n.end as usize))
            .map(|(i, _)| i)
    }
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Paint {
    pub color: u16,
    pub background: u16,
    pub flags: u8,
}
#[derive(Clone, Copy)]
struct Line {
    text: [u8; 64],
    fonts: [u8; 28],
    paints: [Paint; 28],
    len: usize,
    glyphs: usize,
    x: u8,
    width: u8,
    height: u8,
    text_y: u8,
    stroke: u8,
    edges: u8,
    background: u16,
    border: u16,
    flags: u8,
    at: usize,
    box_x: u8,
    box_width: u8,
}
impl Line {
    const EMPTY: Self = Self {
        text: [0; 64],
        fonts: [0; 28],
        paints: [Paint {
            color: 0x2104,
            background: 0xffff,
            flags: 0,
        }; 28],
        len: 0,
        glyphs: 0,
        x: 0,
        width: 160,
        height: 6,
        text_y: 0,
        stroke: 0,
        edges: 0,
        background: 0xffff,
        border: 0,
        flags: 0,
        at: 0,
        box_x: 0,
        box_width: 160,
    };
    fn bytes(&self) -> usize {
        16 + self.len + self.glyphs
    }
}
#[derive(Clone)]
struct Lines<'a> {
    body: &'a str,
    runs: &'a [rich::Run],
    tree: &'a Tree,
    at: usize,
    marker: u8,
    entered: Option<usize>,
    previous: Option<usize>,
    first: bool,
}
impl Lines<'_> {
    fn font(&self, at: usize) -> u8 {
        self.runs
            .iter()
            .rev()
            .find(|r| r.start as usize <= at)
            .map_or(0, |r| r.style)
    }
}
impl Iterator for Lines<'_> {
    type Item = Line;
    fn next(&mut self) -> Option<Line> {
        while self.body.as_bytes().get(self.at) == Some(&b'\n') {
            self.at += 1;
        }
        if self.at == self.body.len() {
            let label = match self.marker {
                m if m & 1 != 0 => "[Text limit reached]",
                m if m & 2 != 0 => "[Style limit reached]",
                m if m & 4 != 0 => "[Structure limit reached]",
                _ => return None,
            };
            let bit = if self.marker & 1 != 0 {
                1
            } else if self.marker & 2 != 0 {
                2
            } else {
                4
            };
            self.marker &= !bit;
            let mut line = Line::EMPTY;
            line.height = 12;
            for c in label.bytes() {
                line.text[line.len] = c;
                line.fonts[line.glyphs] = rich::CAPTION;
                line.len += 1;
                line.glyphs += 1;
            }
            return Some(line);
        }
        let mut line = Line {
            at: self.at,
            ..Line::EMPTY
        };
        let block_index = self.tree.deepest(self.at, true);
        if block_index != self.entered || self.first {
            self.first = false;
            self.entered = block_index;
            let gap = block_index.map_or(0, |i| self.tree.nodes[i].style.margin[0])
                + self
                    .previous
                    .map_or(0, |i| self.tree.nodes[i].style.margin[2]);
            if gap > 0 {
                return Some(Line {
                    height: gap,
                    ..Line::EMPTY
                });
            }
        }
        let mut left = 0usize;
        let mut right = 0usize;
        let mut ancestor = block_index;
        // Parent indices always precede their children; bound traversal even for corrupted internal trees.
        for _ in 0..NODES {
            let Some(i) = ancestor else { break };
            let n = &self.tree.nodes[i];
            left += usize::from(n.style.margin[3] + n.style.padding[3] + n.style.stroke);
            right += usize::from(n.style.margin[1] + n.style.padding[1] + n.style.stroke);
            if matches!(n.kind, 25 | 26) {
                left += 8;
            }
            if line.flags & BACKGROUND == 0 && n.style.flags & BACKGROUND != 0 {
                line.flags |= BACKGROUND;
                line.background = n.style.background;
            }
            if line.stroke == 0 && n.style.stroke > 0 {
                line.stroke = n.style.stroke;
                line.border = n.style.border;
            }
            ancestor = (n.parent != NONE).then_some(n.parent as usize);
        }
        left = left.min(60);
        right = right.min(60);
        let available = WIDTH - left - right;
        line.x = left as u8;
        line.width = available as u8;
        let (pad_left, pad_right) = block_index.map_or((0, 0), |i| {
            let style = self.tree.nodes[i].style;
            (
                usize::from(style.padding[3] + style.stroke),
                usize::from(style.padding[1] + style.stroke),
            )
        });
        line.box_x = left.saturating_sub(pad_left) as u8;
        line.box_width =
            (available + left.min(pad_left) + pad_right).min(WIDTH - line.box_x as usize) as u8;
        let mut tallest = 0usize;
        let mut width = 0;
        let mut space = None;
        while self.at < self.body.len() {
            let c = self.body[self.at..].chars().next().unwrap();
            if c == '\n' {
                self.at += 1;
                break;
            }
            let next_block = self.tree.deepest(self.at, true);
            if line.glyphs > 0 && next_block != block_index {
                break;
            }
            let font = self.font(self.at);
            let advance = rich::advance(c, font);
            if width + advance > available || line.glyphs == 28 || line.len + c.len_utf8() > 64 {
                if let Some((at, len, glyphs)) = space {
                    self.at = at;
                    line.len = len;
                    line.glyphs = glyphs;
                }
                break;
            }
            if c == ' ' {
                space = Some((self.at + 1, line.len, line.glyphs));
            }
            let style = self.tree.paint_at(self.at);
            line.paints[line.glyphs] = Paint {
                color: style.color,
                background: style.background,
                flags: style.flags,
            };
            line.fonts[line.glyphs] = font;
            c.encode_utf8(&mut line.text[line.len..line.len + c.len_utf8()]);
            line.len += c.len_utf8();
            line.glyphs += 1;
            width += advance;
            tallest = tallest.max(rich::height(font));
            self.at += c.len_utf8();
        }
        if line.len > 0 && line.text[line.len - 1] == b' ' {
            line.len -= 1;
            line.glyphs -= 1;
        }
        let mut top = 0;
        let mut bottom = 0;
        if let Some(i) = block_index {
            let n = &self.tree.nodes[i];
            let first = line.at == n.start as usize;
            let last = self.at >= n.end as usize;
            if first {
                top = usize::from(n.style.padding[0] + n.style.stroke);
                line.edges |= 1;
            }
            if last {
                bottom = usize::from(n.style.padding[2] + n.style.stroke);
                line.edges |= 2;
            }
            if n.kind == IMAGE {
                line.flags |= 4;
                if first {
                    tallest = tallest.max(n.image_height as usize);
                }
            }
        }
        self.previous = block_index;
        line.text_y = top as u8;
        line.height = (if line.glyphs == 0 { 6 } else { tallest } + top + bottom).min(HEIGHT) as u8;
        Some(line)
    }
}
#[derive(Clone, Copy, Default, Debug)]
pub struct Row<'a> {
    pub text: &'a str,
    pub codes: &'a [u8],
    pub x: u8,
    pub y: u8,
    pub width: u8,
    pub height: u8,
    pub text_y: u8,
    pub stroke: u8,
    pub edges: u8,
    pub flags: u8,
    pub background: u16,
    pub border: u16,
    pub box_x: u8,
    pub box_width: u8,
}
pub struct Packet<'a> {
    pub document_id: u32,
    pub position: u16,
    pub positions: u16,
    pub link_target: u32,
    pub title: &'a str,
    pub palette: &'a [u8],
    pub rows: [Row<'a>; ROWS],
    pub row_count: usize,
}
impl<'a> Packet<'a> {
    pub fn decode(bytes: &'a [u8]) -> Result<Self, Error> {
        if !(24..=MAX_PACKET_BYTES).contains(&bytes.len()) {
            return Err(Error::Size);
        }
        if &bytes[..4] != b"PFW1" || bytes[4] != 1 || bytes[19] & !1 != 0 {
            return Err(Error::Header);
        }
        let id = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        let position = u16::from_le_bytes(bytes[12..14].try_into().unwrap());
        let positions = u16::from_le_bytes(bytes[14..16].try_into().unwrap());
        if id == 0 || positions == 0 || position >= positions {
            return Err(Error::Pagination);
        }
        let count = bytes[6] as usize;
        let paints = bytes[18] as usize;
        if !(1..=ROWS).contains(&count)
            || !(1..=PAINTS).contains(&paints)
            || bytes[5] != u8::from(position > 0) | (u8::from(position + 1 < positions) << 1)
            || u16::from_le_bytes(bytes[16..18].try_into().unwrap()) as usize != bytes.len() - 24
        {
            return Err(Error::Header);
        }
        let mut at = 24 + bytes[7] as usize;
        let title = crate::text(bytes.get(24..at).ok_or(Error::Size)?)?;
        let palette = bytes.get(at..at + paints * 5).ok_or(Error::Size)?;
        at += paints * 5;
        if palette.chunks_exact(5).any(|p| p[4] & !3 != 0) {
            return Err(Error::Text);
        }
        let mut rows = [Row::default(); ROWS];
        let mut bottom = 0;
        for row in &mut rows[..count] {
            let d = bytes.get(at..at + 16).ok_or(Error::Size)?;
            at += 16;
            let n = d[2] as usize;
            let glyphs = d[3] as usize;
            if d[0] as usize != bottom
                || d[1] == 0
                || bottom + d[1] as usize > HEIGHT
                || n > 64
                || glyphs > 28
                || d[4] as usize + d[5] as usize > WIDTH
                || d[5] == 0
                || d[6] >= d[1]
                || d[7] > 3
                || d[8] & !3 != 0
                || d[9] & !5 != 0
                || d[15] == 0
                || d[14] as usize + d[15] as usize > WIDTH
                || d[4] < d[14]
                || d[4] as usize + d[5] as usize > d[14] as usize + d[15] as usize
            {
                return Err(Error::Text);
            }
            let text = core::str::from_utf8(bytes.get(at..at + n).ok_or(Error::Size)?)
                .map_err(|_| Error::Text)?;
            at += n;
            let codes = bytes.get(at..at + glyphs).ok_or(Error::Size)?;
            at += glyphs;
            if text.chars().count() != glyphs
                || text.chars().any(char::is_control)
                || codes.iter().any(|c| (c >> 4) as usize >= paints)
            {
                return Err(Error::Text);
            }
            let width: usize = text
                .chars()
                .zip(codes)
                .map(|(c, s)| rich::advance(c, s & 15))
                .sum();
            let tallest = text
                .chars()
                .zip(codes)
                .map(|(_, s)| rich::height(s & 15))
                .max()
                .unwrap_or(0);
            if width > d[5] as usize || d[6] as usize + tallest > d[1] as usize {
                return Err(Error::Text);
            }
            *row = Row {
                text,
                codes,
                x: d[4],
                y: d[0],
                width: d[5],
                height: d[1],
                text_y: d[6],
                stroke: d[7],
                edges: d[8],
                flags: d[9],
                background: u16::from_le_bytes(d[10..12].try_into().unwrap()),
                border: u16::from_le_bytes(d[12..14].try_into().unwrap()),
                box_x: d[14],
                box_width: d[15],
            };
            bottom += d[1] as usize;
        }
        if at != bytes.len() {
            return Err(Error::Size);
        }
        Ok(Self {
            document_id: id,
            position,
            positions,
            link_target: u32::from_le_bytes(bytes[20..24].try_into().unwrap()),
            title,
            palette,
            rows,
            row_count: count,
        })
    }
}
#[derive(Clone, Copy)]
pub(crate) struct Content<'a> {
    pub title: &'a str,
    pub body: &'a str,
    pub runs: &'a [rich::Run],
    pub tree: &'a Tree,
    pub marker: u8,
}
pub(crate) fn visible_image(content: Content<'_>, position: usize) -> Option<usize> {
    let mut pixels = 0;
    let mut bytes = 24 + crate::local::short_title(content.title).len() + PAINTS * 5;
    let lines = Lines {
        body: content.body,
        runs: content.runs,
        tree: content.tree,
        at: 0,
        entered: None,
        previous: None,
        first: true,
        marker: content.marker,
    };
    for (count, row) in lines.skip(position).enumerate() {
        if count == ROWS
            || pixels + row.height as usize > HEIGHT
            || bytes + row.bytes() > MAX_PACKET_BYTES
        {
            break;
        }
        if row.flags & 4 != 0 {
            return content.tree.nodes[..content.tree.count as usize]
                .iter()
                .position(|n| {
                    n.kind == IMAGE && row.at >= n.start as usize && row.at < n.end as usize
                });
        }
        pixels += row.height as usize;
        bytes += row.bytes();
    }
    None
}

pub(crate) fn positions(content: Content<'_>) -> usize {
    let base = 24 + crate::local::short_title(content.title).len() + PAINTS * 5;
    let lines = Lines {
        body: content.body,
        runs: content.runs,
        tree: content.tree,
        at: 0,
        entered: None,
        previous: None,
        first: true,
        marker: content.marker | if content.tree.limited != 0 { 4 } else { 0 },
    };
    let mut queue = [(0, 0); ROWS];
    let (mut total, mut used, mut pixels, mut bytes) = (0, 0, 0, base);
    for row in lines.clone() {
        let n = row.bytes();
        while used > 0
            && (used == ROWS
                || pixels + row.height as usize > HEIGHT
                || bytes + n > MAX_PACKET_BYTES)
        {
            pixels -= queue[0].0;
            bytes -= queue[0].1;
            queue.copy_within(1..used, 0);
            used -= 1;
        }
        queue[used] = (row.height as usize, n);
        used += 1;
        total += 1;
        pixels += row.height as usize;
        bytes += n;
    }
    if used == 0 { 0 } else { total - used + 1 }
}
pub(crate) fn encode(
    id: usize,
    content: Content<'_>,
    position: usize,
    out: &mut [u8],
    resolve: impl Fn(usize) -> u32,
) -> usize {
    let count = positions(content);
    encode_cached(id, content, position, count, out, resolve)
}
pub(crate) fn encode_cached(
    id: usize,
    content: Content<'_>,
    position: usize,
    positions: usize,
    out: &mut [u8],
    resolve: impl Fn(usize) -> u32,
) -> usize {
    let Content {
        title,
        body,
        runs,
        tree,
        marker,
    } = content;
    if out.len() < MAX_PACKET_BYTES {
        return 0;
    }
    let title = crate::local::short_title(title);
    let base = 24 + title.len() + PAINTS * 5;
    let lines = Lines {
        body,
        runs,
        tree,
        at: 0,
        entered: None,
        previous: None,
        first: true,
        marker: marker | if tree.limited != 0 { 4 } else { 0 },
    };
    if positions == 0 || position >= positions {
        return 0;
    }
    out[..base].fill(0);
    out[..4].copy_from_slice(b"PFW1");
    out[4] = 1;
    out[5] = u8::from(position > 0) | (u8::from(position + 1 < positions) << 1);
    out[7] = title.len() as u8;
    out[8..12].copy_from_slice(&(id as u32).to_le_bytes());
    out[12..14].copy_from_slice(&(position as u16).to_le_bytes());
    out[14..16].copy_from_slice(&(positions as u16).to_le_bytes());
    out[18] = PAINTS as u8;
    out[24..24 + title.len()].copy_from_slice(title.as_bytes());
    let mut palette = [Paint::default(); PAINTS];
    palette[0] = Paint {
        color: Style::DEFAULT.color,
        background: tree.nodes[..tree.count as usize]
            .iter()
            .find(|n| n.kind == 21 && n.style.flags & BACKGROUND != 0)
            .map_or(0xffff, |n| n.style.background),
        flags: 0,
    };
    let mut palette_used = 1;
    let (mut at, mut y, mut count) = (base, 0, 0);
    for row in lines.skip(position) {
        if count == ROWS || y + row.height as usize > HEIGHT || at + row.bytes() > MAX_PACKET_BYTES
        {
            break;
        }
        out[at..at + 16].copy_from_slice(&[
            y as u8,
            row.height,
            row.len as u8,
            row.glyphs as u8,
            row.x,
            row.width,
            row.text_y,
            row.stroke,
            row.edges,
            row.flags,
            row.background as u8,
            (row.background >> 8) as u8,
            row.border as u8,
            (row.border >> 8) as u8,
            row.box_x,
            row.box_width,
        ]);
        at += 16;
        out[at..at + row.len].copy_from_slice(&row.text[..row.len]);
        at += row.len;
        for i in 0..row.glyphs {
            let paint = row.paints[i];
            let index = palette[..palette_used]
                .iter()
                .position(|p| *p == paint)
                .unwrap_or_else(|| {
                    if palette_used == PAINTS {
                        out[19] |= 1;
                        0
                    } else {
                        let i = palette_used;
                        palette[i] = paint;
                        palette_used += 1;
                        i
                    }
                });
            out[at] = row.fonts[i] | ((index as u8) << 4);
            at += 1;
        }
        if out[20..24] == [0, 0, 0, 0] {
            if let Some(target) = tree.nodes[..tree.count as usize]
                .iter()
                .enumerate()
                .filter(|(_, n)| {
                    n.kind == LINK
                        && (n.start as usize) < row.at + row.len
                        && (n.end as usize) > row.at
                })
                .find_map(|(i, _)| {
                    let target = resolve(i);
                    (target > 0).then_some(target)
                })
            {
                out[20..24].copy_from_slice(&target.to_le_bytes());
            }
        }
        y += row.height as usize;
        count += 1;
    }
    for (i, p) in palette.iter().enumerate() {
        let off = 24 + title.len() + i * 5;
        out[off..off + 5].copy_from_slice(&[
            p.color as u8,
            (p.color >> 8) as u8,
            p.background as u8,
            (p.background >> 8) as u8,
            p.flags & 3,
        ]);
    }
    out[6] = count as u8;
    out[16..18].copy_from_slice(&((at - 24) as u16).to_le_bytes());
    at
}

/// Original layout diagnostic, separate from the retained search corpus.
pub fn probe(position: usize, out: &mut [u8]) -> usize {
    static SHEET: crate::css::Sheet = crate::css::Sheet::EMPTY;
    const BODY: &str = "Pocket Web\nA readable page with bold text and a source link.\n1. Keep the structure.\n2. Reflow to 160 pixels.\n- Read by scrolling.\n[Image: Source illustration]\nImage position and description stay together.\nEnd of the retained TEST page.";
    let mut tree = Tree::EMPTY;
    tree.open(
        "body",
        "body style='background:#f5f7fa;padding:4px'",
        0,
        &SHEET,
    );
    tree.open("h1", "h1 style='color:#174ea6;margin:0 0 6px'", 0, &SHEET);
    tree.close("h1", 10);
    let p = BODY.find("A readable").unwrap() as u16;
    tree.open(
        "p",
        "p style='background:white;border:1px solid #9aa0a6;padding:5px;margin:2px 0 6px'",
        p,
        &SHEET,
    );
    let link = BODY.find("source link").unwrap() as u16;
    tree.open("a", "a href='node2.html'", link, &SHEET);
    tree.close("a", link + 11);
    let list = BODY.find("1.").unwrap() as u16;
    tree.close("p", list - 1);
    tree.open("ol", "ol", list, &SHEET);
    tree.open("li", "li", list, &SHEET);
    tree.close("li", BODY.find("2.").unwrap() as u16 - 1);
    tree.open("li", "li", BODY.find("2.").unwrap() as u16, &SHEET);
    tree.open("ul", "ul", BODY.find("- Read").unwrap() as u16, &SHEET);
    tree.open("li", "li", BODY.find("- Read").unwrap() as u16, &SHEET);
    let image = BODY.find("[Image:").unwrap() as u16;
    tree.close("ol", image - 1);
    tree.open("figure", "figure", image, &SHEET);
    tree.open("img", "img src='article.jpg' height='32'", image, &SHEET);
    let caption = BODY.find("Image position").unwrap() as u16;
    tree.close("img", caption - 1);
    tree.open(
        "figcaption",
        "figcaption style='color:#5f6368'",
        caption,
        &SHEET,
    );
    let end = BODY.find("End of").unwrap() as u16;
    tree.close("figcaption", end - 1);
    tree.close("figure", end - 1);
    tree.open("p", "p", end, &SHEET);
    tree.finish(BODY.len() as u16);
    let bold = BODY.find("bold text").unwrap() as u16;
    let runs = [
        rich::Run {
            start: 0,
            style: rich::HEADING | rich::BOLD,
        },
        rich::Run { start: p, style: 0 },
        rich::Run {
            start: bold,
            style: rich::BOLD,
        },
        rich::Run {
            start: bold + 9,
            style: 0,
        },
        rich::Run {
            start: caption,
            style: rich::CAPTION,
        },
        rich::Run {
            start: end,
            style: 0,
        },
    ];
    encode(
        1,
        Content {
            title: "Flow layout TEST",
            body: BODY,
            runs: &runs,
            tree: &tree,
            marker: 0,
        },
        position,
        out,
        |_| 0,
    )
}
