// SPDX-License-Identifier: GPL-3.0-only
use super::*;
use std::sync::{Arc, Mutex};
struct Temp(PathBuf);
use std::path::PathBuf;
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "pocket-crawl-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
struct Mock {
    replies: BTreeMap<String, Reply>,
    calls: Arc<Mutex<Vec<String>>>,
}
impl Mock {
    fn new() -> Self {
        Self {
            replies: BTreeMap::new(),
            calls: Arc::new(Mutex::new(vec![])),
        }
    }
    fn add(&mut self, url: &str, status: u16, mime: &str, body: &[u8]) {
        self.replies.insert(
            url.into(),
            Reply {
                status,
                content_type: mime.into(),
                location: None,
                robots_tag: String::new(),
                bytes: body.to_vec(),
                error: None,
            },
        );
    }
    fn robots(&mut self, host: &str, body: &str) {
        self.add(
            &format!("https://{host}/robots.txt"),
            200,
            "text/plain; charset=utf-8",
            body.as_bytes(),
        );
    }
    fn html(&mut self, url: &str, body: &str) {
        self.add(url, 200, "text/html; charset=utf-8", body.as_bytes());
    }
}
impl Fetch for Mock {
    fn get(&mut self, url: &Url, limit: usize, _: Duration) -> Result<Reply, &'static str> {
        self.calls.lock().unwrap().push(url.to_string());
        let mut reply = self
            .replies
            .remove(url.as_str())
            .ok_or("mock_unregistered_url")?;
        if reply.bytes.len() > limit {
            reply.bytes.truncate(limit + 1);
            reply.error = Some("body_size_limit");
        }
        Ok(reply)
    }
}
fn config() -> Config {
    Config {
        scope: Scope::PublicWeb,
        seeds: vec!["https://alpha.example.com/".into()],
        delay_ms: 0,
        max_depth: 2,
        ..Config::default()
    }
}
fn page(text: &str) -> String {
    format!("<title>Original test</title><main><p>{text}</p></main>")
}
#[test]
fn public_destination_policy_rejects_all_special_use_forms() {
    for ip in [
        "0.1.2.3",
        "10.0.0.1",
        "100.64.0.1",
        "127.0.0.1",
        "169.254.169.254",
        "172.16.0.1",
        "192.0.0.9",
        "192.0.2.1",
        "192.168.1.1",
        "198.18.0.1",
        "198.51.100.1",
        "203.0.113.1",
        "224.0.0.1",
        "255.255.255.255",
        "::1",
        "::ffff:127.0.0.1",
        "fc00::1",
        "fe80::1",
        "2001:db8::1",
        "2002:7f00:1::",
        "64:ff9b::7f00:1",
        "3fff::1",
    ] {
        assert!(!policy::public_ip(ip.parse().unwrap()), "{ip}");
    }
    for ip in [
        "93.184.215.14",
        "8.8.8.8",
        "2606:4700::1111",
        "2001:4860:4860::8888",
    ] {
        assert!(policy::public_ip(ip.parse().unwrap()), "{ip}");
    }
    for url in [
        "http://alpha.example.com/",
        "https://127.0.0.1/",
        "https://2130706433/",
        "https://[::1]/",
        "https://localhost/",
        "https://a.local/",
        "https://a.home.arpa/",
        "https://u:p@alpha.example.com/",
        "https://alpha.example.com:8443/",
        "https://alpha.example.com./",
        " https://alpha.example.com/",
        "https://alpha.example.com/a\n",
        "https://alpha.example.com/?access_token=secret",
        "https://alpha.example.com/?X-Amz-Signature=secret",
        "https://alpha.example.com/?refresh_token=secret",
        "https://alpha.example.com/?oauth_token=secret",
        "https://alpha.example.com/?%63lient_secret=secret",
        "https://alpha.example.com/?CODE=secret",
    ] {
        assert!(policy::checked_url(url, &[]).is_err(), "{url}");
    }
    assert_eq!(
        policy::checked_url("https://ALPHA.example.com/a#fragment", &[])
            .unwrap()
            .as_str(),
        "https://alpha.example.com/a"
    );
    assert!(
        policy::checked_url("https://beta.example.com/", &["alpha.example.com".into()]).is_err()
    );
}
#[test]
fn mixed_public_private_dns_answers_and_oversized_sets_fail_closed() {
    let public = "93.184.215.14:443".parse().unwrap();
    let private = "127.0.0.1:443".parse().unwrap();
    assert!(transport::validate_addresses(&[public]).is_ok());
    assert!(transport::validate_addresses(&[public, private]).is_err());
    assert!(transport::validate_addresses(&[]).is_err());
    assert!(transport::validate_addresses(&[public; 9]).is_err());
    assert!(transport::validate_addresses(&["93.184.215.14:80".parse().unwrap()]).is_err());
}
#[test]
fn configuration_requires_explicit_scope_and_enforces_hard_caps() {
    let mut c = Config {
        seeds: vec!["https://alpha.example.com/".into()],
        ..Config::default()
    };
    assert!(c.validate().is_err());
    c.scope = Scope::PublicWeb;
    assert!(c.validate().is_ok());
    c.allowed_hosts.push("alpha.example.com".into());
    assert!(c.validate().is_err());
    c.scope = Scope::Hosts;
    assert!(c.validate().is_ok());
    c.max_pages = 17;
    assert!(c.validate().is_err());
    c.max_pages = 16;
    c.delay_ms = 0;
    assert!(c.validate().is_err());
    assert!(serde_json::from_str::<Config>("{\"ignore_robots\":true}").is_err());
}
#[test]
fn robots_rules_longest_match_specific_agent_and_percent_encoding() {
    let body = "User-agent: *\nDisallow: /\nUser-agent: PocketSearchBot\nDisallow: /private\nAllow: /private/public\nDisallow: /*.pdf$\nDisallow: /caf%C3%A9\n";
    for (path, allowed) in [
        ("/", true),
        ("/private", false),
        ("/private/public", true),
        ("/book.pdf", false),
        ("/caf%C3%A9", false),
    ] {
        let url = format!("https://alpha.example.com{path}");
        let mut matcher = robotstxt::DefaultMatcher::default();
        assert_eq!(
            matcher.one_agent_allowed_by_robots(body, ROBOT, &url),
            allowed,
            "{path}"
        );
    }
    assert_eq!(
        robots_delay("User-agent: *\nCrawl-delay: 1.5").unwrap(),
        Duration::from_millis(1500)
    );
    assert!(robots_delay("Crawl-delay: NaN").is_err());
    assert!(robots_delay("Crawl-delay: 61").is_err());
}
#[test]
fn public_web_follows_external_links_and_registers_actual_returned_text() {
    let temp = Temp::new();
    let mut mock = Mock::new();
    mock.robots("alpha.example.com", "");
    mock.robots("beta.example.com", "");
    mock.html("https://alpha.example.com/","<title>First</title><main><p>Actual alpha text.</p><a href='https://beta.example.com/'>External</a></main>");
    mock.html("https://beta.example.com/", &page("Actual beta text."));
    let calls = mock.calls.clone();
    let report = collect(config(), mock, &temp.0).unwrap();
    assert_eq!(report.pages.len(), 2);
    let index = load_snapshot(&temp.0).unwrap();
    assert!(!index.collection.sample);
    assert!(
        index
            .search("beta", 10)
            .iter()
            .any(|hit| hit.url == "https://beta.example.com/")
    );
    assert!(index.documents.iter().any(
        |doc| doc.url == "https://beta.example.com/" && doc.body.contains("Actual beta text.")
    ));
    assert!(
        calls
            .lock()
            .unwrap()
            .iter()
            .any(|u| u == "https://beta.example.com/")
    );
    let route = crate::server::route(&index, "GET", "/v1/chromatic/page?id=1&page=0");
    assert_eq!(route.status, 409);
    let correct = format!(
        "/v1/chromatic/page?id=1&page=0&generation={}",
        index.collection.generation
    );
    assert_eq!(crate::server::route(&index, "GET", &correct).status, 200);
    assert_eq!(
        crate::server::route(
            &index,
            "GET",
            "/v1/chromatic/page?id=1&page=0&generation=old"
        )
        .status,
        409
    );
    assert_eq!(
        crate::server::route(&index, "GET", "/v1/search?q=actual&generation=old").status,
        409
    );
}
#[test]
fn redirects_cannot_reach_local_or_unapproved_destinations() {
    for target in [
        "https://127.0.0.1/",
        "http://alpha.example.com/",
        "https://private.local/",
        "https://beta.example.com/",
    ] {
        let temp = Temp::new();
        let mut mock = Mock::new();
        mock.robots("alpha.example.com", "");
        mock.add("https://alpha.example.com/", 302, "", b"");
        mock.replies
            .get_mut("https://alpha.example.com/")
            .unwrap()
            .location = Some(target.into());
        let calls = mock.calls.clone();
        let mut c = config();
        c.scope = Scope::Hosts;
        c.allowed_hosts = vec!["alpha.example.com".into()];
        let report = collect(c, mock, &temp.0).unwrap();
        assert!(report.pages.is_empty());
        assert!(!calls.lock().unwrap().iter().any(|u| u == target));
    }
}
#[test]
fn redirected_page_checks_destination_robots_before_fetching() {
    let temp = Temp::new();
    let mut mock = Mock::new();
    mock.robots("alpha.example.com", "");
    mock.add("https://alpha.example.com/", 301, "", b"");
    mock.replies
        .get_mut("https://alpha.example.com/")
        .unwrap()
        .location = Some("https://beta.example.com/private".into());
    mock.robots("beta.example.com", "User-agent: *\nDisallow: /private");
    let calls = mock.calls.clone();
    let report = collect(config(), mock, &temp.0).unwrap();
    assert!(report.pages.is_empty());
    assert!(
        !calls
            .lock()
            .unwrap()
            .iter()
            .any(|u| u == "https://beta.example.com/private")
    );
}
#[test]
fn robots_failure_and_invalid_rules_deny_page_acquisition() {
    for (status, mime, body) in [
        (503, "text/plain", ""),
        (403, "text/plain", ""),
        (200, "text/html", "<h1>Consent</h1>"),
        (200, "text/plain", "Crawl-delay: NaN"),
    ] {
        let temp = Temp::new();
        let mut mock = Mock::new();
        mock.add(
            "https://alpha.example.com/robots.txt",
            status,
            mime,
            body.as_bytes(),
        );
        let calls = mock.calls.clone();
        let report = collect(config(), mock, &temp.0).unwrap();
        assert!(report.pages.is_empty());
        assert_eq!(calls.lock().unwrap().len(), 1);
    }
}
#[test]
fn missing_robots_allows_but_noindex_follows_only_permitted_links() {
    let temp = Temp::new();
    let mut mock = Mock::new();
    mock.add("https://alpha.example.com/robots.txt", 404, "", b"");
    mock.html("https://alpha.example.com/","<meta name=robots content=noindex><main><p>Not indexed.</p><a href='/next'>Continue</a></main>");
    mock.html(
        "https://alpha.example.com/next",
        &page("Index this actual document."),
    );
    let report = collect(config(), mock, &temp.0).unwrap();
    assert_eq!(report.pages.len(), 1);
    assert_eq!(report.pages[0].url, "https://alpha.example.com/next");
    assert!(report.issues.iter().any(|i| i.reason == "noindex"));
}
#[test]
fn meta_and_http_nofollow_do_not_enqueue_links() {
    for header in [false, true] {
        let temp = Temp::new();
        let mut mock = Mock::new();
        mock.robots("alpha.example.com", "");
        mock.html(
            "https://alpha.example.com/",
            &format!(
                "{}<main><p>Keep this text.</p><a href='/secret'>Other</a></main>",
                if header {
                    ""
                } else {
                    "<meta name=robots content=nofollow>"
                }
            ),
        );
        if header {
            mock.replies
                .get_mut("https://alpha.example.com/")
                .unwrap()
                .robots_tag = "nofollow".into();
        }
        let calls = mock.calls.clone();
        let report = collect(config(), mock, &temp.0).unwrap();
        assert_eq!(report.pages.len(), 1);
        assert_eq!(calls.lock().unwrap().len(), 2);
    }
}
#[test]
fn duplicates_do_not_become_multiple_search_hits() {
    let temp = Temp::new();
    let mut mock = Mock::new();
    mock.robots("alpha.example.com", "");
    mock.html(
        "https://alpha.example.com/",
        &page("Same original content."),
    );
    mock.html(
        "https://alpha.example.com/copy",
        &page("Same original content."),
    );
    let mut c = config();
    c.seeds.push("https://alpha.example.com/copy".into());
    let report = collect(c, mock, &temp.0).unwrap();
    assert_eq!(report.pages.len(), 1);
    assert!(report.issues.iter().any(|i| i.reason == "duplicate_body"));
}
#[test]
fn interrupted_job_retains_priority_with_a_full_frontier() {
    let mut queue: VecDeque<_> = (0..FRONTIER)
        .map(|i| Pending {
            url: format!("https://alpha.example.com/{i}"),
            depth: 1,
        })
        .collect();
    defer_job(
        &mut queue,
        Pending {
            url: "https://alpha.example.com/retry".into(),
            depth: 0,
        },
    );
    assert_eq!(queue.len(), FRONTIER);
    assert_eq!(
        queue.front().unwrap().url,
        "https://alpha.example.com/retry"
    );
    assert_eq!(
        queue.back().unwrap().url,
        format!("https://alpha.example.com/{}", FRONTIER - 2)
    );
}
#[test]
fn depth_request_and_page_limits_preserve_a_bounded_frontier() {
    let temp = Temp::new();
    let mut mock = Mock::new();
    mock.robots("alpha.example.com", "");
    mock.html(
        "https://alpha.example.com/",
        "<main><p>First.</p><a href='/next'>Next</a></main>",
    );
    let mut c = config();
    c.max_pages = 1;
    let report = collect(c, mock, &temp.0).unwrap();
    assert_eq!(report.pages.len(), 1);
    assert_eq!(report.pending.len(), 1);
    assert_eq!(report.stopped, "page_budget");
    let temp = Temp::new();
    let mut mock = Mock::new();
    mock.robots("alpha.example.com", "");
    let mut c = config();
    c.max_requests = 1;
    let report = collect(c, mock, &temp.0).unwrap();
    assert!(report.pages.is_empty());
    assert_eq!(report.requests, 1);
    assert_eq!(report.stopped, "request_budget");
    assert_eq!(report.pending.len(), 1);
}
#[test]
fn oversized_non_utf8_wrong_type_and_compressed_pages_are_not_published() {
    for case in 0..4 {
        let temp = Temp::new();
        let mut mock = Mock::new();
        mock.robots("alpha.example.com", "");
        let body = if case == 0 {
            vec![b'x'; HTML_BYTES + 1]
        } else if case == 1 {
            vec![0xff]
        } else {
            page("Untrusted text.").into_bytes()
        };
        mock.add(
            "https://alpha.example.com/",
            200,
            if case == 2 {
                "application/octet-stream"
            } else {
                "text/html"
            },
            &body,
        );
        if case == 3 {
            mock.replies
                .get_mut("https://alpha.example.com/")
                .unwrap()
                .error = Some("compressed_response_rejected");
        }
        let report = collect(config(), mock, &temp.0).unwrap();
        assert!(report.pages.is_empty());
        assert!(report.body_bytes_read <= HTML_BYTES + 1);
    }
}
#[test]
fn downloaded_png_is_reencoded_and_all_files_are_bound_to_snapshot() {
    let temp = Temp::new();
    let mut mock = Mock::new();
    mock.robots("alpha.example.com", "");
    mock.html("https://alpha.example.com/","<title>Color</title><main><p>Original picture.</p><img src='/color.png' alt='Color'></main>");
    let mut raw = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(2, 2, image::Rgb([255, 0, 0]))
        .write_to(&mut raw, image::ImageFormat::Png)
        .unwrap();
    mock.add(
        "https://alpha.example.com/color.png",
        200,
        "IMAGE/PNG; charset=binary",
        raw.get_ref(),
    );
    let report = collect(config(), mock, &temp.0).unwrap();
    assert_eq!(report.pages[0].images.len(), 1);
    let index = load_snapshot(&temp.0).unwrap();
    assert_eq!(index.dom_documents[&1].images.len(), 1);
    fs::write(temp.0.join(&report.pages[0].file.path), b"tampered").unwrap();
    assert!(load_snapshot(&temp.0).is_err());
}
#[test]
fn credential_references_never_reach_fetches_snapshots_or_reader_metadata() {
    let temp = Temp::new();
    let mut mock = Mock::new();
    mock.robots("alpha.example.com", "");
    mock.html("https://alpha.example.com/", "<title>Public article</title><main><p>Keep this text.</p><a href='/auth?refresh_token=original_secret_marker'>Continue</a><img src='/picture.png?client_secret=original_secret_marker' alt='Hidden destination'><a href='/safe#part'>Safe link</a></main>");
    let calls = mock.calls.clone();
    let mut c = config();
    c.max_pages = 1;
    let report = collect(c, mock, &temp.0).unwrap();
    assert_eq!(report.pending.len(), 1);
    assert_eq!(report.pending[0].url, "https://alpha.example.com/safe");
    assert_eq!(calls.lock().unwrap().len(), 2);
    let index = load_snapshot(&temp.0).unwrap();
    assert!(index.documents[0].body.contains("Continue"));
    assert!(index.documents[0].body.contains("Hidden destination"));
    for file in fs::read_dir(&temp.0).unwrap() {
        let bytes = fs::read(file.unwrap().path()).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("original_secret_marker"));
    }
    for page in 0..index.dom_documents[&1].pages.len() {
        let response = crate::server::route(
            &index,
            "GET",
            &format!(
                "/v1/dom?id=1&page={page}&generation={}",
                index.collection.generation
            ),
        );
        assert_eq!(response.status, 200);
        assert!(!String::from_utf8_lossy(&response.body).contains("original_secret_marker"));
    }
}
#[test]
fn failed_images_leave_readable_text_and_a_visible_placeholder() {
    let temp = Temp::new();
    let mut mock = Mock::new();
    mock.robots("alpha.example.com", "");
    mock.html("https://alpha.example.com/","<main><p>Readable original text.</p><img src='https://127.0.0.1/private.png' alt='Blocked'><img src='/bad.png' alt='Broken'></main>");
    mock.add(
        "https://alpha.example.com/bad.png",
        200,
        "image/png",
        b"invalid codec data",
    );
    let report = collect(config(), mock, &temp.0).unwrap();
    assert_eq!(report.pages.len(), 1);
    assert!(report.pages[0].images.is_empty());
    let index = load_snapshot(&temp.0).unwrap();
    assert!(index.documents[0].body.contains("Broken"));
}
#[test]
fn existing_output_cannot_be_overwritten() {
    let temp = Temp::new();
    let cfg = temp.0.join("config.json");
    let mut c = config();
    c.delay_ms = 1000;
    fs::write(&cfg, serde_json::to_vec(&c).unwrap()).unwrap();
    fs::write(temp.0.join("keep"), b"existing corpus").unwrap();
    assert!(crawl(&cfg, &temp.0).is_err());
    assert_eq!(fs::read(temp.0.join("keep")).unwrap(), b"existing corpus");
}
#[test]
#[cfg(unix)]
fn wall_supervisor_terminates_and_reaps_a_stalled_worker() {
    let mut child = Command::new("/bin/sleep").arg("10").spawn().unwrap();
    let start = Instant::now();
    assert!(supervise(&mut child, Duration::from_millis(100)).is_err());
    assert!(start.elapsed() < Duration::from_secs(2));
    assert!(child.try_wait().unwrap().is_some());
}
