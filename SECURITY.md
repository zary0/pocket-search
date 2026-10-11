# Security policy

Pocket Search is an experimental autonomous ModRetro Chromatic search/reader with optional Rust companion tools. Scoped standalone hardware verification does not establish a physically qualified release. No public service or qualified Pocket Search firmware has been released. This policy covers this repository; earlier device experiments and third-party firmware are separate projects.

## Supported versions

| Revision | Security maintenance |
| --- | --- |
| Current `develop` head | Best-effort investigation and fixes during development; not a production support promise |
| Current `main` head | Release preparation; physical qualification is still required |
| Stable `vX.Y.Z` releases | None exist yet. After the first qualified release, only the latest stable version will receive security fixes |
| Older tags, development packages, and forks | No backport guarantee; report the issue and reproduce against the current code if possible |

Package version, commit, protocol version, and firmware identity matter. Include the output of `pocket-search --build-info` or the exact commit in a report. A `v` tag alone does not establish physical qualification; see the [release policy](docs/releases.md).

## Reporting a vulnerability

**Current channel status:** this repository is private, and GitHub private vulnerability reporting has not been enabled or verified. No separate security email address is published by this project. Do not assume that an issue, pull request, or discussion is a confidential channel, even while the repository is private.

If you have repository access, [request a private security contact](https://github.com/zary0/pocket-search/issues/new?title=Private%20security%20contact%20request&body=Please%20provide%20a%20confidential%20channel%20for%20a%20security%20report.%20No%20vulnerability%20details%20are%20included%20here.). Post only that contact request. Do not include the affected component, exploit, payload, logs, or screenshots. If you do not have repository access, ask the maintainer who provided the project through your existing private contact; keep technical details out of any public contact request.

Before public release, maintainers must enable GitHub private vulnerability reporting and verify that **Report a vulnerability** is available under this repository's Security tab. Once verified, that feature will be the preferred reporting channel and this status will be updated. GitHub documents both the [private reporting process and the contact-request fallback](https://docs.github.com/en/code-security/how-tos/report-and-fix-vulnerabilities/report-privately). The fallback issue itself is not a vulnerability report.

In the agreed confidential channel, include:

- The affected commit/version, component, and impact.
- Minimal reproduction steps and a sanitized input or proof of concept.
- Expected and actual behavior, with relevant error output.
- For device issues: model, firmware build identity, and whether this was a physical test or host/emulator test. Omit device serial numbers and network identifiers.
- Any suggested fix or workaround, plus disclosure timing or credit preferences.

Do not send Wi-Fi passwords, activation keys, tokens, private page contents, NVS dumps, full-flash backups, or unredacted captures. Keep recovery backups locally under the device owner's control.

## Scope and coordination

Relevant issues include malformed-packet handling, UTF-8 or bounds failures, excessive resource use, content or query injection, unsafe URL fetching, transport validation, credential exposure, and build/release or lab-runner compromise. Report concerns about planned components too, while clearly identifying that the code is not implemented yet. General feature requests and visual problems can use ordinary issues.

Maintainers will investigate reproducible reports, assess affected revisions, coordinate a fix or mitigation, and agree on disclosure and reporter credit through the confidential channel. This small project has no guaranteed response time, fix deadline, or paid bounty program. Keep exploit details private while coordination is ongoing; communicate any planned disclosure date to the maintainer.

Security fixes follow the same `develop` → `main` → `vX.Y.Z` path as other releases. A fix does not waive version checks, image identity checks, or physical E2E. Publish an advisory with the affected versions, impact, mitigation, and fixed version when appropriate. Credit is optional and requires the reporter's consent.

Test with your own data and hardware or explicit operator authorization. Avoid scanning unrelated systems, extracting others' data, or disrupting shared Wi-Fi and CI infrastructure. For firmware issues, start with a host reproduction where possible and preserve the established device recovery procedure.

## Trust boundaries and safeguards

| Boundary | Implemented in this repository | Required before the corresponding feature ships |
| --- | --- | --- |
| Public source → autonomous native collection | Fixed HTTPS host/path profile, no redirects, native SNTP and certificate/hostname checks; one serial worker, four documents, eight frontier entries, bounded reads/body/time; conservative bounded robots policy; no active-content execution | Physical failures/reconnection/cancellation, adversarial parser/network acceptance, provenance UI and reproducible full firmware; compact robots/HTML parsers are not general web standards implementations |
| Native collection → local search/reader | Allocation-free `no_std` Rust state in a 22,272-byte reserved region; 2,560 UTF-8 body bytes per document with visible truncation; exact text checks for hashed candidates and bounded full-table fallback; failed partial pages never published; PFW1 validates palette/style codes, UTF-8, row geometry, exact payload consumption and viewport bounds before drawing | Fuzz acceptance, complete physical font/input/endurance tests and persistent-storage design. The volatile collection is not a whole-device memory or malware/phishing guarantee |
| Native CSS/fonts → renderer | Exact HTTPS public host allowlist, certificate/hostname verification, no redirects or compression; stylesheet <=3 KiB / 8 simple selectors and raw static TrueType <=24 KiB; integer-only safe Rust parser, 128 points / 16 contours / 64 crossings per glyph scanline, eight fixed quadratic subdivisions; ASCII-only immutable 867-byte atlas, raw bytes released, no TrueType bytecode execution | Physical font/display and battery acceptance, broader standards conformance, native adversarial-input and endurance acceptance; TLS authenticity does not make a site trustworthy |
| Service response → device core | PCS1 decoding validates packet length, kind, payload structure, UTF-8, and display bounds. Text packets are limited to 512 bytes. PCI1 validates <=2,072-byte stripes, CRC, dimensions and payload; the adapter checks identity/order/complete image and caps the temporary cache at 20 KiB. The `no_std` decoder borrows its input without heap allocation and forbids unsafe Rust | Physical malformed-response tests, full font/display acceptance and peak-memory validation; the adapter already caps receipt and checks complete packets |
| Query → companion | HTTP API rejects invalid UTF-8/controls, duplicate/unknown parameters, oversized queries and invalid IDs/pages; headers, response sizes, workers and queue are bounded | Production rate limiting, hardened deployment and service abuse tests |
| Registered files → HTML/image preparation | Explicit manifest/root, canonical path containment, startup-only registration; <=64 KiB HTML, depth/node/block bounds, ignored active content and selected hidden markers; PNG/JPEG-only decoding with byte/dimension/allocation limits; no source HTML insertion or script execution | Parser/codec time isolation and comprehensive adversarial/fuzz acceptance |
| Public URL/content → crawler | Explicit public-web or exact-host scope, verified HTTPS, fresh public DNS/address pinning and peer checks, no proxy/cookies/compression, bounded robots/redirect/page/image acquisition, separate time-supervised worker, immutable hashed snapshots and generation-bound readers | OS memory/privilege isolation, fuzzing and real adversarial network acceptance; no malware/phishing or whole-process memory guarantee; see [crawler boundaries](docs/crawling.md) |
| Companion Wi-Fi → HTTPS origin (historical adapter) | Earlier experimental adapter requires native SNTP, certificate/hostname verification, no redirects, complete <=512-byte PCS1 scenes or <=20,720-byte PCI1 streams and bounded stripe buffers. The current autonomous adapter does not request these companion endpoints | Any resumed companion firmware requires its own physical failure/reconnection and full firmware qualification; never bypass TLS checks |
| Source/build → release | Unified versions, exact build identity, asset hashes, annotated-tag/main provenance checks, and exact-byte promotion after validated evidence | Working native firmware, approved lab driver, controlled runner, reachable HTTPS fixtures, and real physical E2E |
| Repository code → physical lab | Workflow orchestration checks readiness and the installed driver's path/digest. Fork PRs do not invoke the lab workflow | Operator-controlled runner, trusted branch/tag environment restrictions, recovery/settings preservation, and capture permissions. GitHub protection settings must be enabled and verified; workflow files alone do not enforce repository access policy. See [governance and activation status](docs/github-governance.md) |

Memory and CPU optimization must preserve bounds, UTF-8 validation, certificate validation, and result correctness. The in-memory host index is not a hardened Internet service. Do not execute fetched JavaScript, active HTML, or downloaded code as part of the planned reader. Serve extracted content and bounded image data rather than arbitrary active web pages.

The lab runner and approved driver are trusted infrastructure. JSON, JUnit, PNG metadata, and checksums cannot independently prove that a test occurred on physical hardware or authenticate an uncompromised driver. Release-gate unit fixtures are explicitly synthetic and must never be submitted as physical evidence.

## Credentials and privacy

Credentials, NVS contents, device identifiers, full-flash backups, private logs, and private corpus data stay outside Git history and uploaded artifacts. Use the hosting platform's secret mechanism for future service credentials; do not place secrets in repository variables, fixtures, command lines, or report attachments. Limit CI permissions and retain only sanitized evidence needed for qualification.

The current autonomous client has no bookmarks, query/browsing history or analytics. Its collection is held in RAM and discarded on restart; searches are performed locally and are not sent as requests to the source site. Direct page acquisition still exposes source requests to the site and network operators. Existing Wi-Fi settings stay in native NVS; any old companion-origin setting is unused. Sanitized diagnostics report counters and sizes, not page/query bodies.

The optional companion service disables request/query/body logging and sends no-store responses. Earlier temporary HTTPS providers could observe requests; no production retention or hosting privacy claim is made. Any future logging, persistence or collection behavior must be documented before deployment.

If a secret is accidentally exposed, revoke or rotate it with its provider before treating file deletion as a fix. Notify the maintainer privately, review history and artifacts for exposure, and coordinate cleanup without copying the secret into an issue. Local ignore rules are only a convenience, not a substitute for review.

## Native retained content and image limits

The reviewed source registry is compile-time configuration. Public HTTPS names do not replace destination/robots review; arbitrary user URL entry is not enabled. Native snapshots retain public page text, styles, source URLs and validators, never queries or reading history. The unencrypted two-bank flash cache checks ownership, CRC, bounds and semantic structure. It is not cryptographic authentication, and logical clear does not securely erase older banks. Do not publish full-flash backups or corpus banks. Storage format changes can require recollection.

The ROM JPEG path is limited to small baseline envelopes with checked table bounds, bounded dimensions/output, allocation cleanup and cooperative cancellation. Progressive JPEG and other image formats fail visibly. These checks do not prove the ROM entropy decoder safe for hostile unrestricted web content. Keep the small reviewed profile scope and expand native adversarial testing before widening it.
