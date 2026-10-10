# Architecture

## Primary target

Pocket Search is a Wi-Fi search and reader application for the ModRetro Chromatic MCU. The physical display and buttons determine interaction, pagination, response sizes, and image budgets. A host browser can help debug the service but is not the primary product.

The official [Chromatic MCU project](https://github.com/ModRetro/oss-chromatic-console-mcu) uses ESP-IDF. Initial integration should preserve the existing display and button drivers, Wi-Fi, native SNTP, and certificate bundle. The Rust client core validates packets and maps buttons to actions; a firmware adapter must connect it to the platform. The experimental C/LVGL adapter links `chromatic-ffi`, compiled for `xtensa-esp32-none-elf`, into the existing development firmware. Full upstream/font/build integration remains operator-local; this is not a reproducible qualified release.

## Device responsibilities

1. Enter a bounded English or Japanese query with physical controls.
2. Send one request after the user submits the query.
3. Receive at most 512 bytes for one PCS1 text packet, rejecting oversized bodies before writing beyond the buffer.
4. Validate the complete packet before updating the screen.
5. Draw a title and up to seven lines through the font/display adapter.
6. Fetch another result or reader page when navigating; discard the previous response.
7. Return to query entry or home through physical controls.

`crates/chromatic-client` is dependency-free and `no_std`. Its decoder borrows the receive buffer and checks sizes, UTF-8, cell widths, pagination, flags, and reserved bytes. It does not allocate, draw, open sockets, or operate buttons directly. The encoded packet and decoded view can share one 512-byte receive buffer. TLS, networking, fonts, UI state, and the existing framebuffer have separate costs.

The 24-cell row profile treats ASCII as one cell and other Unicode as two. It is a conservative layout policy, not Unicode shaping or a font coverage guarantee. The firmware must visibly substitute unsupported glyphs. Font metrics, title/footer placement, contrast, and clipping require physical inspection.

## Companion responsibilities

The companion owns collection, HTML extraction, ranking, source metadata, and image preparation. The current host code implements an in-memory inverted index, Japanese/English tokenization, BM25-style scoring, and packet formatting. It reads JSON fixtures; it exposes bounded read-only HTTP routes but does not fetch websites.

The device receives one result per response, up to ten ranked results. Each result references a document through a numeric ID. IDs are currently one-based positions in the loaded corpus. A service must keep that corpus immutable while a client uses its IDs. Persistent storage and a corpus-generation token must be added before live index replacement, preventing an ID from opening an unrelated document after a rebuild.

Bodies are split into seven-row pages. Titles can be shortened with an ellipsis; result excerpts can be shortened with an instruction to open the full page. Full bodies remain on the host and are paginated rather than silently discarded. Full source URLs stay on the host; a source/details action is needed to inspect URLs too long for a result screen.

## Experimental service routes

`src/server.rs` uses a small standard-library HTTP service with four workers and eight queued sockets. It serves one immutable sample corpus:

- `GET /v1/chromatic/search?q=...&result=0`: one PCS1 result packet.
- `GET /v1/chromatic/page?id=...&page=0`: one PCS1 reader packet.
- `GET /v1/search?q=...`: up to ten JSON results for the companion browser.
- `GET /v1/collection`: the sample collection label and count.
- A later registered-image route: bounded RGB565 stripes for an image referenced by a collected document.

Reject queries longer than 96 UTF-8 bytes and unknown document/page IDs. Use explicit content lengths, bounded request/response times, and no query/body logging. A zero-result packet has document ID zero and cannot open an invented document. Collection metadata is currently a fixed sample label. Corpus generation and collected-page provenance must be added before live updates. See [API limits](search-results.md). This protocol is not Google search or a general-purpose HTML browser.

## Planned public-page collection

Use explicit seed URLs and domains, obey robots.txt, and cap page count, depth, response bytes, timeouts, and per-host frequency. Fetch public HTTPS pages with certificate verification. Validate resolved addresses, pin the approved destination, and validate redirects before following them. Exclude private/local destinations and authenticated content.

Extract title, source URL, body, links, image references, and collection time. Render extracted content as data. Do not execute page scripts on the handheld or invent content for unsupported sites. Crawling and network boundary tests remain pending.

## Image path

Resize an actual site's image on the companion to fit within 128 × 80 pixels and send RGB565 stripes. Eight rows cost 2,048 bytes, avoiding a second 46,080-byte full-display buffer. Stripe headers, ordering, interrupted transfers, and display integration need implementation. Peak memory with TLS active must be measured before treating this budget as viable. Use actual source images, not generated photos presented as web content.

## Connectivity and acceptance

Use a reachable HTTPS endpoint with a certificate accepted after native time synchronization. Earlier hardware tests verified outbound HTTPS but did not establish device-to-computer LAN reachability. A USB relay or host example must not be reported as Wi-Fi-native search.

The first hardware acceptance test is query entry → verified HTTPS response → result selection → complete text pagination, including empty results, network errors, and repeated navigation. Record sanitized firmware revision, packet sizes, minimum/largest free heap, timing, and physical observations. Hardware writes and captures follow the ModRetro deployment workflow and preserve existing backups and settings.
