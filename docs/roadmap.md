# Product-first Chromatic roadmap

The primary outcome is **Chromatic directly collects a small public-web corpus, builds an index, searches it and reads it without a Mac, companion service or temporary tunnel**. Search scope may be small. Infrastructure and previous companion demonstrations do not complete this outcome.

## Priority and execution order

| Priority | User outcome | Completion boundary |
| --- | --- | --- |
| P0 | Standalone collection, search and text reading | Native direct HTTPS, local index/ranking/pagination, physical controls, battery and cold-start/error acceptance |
| P1 | Useful sources and retained local content | Configurable public profiles, provenance, policy expansion and measured persistent storage |
| P2 | Autonomous source images | Native acquisition/decoding/resizing within measured TLS/display limits |
| P3 | Faster and clearer device use | Measured CPU/heap/stack/display improvements against the working standalone flow |

## P0 — Autonomous mode takes priority

- [x] Implement a dependency-free Rust crawl frontier, streaming text parser and fixed-capacity inverted index, with safe exact matching and a bounded fallback when the term table fills.
- [x] Replace companion requests with direct native verified HTTPS collection; keep source scope, byte/time/frontier/request bounds explicit.
- [x] Generate results and text pages locally without sockets or external formatting after collection. Exclude sample content from the autonomous index.
- [x] Build the Xtensa Rust archive and an app-only native image within the existing 2 MiB partition.
- [x] Verify standalone direct collection/search with the Mac services/tunnels stopped, all 18 retained pages of one reader document, and installation/settings integrity. The operator confirmed battery-only restart, four `AI` results and A to read; see [scoped evidence](on-device-search.md).
- [ ] Complete physical input, battery-only, empty/error/cancellation and endurance acceptance, and make the complete firmware build reproducible for release.

No standalone image support is claimed by the earlier host-prepared source-image experiment. Bookmarks, history and Codex approval remain outside scope. Retained text has a visible fixed-capacity limit; a persistent, complete-page storage path is P1 work. The checked standalone evidence does not complete the broader release-qualification checkbox.

## Earlier work and remaining acceptance

The following records describe the preceding companion architecture and reusable enablers. They do not override autonomous P0.

## P0 — First usable native search and reader

### Historical companion API — retained development tooling

- [x] Expose the existing Rust index and PCS1 encoder through `GET /v1/chromatic/search?q=...&result=0` and `GET /v1/chromatic/page?id=...&page=0`.
- [x] Load one immutable, original English/Japanese fixture collection; keep document IDs stable for the experiment and mark fixture content visibly.
- [x] Enforce the 96-byte decoded UTF-8 query limit and 512-byte packet limit; reject invalid IDs/pages, return honest empty results, and bound request time/concurrency with explicit response lengths.
- [ ] Add deterministic failure cases and test hooks for timeouts, malformed/oversized packets, and request correlation. Keep query/body logging disabled.
- [x] Make a certificate-validated HTTPS origin reachable from the actual device network. Verify this before relying on a local service: earlier tests did not establish device-to-computer LAN reachability.

**Completion:** known queries produce query-dependent PCS1 results and complete reader pages through the service, with compatible decoding and deterministic errors. This API check alone is not a physical-device pass.

### 2. Reproducible native application and adapter

- [x] Build `chromatic-client` for the actual ESP32 Xtensa toolchain and link it through a defined C ABI or native Rust integration. A host `no_std` check does not meet this requirement.
- [ ] Build the Pocket Search application from repository-controlled source and documented, licensed dependencies, including the display/button/font adapter and build configuration. An operator-local experimental image is not the application deliverable.
- [x] Reuse ESP-IDF/FreeRTOS, native Wi-Fi, SNTP, certificate/hostname validation, and the measured display components. Start with the 48-row graphics-optimized prototype and verify it with the reader workload; retain the 16-row alternative for measured memory constraints.
- [x] Correct the retained LVGL tick-unit issue and verify application timer/input behavior separately from the recorded display comparison.
- [x] Keep one request in flight; cap the receive buffer before writing, decode only a complete packet, and retain its borrowed data until drawing/action dispatch finishes.
- [x] Produce an application-only image within 2 MiB, with exact source/version/image identity and the established recovery, settings-preservation, and installation verification procedure.

**Completion:** the exact image runs the Rust decoder on the intended MCU and renders a verified packet through the real font/display adapter. This integration check does not yet complete the search flow.

### 3. Physical input and text interaction

- [x] Implement a bounded on-device keyboard with editing, deletion, and submit. State the initial supported character set explicitly; retain UTF-8 limits as input support expands.
- [x] Connect buttons to query entry, result up/down, A to open, reader left/right, B to return, Start for query entry, and Select to go home.
- [ ] Show loading, no results, timeout, disconnected, and invalid-response states without blocking controls or inventing content.
- [ ] Display title, seven body rows, and navigation hints without overlap. Inspect English/Japanese fixture text physically, with visible unsupported-glyph fallback.
- [ ] Preserve the complete stored body across pagination, enforce first/last-page boundaries, and keep only temporary navigation state for returning to results.

**Completion:** a person can type a query on the handheld, select its actual returned document, read every page, and return to results or query entry using physical controls.

### 4. Physical acceptance and repeatable E2E

- [ ] Exercise the complete flow over device-native Wi-Fi after native clock synchronization, with certificate validation enabled and no host fetch/USB proxy.
- [ ] Check changed queries, empty results, malformed/oversized replies, timeout/reconnection, repeated navigation, and full mixed-language pagination without a frozen UI or silent text loss.
- [ ] Measure pinned-core decoder cycles/allocations, stack headroom, packet sizes, HTTPS/UI timing, minimum free heap, and largest free block with the actual application.
- [ ] Implement and verify the operator-installed lab driver, dedicated physical runner, reachable fixture origin, and physical USB-video capture required by the E2E contract.
- [ ] Bind the tested image and evidence to the exact candidate/source; pass all five required scenarios without failures or skips before enabling `CHROMATIC_E2E_READY`.
- [ ] Verify battery-only operation separately; USB-powered diagnostics do not establish it.

