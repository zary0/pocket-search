// SPDX-License-Identifier: GPL-3.0-only
//! Offline bounded public-HTTPS collection. Never invoked by a search request.
mod policy;
#[cfg(test)]
mod tests;
mod transport;
use crate::{
    SearchIndex,
    dom::{self, Block},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    io::Read,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use transport::{Fetch, Https, Reply};
use url::Url;
pub const USER_AGENT: &str =
    "PocketSearchBot/0.1 (+https://github.com/zary0/pocket-search; bounded reader collection)";
const ROBOT: &str = "PocketSearchBot";
const HTML_BYTES: usize = 65536;
const ROBOTS_BYTES: usize = 512 * 1024;
const IMAGE_BYTES: usize = 2 * 1024 * 1024;
const FRONTIER: usize = 512;
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    #[default]
    Hosts,
    PublicWeb,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub seeds: Vec<String>,
    pub scope: Scope,
    pub allowed_hosts: Vec<String>,
    pub max_pages: usize,
    pub max_depth: usize,
    pub max_requests: usize,
    pub max_bytes: usize,
    pub max_seconds: u64,
    pub request_seconds: u64,
    pub delay_ms: u64,
    pub images_per_page: usize,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            seeds: vec![],
            scope: Scope::Hosts,
            allowed_hosts: vec![],
            max_pages: 8,
            max_depth: 1,
            max_requests: 64,
            max_bytes: 8 * 1024 * 1024,
            max_seconds: 120,
            request_seconds: 10,
            delay_ms: 1000,
            images_per_page: 2,
        }
    }
}
impl Config {
    pub fn read(path: &Path) -> Result<Self, String> {
        let mut config: Self = serde_json::from_slice(&read_file(path, 32768)?)
            .map_err(|_| "Invalid crawl configuration")?;
        config.validate()?;
        Ok(config)
    }
    fn validate(&mut self) -> Result<(), String> {
        if self.seeds.is_empty()
            || self.seeds.len() > 16
            || self.allowed_hosts.len() > 32
            || !(1..=16).contains(&self.max_pages)
            || self.max_depth > 8
            || !(1..=256).contains(&self.max_requests)
            || !(1024 * 1024..=32 * 1024 * 1024).contains(&self.max_bytes)
            || !(5..=300).contains(&self.max_seconds)
            || !(1..=20).contains(&self.request_seconds)
            || !(250..=5000).contains(&self.delay_ms)
            || self.images_per_page > 8
        {
            return Err("Crawl limits out of range".into());
        }
        if (self.scope == Scope::Hosts && self.allowed_hosts.is_empty())
            || (self.scope == Scope::PublicWeb && !self.allowed_hosts.is_empty())
        {
            return Err(
                "Hosts scope requires exact allowed_hosts; public_web requires an empty list"
                    .into(),
            );
        }
        for host in &mut self.allowed_hosts {
            if host.chars().any(|c| "/:?#@".contains(c)) {
                return Err("Invalid allowed host".into());
            }
            let url = policy::checked_url(&format!("https://{host}/"), &[])
                .map_err(|_| "Invalid allowed host")?;
            if url.path() != "/" || url.query().is_some() {
                return Err("Invalid allowed host".into());
            }
            *host = url.host_str().ok_or("Invalid allowed host")?.to_owned();
        }
        self.allowed_hosts.sort();
        self.allowed_hosts.dedup();
        for seed in &mut self.seeds {
            *seed = policy::checked_url(seed, &self.allowed_hosts)
                .map_err(str::to_owned)?
                .to_string();
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pending {
    pub url: String,
    pub depth: usize,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFile {
    pub path: String,
    pub sha256: String,
    pub bytes: usize,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImageSource {
    pub url: String,
    pub requested_url: String,
    pub fetched_at: u64,
    pub source_sha256: String,
    pub file: SourceFile,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PageSource {
    pub url: String,
    pub requested_url: String,
    pub fetched_at: u64,
    pub source_sha256: String,
    pub file: SourceFile,
    pub images: Vec<ImageSource>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Issue {
    pub url: String,
    pub reason: String,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub schema_version: u8,
    pub started_at: u64,
    pub finished_at: u64,
    pub scope: Scope,
    pub requests: usize,
    pub body_bytes_read: usize,
    pub stopped: String,
    pub pages: Vec<PageSource>,
    pub issues: Vec<Issue>,
    pub pending: Vec<Pending>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Ready {
    schema_version: u8,
    generation: String,
    manifest_sha256: String,
    report_sha256: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    url: String,
    html: String,
    images: BTreeMap<String, String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    root: String,
    pages: Vec<Registration>,
}
struct Robots {
    body: String,
    delay: Duration,
}
struct Engine<F: Fetch> {
    config: Config,
    fetch: F,
    start: Instant,
    last: Option<Instant>,
    hosts: BTreeMap<String, Robots>,
    host_last: BTreeMap<String, Instant>,
    requests: usize,
    bytes: usize,
    issues: Vec<Issue>,
    stop: Option<&'static str>,
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read_file(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|_| "File unavailable")?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "File read failed")?;
    if bytes.len() > limit {
        return Err("File size limit".into());
    }
    Ok(bytes)
}
fn utf8(reply: &Reply, mime: &str) -> Result<String, &'static str> {
    let mut parts = reply.content_type.split(';');
    if !parts.next().unwrap_or("").trim().eq_ignore_ascii_case(mime) {
        return Err("unsupported_content_type");
    }
    for part in parts {
        if let Some((key, value)) = part.trim().split_once('=')
            && key.trim().eq_ignore_ascii_case("charset")
            && !value.trim().trim_matches('"').eq_ignore_ascii_case("utf-8")
        {
            return Err("unsupported_charset");
        }
    }
    String::from_utf8(reply.bytes.clone()).map_err(|_| "invalid_utf8")
}
fn robots_delay(body: &str) -> Result<Duration, &'static str> {
    // Crawl-delay is an extension. Conservatively honor the largest declared
    // value, including other groups, rather than silently reducing a site's rate.
    let mut delay = 0f64;
    for line in body.lines() {
        let line = line.split('#').next().unwrap_or("");
        if let Some((key, value)) = line.split_once(':')
            && key.trim().eq_ignore_ascii_case("crawl-delay")
        {
            let n = value
                .trim()
                .parse::<f64>()
                .map_err(|_| "invalid_crawl_delay")?;
            if !n.is_finite() || !(0.0..=60.0).contains(&n) {
                return Err("unsupported_crawl_delay");
            }
            delay = delay.max(n);
        }
    }
    Ok(Duration::from_secs_f64(delay))
}
impl<F: Fetch> Engine<F> {
    fn issue(&mut self, url: &str, reason: &str) {
        if self.issues.len() < 256 {
            self.issues.push(Issue {
                url: url.into(),
                reason: reason.into(),
            });
        }
    }
    fn remaining(&mut self) -> Result<Duration, &'static str> {
        if let Some(reason) = self.stop {
            return Err(reason);
        }
        if self.requests >= self.config.max_requests {
            self.stop = Some("request_budget");
            return Err("request_budget");
        }
        if self.bytes + 1 >= self.config.max_bytes {
            self.stop = Some("body_budget");
            return Err("body_budget");
        }
        let left =
            Duration::from_secs(self.config.max_seconds).saturating_sub(self.start.elapsed());
        if left.is_zero() {
            self.stop = Some("time_budget");
            return Err("time_budget");
        }
        Ok(left)
    }
    fn raw(&mut self, url: &Url, limit: usize) -> Result<Reply, &'static str> {
        let host = url.host_str().ok_or("domain_required")?.to_owned();
        let mut wait = self.last.map_or(Duration::ZERO, |last| {
            Duration::from_millis(self.config.delay_ms).saturating_sub(last.elapsed())
        });
        if let (Some(r), Some(last)) = (self.hosts.get(&host), self.host_last.get(&host)) {
            wait = wait.max(r.delay.saturating_sub(last.elapsed()));
        }
        if wait >= self.remaining()? {
            self.stop = Some("time_budget");
            return Err("time_budget");
        }
        if !wait.is_zero() {
            thread::sleep(wait);
        }
        let timeout = self
            .remaining()?
            .min(Duration::from_secs(self.config.request_seconds));
        let limit = limit.min(self.config.max_bytes - self.bytes - 1);
        self.requests += 1;
        let time = Instant::now();
        self.last = Some(time);
        self.host_last.insert(host, time);
        let reply = self.fetch.get(url, limit, timeout)?;
        self.bytes += reply.bytes.len();
        if let Some(error) = reply.error {
            return Err(error);
        }
        Ok(reply)
    }
    fn allowed(&mut self, url: &Url) -> Result<bool, &'static str> {
        let host = url.host_str().ok_or("domain_required")?.to_owned();
        if !self.hosts.contains_key(&host) {
            if self.hosts.len() >= 32 {
                return Err("robots_host_limit");
            }
            let origin = url.join("/robots.txt").map_err(|_| "invalid_url")?;
            let result = self.resource(origin, ROBOTS_BYTES, true);
            let robots = match result {
                Ok((_, reply)) if reply.status == 200 => utf8(&reply, "text/plain")
                    .and_then(|body| robots_delay(&body).map(|delay| Robots { body, delay })),
                Ok((_, reply)) if matches!(reply.status, 404 | 410) => Ok(Robots {
                    body: String::new(),
                    delay: Duration::ZERO,
                }),
                _ => Err("robots_unavailable"),
            };
            let robots = robots.unwrap_or_else(|reason| {
                self.issue(url.as_str(), reason);
                Robots {
                    body: "User-agent: *\nDisallow: /\n".into(),
                    delay: Duration::ZERO,
                }
            });
            self.hosts.insert(host.clone(), robots);
        }
        let body = &self.hosts.get(&host).ok_or("robots_unavailable")?.body;
        Ok(
            robotstxt::DefaultMatcher::default().one_agent_allowed_by_robots(
                body,
                ROBOT,
                url.as_str(),
            ),
        )
    }
    fn resource(
        &mut self,
        mut url: Url,
        limit: usize,
        robots: bool,
    ) -> Result<(Url, Reply), &'static str> {
        for hop in 0..=5 {
            url = policy::checked_url(url.as_str(), &self.config.allowed_hosts)?;
            if !robots && !self.allowed(&url)? {
                return Err("robots_disallow");
            }
            let reply = self.raw(&url, limit)?;
            if matches!(reply.status, 301 | 302 | 303 | 307 | 308) {
                if hop == 5 {
                    return Err("redirect_limit");
                }
                let location = reply.location.ok_or("redirect_without_location")?;
                let next = url.join(&location).map_err(|_| "invalid_redirect")?;
                url = policy::checked_url(next.as_str(), &self.config.allowed_hosts)?;
            } else {
                return Ok((url, reply));
            }
        }
        Err("redirect_limit")
    }
}
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn sanitize_references(parsed: &mut dom::Parsed, config: &Config) -> bool {
    let mut rejected = false;
    for block in &mut parsed.blocks {
        let (source, fallback) = match block {
            Block::Link { source, text } => (source, text.clone()),
            Block::Image { source, alt } => (source, format!("[Image unavailable: {alt}]")),
            Block::Text { .. } => continue,
        };
        match policy::checked_url(source, &config.allowed_hosts) {
            Ok(url) => *source = url.to_string(),
            Err(_) => {
                rejected = true;
                *block = Block::Text {
                    style: "paragraph",
                    level: 0,
                    text: fallback,
                };
            }
        }
    }
    rejected
}
fn write_file(root: &Path, name: &str, bytes: &[u8]) -> Result<SourceFile, String> {
    fs::write(root.join(name), bytes).map_err(|_| "Snapshot write failed")?;
    Ok(SourceFile {
        path: name.into(),
        sha256: hash(bytes),
        bytes: bytes.len(),
    })
}
fn generated_html(parsed: &dom::Parsed) -> String {
    let mut out = format!(
        "<!doctype html><meta charset=\"utf-8\"><title>{}</title><main>",
        escape(&parsed.title)
    );
    let mut links = 0;
    let mut images = 0;
    for block in &parsed.blocks {
        match block {
            Block::Text { style, level, text } => {
                let tag = match *style {
                    "heading" => format!("h{level}"),
                    "pre" => "pre".into(),
                    "quote" => "blockquote".into(),
                    _ => "p".into(),
                };
                if *style == "list" {
                    out.push_str(&format!(
                        "<ul><li>{}</li></ul>",
                        escape(text.strip_prefix("- ").unwrap_or(text))
                    ));
                } else {
                    out.push_str(&format!("<{tag}>{}</{tag}>", escape(text)));
                }
            }
            Block::Link { text, source } => {
                links += 1;
                if links <= 32 {
                    out.push_str(&format!(
                        "<a href=\"{}\">{}</a>",
                        escape(source),
                        escape(text)
                    ));
                } else {
                    out.push_str(&format!("<p>{}</p>", escape(text)));
                }
            }
            Block::Image { alt, source } => {
                images += 1;
                if images <= 8 {
                    out.push_str(&format!(
                        "<img src=\"{}\" alt=\"{}\">",
                        escape(source),
                        escape(alt)
                    ));
                } else {
                    out.push_str(&format!("<p>[Image: {}]</p>", escape(alt)));
                }
            }
        }
    }
    out.push_str("</main>");
    out
}
fn directives(value: &str) -> (bool, bool) {
    let tokens: Vec<_> = value
        .split(|c: char| c.is_whitespace() || c == ',' || c == ':')
        .collect();
    let has = |s: &str| tokens.iter().any(|t| t.eq_ignore_ascii_case(s));
    (
        has("noindex") || has("none"),
        has("nofollow") || has("none"),
    )
}
fn defer_job(queue: &mut VecDeque<Pending>, job: Pending) {
    // Keep the interrupted page first without exceeding the persisted frontier.
    if queue.len() == FRONTIER {
        queue.pop_back();
    }
    queue.push_front(job);
}
fn collect<F: Fetch>(config: Config, fetch: F, root: &Path) -> Result<Report, String> {
    let started_at = now();
    let mut engine = Engine {
        config: config.clone(),
        fetch,
        start: Instant::now(),
        last: None,
        hosts: BTreeMap::new(),
        host_last: BTreeMap::new(),
        requests: 0,
        bytes: 0,
        issues: vec![],
        stop: None,
    };
    let mut queue: VecDeque<Pending> = config
        .seeds
        .iter()
        .map(|url| Pending {
            url: url.clone(),
            depth: 0,
        })
        .collect();
    let mut seen: BTreeSet<String> = config.seeds.iter().cloned().collect();
    let mut fingerprints = BTreeSet::new();
    let mut pages = Vec::new();
    let mut registrations = Vec::new();
    while pages.len() < config.max_pages && !queue.is_empty() && engine.remaining().is_ok() {
        let job = queue.pop_front().ok_or("Empty crawl frontier")?;
        let url = policy::checked_url(&job.url, &config.allowed_hosts).map_err(str::to_owned)?;
        let fetched = engine
            .resource(url, HTML_BYTES, false)
            .and_then(|(url, reply)| {
                if reply.status != 200 {
                    return Err("page_http_status");
                }
                let source = utf8(&reply, "text/html")?;
                Ok((url, reply, source))
            });
        let (url, reply, source) = match fetched {
            Ok(v) => v,
            Err(reason) => {
                engine.issue(&job.url, reason);
                if engine.stop.is_some() {
                    defer_job(&mut queue, job);
                    break;
                }
                continue;
            }
        };
        let fetched_at = now();
        seen.insert(url.to_string());
        let mut parsed = match dom::parse(&source, url.as_str()) {
            Ok(p) => p,
            Err(_) => {
                engine.issue(url.as_str(), "dom_limits_or_invalid");
                continue;
            }
        };
        if sanitize_references(&mut parsed, &config) {
            engine.issue(url.as_str(), "reference_destination_rejected");
        }
        let (header_noindex, header_nofollow) = directives(&reply.robots_tag);
        let noindex = parsed.noindex || header_noindex;
        let nofollow = parsed.nofollow || header_nofollow;
        if !nofollow && job.depth < config.max_depth {
            for block in &parsed.blocks {
                if let Block::Link { source, .. } = block
                    && let Ok(next) = policy::checked_url(source, &config.allowed_hosts)
                    && seen.len() < FRONTIER + 256
                    && queue.len() < FRONTIER
                    && seen.insert(next.to_string())
                {
                    queue.push_back(Pending {
                        url: next.to_string(),
                        depth: job.depth + 1,
                    });
                }
            }
        }
        if noindex {
            engine.issue(url.as_str(), "noindex");
            continue;
        }
        let text = parsed
            .blocks
            .iter()
            .map(|b| match b {
                Block::Text { text, .. } | Block::Link { text, .. } => text.as_str(),
                Block::Image { alt, .. } => alt.as_str(),
            })
            .collect::<Vec<_>>()
            .join("\n");
        if text.trim().is_empty() {
            engine.issue(url.as_str(), "empty_body");
            continue;
        }
        let fingerprint = hash(text.as_bytes());
        if fingerprints.contains(&fingerprint) {
            engine.issue(url.as_str(), "duplicate_body");
            continue;
        }
        let html = generated_html(&parsed);
        if html.len() > HTML_BYTES {
            engine.issue(url.as_str(), "prepared_html_limit");
            continue;
        }
        let id = pages.len() + 1;
        let file = write_file(root, &format!("page-{id}.html"), html.as_bytes())?;
        let mut images = Vec::new();
        let mut mapping = BTreeMap::new();
        let mut attempted = BTreeSet::new();
        let mut refs = 0;
        for block in &parsed.blocks {
            if let Block::Image { source, .. } = block {
                refs += 1;
                if refs > 8 || attempted.len() >= config.images_per_page {
                    break;
                }
                if !attempted.insert(source.clone()) {
                    continue;
                }
                let image = (|| {
                    let original = policy::checked_url(source, &config.allowed_hosts)?;
                    let (final_url, reply) = engine.resource(original, IMAGE_BYTES, false)?;
                    if reply.status != 200
                        || !["image/png", "image/jpeg"].iter().any(|mime| {
                            reply
                                .content_type
                                .split(';')
                                .next()
                                .unwrap_or("")
                                .trim()
                                .eq_ignore_ascii_case(mime)
                        })
                    {
                        return Err("image_type_or_status");
                    }
                    let asset =
                        dom::prepare_image(&reply.bytes).map_err(|_| "image_limits_or_invalid")?;
                    Ok((final_url, reply.bytes, asset.png))
                })();
                match image {
                    Ok((final_url, bytes, png)) => {
                        let file = write_file(
                            root,
                            &format!("image-{id}-{}.png", images.len() + 1),
                            &png,
                        )?;
                        mapping.insert(source.clone(), file.path.clone());
                        images.push(ImageSource {
                            url: final_url.to_string(),
                            requested_url: source.clone(),
                            fetched_at: now(),
                            source_sha256: hash(&bytes),
                            file,
                        });
                    }
                    Err(reason) => engine.issue(source, reason),
                }
            }
        }
        // Validate the complete prepared page before it becomes a collection member.
        let registration = Registration {
            url: url.to_string(),
            html: file.path.clone(),
            images: mapping,
        };
        let validation = serde_json::to_vec(&Manifest {
            root: ".".into(),
            pages: vec![Registration {
                url: registration.url.clone(),
                html: registration.html.clone(),
                images: registration.images.clone(),
            }],
        })
        .map_err(|_| "Manifest JSON")?;
        fs::write(root.join("validate.json"), validation).map_err(|_| "Snapshot write failed")?;
        if dom::append(
            SearchIndex::new(vec![]),
            root.join("validate.json").to_str().ok_or("Snapshot path")?,
        )
        .is_err()
        {
            engine.issue(url.as_str(), "reader_limits_or_invalid");
            continue;
        }
        let mut candidates: Vec<&Registration> = registrations.iter().collect();
        candidates.push(&registration);
        if serde_json::to_vec_pretty(&serde_json::json!({"root":".","pages":candidates}))
            .map_err(|_| "Manifest JSON")?
            .len()
            > HTML_BYTES
        {
            engine.issue(url.as_str(), "manifest_budget");
            engine.stop = Some("manifest_budget");
            defer_job(&mut queue, job);
            break;
        }
        fingerprints.insert(fingerprint);
        registrations.push(registration);
        pages.push(PageSource {
            url: url.to_string(),
            requested_url: job.url,
            fetched_at,
            source_sha256: hash(&reply.bytes),
            file,
            images,
        });
    }
    let _ = fs::remove_file(root.join("validate.json"));
    let report = Report {
        schema_version: 1,
        started_at,
        finished_at: now(),
        scope: config.scope,
        requests: engine.requests,
        body_bytes_read: engine.bytes,
        stopped: engine
            .stop
            .unwrap_or(if pages.len() == config.max_pages {
                "page_budget"
            } else {
                "frontier_empty"
            })
            .into(),
        pages,
        issues: engine.issues,
        pending: queue.into_iter().collect(),
    };
    let manifest = serde_json::to_vec_pretty(&Manifest {
        root: ".".into(),
        pages: registrations,
    })
    .map_err(|_| "Manifest JSON")?;
    let report_bytes = serde_json::to_vec_pretty(&report).map_err(|_| "Report JSON")?;
    fs::write(root.join("manifest.json"), &manifest).map_err(|_| "Snapshot write failed")?;
    fs::write(root.join("report.json"), &report_bytes).map_err(|_| "Snapshot write failed")?;
    let ready = Ready {
        schema_version: 1,
        generation: hash(&[manifest.as_slice(), report_bytes.as_slice()].concat()),
        manifest_sha256: hash(&manifest),
        report_sha256: hash(&report_bytes),
    };
    fs::write(
        root.join("ready.json"),
        serde_json::to_vec(&ready).map_err(|_| "Ready JSON")?,
    )
    .map_err(|_| "Snapshot write failed")?;
    Ok(report)
}

/// The parent process publishes only after a complete worker result. Existing
/// destinations are never replaced. The deadline also covers OS DNS and codecs.
pub fn crawl(config_path: &Path, output: &Path) -> Result<Report, String> {
    let config = Config::read(config_path)?;
    if output.exists() {
        return Err("Output already exists; use a new snapshot directory".into());
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(|_| "Output parent unavailable")?;
    let stage = parent.join(format!(
        ".pocket-crawl-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "Clock unavailable")?
            .as_nanos()
    ));
    fs::create_dir(&stage).map_err(|_| "Cannot create staging directory")?;
    let result = (|| {
        let config_file = stage.join("worker-config.json");
        fs::write(
            &config_file,
            serde_json::to_vec(&config).map_err(|_| "Config JSON")?,
        )
        .map_err(|_| "Config write")?;
        let mut worker =
            Command::new(std::env::current_exe().map_err(|_| "Executable unavailable")?)
                .arg("__crawl_worker")
                .arg(&config_file)
                .arg(&stage)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()
                .map_err(|_| "Crawler worker unavailable")?;
        supervise(&mut worker, Duration::from_secs(config.max_seconds))?;
        fs::remove_file(config_file).map_err(|_| "Config cleanup")?;
        let report: Report =
            serde_json::from_slice(&read_file(&stage.join("report.json"), 2 * 1024 * 1024)?)
                .map_err(|_| "Report JSON")?;
        load_snapshot(&stage)?;
        // Reservation prevents overwriting an existing user directory. Move a
        // single immutable subtree atomically; serving reads only that subtree.
        fs::create_dir(output).map_err(|_| "Output already exists or unavailable")?;
        if fs::rename(&stage, output.join("snapshot")).is_err() {
            let _ = fs::remove_dir(output);
            return Err("Snapshot publication failed".into());
        }
        Ok(report)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&stage);
    }
    result
}
fn supervise(worker: &mut std::process::Child, limit: Duration) -> Result<(), String> {
    let deadline = Instant::now() + limit;
    loop {
        match worker.try_wait() {
            Ok(Some(status)) => {
                return if status.success() {
                    Ok(())
                } else {
                    Err("Crawler worker failed".into())
                };
            }
            Err(_) => {
                let _ = worker.kill();
                let _ = worker.wait();
                return Err("Crawler worker status unavailable".into());
            }
            Ok(None) => {}
        }
        if Instant::now() >= deadline {
            let _ = worker.kill();
            let _ = worker.wait();
            return Err("Crawler wall deadline reached; no snapshot published".into());
        }
        thread::sleep(Duration::from_millis(50));
    }
}
pub fn worker(config_path: &Path, stage: &Path) -> Result<(), String> {
    let config = Config::read(config_path)?;
    // Only the parent-created directory is written; this is a private CLI
    // command, not a server route or a user-supplied remote filesystem path.
    if !stage.is_dir() {
        return Err("Staging directory missing".into());
    }
    collect(config, Https, stage)?;
    Ok(())
}
fn contained(root: &Path, file: &SourceFile) -> Result<(), String> {
    use std::path::Component;
    if Path::new(&file.path)
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
        || file.bytes > IMAGE_BYTES
    {
        return Err("Invalid snapshot file".into());
    }
    let path = root
        .join(&file.path)
        .canonicalize()
        .map_err(|_| "Snapshot file missing")?;
    if !path.starts_with(root) || !path.is_file() {
        return Err("Snapshot file outside root".into());
    }
    let bytes = read_file(&path, file.bytes)?;
    if bytes.len() != file.bytes || hash(&bytes) != file.sha256 {
        return Err("Snapshot file integrity mismatch".into());
    }
    Ok(())
}
fn load_snapshot(root: &Path) -> Result<SearchIndex, String> {
    let root = root.canonicalize().map_err(|_| "Snapshot missing")?;
    let ready: Ready = serde_json::from_slice(&read_file(&root.join("ready.json"), 1024)?)
        .map_err(|_| "Snapshot ready metadata")?;
    let manifest = read_file(&root.join("manifest.json"), 65536)?;
    let report_bytes = read_file(&root.join("report.json"), 2 * 1024 * 1024)?;
    if ready.schema_version != 1
        || ready.manifest_sha256 != hash(&manifest)
        || ready.report_sha256 != hash(&report_bytes)
        || ready.generation != hash(&[manifest.as_slice(), report_bytes.as_slice()].concat())
    {
        return Err("Snapshot metadata integrity mismatch".into());
    }
    let report: Report = serde_json::from_slice(&report_bytes).map_err(|_| "Snapshot report")?;
    if report.schema_version != 1
        || report.pages.len() > 16
        || report.issues.len() > 256
        || report.pending.len() > FRONTIER
    {
        return Err("Snapshot report limits".into());
    }
    let manifest_data: Manifest =
        serde_json::from_slice(&manifest).map_err(|_| "Snapshot manifest")?;
    if manifest_data.root != "." || manifest_data.pages.len() != report.pages.len() {
        return Err("Snapshot manifest/root mismatch".into());
    }
    for (registered, page) in manifest_data.pages.iter().zip(&report.pages) {
        let expected: BTreeMap<_, _> = page
            .images
            .iter()
            .map(|image| (image.requested_url.clone(), image.file.path.clone()))
            .collect();
        if registered.url != page.url
            || registered.html != page.file.path
            || registered.images != expected
            || page.file.bytes > HTML_BYTES
        {
            return Err("Snapshot file registration mismatch".into());
        }
    }
    for page in &report.pages {
        contained(&root, &page.file)?;
        if page.images.len() > 8 {
            return Err("Snapshot image count".into());
        }
        for image in &page.images {
            contained(&root, &image.file)?;
        }
    }
    let mut index = dom::append(
        SearchIndex::new(vec![]),
        root.join("manifest.json").to_str().ok_or("Snapshot path")?,
    )?;
    if index.document_count() != report.pages.len()
        || index
            .documents
            .iter()
            .zip(&report.pages)
            .any(|(doc, page)| doc.url != page.url)
    {
        return Err("Snapshot manifest/report mismatch".into());
    }
    index.collection = crate::Collection {
        sample: false,
        label: format!("Collected public pages · {}", report.finished_at),
        generation: ready.generation,
    };
    index.provenance = report
        .pages
        .into_iter()
        .enumerate()
        .map(|(i, p)| {
            (
                i + 1,
                crate::Provenance {
                    fetched_at: p.fetched_at,
                    source_sha256: p.source_sha256,
                },
            )
        })
        .collect();
    Ok(index)
}
pub fn load(output: &Path) -> Result<SearchIndex, String> {
    load_snapshot(&output.join("snapshot"))
}
