# Architecture

## Primary native data path

Pocket Search runs a small collector, index and reader on the Chromatic MCU. Public HTTPS pages flow directly to the device; a host computer does not collect, rank, format or proxy native page data. The current firmware keeps ESP-IDF 5.3, FreeRTOS, Wi-Fi, SNTP, verified TLS, LVGL and the established input/display drivers. Its dependency-free Rust core is compiled for `xtensa-esp32-none-elf` and called through `chromatic-ffi`.

On first search after boot or refresh, the adapter checks Wi-Fi and clock synchronization, retrieves the source host's robots policy, and asks Rust for the next permitted URL. Rust starts with one public seed, recognizes only relative or exact HTTPS links within the fixed source profile, deduplicates the frontier, and admits at most four complete pages. The C adapter verifies certificates and hostnames, disables redirects, requests identity encoding, reads at most 512 bytes at a time, and caps each HTML response at 64 KiB and the collection at 90 seconds. It does not send search terms to the website.

The streaming parser keeps a title and at most 2,560 valid UTF-8 body bytes per document. It excludes scripts, styles and comments, normalizes whitespace while retaining paragraph breaks, records up to 48 size/weight transitions, decodes basic/numeric entities, and never runs page code. It interprets a bounded subset of semantic tags, legacy font sizes and inline font-size/font-weight. It is a compact parser, not an HTML5 DOM. A separate bounded external stylesheet cascade and static TrueType subset now extend the typography; see [resource pipeline and limits](native-css-webfonts.md). Truncation is visible at the end of the local reader.

The fixed-capacity inverted index maps 256 term hashes to document masks. Exact retained-text matching verifies each candidate, avoiding false results from hash collisions. If the table fills, matching falls back to scanning the four bounded documents. ASCII query words all have to match; non-ASCII terms use substring matching. Title occurrences receive higher weight than body occurrences; ties retain collection order. This is independent search, not the host BM25 model or Google results.

## Memory, ownership and concurrency

The C adapter reserves one aligned 16,640-byte opaque state region and a 2,049-byte robots buffer. Rust performs no allocation. The one network worker exclusively owns Rust state; status reads use copied counters, and refresh refuses a running job or pending response. The FFI checks pointer alignment, region size and input lengths. Calls still require readable/writable, non-overlapping caller-owned regions; those contracts are explicit in the source.

A retained collection is immutable while results/readers are in use. Rust generates one bounded PCS1 result or PSR1 styled reader viewport locally. Its decoder validates borrowed text, packed styles and exact pixel geometry before LVGL displays it. In autonomous text reading, the position is the viewport's top wrapped line: up/down moves one line and left/right jumps seven, clamped at both ends. Reader rows use 10/12/14/16-pixel fonts with two pixels of line spacing and six-pixel empty paragraph rows. Up to eight rows fit the 144 × 92 viewport; the final position is computed against both pixel and packet budgets. No persistent full-page text cache or additional framebuffer is allocated. A temporary <=3 KiB snapshot preserves a complete original document across a CSS reparse. Scaled views of the existing bitmap font use 656 bytes of glyph scratch/cache, plus one immutable 867-byte downloaded font atlas. See [typography](typography.md). No request to a companion is made during search or navigation. Verified flags on cached packets describe source provenance from collection, not a new TLS handshake. Cancellation discards incomplete page text. Refresh clears the volatile collection and the next submitted query recollects it. Reset discards the index; Wi-Fi configuration remains in NVS.

Networking, TLS, the 10 KiB worker stack, GUI objects, fonts and the 15 KiB display band have separate memory costs. The fixed index budget is not total firmware RAM use. Physical minimum free heap, largest block, stack high-water mark and local compute time must be measured with collection and reading active.

## Scope and security boundaries

The initial profile is one fixed public host and the `jmc/whatisai/node*.html` path family. External hosts, credentials, ports, query strings, fragments, parent paths and arbitrary user-supplied URLs cannot enter the frontier. One serial request stream waits at least a second between page requests. A missing robots resource confirmed with HTTP 404/410 allows collection; other status errors, redirects, unsupported encoding, invalid or oversized policies fail closed. The compact policy evaluator handles matching agent groups, longest allow/disallow rules and basic wildcard/end markers for these ASCII paths; it refuses percent-encoded policies instead of guessing. It is not a general RFC-compliant robots implementation.

The on-device mode does not implement general robots metadata/noindex/nofollow interpretation, image decoding, arbitrary links, forms, authentication, JavaScript or video. The restricted research profile and clear limits remain part of the experiment. Expand the parser and policy only with equivalent bounded tests and physical measurements.

## Optional host path

The original Rust CLI remains useful for broad public-page collection, richer HTML5 parsing, BM25/Japanese tokenization, immutable snapshots, image preparation and browser development. Its API and browser are optional companion tools, with their own collection-generation and transport checks. They do not participate in the autonomous native path and must not be used as its acceptance evidence.

Earlier native image display used host-prepared RGB565/PCI1 stripes. Autonomous image acquisition/decoding/resizing is a separate pending milestone. Existing image and API evidence remains labeled with its actual firmware and execution mode.

## Acceptance

See [on-device scope and verification](on-device-search.md). Verify exact app-only image identity and settings preservation, stop the task-owned companion and tunnel, cold-start the native collector, search real fetched pages, navigate text, test honest empty results and cancellation, and distinguish synthetic USB controls/render-buffer reads from physical-button/LCD/battery observations. No USB page proxy or host-emulated frame proves standalone execution. Release remains subject to the complete physical E2E and recovery gate.
