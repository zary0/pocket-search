// SPDX-License-Identifier: GPL-3.0-only
use std::net::IpAddr;
use url::{Host, Url};

// Conservative public-unicast policy. Special-use ranges stay excluded even
// when an individual address within them has an IANA global exception.
pub fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let n = u32::from(ip);
            ![
                (0, 8),
                (0x0a000000, 8),
                (0x64400000, 10),
                (0x7f000000, 8),
                (0xa9fe0000, 16),
                (0xac100000, 12),
                (0xc0000000, 24),
                (0xc0000200, 24),
                (0xc0586300, 24),
                (0xc0a80000, 16),
                (0xc6120000, 15),
                (0xc6336400, 24),
                (0xcb007100, 24),
                (0xe0000000, 4),
                (0xf0000000, 4),
            ]
            .iter()
            .any(|&(base, bits)| n >> (32 - bits) == base >> (32 - bits))
        }
        IpAddr::V6(ip) => {
            let n = u128::from(ip);
            n >> 125 == 1
                && ![
                    (0x20010000u128 << 96, 23),
                    (0x20010db8u128 << 96, 32),
                    (0x2002u128 << 112, 16),
                    (0x3fff0u128 << 108, 20),
                ]
                .iter()
                .any(|&(base, bits)| n >> (128 - bits) == base >> (128 - bits))
        }
    }
}
pub fn checked_url(value: &str, allowed: &[String]) -> Result<Url, &'static str> {
    if value.len() > 1024 || value.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err("invalid_url");
    }
    let mut url = Url::parse(value).map_err(|_| "invalid_url")?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port_or_known_default() != Some(443)
    {
        return Err("https_origin_required");
    }
    let Some(Host::Domain(host)) = url.host() else {
        return Err("domain_required");
    };
    if !host.contains('.')
        || host.ends_with('.')
        || host.split('.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
        || [
            "localhost",
            "local",
            "internal",
            "home.arpa",
            "test",
            "invalid",
            "example",
        ]
        .iter()
        .any(|suffix| host == *suffix || host.ends_with(&format!(".{suffix}")))
    {
        return Err("public_domain_required");
    }
    if !allowed.is_empty() && !allowed.iter().any(|h| h == host) {
        return Err("host_out_of_scope");
    }
    if url.query_pairs().any(|(key, _)| {
        let key = key.to_ascii_lowercase();
        [
            "token",
            "access_token",
            "refresh_token",
            "id_token",
            "oauth_token",
            "client_secret",
            "code",
            "auth_code",
            "authorization_code",
            "oauth_verifier",
            "samlresponse",
            "samlrequest",
            "assertion",
            "auth",
            "authorization",
            "password",
            "passwd",
            "secret",
            "api_key",
            "apikey",
            "key",
            "session",
            "sid",
            "signature",
            "sig",
            "credential",
            "jwt",
        ]
        .contains(&key.as_str())
            || key.split(['_', '-']).any(|part| {
                matches!(
                    part,
                    "token"
                        | "secret"
                        | "password"
                        | "passwd"
                        | "credential"
                        | "session"
                        | "signature"
                )
            })
            || ["token", "secret", "password", "apikey"]
                .iter()
                .any(|suffix| key.ends_with(suffix))
            || key.starts_with("x-amz-")
            || key.starts_with("x-goog-")
    }) {
        return Err("credential_query_rejected");
    }
    url.set_fragment(None);
    if url.as_str().len() > 1024 {
        return Err("invalid_url");
    }
    Ok(url)
}
