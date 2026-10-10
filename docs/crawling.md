# Bounded public-web crawler

The Rust companion can now fetch public HTTPS pages, extract text/links, prepare PNG/JPEG images, and persist an immutable collection for the existing browser and Chromatic protocol. Search queries never trigger arbitrary URL fetching. Acquisition runs as a separate CLI worker process.

## Run

```sh
cargo build --locked
./target/debug/pocket-search crawl examples/crawl-public.json data/crawl-001
./target/debug/pocket-search serve-crawl data/crawl-001 127.0.0.1:8838
```

Use a new output directory for every run. It contains a `snapshot/` subtree with a generated HTML manifest, prepared images, source URL/time/SHA-256 records, skipped-item reasons, pending URLs, and a ready record. Original active HTML is not persisted or served; stored HTML is generated from extracted reading blocks and is parsed as data, never inserted into a web view. Raw image bytes are reencoded to small PNG files before persistence. The service verifies metadata/file hashes and registration consistency before loading. Hashes detect changes; they do not authenticate a publisher or prove that content is true or malware-free.

`scope: "public_web"` follows discovered links across public domains. `scope: "hosts"` instead requires exact `allowed_hosts`; subdomains are not implicitly included. Both modes start from operator-selected seeds and retain the same destination, robots, and resource protections. No search-provider API is used.

## Search actual AI pages

```sh
./target/debug/pocket-search crawl examples/crawl-ai.json data/ai-001
./target/debug/pocket-search serve-crawl data/ai-001 127.0.0.1:8858
```

The AI profile contains source URLs, not bundled website content. Stanford's John McCarthy pages are historical educational material; W3C pages cover web machine learning. Their availability, robots rules and contents may change. The original `serve examples/documents.json ...` command serves samples only; the deployed origin must point to `serve-crawl` to search collected sites. A successful query never invents a missing article.

The browser returns ten results per window, with previous/next controls and the total number of matches plus indexed pages. Native Up/Down reaches every match in the admitted collection, one bounded packet at a time. Search ranks document IDs first and copies only the requested window's text. `/v1/search?q=AI&offset=10&generation=...` selects the next browser window; later windows require the search generation. Collected reader/image requests retain the same generation checks. Up to 256 admitted documents are supported; this is collection coverage, not all Internet pages. The native decoder and PCS1 ABI are unchanged, but older adapters enforcing ten results must be updated.

## Limits

| Resource | Default | Hard maximum / policy |
| --- | --- | --- |
| Indexed pages per snapshot | 8 | 256; companion memory and all other budgets still apply |
| Link depth | 1 | 8 |
| Network attempts, including robots/redirects/images | 64 | 256 |
| Body bytes actually read across the run | 8 MiB | 32 MiB; headers/socket buffering are not included |
| Worker wall time | 120 s | 300 s; supervisor terminates and reaps a stalled worker |
| HTTP connect/request timeout | 10 s | 20 s, additionally constrained by the worker deadline; OS DNS is covered by the supervisor |
| Request spacing | 1 s | At least 250 ms globally; one request at a time |
| robots.txt | 512 KiB | 32 cached origins; failures deny collection |
| HTML | 64 KiB UTF-8 | Existing node/depth/block/body limits also apply |
| Source image | 2 MiB | PNG/JPEG only; existing dimensions/pixel/allocation limits apply |
| Images downloaded per page | 2 | 8; unavailable images retain an alt-text placeholder |
| Prepared images retained in a collection | — | 8 MiB total pixels + encoded PNG; excess images become placeholders |
| Pending URLs | 512 | Bounded discovery queue; not an unbounded persistent frontier |
| Report / manifest | 2 MiB / 256 KiB | Collection admission stops before the manifest bound |

The configured page budget counts successfully indexed pages. Attempts, bytes, depth and time also bound skipped/duplicate/noindex pages. Response bodies with unsupported compression are rejected; automatic decompression is disabled by the dependency feature set and `Accept-Encoding: identity` is requested. This avoids silently expanding compressed payloads. Unsupported MIME types, non-UTF-8 text, invalid/oversized DOM/images, and empty bodies are skipped with a reason rather than replaced with sample text.

The worker is time-isolated, **not an OS privilege/memory sandbox**. The image decoder's 32 MiB allowance is not a whole-process memory limit. Host memory limits, privilege isolation, parser fuzzing and endurance acceptance remain work before unattended public operation. Generated snapshot validation on the parent/service still uses bounded local preparation code.

## Network and indexing rules

