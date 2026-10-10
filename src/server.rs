// SPDX-License-Identifier: GPL-3.0-only
//! Bounded, read-only companion HTTP service. TLS belongs to the deployment origin.
use crate::{Document, SearchIndex, chromatic};
use serde_json::json;
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};

const REQUEST_BYTES: usize = 2048;
const RESPONSE_BYTES: usize = 16384;
pub const COLLECTION: &str = "Original sample collection · 2026-10-10";

pub fn load(path: &str) -> Result<SearchIndex, String> {
    let mut file = std::fs::File::open(path).map_err(|_| "Cannot open collection")?;
    let mut data = Vec::new();
    Read::by_ref(&mut file)
        .take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut data)
        .map_err(|_| "Cannot read collection")?;
    if data.len() > 4 * 1024 * 1024 {
        return Err("Collection exceeds 4 MiB".into());
    }
    let docs: Vec<Document> =
        serde_json::from_slice(&data).map_err(|_| "Invalid collection JSON")?;
    if docs.len() > 256
        || docs.iter().any(|d| {
            d.url.len() > 1024
                || d.title.len() > 1024
                || d.body.len() > 65536
                || d.image_urls.len() > 8
                || d.image_urls.iter().any(|url| url.len() > 1024)
        })
    {
        return Err("Collection exceeds document limits".into());
    }
    Ok(SearchIndex::new(docs))
}

