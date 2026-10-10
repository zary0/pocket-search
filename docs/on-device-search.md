# Autonomous Chromatic search

The current native application collects, indexes, searches and paginates a small public-web collection on the Chromatic itself. A Mac search service, USB page proxy and temporary HTTPS tunnel are not part of its data path. The inspected development unit runs application `0.13.17-local` with the existing FPGA `18.14` unchanged.

## Collection and search

The first submitted query after boot starts direct, certificate- and hostname-verified HTTPS collection after native clock synchronization. The seed is `https://www-formal.stanford.edu/jmc/whatisai/node1.html`. Only `node[0-9]{1,2}.html` links under that exact HTTPS host and directory can enter the frontier. External links, redirects, credentials, ports, queries and parent paths are rejected. No page text is bundled into the native index.

Collection retains at most four documents from an eight-entry frontier. One worker on CPU 0 uses 512-byte reads, a 64 KiB response-body limit, an eight-second I/O timeout and a cooperatively checked 90-second collection deadline. A blocking I/O call can finish after that deadline; it is not a hard real-time guarantee. Page requests are serial and separated by at least one second. A failed or incomplete page is never published as a searchable document. Successfully completed pages can remain usable when a later page fails.

The native crawler checks the source host's `robots.txt` before collecting pages. A verified HTTP 404/410 permits this limited collection; other failed requests, redirects and unsupported policies fail closed. The compact parser accepts at most 2,048 bytes of UTF-8 policy text, handles specific/wildcard groups and longest Allow/Disallow rules, and rejects percent-encoded policies. This conservative fixed-profile implementation is not a general RFC 9309 implementation. See the separate [companion crawler](crawling.md) for the richer host implementation.

Rust extracts a title and visible text incrementally, decodes basic/numeric entities, and excludes comments, script, style and non-title head content. This is a small text parser, not HTML5 DOM layout. It does not execute JavaScript, CSS, downloads or active content. Navigation labels may still appear in extracted text.

The local inverted index holds 256 term hashes and document masks. ASCII query words must all match; exact retained-text checks prevent a hash collision from producing a result. Title matches receive a greater weight than body matches, with stable ties. A full term table falls back to a bounded text scan. Non-ASCII queries use substring matching; Japanese keyboard and font coverage remain separate work. This device ranking differs from the optional host BM25/tokenizer implementation.

After collection, searches and reader pagination use RAM without network requests, clock synchronization or Wi-Fi availability. The `tls_verified` and `clock_synced` flags on cached replies describe collection provenance, not a new handshake. Results cover only retained text in the admitted collection; they are not Google results or an Internet-wide index.

## Controls and storage

From the search home, A opens the keyboard and Start submits a query. Results use up/down to select a match and A to open it. The reader uses left/right for text pages and B to return. Select goes home; Select on home opens Wi-Fi setup. Right on home clears the collection; the next submitted query collects again.

Queries are limited to 96 UTF-8 bytes. Results and text pages are generated locally as bounded 512-byte PCS1 packets. Each document retains at most 2,560 UTF-8 body bytes; the last reader page displays `[Text limit reached]` when text exceeds that capacity. Text beyond the limit is neither searchable nor readable. Document IDs are local to the current collection and reset with it. Empty results use document ID zero and cannot open a document.

The collection is volatile. A restart discards it and the next search repeats direct collection. There are no bookmarks, history, saved reading positions or flash index writes. Existing Wi-Fi settings remain in their established native storage. Any old companion-origin setting is unused; `search_endpoint status` reports the current mode, while `search_local status` and `search_local refresh` expose local counters and reset. Autonomous image acquisition/decoding/resizing and video are not implemented. Earlier companion-prepared image evidence does not establish native image support.

## Physical measurement on 2026-10-10

Measurements below describe one ESP32-U4WDH/Xtensa development unit at 160 MHz, without enabled PSRAM, using ESP-IDF 5.3/FreeRTOS and the size-profile Rust archive. They are one observed run, not distributions or worst-case guarantees.

| Measurement | Observed value |
| --- | --- |
| Published documents / `AI` matches | 4 / 4 |
| HTTPS requests for first collection | 5: robots plus four HTML pages |
| Received response-body bytes | 36,929 |
| First collection plus initial result | 18,241 ms |
| Rust local ranking and packet generation | 5,042 µs |
| Reserved opaque Rust state / target structure size | 16,384 / 14,816 bytes |
| Separate robots buffer | 2,049 bytes |
| Minimum sampled free heap / largest free block | 46,212 / 38,912 bytes |
| Worker stack high-water free margin | 3,692 bytes |
| Worker stack allocation | 10,240 bytes |
| Installed application | 1,900,144 bytes in the existing 2 MiB app partition |

The 5,042 µs timer brackets local Rust ranking/formatting, excludes HTTPS, and can include scheduling/interrupt time. It is not a CPU-cycle benchmark or a complete UI-latency measurement. Heap measurements are sampled during the worker; other tasks, TLS, fonts and the display band have additional memory costs. A fixed Rust state budget does not bound the entire firmware heap.

The application SHA-256 is `8ec7c0215ec5c537387e28a4d54906eebc8765edfe352ded7e9f1b2144b8c40d`. The linked Rust archive SHA-256 is `684736ab876d8f4906a313010bdef69581cf399e7ebac62d18d919e5ca680517`. Installation verified the previous app, the recovery backup, unchanged partition table and preserved NVS, then read back the installed app. Bootloader, FPGA and eFuses were not written. Private backups, credentials, vendor tools/licenses and firmware binaries are excluded from repository evidence.

## Acceptance evidence and limits

All Mac search services and their temporary tunnels were stopped before the standalone run. USB diagnostics queued synthetic controls and read status/render buffers; no pages, index or formatted results were supplied over USB. The diagnostic run observed four results, changed queries, an honest zero-result state, A/B navigation, every page index 0–17 of one retained reader document, first/last bounds and the visible truncation marker. Cached searches and reading left the request count at five.

After the diagnostic connection closed, the operator unplugged USB, restarted on batteries, entered `AI` with physical controls, obtained four results and opened the body with A. This is operator-confirmed battery-only cold-start collection/search/reading. Render-buffer captures are development evidence, not a camera or USB-video observation of the LCD.

See [sanitized verification](evidence/on-device-search/verification.json). This scoped standalone acceptance does not qualify a release. Complete physical controls, error/reconnection/cancellation, endurance, mixed-language font acceptance, a reproducible full firmware build and the updated trusted physical lab driver remain required. `CHROMATIC_E2E_READY` stays false; the older companion-oriented lab contract must be adapted before use. See [hardware E2E](hardware-e2e.md) and [release policy](releases.md).

The next priorities are configurable bounded public source profiles, improved text extraction, measured persistent content storage and native image decoding. Memory/CPU/display changes must preserve this working standalone flow and its transport, bounds and result correctness.