- Only HTTPS on port 443 with domain names and no URL credentials; obvious credential/signature query parameters (including OAuth token/secret/code and SAML assertion keys) are rejected. Rejected link/image references retain only text or an alt-text placeholder; their URL is not persisted or returned to readers. URL-path/query semantics cannot prove that every page is public. TLS certificate/hostname verification uses Reqwest/Rustls and WebPKI roots. There is no insecure override, cookie jar, login, proxy, or script execution.
- Resolve the host for every individual request. Reject empty/oversized DNS answer sets and **any** non-public address in the set. Pin that exact validated address set into a new HTTP/1 client and verify its connected peer. No pooled connection or second DNS lookup selects an unchecked destination. IPv4 special-use ranges and conservative IPv6 non-global/special-use ranges are excluded; address literals and local/internal domain forms are rejected.
- Follow at most five redirects. Validate the new URL and DNS again. Pages and images also require the destination origin's robots rules before fetching; robots redirects retain the original origin's policy context.
- Use `PocketSearchBot` with an identifying project URL. The unmodified `robotstxt` matcher implements specific-agent/wildcard groups, longest-match allow/disallow rules, and percent-encoded matching. A missing robots file (404/410) permits crawling; other HTTP/network/content errors deny it. This is deliberately more restrictive than RFC 9309's general 4xx allowance.
- Honor `noindex` and `nofollow` in relevant HTML meta tags and conservatively in X-Robots-Tag directives. These are publisher instructions, not a security boundary. `Crawl-delay` is an extension: conservatively use the largest declared value across groups; invalid values or delays over 60 seconds deny that origin. Base request spacing is also retained.
- Ignore active/selected hidden/navigation/footer/aside content with the existing DOM extractor. Keep source/time metadata, deduplicate extracted text within a run, preserve bounded reading blocks, and limit actionable links/images to the reader's supported counts. This is basic extraction/deduplication, not a spam, phishing or malware classifier.

Standards/design references: [RFC 9309](https://www.rfc-editor.org/rfc/rfc9309.html), [OWASP SSRF guidance](https://cheatsheetseries.owasp.org/cheatsheets/Server_Side_Request_Forgery_Prevention_Cheat_Sheet.html), and the [Reqwest builder](https://docs.rs/reqwest/0.12.28/reqwest/blocking/struct.ClientBuilder.html). Dependency grants and source availability are preserved in [third-party notices](../THIRD_PARTY_NOTICES.md).

## Collection identity and device compatibility

`/v1/collection` and search JSON distinguish original samples from collected public pages and return a collection generation. HTTP replies include `X-Pocket-Collection` and `X-Pocket-Sample`. Collected reader, DOM and image requests must include `generation=...`; stale/missing tokens return HTTP 409. Browser navigation and the updated native adapter bind results, reader pages and image stripes to that generation. Start submits a new native search and clears the previous generation. PCS1/PCI1 and the allocation-free Rust decoder ABI remain unchanged.

The original fixture routes retain initial legacy access for their historical experiments. Older native adapters cannot open the protected collected reader; build the updated C adapter. A request generation combines the immutable snapshot identity with the reader preparation revision. URL destinations remain visible in reader link scenes but are excluded from searchable prose and snippets. There is no hot reload, live index replacement, cross-snapshot search, automatic resumption, deletion/update policy or background scheduler yet. `pending` records allow inspecting the next work, but subsequent seeds and collection management currently require the operator.

Public-web scope describes where the crawler may follow links; it does **not** mean the whole Internet has been indexed. Broad coverage requires a durable frontier, global deduplication, incremental/partitioned storage, update scheduling, provenance/source inspection, and a scalable index. Those are separate next steps. The first implementation deliberately fits the existing Chromatic collection contract.

Fetched third-party content stays under ignored `data/`, is not repository-authored GPL fixture material, and is not bundled into source/release packages. The browser UI labels it as collected public content. A crawler cannot establish redistribution rights or content truth merely by fetching a public URL.

## Verification boundary

Hosted tests use original mocked replies to exercise public-address/URL checks, mixed DNS sets, redirect destination policy and robots checks, noindex/nofollow, MIME/UTF-8/size failures, duplicates, image reencoding, snapshot tamper detection, generation mismatch and existing-output preservation. A Unix subprocess test verifies stalled-worker termination/reaping. These mocks are not actual network/DNS-rebinding or physical-device tests.

Live verification separately collected Example Domain, two IANA pages and one W3C page with certificate-verified HTTPS. A three-page IANA/W3C snapshot required seven requests and 55,462 body bytes, produced two reduced source images, and left 77 discovered URLs pending at its configured page budget. The browser searched/read actual text and displayed the prepared images. See the sanitized run evidence and current native build/verification status in [crawler evidence](evidence/crawler/verification.json). Website text/image bytes remain local; no corpus or native binary is committed. Physical qualification and `CHROMATIC_E2E_READY` remain unchanged.


## Real-site development verification (2026-10-10)

A separate, operator-authorized live run collected 13 Stanford/W3C pages and three reduced PNG images. The 31 requests read 419,275 body bytes. Searching `AI` returns nine matches from the collected corpus; searching `the` returns all 13, including a second browser window and native result 11. These are fetched source pages, not generated answers, Google results or Internet-wide coverage.

Experimental MCU image `0.13.16-web` combines the collection-generation adapter with the saved-origin repair. Its 1,875,664-byte application was verified after an application-only update, preserving NVS and the partition table. Native Wi-Fi requests verified certificates and synchronized time, opened text scenes and decoded an 80 × 80 RGB565 source image in stripes. The operator confirmed the source image on the LCD. Synthetic commands only control the UI; USB did not proxy page data. Full physical-button, battery-only, endurance and release E2E acceptance are still pending.

See [sanitized evidence](evidence/real-sites/verification.json). The third-party corpus, render-buffer captures, native image, temporary origin and private identifiers remain outside Git and release packages. The temporary development origin requires the host service and tunnel to keep running.
