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
        if out.insert(key, decode(value)?).is_some() || out.len() > 2 {
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
            body: json!({"label":COLLECTION,"sample":true,"documents":index.document_count()})
                .to_string()
                .into_bytes(),
        };
    }
    if !["/v1/search", "/v1/chromatic/search", "/v1/chromatic/page"].contains(&path) {
        return reply(404, "Not found");
    }
    let Ok(p) = parameters(query) else {
        return reply(400, "Invalid parameters");
    };
    if path == "/v1/chromatic/page" {
        if p.keys().any(|k| k != "id" && k != "page") || !p.contains_key("id") {
            return reply(400, "Invalid parameters");
        }
        let (Ok(id), Ok(page)) = (number(p.get("id"), 0), number(p.get("page"), 0)) else {
            return reply(400, "Invalid parameters");
        };
        return packet(index.chromatic_page(id, page));
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
    let body = json!({"collection":COLLECTION,"sample":true,"results":index.search(q,10)})
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
fn write_response(stream: &mut TcpStream, response: Response) {
    let reason = match response.status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        503 => "Service Unavailable",
        _ => "Internal Server Error",
    };
    let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self'; connect-src 'self'; base-uri 'none'; frame-ancestors 'none'\r\nConnection: close\r\n\r\n",
        response.status,
        reason,
        response.content_type,
        response.body.len()
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
            write_response(&mut stream, reply(400, "Request too large"));
            return;
        }
        if start.elapsed() > Duration::from_secs(3) {
            write_response(&mut stream, reply(408, "Request timeout"));
            return;
        }
        let n = match stream.read(&mut request[used..]) {
            Ok(0) | Err(_) => {
                write_response(&mut stream, reply(408, "Incomplete request"));
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
        write_response(&mut stream, reply(400, "Invalid request"));
        return;
    };
    let Some(line) = text.split("\r\n").next() else {
        return;
    };
    let parts: Vec<_> = line.split(' ').collect();
    if parts.len() != 3 || !["HTTP/1.1", "HTTP/1.0"].contains(&parts[2]) {
        write_response(&mut stream, reply(400, "Invalid request"));
        return;
    }
    write_response(&mut stream, route(index, parts[0], parts[1]));
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
        "Pocket Search read-only service listening on {} (original samples; no request logging)",
        listener.local_addr()?
    );
    for stream in listener.incoming() {
        let stream = stream?;
        match tx.try_send(stream) {
            Ok(()) => {}
            Err(mpsc::TrySendError::Full(mut stream)) => {
                write_response(&mut stream, reply(503, "Busy; retry later"))
            }
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
