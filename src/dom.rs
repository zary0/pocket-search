// SPDX-License-Identifier: GPL-3.0-only
//! HTML5 DOM to immutable, bounded reading scenes. No scripts or URL fetching.
use crate::{Document, SearchIndex, chromatic};
use chromatic_client::{IMAGE_HEIGHT, IMAGE_STRIPE_ROWS, IMAGE_WIDTH, Kind, TEXT_ROWS, image_crc};
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits, RgbImage};
use scraper::{ElementRef, Html, Node, Selector};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    io::{Cursor, Read},
    path::{Component, Path},
};
use url::Url;

pub struct ImageAsset {
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<u8>,
    pub png: Vec<u8>,
}
pub struct Page {
    pub kind: Kind,
    pub style: &'static str,
    pub heading_level: u8,
    pub lines: Vec<String>,
    pub link_target: u32,
    pub source: String,
    pub asset: Option<usize>,
}
pub struct DomDocument {
    pub pages: Vec<Page>,
    pub images: Vec<ImageAsset>,
}
#[derive(Debug)]
pub enum Block {
    Text {
        style: &'static str,
        level: u8,
        text: String,
    },
    Image {
        alt: String,
        source: String,
    },
    Link {
        text: String,
        source: String,
    },
}
pub struct Parsed {
    pub title: String,
    pub blocks: Vec<Block>,
}