**P0 is done only when** the physical query → verified HTTPS result → complete text → back loop works and its required E2E is repeatable for the exact native image. Host tests, a rendering probe, and screenshots alone cannot substitute for this outcome. See [hardware acceptance](hardware-e2e.md), [protocol](chromatic-protocol.md), and [release policy](releases.md).

P0 uses controlled fixtures to prove integration. It must not be presented as searching the real web; that is P1.

## P1 — Real web content and book text

- [x] Add a bounded public-HTTPS crawler/extractor for explicit seeds with public-web or exact-domain scope. Respect robots.txt; bound page count, depth, bytes, parsing time, redirects, and per-host request rate; validate destinations before fetching.
- [x] Collect bounded public pages with source URL, collection time, SHA-256 records, extracted text/links and protected PNG/JPEG preparation. Live host acquisition is verified on Example Domain, IANA and W3C, including two prepared source images; physical real-page acceptance remains pending. Do not execute fetched scripts or substitute invented content for unsupported sites.
- [x] Add within-run deduplication, immutable snapshot persistence and generation checks so collected reader IDs fail on a mismatched collection.
- [ ] Add a durable cross-domain frontier, cross-run deduplication, partitioned corpus/index storage, automatic resumption and incremental updates; current snapshots are capped at 256 pages and never hot-reloaded.
- [ ] Add a bounded source/details view for complete URLs and collection information that do not fit a result screen.
- [ ] Import user-provided or redistributable plain-text books into the companion, with bounded file/page sizes and source metadata. Reuse the reader; device requests refer to registered document IDs rather than arbitrary filesystem paths.
- [ ] Make Japanese query entry usable and verify input, wrapping, glyph fallback, and complete reading against actual Japanese content.
- [x] Search actual indexed public pages on the handheld over certificate-verified HTTPS; reach results beyond the first ten and open reading scenes. Scoped synthetic-input/render-buffer evidence is recorded separately from operator LCD observations and release E2E.
- [ ] Open imported book text on the handheld; complete physical-control, provenance, first/last body page, zero-result and mixed-language acceptance.

**Completion:** physical searches return actual collected content, and the same handheld reader opens real web text and imported book text with inspectable source information. Collection scope stays explicit; this is independent search, not Google's results or a web-scale index.

## P2 — Source images

- [x] Fetch registered images referenced by collected documents, applying the crawler's origin/redirect/size protections, and resize them on the companion.
- [x] Define and implement PCI1 RGB565 stripes, CRC/order/document/page validation, cooperative cancellation and incomplete-transfer rejection for an image up to 128 × 80 pixels.
- [x] Display registered image stripes through LVGL and the bounded band driver using one temporary <=20 KiB image cache; release it when leaving the image.
- [ ] Display an actual source image and return to text with B; measure TLS-active peak memory, responsiveness, and interrupted-transfer recovery.

**Completion:** an image from the selected source displays correctly on the physical console within the measured memory budget, and failure/back navigation works.

Registered local PNG/JPEG resizing and two original source images have native Wi-Fi/render-buffer verification; see [DOM/image evidence](dom-images.md). Live network collection and image preparation are now verified on the host; physical source-image/interruption acceptance and TLS allocator peak tracing remain pending.

## P3 — Quality and measured improvements

- [ ] Refine Japanese input, keyboard ergonomics, long-text layout, links, reading controls, and search relevance using the working product.
- [ ] Add native character-commit click feedback after a verified FPGA audio command/path is available; preserve the wheel/mute and game/USB audio. Browser input feedback is implemented separately; see [UI scope](search-ui.md#input-click-feedback).
- [ ] Optimize actual decoder, HTTPS, and rendering bottlenecks while preserving validation, content correctness, and source information. Compare target size/speed profiles on the real Xtensa build.
- [ ] Evaluate stack/task consolidation or platform replacement only against equivalent networking, recovery, and physical E2E behavior.
- [ ] Evaluate externally prepared low-resolution video after text and still-image acceptance.

Bookmarks, browsing history, launch-time reading restoration, and the Codex approval application remain outside the product scope.

## Completed enablers — not product completion

The [search-first UI](search-ui.md) now connects to the bounded API and Xtensa Rust decoder. [Experimental evidence](search-results.md) verifies native Wi-Fi search, reader navigation, and user-confirmed LCD/A/Left/Right/B behavior. Full reproducible firmware, error/mixed-language acceptance and qualified physical E2E remain P0 work. Checked image/build items describe this local experiment, not release completion.

- [x] Chromatic-first MCU/service architecture, English documentation, and GPL-3.0-only contribution/security guidance.
- [x] Rust search/ranking, bounded text encoding, allocation-free `no_std` decoding, and button action mapping with host validation.
- [x] Repeatable host timing/allocation measurements and compact bounded search selection.
- [x] Target compilation of the ESP32 CPU-cycle/heap probe; native Rust decoder execution is now verified in the experimental search image.
- [x] Physical C/LVGL rendering measurements, 16/48-row display transports, and an opt-in graphics compiler experiment. These do not verify the reader or native Wi-Fi search.
- [x] `develop` → `main` → annotated `vX.Y.Z` version/package policy and CI/CD orchestration requiring physical evidence before release.

The complete reproducible application and qualified physical search E2E are still pending; the sample API/reader loop and a bounded real-web snapshot collector are implemented. Keep [implementation status](../README.md) accurate as each acceptance boundary is met.
