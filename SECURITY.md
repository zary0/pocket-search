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

Use GitHub's [private vulnerability report form](https://github.com/zary0/pocket-search/security/advisories/new) to send vulnerability details confidentially. Maintainers must enable and read back private vulnerability reporting before making this repository public. The form's availability and repository controls are recorded in [GitHub governance](docs/github-governance.md).

If the private form is unavailable, [request a private security contact](https://github.com/zary0/pocket-search/issues/new?title=Private%20security%20contact%20request&body=Please%20provide%20a%20confidential%20channel%20for%20a%20security%20report.%20No%20vulnerability%20details%20are%20included%20here.). Post only that contact request, without affected components, exploits, payloads, logs or screenshots. The issue itself is not a confidential vulnerability report. No separate security email is currently published. See [GitHub's private reporting guidance](https://docs.github.com/en/code-security/how-tos/report-and-fix-vulnerabilities/report-privately).

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
| Native collection → local search/reader | Allocation-free `no_std` Rust state in a 22,272-byte reservation; four documents with 2,560 retained UTF-8 body bytes each; exact verification of hashed candidates; bounded packet/layout validation; owned two-bank unencrypted flash snapshots with CRC, commit-last headers, semantic validation and index rebuild | Latest physical restart/resume, brownout/power-loss and failure/endurance acceptance; CRC is corruption detection, not authentication, and logical clear is not secure erasure |
| Native CSS/fonts → renderer | Reviewed HTTPS resource hosts, certificate/hostname checks and no redirects; external CSS <=3 KiB / eight selectors plus four bounded embedded rules; static TrueType/WOFF2 wire/SFNT/transformed buffers <=24 KiB each; <=64 KiB tracked allocator requests with a 16 KiB free-heap reserve; bounded native C++/Brotli reconstruction and Rust parsing/rasterization; 128-slot 8,708-byte BMP grayscale atlas, 256 outline points / 32 contours / 64 crossings; cooperative deadlines/cancellation; no TrueType bytecode execution; complete built-in Pocket JP fallback and horizontal kana clusters | Larger/composite/variable faces and general GSUB/GPOS remain unsupported; tracked requests exclude SDK allocator metadata/rounding and exception overhead; latest independent physical/battery/adversarial/endurance acceptance remains required; TLS authenticity does not make a site trustworthy. See [font limits and evidence](docs/native-webfonts.md) |
| Service response → device core | PCS1 decoding validates packet length, kind, payload structure, UTF-8, and display bounds. Text packets are limited to 512 bytes. PCI1 validates <=2,072-byte stripes, CRC, dimensions and payload; the adapter checks identity/order/complete image and caps the temporary cache at 20 KiB. The `no_std` decoder borrows its input without heap allocation and forbids unsafe Rust | Physical malformed-response tests, full font/display acceptance and peak-memory validation; the adapter already caps receipt and checks complete packets |
| Query → companion | HTTP API rejects invalid UTF-8/controls, duplicate/unknown parameters, oversized queries and invalid IDs/pages; headers, response sizes, workers and queue are bounded | Production rate limiting, hardened deployment and service abuse tests |
| Registered files → HTML/image preparation | Explicit manifest/root, canonical path containment, startup-only registration; <=64 KiB HTML, depth/node/block bounds, ignored active content and selected hidden markers; PNG/JPEG-only decoding with byte/dimension/allocation limits; no source HTML insertion or script execution | Parser/codec time isolation and comprehensive adversarial/fuzz acceptance |
| Public URL/content → crawler | Explicit public-web or exact-host scope, verified HTTPS, fresh public DNS/address pinning and peer checks, no proxy/cookies/compression, bounded robots/redirect/page/image acquisition, separate time-supervised worker, immutable hashed snapshots and generation-bound readers | OS memory/privilege isolation, fuzzing and real adversarial network acceptance; no malware/phishing or whole-process memory guarantee; see [crawler boundaries](docs/crawling.md) |
| Companion Wi-Fi → HTTPS origin (historical adapter) | Earlier experimental adapter requires native SNTP, certificate/hostname verification, no redirects, complete <=512-byte PCS1 scenes or <=20,720-byte PCI1 streams and bounded stripe buffers. The current autonomous adapter does not request these companion endpoints | Any resumed companion firmware requires its own physical failure/reconnection and full firmware qualification; never bypass TLS checks |
| Source/build → release | Complete pinned public MCU source build, unified versions, exact build identity, asset hashes, annotated-tag/main provenance and exact-byte promotion after validated evidence | Complete binary distribution/linked-component licensing review, isolated trusted lab, reachable HTTPS fixtures and candidate-bound physical E2E |
| Repository code → physical lab | Workflow orchestration checks readiness and the installed driver's path/digest. Fork PRs do not invoke the lab workflow | Operator-controlled runner, trusted branch/tag environment restrictions, recovery/settings preservation, and capture permissions. GitHub protection settings must be enabled and verified; workflow files alone do not enforce repository access policy. See [governance and activation status](docs/github-governance.md) |

Memory and CPU optimization must preserve bounds, UTF-8 validation, certificate validation, and result correctness. The in-memory host index is not a hardened Internet service. Do not execute fetched JavaScript, active HTML, or downloaded code as part of the planned reader. Serve extracted content and bounded image data rather than arbitrary active web pages.

The lab runner and approved driver are trusted infrastructure. JSON, JUnit, PNG metadata, and checksums cannot independently prove that a test occurred on physical hardware or authenticate an uncompromised driver. Release-gate unit fixtures are explicitly synthetic and must never be submitted as physical evidence.

## Credentials and privacy

Credentials, NVS contents, device identifiers, full-flash backups, private logs, and private corpus data stay outside Git history and uploaded artifacts. Use the hosting platform's secret mechanism for future service credentials; do not place secrets in repository variables, fixtures, command lines, or report attachments. Limit CI permissions and retain only sanitized evidence needed for qualification.

The current autonomous client has no bookmarks, query/browsing history or analytics. Its bounded public-page corpus, styles, source URLs, validators and crawl cursor are retained in owned, unencrypted flash banks. Restart validates that snapshot and rebuilds the RAM index. Search queries and reading positions are not persisted, and local searches are not sent to source sites. Images and downloaded font atlases are volatile; direct page/resource acquisition still exposes requests to sites and network operators. Existing Wi-Fi settings stay in native NVS; old companion-origin settings are unused. Sanitized diagnostics report counters and sizes rather than private page/query bodies. Logical clear does not securely erase previous flash banks.

The optional companion service disables request/query/body logging and sends no-store responses. Earlier temporary HTTPS providers could observe requests; no production retention or hosting privacy claim is made. Any future logging, persistence or collection behavior must be documented before deployment.

If a secret is accidentally exposed, revoke or rotate it with its provider before treating file deletion as a fix. Notify the maintainer privately, review history and artifacts for exposure, and coordinate cleanup without copying the secret into an issue. Local ignore rules are only a convenience, not a substitute for review.

## Native retained content and image limits

The reviewed source registry is compile-time configuration. Public HTTPS names do not replace destination/robots review; arbitrary user URL entry is not enabled. Native snapshots retain public page text, styles, source URLs and validators, never queries or reading history. The unencrypted two-bank flash cache checks ownership, CRC, bounds and semantic structure. It is not cryptographic authentication, and logical clear does not securely erase older banks. Do not publish full-flash backups or corpus banks. Storage format changes can require recollection.

The ROM JPEG path is limited to small baseline envelopes with checked table bounds, bounded dimensions/output, allocation cleanup and cooperative cancellation. The PNG and progressive-JPEG paths have independent byte, geometry, chunk/scan, time and private-allocation bounds, allocation-failure/cancellation tests, and exact PNG output validation. The pinned JPEG dependency retains upstream licensing and source hash. Unsupported variants, memory exhaustion and malformed sources fail visibly without replacing text. These checks do not prove any image decoder safe for hostile unrestricted web content; see [format limits and measured scope](docs/native-image-formats.md). Keep the small reviewed profile scope and expand native adversarial testing before widening it.
