# Product-first Chromatic roadmap

The top priority is a working handheld search and reading loop: **enter a query on Chromatic → fetch results over native Wi-Fi → select a result → read its complete text → return to search**. Completed infrastructure and rendering measurements enable this work; they do not complete the product.

## Priority and execution order

| Priority | User outcome | Completion boundary |
| --- | --- | --- |
| P0 — implement next | Search and read a known collection on the physical handheld | Native application, bounded API, reachable verified HTTPS, physical controls, and repeatable E2E |
| P1 | Search actual collected web pages and read imported book text | P0 passes; real content, source details, and collection identity work |
| P2 | View a site's reduced image and return to text | Text flow works; image transfer fits measured TLS/display memory budgets |
| P3 | Improve input, reading quality, search relevance, and performance | Improvements are evaluated against the working device flow |

Keep P0 acceptance as the release priority. The operator-requested DOM/image experiment implements selected P1/P2 foundations early; it does not waive P0 or complete real-web collection. During P0, fix bounds, transport validation, layout, crashes, and memory/responsiveness failures needed to finish the loop. Further standalone benchmarks, visual polish, a launcher/app system, and OS replacement must not displace that implementation. Stable publication remains subject to the physical release gate at every priority.

## P0 — First usable native search and reader

### 1. Bounded companion API — start here

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

- [ ] Add a public-HTTPS crawler/extractor for explicit seeds and allowed domains. Respect robots.txt; bound page count, depth, bytes, parsing time, redirects, and per-host request rate; validate destinations before fetching.
- [ ] Collect actual public pages and collection time. Registered local HTML now provides title, complete extracted text, source URL, links and image references; live-page acquisition/provenance remains pending. Do not execute fetched scripts or substitute invented content for unsupported sites.
- [ ] Add deduplication, persistence, and corpus-generation checks before live index replacement so a result ID cannot open an unrelated document.
- [ ] Add a bounded source/details view for complete URLs and collection information that do not fit a result screen.
- [ ] Import user-provided or redistributable plain-text books into the companion, with bounded file/page sizes and source metadata. Reuse the reader; device requests refer to registered document IDs rather than arbitrary filesystem paths.
- [ ] Make Japanese query entry usable and verify input, wrapping, glyph fallback, and complete reading against actual Japanese content.
- [ ] Search actual indexed pages and open imported book text on the handheld; verify query-dependent ranking, honest zero results, provenance, and complete pagination.

**Completion:** physical searches return actual collected content, and the same handheld reader opens real web text and imported book text with inspectable source information. Collection scope stays explicit; this is independent search, not Google's results or a web-scale index.

## P2 — Source images

- [ ] Fetch registered images referenced by collected documents, applying the crawler's origin/redirect/size protections, and resize them on the companion.
- [x] Define and implement PCI1 RGB565 stripes, CRC/order/document/page validation, cooperative cancellation and incomplete-transfer rejection for an image up to 128 × 80 pixels.
- [x] Display registered image stripes through LVGL and the bounded band driver using one temporary <=20 KiB image cache; release it when leaving the image.
- [ ] Display an actual source image and return to text with B; measure TLS-active peak memory, responsiveness, and interrupted-transfer recovery.

**Completion:** an image from the selected source displays correctly on the physical console within the measured memory budget, and failure/back navigation works.

Registered local PNG/JPEG resizing and two original source images have native Wi-Fi/render-buffer verification; see [DOM/image evidence](dom-images.md). Actual network collection, physical interruption acceptance and TLS allocator peak tracing remain pending.

## P3 — Quality and measured improvements

- [x] Implement optional FPGA physical menu-button clicks and validate the waveform/mixer in RTL simulation with pinned official-source integration. See [native click scope](native-click.md).
- [ ] Build that FPGA design with the licensed Gowin toolchain, verify timing/resources and recovery, and physically qualify speaker/headphone/USB feedback. The current implementation has not been installed or heard on the console.

- [ ] Refine Japanese input, keyboard ergonomics, long-text layout, links, reading controls, and search relevance using the working product.
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

The complete reproducible application, collected real-web corpus, and qualified physical search E2E are still pending; the experimental sample API/reader loop is implemented. Keep [implementation status](../README.md) accurate as each acceptance boundary is met.