fn selector(value: &str) -> Selector {
    Selector::parse(value).expect("constant selector")
}
fn safe_url(base: &Url, value: &str) -> Option<String> {
    let url = base.join(value).ok()?;
    (url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.host_str().is_some()
        && url.port().is_none()
        && url.as_str().len() <= 1024)
        .then(|| url.to_string())
}
fn ignored(el: ElementRef<'_>) -> bool {
    matches!(
        el.value().name(),
        "script"
            | "style"
            | "template"
            | "iframe"
            | "object"
            | "embed"
            | "svg"
            | "canvas"
            | "nav"
            | "footer"
            | "aside"
    ) || el.value().attr("hidden").is_some()
        || el.value().attr("aria-hidden") == Some("true")
        || el.value().attr("style").is_some_and(|s| {
            s.chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>()
                .to_ascii_lowercase()
                .contains("display:none")
        })
}
fn plain(el: ElementRef<'_>, depth: usize) -> Result<String, String> {
    if depth > 64 {
        return Err("DOM depth limit".into());
    }
    if ignored(el) {
        return Ok(String::new());
    }
    let mut out = String::new();
    for child in el.children() {
        match child.value() {
            Node::Text(value) => out.push_str(&value.text),
            Node::Element(_) => {
                if let Some(el) = ElementRef::wrap(child) {
                    out.push_str(&plain(el, depth + 1)?);
                }
            }
            _ => {}
        }
    }
    Ok(out)
}
struct Collector<'a> {
    base: &'a Url,
    blocks: Vec<Block>,
    text: String,
    style: &'static str,
    level: u8,
}
impl Collector<'_> {
    fn flush(&mut self) -> Result<(), String> {
        let value = if self.style == "pre" {
            self.text.trim_matches('\n').to_string()
        } else {
            self.text.trim().to_string()
        };
        self.text.clear();
        if !value.trim().is_empty() {
            self.blocks.push(Block::Text {
                style: self.style,
                level: self.level,
                text: if self.style == "list" {
                    format!("- {value}")
                } else {
                    value
                },
            });
        }
        if self.blocks.len() > 256 {
            return Err("DOM block limit".into());
        }
        Ok(())
    }
    fn append(&mut self, text: &str) -> Result<(), String> {
        for c in text.chars() {
            if self.style == "pre" {
                self.text
                    .push(if c.is_control() && c != '\n' { ' ' } else { c });
            } else if c.is_whitespace() || c.is_control() {
                if !self.text.is_empty() && !self.text.ends_with(' ') {
                    self.text.push(' ');
                }
            } else {
                self.text.push(c);
            }
        }
        if self.text.len() > 65536 {
            return Err("DOM text limit".into());
        }
        Ok(())
    }
    fn walk(&mut self, el: ElementRef<'_>, depth: usize) -> Result<(), String> {
        if depth > 64 {
            return Err("DOM depth limit".into());
        }
        if ignored(el) {
            return Ok(());
        }
        let name = el.value().name();
        if name == "img" {
            self.flush()?;
            let alt = el
                .value()
                .attr("alt")
                .unwrap_or("Image")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            if let Some(source) = el
                .value()
                .attr("src")
                .and_then(|src| safe_url(self.base, src))
            {
                self.blocks.push(Block::Image { alt, source });
            } else {
                self.append(&format!("[Image: {alt}]"))?;
                self.flush()?;
            }
            return Ok(());
        }
        if name == "a"
            && let Some(source) = el
                .value()
                .attr("href")
                .and_then(|href| safe_url(self.base, href))
        {
            self.flush()?;
            let text = plain(el, depth)?
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            // Preserve linked images as images, while the link is a separate action.
            for child in el.select(&selector("img")) {
                if !child.ancestors().filter_map(ElementRef::wrap).any(ignored) {
                    self.walk(child, depth + 1)?;
                }
            }
            self.blocks.push(Block::Link {
                text: if text.is_empty() {
                    "Open source".into()
                } else {
                    text
                },
                source,
            });
            return Ok(());
        }
        if name == "br" {
            self.text.push('\n');
            return Ok(());
        }
        let block = matches!(
            name,
            "p" | "h1"
                | "h2"
                | "h3"
                | "h4"
                | "h5"
                | "h6"
                | "li"
                | "pre"
                | "blockquote"
                | "figcaption"
                | "div"
                | "section"
                | "article"
                | "tr"
        );
        let previous = (self.style, self.level);
        if block {
            self.flush()?;
            self.style = match name {
                "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => "heading",
                "li" => "list",
                "pre" => "pre",
                "blockquote" => "quote",
                "figcaption" => "caption",
                _ if matches!(previous.0, "list" | "quote") => previous.0,
                _ => "paragraph",
            };
            self.level = if self.style == "heading" {
                name.as_bytes()[1] - b'0'
            } else {
                0
            };
        }
        for child in el.children() {
            match child.value() {
                Node::Text(value) => self.append(&value.text)?,
                Node::Element(_) => {
                    if let Some(el) = ElementRef::wrap(child) {
                        self.walk(el, depth + 1)?;
                    }
                }
                _ => {}
            }
        }
        if block {
            self.flush()?;
            (self.style, self.level) = previous;
        }
        Ok(())
    }
}
pub fn parse(source: &str, url: &str) -> Result<Parsed, String> {
    if source.len() > 65536 {
        return Err("HTML exceeds 64 KiB".into());
    }
    let base = Url::parse(url).map_err(|_| "Invalid source URL")?;
    if safe_url(&base, "").is_none() {
        return Err("HTTPS source URL required".into());
    }
    let html = Html::parse_document(source);
    if html.tree.nodes().count() > 8192 {
        return Err("DOM node limit".into());
    }
    let title = html
        .select(&selector("title"))
        .next()
        .map(|el| plain(el, 0))
        .transpose()?
        .unwrap_or_else(|| "Untitled document".into())
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if title.len() > 1024 {
        return Err("Title limit".into());
    }
    let root = html
        .select(&selector("main"))
        .next()
        .or_else(|| html.select(&selector("article")).next())
        .or_else(|| html.select(&selector("body")).next())
        .ok_or("No document body")?;
    let mut collector = Collector {
        base: &base,
        blocks: Vec::new(),
        text: String::new(),
        style: "paragraph",
        level: 0,
    };
    if !root.ancestors().filter_map(ElementRef::wrap).any(ignored) {
        collector.walk(root, 0)?;
    }
    collector.flush()?;
    if collector.blocks.len() > 256 {
        return Err("DOM block limit".into());
    }
    Ok(Parsed {
        title,
        blocks: collector.blocks,
    })
}
pub fn prepare_image(bytes: &[u8]) -> Result<ImageAsset, String> {
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("Image source exceeds 2 MiB".into());
    }
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| "Invalid image")?;
    if !matches!(reader.format(), Some(ImageFormat::Png | ImageFormat::Jpeg)) {
        return Err("PNG or JPEG required".into());
    }
    let mut limits = Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(32 * 1024 * 1024);
    // Apply limits before metadata parsing and reuse the same decoder for pixels.
    reader.limits(limits);
    let decoder = reader
        .into_decoder()
        .map_err(|_| "Image dimensions/limits")?;
    let (w, h) = decoder.dimensions();
    if w == 0 || h == 0 || w > 4096 || h > 4096 || u64::from(w) * u64::from(h) > 4 * 1024 * 1024 {
        return Err("Image dimension limit".into());
    }
    let thumbnail = DynamicImage::from_decoder(decoder)
        .map_err(|_| "Image decode/limits")?
        .thumbnail(w.min(IMAGE_WIDTH as u32), h.min(IMAGE_HEIGHT as u32))
        .to_rgba8();
    let (w, h) = thumbnail.dimensions();
    let mut pixels = Vec::with_capacity((w * h * 2) as usize);
    let mut preview = RgbImage::new(w, h);
    for (x, y, p) in thumbnail.enumerate_pixels() {
        let blend = |c: u8| {
            ((u32::from(c) * u32::from(p[3]) + 255 * (255 - u32::from(p[3])) + 127) / 255) as u8
        };
        let (r, g, b) = (blend(p[0]), blend(p[1]), blend(p[2]));
        let value = (u16::from(r >> 3) << 11) | (u16::from(g >> 2) << 5) | u16::from(b >> 3);
        pixels.extend_from_slice(&value.to_le_bytes());
        let (r, g, b) = (
            (value >> 11) as u8,
            ((value >> 5) & 63) as u8,
            (value & 31) as u8,
        );
        preview.put_pixel(
            x,
            y,
            image::Rgb([
                (r << 3) | (r >> 2),
                (g << 2) | (g >> 4),
                (b << 3) | (b >> 2),
            ]),
        );
    }
    let mut png = Cursor::new(Vec::new());
    preview
        .write_to(&mut png, ImageFormat::Png)
        .map_err(|_| "Preview encoding")?;
    Ok(ImageAsset {
        width: w as u16,
        height: h as u16,
        pixels,
        png: png.into_inner(),
    })
}
impl ImageAsset {
    pub(crate) fn stream(&self, document: usize, page: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.pixels.len() + 240);
        for y in (0..usize::from(self.height)).step_by(IMAGE_STRIPE_ROWS) {
            let rows = (usize::from(self.height) - y).min(IMAGE_STRIPE_ROWS);
            let start = y * usize::from(self.width) * 2;
            let data = &self.pixels[start..start + rows * usize::from(self.width) * 2];
            let mut header = [0u8; 24];
            header[..4].copy_from_slice(b"PCI1");
            header[4] = 1;
            header[5] = 1;
            header[6] = self.width as u8;
            header[7] = rows as u8;
            header[8..12].copy_from_slice(&(document as u32).to_le_bytes());
            header[12..14].copy_from_slice(&(page as u16).to_le_bytes());
            header[14..16].copy_from_slice(&(y as u16).to_le_bytes());
            header[16..18].copy_from_slice(&self.height.to_le_bytes());
            header[18..20].copy_from_slice(&(data.len() as u16).to_le_bytes());
            header[20..24].copy_from_slice(&image_crc(data).to_le_bytes());
            out.extend_from_slice(&header);
            out.extend_from_slice(data);
        }
        out
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    root: String,
    pages: Vec<Registration>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    url: String,
    html: String,
    #[serde(default)]
    images: BTreeMap<String, String>,
}
fn file(root: &Path, value: &str, limit: usize) -> Result<Vec<u8>, String> {
    let path = Path::new(value);
    if path
        .components()
        .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
    {
        return Err("Only registered relative files are permitted".into());
    }
    let path = root
        .join(path)
        .canonicalize()
        .map_err(|_| "Registered file missing")?;
    if !path.starts_with(root) || !path.is_file() {
        return Err("Registered file outside corpus root".into());
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|_| "Registered file unreadable")?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Registered file read")?;
    if bytes.len() > limit {
        return Err("Registered file size limit".into());
    }
    Ok(bytes)
}
pub fn append(index: SearchIndex, path: &str) -> Result<SearchIndex, String> {
    let path = Path::new(path)
        .canonicalize()
        .map_err(|_| "HTML manifest missing")?;
    let mut bytes = Vec::new();
    std::fs::File::open(&path)
        .map_err(|_| "HTML manifest read")?
        .take(65537)
        .read_to_end(&mut bytes)
        .map_err(|_| "HTML manifest read")?;
    if bytes.len() > 65536 {
        return Err("HTML manifest limit".into());
    }
    let manifest: Manifest = serde_json::from_slice(&bytes).map_err(|_| "HTML manifest JSON")?;
    if manifest.pages.len() > 16 || index.document_count() + manifest.pages.len() > 256 {
        return Err("HTML collection limit".into());
    }
    let root = path
        .parent()
        .ok_or("Manifest directory")?
        .join(&manifest.root)
        .canonicalize()
        .map_err(|_| "Corpus root missing")?;
    let mut urls: BTreeMap<String, usize> = index
        .documents
        .iter()
        .enumerate()
        .map(|(i, d)| (d.url.clone(), i + 1))
        .collect();
    for (i, page) in manifest.pages.iter().enumerate() {
        let base = Url::parse(&page.url).map_err(|_| "Source URL")?;
        let source = safe_url(&base, "").ok_or("HTTPS source URL")?;
        if urls
            .insert(source, index.document_count() + i + 1)
            .is_some()
        {
            return Err("Duplicate document URL".into());
        }
        if page.images.len() > 8 {
            return Err("Image registration limit".into());
        }
    }
    let mut docs = index.documents;
    let mut doms = index.dom_documents;
    let mut total_images = 0usize;
    for page in manifest.pages {
        let source =
            String::from_utf8(file(&root, &page.html, 65536)?).map_err(|_| "HTML must be UTF-8")?;
        let parsed = parse(&source, &page.url)?;
        let base = Url::parse(&page.url).map_err(|_| "Source URL")?;
        let image_files: BTreeMap<String, String> = page
            .images
            .into_iter()
            .map(|(src, file)| {
                safe_url(&base, &src)
                    .map(|url| (url, file))
                    .ok_or("Image registration URL".to_string())
            })
            .collect::<Result<_, _>>()?;
        let mut dom = DomDocument {
            pages: Vec::new(),
            images: Vec::new(),
        };
        let mut body = String::new();
        let mut image_urls = Vec::new();
        let mut loaded = BTreeMap::new();
        let mut links = 0;
        for block in parsed.blocks {
            let (kind, style, level, text, target, source, asset) = match block {
                Block::Text { style, level, text } => (
                    if style == "heading" {
                        Kind::Heading
                    } else {
                        Kind::Reader
                    },
                    style,
                    level,
                    text,
                    0,
                    String::new(),
                    None,
                ),
                Block::Link { text, source } => {
                    links += 1;
                    if links > 32 {
                        return Err("Link limit".into());
                    }
                    let mut key = Url::parse(&source).map_err(|_| "Link URL")?;
                    key.set_fragment(None);
                    let target = urls.get(key.as_str()).copied().unwrap_or(0) as u32;
                    (
                        Kind::Link,
                        "link",
                        0,
                        format!("{text}\n{source}"),
                        target,
                        source,
                        None,
                    )
                }
                Block::Image { alt, source } => {
                    let asset = if let Some(file_name) = image_files.get(&source) {
                        if let Some(id) = loaded.get(&source) {
                            Some(*id)
                        } else {
                            let image = prepare_image(&file(&root, file_name, 2 * 1024 * 1024)?)?;
                            total_images += image.pixels.len() + image.png.len();
                            if total_images > 8 * 1024 * 1024 {
                                return Err("Prepared image collection limit".into());
                            }
                            let id = dom.images.len();
                            dom.images.push(image);
                            loaded.insert(source.clone(), id);
                            Some(id)
                        }
                    } else {
                        None
                    };
                    if image_urls.len() >= 8 {
                        return Err("DOM image limit".into());
                    }
                    image_urls.push(source.clone());
                    if asset.is_some() {
                        (Kind::Image, "image", 0, alt, 0, source, asset)
                    } else {
                        (
                            Kind::Reader,
                            "missing-image",
                            0,
                            format!("[Image unavailable: {alt}]"),
                            0,
                            source,
                            None,
                        )
                    }
                }
            };
            body.push_str(&text);
            body.push('\n');
            if body.len() > 65536 {
                return Err("Extracted body limit".into());
            }
            let lines = chromatic::wrap(&text);
            if kind == Kind::Image {
                dom.pages.push(Page {
                    kind,
                    style,
                    heading_level: level,
                    lines: lines.iter().take(1).cloned().collect(),
                    link_target: 0,
                    source,
                    asset,
                });
                // The image has one caption row; retain the remaining alt text on pages.
                for chunk in lines.get(1..).unwrap_or_default().chunks(TEXT_ROWS) {
                    dom.pages.push(Page {
                        kind: Kind::Reader,
                        style: "caption",
                        heading_level: 0,
                        lines: chunk.to_vec(),
                        link_target: 0,
                        source: String::new(),
                        asset: None,
                    });
                }
            } else {
                for chunk in lines.chunks(TEXT_ROWS) {
                    dom.pages.push(Page {
                        kind,
                        style,
                        heading_level: level,
                        lines: chunk.to_vec(),
                        link_target: target,
                        source: source.clone(),
                        asset,
                    });
                }
            }
        }
        if dom.pages.is_empty() || dom.pages.len() > u16::MAX as usize {
            return Err("Empty/oversized reading scenes".into());
        }
        docs.push(Document {
            url: page.url,
            title: parsed.title,
            body,
            image_urls,
        });
        doms.insert(docs.len(), dom);
    }
    let mut index = SearchIndex::new(docs);
    index.dom_documents = doms;
    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dom_preserves_order_entities_and_inline_spaces_without_active_content() {
        let p=parse("<title>DOM &amp; images</title><main><h1>Heading</h1><p>Hello <b>world</b> &amp; reader.</p><script>secret()</script><p hidden>hidden</p><img src='photo.png' alt='Photo'><a href='next'>Next</a></main>","https://example.invalid/article").unwrap();
        assert_eq!(p.title, "DOM & images");
        assert_eq!(p.blocks.len(), 4);
        assert!(matches!(&p.blocks[1],Block::Text{text,..} if text=="Hello world & reader."));
        assert!(
            matches!(&p.blocks[2],Block::Image{source,..} if source=="https://example.invalid/photo.png")
        );
        assert!(
            matches!(&p.blocks[3],Block::Link{source,..} if source=="https://example.invalid/next")
        );
    }
    #[test]
    fn nested_list_and_quote_paragraphs_keep_context_without_orphan_markers() {
        let p=parse("<main><ul><li><p>Item</p></li></ul><blockquote><p>Quoted</p><p>Again</p></blockquote></main>","https://example.invalid/").unwrap();
        assert_eq!(p.blocks.len(), 3);
        assert!(matches!(&p.blocks[0],Block::Text{style:"list",text,..} if text=="- Item"));
        assert!(
            p.blocks[1..]
                .iter()
                .all(|b| matches!(b, Block::Text { style: "quote", .. }))
        );
    }
    #[test]
    fn preformatted_first_line_keeps_indentation() {
        let parsed = parse(
            "<main><pre>  first\n    second</pre></main>",
            "https://example.invalid/",
        )
        .unwrap();
        assert!(
            matches!(&parsed.blocks[0],Block::Text{style:"pre",text,..} if text=="  first\n    second")
        );
    }
    #[test]
    fn unsafe_links_retain_only_their_text_and_parse_limits_fail_closed() {
        let p=parse("<main><a href='javascript:alert(1)'>Read</a><img src='file:///secret' alt='Alt'></main>","https://example.invalid/").unwrap();
        assert!(p.blocks.iter().all(|b| matches!(b, Block::Text { .. })));
        assert!(parse(&"<div>".repeat(66), "https://example.invalid/").is_err());
        assert!(parse(&"x".repeat(65537), "https://example.invalid/").is_err());
    }
    #[test]
    fn hidden_linked_image_is_not_extracted() {
        let p = parse(
            "<main><a href='/next'><span hidden><img src='/hidden.png'></span>Visible</a></main>",
            "https://example.invalid/",
        )
        .unwrap();
        assert_eq!(p.blocks.len(), 1);
        assert!(matches!(&p.blocks[0],Block::Link{text,..} if text=="Visible"));
        assert!(
            parse(
                "<div hidden><main>Hidden article</main></div>",
                "https://example.invalid/"
            )
            .unwrap()
            .blocks
            .is_empty()
        );
    }
    #[test]
    fn registered_files_reject_absolute_parent_and_symlink_escapes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .canonicalize()
            .unwrap();
        assert!(file(&root, "/etc/hosts", 1024).is_err());
        assert!(file(&root, "../Cargo.toml", 1024).is_err());
        assert!(file(&root, "examples/fixtures/dom.html", 1).is_err());
        #[cfg(unix)]
        {
            let dir = std::env::temp_dir().join(format!("pocket-dom-path-{}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            let link = dir.join("escape");
            std::os::unix::fs::symlink(root.join("Cargo.toml"), &link).unwrap();
            assert!(file(&dir.canonicalize().unwrap(), "escape", 65536).is_err());
            std::fs::remove_file(link).unwrap();
            std::fs::remove_dir(dir).unwrap();
        }
    }
    #[test]
    fn image_codec_and_source_limits_fail_closed() {
        assert!(prepare_image(b"not an image").is_err());
        assert!(prepare_image(b"GIF89a").is_err());
        assert!(prepare_image(&vec![0; 2 * 1024 * 1024 + 1]).is_err());
    }
    #[test]
    fn transparent_png_and_jpeg_prepare_with_correct_dimensions() {
        let mut png = Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(1, 1, image::Rgba([10, 20, 30, 0]))
            .write_to(&mut png, ImageFormat::Png)
            .unwrap();
        assert_eq!(prepare_image(png.get_ref()).unwrap().pixels, [255, 255]);
        let mut jpeg = Cursor::new(Vec::new());
        RgbImage::from_pixel(2, 2, image::Rgb([255, 0, 0]))
            .write_to(&mut jpeg, ImageFormat::Jpeg)
            .unwrap();
        let image = prepare_image(jpeg.get_ref()).unwrap();
        assert_eq!((image.width, image.height, image.pixels.len()), (2, 2, 8));
        assert!(chromatic_client::ImageStripe::decode(&image.stream(1, 0)).is_ok());
    }
    #[test]
    fn actual_html_fixture_round_trips_image_stripes_and_registered_link() {
        let index = append(SearchIndex::new(Vec::new()), "examples/html-manifest.json").unwrap();
        let dom = &index.dom_documents[&1];
        let at = dom
            .pages
            .iter()
            .position(|p| p.kind == Kind::Image)
            .unwrap();
        let image = &dom.images[dom.pages[at].asset.unwrap()];
        let stream = image.stream(1, at);
        let mut read = 0;
        let mut row = 0;
        let mut pixels = Vec::new();
        while read < stream.len() {
            let n = 24 + usize::from(u16::from_le_bytes([stream[read + 18], stream[read + 19]]));
            let stripe = chromatic_client::ImageStripe::decode(&stream[read..read + n]).unwrap();
            assert_eq!(stripe.y, row);
            row += stripe.rows;
            pixels.extend_from_slice(stripe.pixels);
            read += n;
        }
        assert_eq!(pixels, image.pixels);
        assert_eq!(row, image.height);
        assert!(
            dom.pages
                .iter()
                .any(|p| p.kind == Kind::Link && p.link_target == 2)
        );
    }
}