pub struct Response {
    pub status: u16,
    pub content_type: &'static str,
    pub body: Vec<u8>,
}
fn reply(status: u16, body: &str) -> Response {
    Response {
        status,
        content_type: "text/plain; charset=utf-8",
        body: body.as_bytes().to_vec(),
    }
}
fn decode(value: &str) -> Result<String, ()> {
    let mut out = Vec::with_capacity(value.len().min(288));
    let input = value.as_bytes();
    let mut i = 0;
    if input.len() > 288 {
        return Err(());
    }
    while i < input.len() {
        match input[i] {
            b'%' => {
                if i + 2 >= input.len() {
                    return Err(());
                }
                let a = (input[i + 1] as char).to_digit(16).ok_or(())?;
                let b = (input[i + 2] as char).to_digit(16).ok_or(())?;
                out.push((a * 16 + b) as u8);
                i += 3;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    let value = String::from_utf8(out).map_err(|_| ())?;
    if value.chars().any(char::is_control) {
        return Err(());
    }
    Ok(value)
}
fn parameters(value: &str) -> Result<BTreeMap<String, String>, ()> {
    let mut out = BTreeMap::new();
    for part in value.split('&') {
        let (key, value) = part.split_once('=').ok_or(())?;
        let key = decode(key)?;
        if out.insert(key, decode(value)?).is_some() || out.len() > 3 {
            return Err(());
        }
    }
    Ok(out)
}
fn number(value: Option<&String>, default: usize) -> Result<usize, ()> {
    match value {
        None => Ok(default),
        Some(s) if !s.is_empty() && s.len() <= 5 && s.bytes().all(|b| b.is_ascii_digit()) => {
            s.parse().map_err(|_| ())
        }
        _ => Err(()),
    }
}
fn packet(result: Result<Vec<u8>, chromatic::Error>) -> Response {
    match result {
        Ok(body) => Response {
            status: 200,
            content_type: "application/vnd.pocket-search.pcs1",
            body,
        },
        Err(chromatic::Error::NotFound) => reply(404, "Not found"),
        Err(_) => reply(400, "Invalid request"),
    }
}
// Browser summaries are presentation excerpts. Complete source/body stays in the index.
fn excerpt(value: &str, limit: usize) -> (String, bool) {
    let mut out = String::new();
    for c in value.chars().map(|c| if c.is_control() { ' ' } else { c }) {
        if out.len() + c.len_utf8() > limit - 3 {
            out.push('…');
            return (out, true);
        }
        out.push(c);
    }
    (out, false)
}
pub fn route(index: &SearchIndex, method: &str, target: &str) -> Response {
    if method != "GET" {
        return reply(405, "GET required");
    }
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let asset = match path {
        "/" | "/index.html" => Some((
            "text/html; charset=utf-8",
            include_bytes!("../ui/index.html").as_slice(),
        )),
        "/style.css" => Some((
            "text/css; charset=utf-8",
            include_bytes!("../ui/style.css").as_slice(),
        )),
        "/app.mjs" => Some((
            "text/javascript; charset=utf-8",
            include_bytes!("../ui/app.mjs").as_slice(),
        )),
        "/input.mjs" => Some((
            "text/javascript; charset=utf-8",
            include_bytes!("../ui/input.mjs").as_slice(),
        )),
        "/feedback.mjs" => Some((
            "text/javascript; charset=utf-8",
            include_bytes!("../ui/feedback.mjs").as_slice(),
        )),
        "/images.mjs" => Some((
            "text/javascript; charset=utf-8",
            include_bytes!("../ui/images.mjs").as_slice(),
        )),
        "/pcs1.mjs" => Some((
            "text/javascript; charset=utf-8",
            include_bytes!("../ui/pcs1.mjs").as_slice(),
        )),
        _ => None,
    };
    if let Some((content_type, data)) = asset {
        return Response {
            status: 200,
            content_type,
            body: data.to_vec(),
        };
    }
    if path == "/v1/collection" {
        return Response {
            status: 200,
            content_type: "application/json",
            body: json!({"label":index.collection.label,"sample":index.collection.sample,"generation":index.collection.generation,"documents":index.document_count()})
                .to_string()
                .into_bytes(),
        };
    }
    if ![
        "/v1/search",
        "/v1/chromatic/search",
        "/v1/chromatic/page",
        "/v1/dom",
        "/v1/image",
        "/v1/chromatic/image",
    ]
    .contains(&path)
    {
        return reply(404, "Not found");
    }
    let Ok(mut p) = parameters(query) else {
        return reply(400, "Invalid parameters");
    };
    let generation = p.remove("generation");
    if generation
        .as_ref()
        .is_some_and(|g| g != &index.collection.generation)
        || (!index.collection.sample
            && path != "/v1/search"
            && path != "/v1/chromatic/search"
            && generation.is_none())
    {
        return reply(409, "Collection changed; search again");
    }
    if [
        "/v1/chromatic/page",
        "/v1/dom",
        "/v1/image",
        "/v1/chromatic/image",
    ]
    .contains(&path)
    {
        if p.keys().any(|k| k != "id" && k != "page") || !p.contains_key("id") {
            return reply(400, "Invalid parameters");
        }
        let (Ok(id), Ok(page)) = (number(p.get("id"), 0), number(p.get("page"), 0)) else {
            return reply(400, "Invalid parameters");
        };
        if path == "/v1/chromatic/page" {
            return packet(index.chromatic_page(id, page));
        }
        if path == "/v1/dom" {
            return dom_page(index, id, page);
        }
        let Some(dom) = index.dom_documents.get(&id) else {
            return reply(404, "Image not found");
        };
        let Some(scene) = dom.pages.get(page) else {
            return reply(404, "Page not found");
        };
        let Some(image) = scene.asset.and_then(|at| dom.images.get(at)) else {
            return reply(404, "Image unavailable");
        };
        return Response {
            status: 200,
            content_type: if path == "/v1/image" {
                "image/png"
            } else {
                "application/vnd.pocket-search.pci1"
            },
            body: if path == "/v1/image" {
                image.png.clone()
            } else {
                image.stream(id, page)
            },
        };
    }
    if p.keys()
        .any(|k| k != "q" && (k != "result" || path == "/v1/search"))
    {
        return reply(400, "Invalid parameters");
    }
    let Some(q) = p.get("q") else {
        return reply(400, "Query required");
    };
    if q.len() > 96 || q.trim().is_empty() {
        return reply(400, "Query must contain 1–96 UTF-8 bytes");
    }
    if path == "/v1/chromatic/search" {
        let Ok(at) = number(p.get("result"), 0) else {
            return reply(400, "Invalid result index");
        };
        return packet(index.chromatic_result(q, at));
    }
    let hits: Vec<_> = index
        .search(q, 10)
        .into_iter()
        .map(|hit| {
            let (title, _) = excerpt(&hit.title, 128);
            let (url, truncated) = excerpt(&hit.url, 256);
            let (snippet, _) = excerpt(&hit.snippet, 256);
            json!({"document_id":hit.document_id,"title":title,"url":url,"has_dom":index.dom_documents.contains_key(&hit.document_id),
            "source_url_truncated":truncated,"snippet":snippet})
        })
        .collect();
    let body = json!({"collection":index.collection.label,"sample":index.collection.sample,"generation":index.collection.generation,"results":hits})
        .to_string()
        .into_bytes();
    if body.len() > RESPONSE_BYTES {
        return reply(500, "Response limit exceeded");
    }
    Response {
        status: 200,
        content_type: "application/json",
        body,
    }
}
fn dom_page(index: &SearchIndex, id: usize, page: usize) -> Response {
    let Some(doc) = id.checked_sub(1).and_then(|at| index.documents.get(at)) else {
        return reply(404, "Document not found");
    };
    let Ok(encoded) = index.chromatic_page(id, page) else {
        return reply(404, "Page not found");
    };
    let Ok(packet) = chromatic_client::Packet::decode(&encoded) else {
        return reply(500, "Invalid stored page");
    };
    let scene = index
        .dom_documents
        .get(&id)
        .and_then(|dom| dom.pages.get(page));
    let image = scene.and_then(|scene| {
        scene
            .asset
            .and_then(|at| index.dom_documents.get(&id)?.images.get(at))
    });
    let body=json!({"document_id":id,"page_index":page,"page_count":packet.page_count,"kind":packet.kind as u8,
        "title":excerpt(&doc.title,256).0,"url":excerpt(&doc.url,1024).0,"generation":index.collection.generation,"provenance":index.provenance.get(&id),"has_dom":index.dom_documents.contains_key(&id),
        "style":scene.map_or("paragraph",|p|p.style),"heading_level":scene.map_or(0,|p|p.heading_level),
        "source":scene.map_or("".to_string(),|p|excerpt(&p.source,1024).0),"link_target":packet.link_target,
        "image_width":image.map_or(0,|i|i.width),"image_height":image.map_or(0,|i|i.height)
    }).to_string().into_bytes();
    if body.len() > 8192 {
        return reply(500, "Metadata limit");
    }
    Response {
        status: 200,
        content_type: "application/json",
        body,
    }
}
fn write_response(stream: &mut TcpStream, response: Response, collection: &crate::Collection) {
    let reason = match response.status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict",
        503 => "Service Unavailable",
        _ => "Internal Server Error",
    };
    let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nX-Pocket-Collection: {}\r\nX-Pocket-Sample: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self'; connect-src 'self'; base-uri 'none'; frame-ancestors 'none'\r\nConnection: close\r\n\r\n",
        response.status,
        reason,
        response.content_type,
        response.body.len(),
        collection.generation,
        if collection.sample { 1 } else { 0 }
    );
    if stream.write_all(header.as_bytes()).is_ok() {
        let _ = stream.write_all(&response.body);
    }
}
fn connection(mut stream: TcpStream, index: &SearchIndex) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
    let start = Instant::now();
    let mut request = [0; REQUEST_BYTES];
    let mut used = 0;
    loop {
        if used == request.len() {
            write_response(
                &mut stream,
                reply(400, "Request too large"),
                &index.collection,
            );
            return;
        }
        if start.elapsed() > Duration::from_secs(3) {
            write_response(
                &mut stream,
                reply(408, "Request timeout"),
                &index.collection,
            );
            return;
        }
        let n = match stream.read(&mut request[used..]) {
            Ok(0) | Err(_) => {
                write_response(
                    &mut stream,
                    reply(408, "Incomplete request"),
                    &index.collection,
                );
                return;
            }
            Ok(n) => n,
        };
        used += n;
        if request[..used].windows(4).any(|s| s == b"\r\n\r\n") {
            break;
        }
    }
    let Ok(text) = std::str::from_utf8(&request[..used]) else {
        write_response(
            &mut stream,
            reply(400, "Invalid request"),
            &index.collection,
        );
        return;
    };
    let Some(line) = text.split("\r\n").next() else {
        return;
    };
    let parts: Vec<_> = line.split(' ').collect();
    if parts.len() != 3 || !["HTTP/1.1", "HTTP/1.0"].contains(&parts[2]) {
        write_response(
            &mut stream,
            reply(400, "Invalid request"),
            &index.collection,
        );
        return;
    }
    write_response(
        &mut stream,
        route(index, parts[0], parts[1]),
        &index.collection,
    );
}
pub fn serve(index: SearchIndex, address: &str) -> std::io::Result<()> {
    let listener = TcpListener::bind(address)?;
    let index = Arc::new(index);
    let (tx, rx) = mpsc::sync_channel::<TcpStream>(8);
    let rx = Arc::new(Mutex::new(rx));
    for _ in 0..4 {
        let rx = Arc::clone(&rx);
        let index = Arc::clone(&index);
        thread::spawn(move || {
            loop {
                let stream = rx.lock().unwrap().recv();
                match stream {
                    Ok(stream) => connection(stream, &index),
                    Err(_) => break,
                }
            }
        });
    }
    eprintln!(
        "Pocket Search read-only service listening on {} (immutable collection; no request logging)",
        listener.local_addr()?
    );
    for stream in listener.incoming() {
        let stream = stream?;
        match tx.try_send(stream) {
            Ok(()) => {}
            Err(mpsc::TrySendError::Full(mut stream)) => write_response(
                &mut stream,
                reply(503, "Busy; retry later"),
                &index.collection,
            ),
            Err(mpsc::TrySendError::Disconnected(_)) => break,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chromatic_client::Packet;
    fn index() -> SearchIndex {
        SearchIndex::new(vec![Document {
            url: "https://example.invalid/sample".into(),
            title: "Wifi sample".into(),
            body: "Original wifi text. ".repeat(80),
            image_urls: vec![],
        }])
    }
    #[test]
    fn route_round_trips_ranked_results_and_reader() {
        let i = index();
        let r = route(&i, "GET", "/v1/chromatic/search?q=wifi&result=0");
        assert_eq!(r.status, 200);
        assert_eq!(Packet::decode(&r.body).unwrap().document_id, 1);
        let r = route(&i, "GET", "/v1/chromatic/page?id=1&page=0");
        let p = Packet::decode(&r.body).unwrap();
        assert!(p.page_count > 1);
        assert_eq!(route(&i, "GET", "/v1/chromatic/page?id=0").status, 404);
        let r = route(&i, "GET", "/v1/chromatic/search?q=missing");
        assert_eq!(Packet::decode(&r.body).unwrap().document_id, 0);
    }
    #[test]
    fn dom_metadata_images_and_links_only_serve_registered_scenes() {
        let index =
            crate::dom::append(SearchIndex::new(vec![]), "examples/html-manifest.json").unwrap();
        let dom = &index.dom_documents[&1];
        for (page, scene) in dom.pages.iter().enumerate() {
            let response = route(&index, "GET", &format!("/v1/dom?id=1&page={page}"));
            assert_eq!(response.status, 200);
            let meta: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
            assert_eq!(meta["page_index"], page);
            assert_eq!(meta["kind"], scene.kind as u8);
            let image = route(
                &index,
                "GET",
                &format!("/v1/chromatic/image?id=1&page={page}"),
            );
            if scene.asset.is_some() {
                assert_eq!(image.status, 200);
                assert!(image.body.len() <= chromatic_client::MAX_IMAGE_WIRE_BYTES);
                let first = 24 + usize::from(u16::from_le_bytes([image.body[18], image.body[19]]));
                assert_eq!(
                    chromatic_client::ImageStripe::decode(&image.body[..first])
                        .unwrap()
                        .page_index,
                    page as u16
                );
                let png = route(&index, "GET", &format!("/v1/image?id=1&page={page}"));
                assert_eq!(png.content_type, "image/png");
                assert_eq!(&png.body[..8], b"\x89PNG\r\n\x1a\n");
            } else {
                assert_eq!(image.status, 404);
            }
        }
        for route_name in ["dom", "image", "chromatic/image"] {
            assert_eq!(
                route(&index, "GET", &format!("/v1/{route_name}?id=0")).status,
                404
            );
            assert_eq!(
                route(&index, "GET", &format!("/v1/{route_name}?id=1&page=65535")).status,
                404
            );
            assert_eq!(
                route(
                    &index,
                    "GET",
                    &format!("/v1/{route_name}?id=1&url=https%3A%2F%2Fexample.com")
                )
                .status,
                400
            );
        }
        assert_eq!(
            route(&index, "GET", "/examples/fixtures/dom.html").status,
            404
        );
    }
    #[test]
    fn rejects_ambiguous_invalid_and_oversized_queries() {
        let i = index();
        for path in [
            "/v1/search?q=%ff",
            "/v1/search?q=%",
            "/v1/search?q=x&q=y",
            "/v1/search?q=%00",
            "/v1/search?q=%C2%80",
            "/v1/search?q=x&unknown=1",
            "/v1/chromatic/search?q=x&result=-1",
        ] {
            assert_eq!(route(&i, "GET", path).status, 400, "{path}");
        }
        assert_eq!(
            route(&i, "GET", &format!("/v1/search?q={}", "x".repeat(97))).status,
            400
        );
        assert_eq!(route(&i, "POST", "/v1/search?q=wifi").status, 405);
        assert_eq!(route(&i, "GET", "/../Cargo.toml").status, 404);
    }
    #[test]
    fn maximum_browser_summaries_fit_without_unused_images_or_text_loss_in_reader() {
        let body = format!("wifi {}", "quoted \\\" words ".repeat(100));
        let docs = (0..10)
            .map(|_| Document {
                url: "\\".repeat(1024),
                title: "\"".repeat(1024),
                body: body.clone(),
                image_urls: vec!["x".repeat(1024); 8],
            })
            .collect();
        let index = SearchIndex::new(docs);
        let response = route(&index, "GET", "/v1/search?q=wifi");
        assert_eq!(response.status, 200);
        assert!(response.body.len() <= RESPONSE_BYTES);
        let value: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        let hits = value["results"].as_array().unwrap();
        assert_eq!(hits.len(), 10);
        for hit in hits {
            assert!(hit.get("image_urls").is_none());
            assert_eq!(hit["source_url_truncated"], true);
            assert!(hit["title"].as_str().unwrap().len() <= 128);
            assert!(hit["url"].as_str().unwrap().len() <= 256);
            assert!(hit["snippet"].as_str().unwrap().len() <= 256);
        }
        let packet = index.chromatic_page(1, 0).unwrap();
        assert!(Packet::decode(&packet).unwrap().page_count > 1);
        assert_eq!(excerpt("あいう", 8), ("あ…".into(), true));
        assert_eq!(excerpt("a\nb", 128), ("a b".into(), false));
    }
    #[test]
    fn actual_http_response_has_bounded_body_and_no_store() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let worker = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            connection(stream, &index());
        });
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .write_all(b"GET /v1/chromatic/search?q=wifi HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes).unwrap();
        worker.join().unwrap();
        let split = bytes.windows(4).position(|s| s == b"\r\n\r\n").unwrap() + 4;
        let header = std::str::from_utf8(&bytes[..split]).unwrap();
        assert!(header.contains("Cache-Control: no-store"));
        assert!(bytes.len() - split <= 512);
        assert_eq!(Packet::decode(&bytes[split..]).unwrap().document_id, 1);
    }
}
