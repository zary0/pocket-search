// SPDX-License-Identifier: GPL-3.0-only
use super::policy::public_ip;
use reqwest::{blocking::Client, header, redirect::Policy};
use std::{
    io::Read,
    net::{SocketAddr, ToSocketAddrs},
    time::Duration,
};
use url::Url;

pub struct Reply {
    pub status: u16,
    pub location: Option<String>,
    pub content_type: String,
    pub robots_tag: String,
    pub bytes: Vec<u8>,
    pub error: Option<&'static str>,
}
pub trait Fetch {
    fn get(&mut self, url: &Url, limit: usize, timeout: Duration) -> Result<Reply, &'static str>;
}
pub struct Https;
impl Fetch for Https {
    fn get(&mut self, url: &Url, limit: usize, timeout: Duration) -> Result<Reply, &'static str> {
        let host = url.host_str().ok_or("domain_required")?;
        // Runs only in the CLI worker process. The supervising process also
        // enforces a wall deadline on OS DNS and parser/decoder work.
        let addresses: Vec<SocketAddr> = (host, 443)
            .to_socket_addrs()
            .map_err(|_| "dns_failed")?
            .take(9)
            .collect();
        validate_addresses(&addresses)?;
        let client = Client::builder()
            .no_proxy()
            .https_only(true)
            .http1_only()
            .redirect(Policy::none())
            .resolve_to_addrs(host, &addresses)
            .pool_max_idle_per_host(0)
            .connect_timeout(timeout)
            .timeout(timeout)
            .user_agent(super::USER_AGENT)
            .build()
            .map_err(|_| "https_client_failed")?;
        let mut response = client
            .get(url.clone())
            .header(header::ACCEPT_ENCODING, "identity")
            .send()
            .map_err(|_| "https_request_failed")?;
        if !response
            .remote_addr()
            .is_some_and(|peer| addresses.contains(&peer))
        {
            return Err("unvalidated_peer");
        }
        let headers = response.headers();
        let text = |key: header::HeaderName| -> Result<String, &'static str> {
            let values: Vec<_> = headers.get_all(key).iter().collect();
            if values.len() > 1 {
                return Err("ambiguous_header");
            }
            values.first().map_or(Ok(String::new()), |v| {
                v.to_str().map(str::to_owned).map_err(|_| "invalid_header")
            })
        };
        let location = text(header::LOCATION)?;
        let content_type = text(header::CONTENT_TYPE)?;
        let encoding = text(header::CONTENT_ENCODING)?;
        let robots_tag = headers
            .get_all("x-robots-tag")
            .iter()
            .map(|v| v.to_str().unwrap_or("noindex, nofollow"))
            .collect::<Vec<_>>()
            .join(",");
        if location.len() > 1024 || content_type.len() > 256 || robots_tag.len() > 1024 {
            return Err("header_limit");
        }
        let mut reply = Reply {
            status: response.status().as_u16(),
            location: (!location.is_empty()).then_some(location),
            content_type,
            robots_tag,
            bytes: Vec::new(),
            error: None,
        };
        if !(200..300).contains(&reply.status) {
            return Ok(reply);
        }
        if !encoding.is_empty() && !encoding.eq_ignore_ascii_case("identity") {
            reply.error = Some("compressed_response_rejected");
            return Ok(reply);
        }
        if response.content_length().is_some_and(|n| n > limit as u64) {
            reply.error = Some("body_size_limit");
            return Ok(reply);
        }
        if (&mut response)
            .take(limit as u64 + 1)
            .read_to_end(&mut reply.bytes)
            .is_err()
        {
            reply.error = Some("body_read_failed");
        } else if reply.bytes.len() > limit {
            reply.error = Some("body_size_limit");
        }
        Ok(reply)
    }
}
pub fn validate_addresses(addresses: &[SocketAddr]) -> Result<(), &'static str> {
    if addresses.is_empty()
        || addresses.len() > 8
        || addresses
            .iter()
            .any(|a| a.port() != 443 || !public_ip(a.ip()))
    {
        Err("non_public_dns_answer")
    } else {
        Ok(())
    }
}
