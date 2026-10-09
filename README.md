# Pocket Search for ModRetro Chromatic

A small web search and reading application designed around the **ModRetro Chromatic's 160 × 144 display, physical buttons, and ESP32 MCU**. Enter a query on the handheld, select a result, and read the site's text and reduced images over Wi-Fi.

Chromatic is the primary platform. Host tools develop the search service and verify the device protocol. The application targets the console's MCU firmware and menu interface; a Game Boy cartridge ROM is not the deployment target for this Wi-Fi reader.

**Status: development foundation, not an installed browser.** Implemented: Rust search core, bounded text packet encoder, allocation-free client decoder, and button action mapping. Pending: crawling, HTTP API, firmware integration, and image display. No Pocket Search firmware has been flashed or tested on the physical console.

## Architecture

```text
Public HTTPS sites
        │
        ▼
Rust companion service
  crawl → extract text/images → index → rank → format small responses
        │
        │ verified HTTPS, one bounded response at a time
        ▼
Chromatic MCU application
  button input → query → results → paginated text → reduced image
```

The handheld owns the interaction and display. A companion service on a computer or server owns crawling, the index, and image preparation. Independent search ranks our own collected documents; it does not require storing a web-scale index on the handheld. The service must be reachable from the device. No public endpoint is deployed yet.

| Component | Implemented | Still needed |
| --- | --- | --- |
| Crawler / extractor | None | Bounded HTTPS fetching, robots.txt, HTML extraction, collection timestamps |
| Index / ranking | In-memory inverted index, BM25-style ranking, English words and Japanese character/bigram tokens | Persistent corpus, updates, deduplication, quality evaluation |
| Companion API | Host-side PCS1 text packet encoder | HTTP routes, corpus identity, deployment, real content |
| Device client | `no_std` decoder and button actions | Xtensa build, firmware adapter, Wi-Fi transport, input UI, font rendering, physical tests |
| Images | Proposed 128 × 80 RGB565 stripe budget | Fetching, resizing, stripe protocol, display and heap measurements |

## Designed for the handheld

- **160 × 144 pixels**, one screen at a time.
- A title, **seven body rows**, and a button hint area. Each row has 24 conservative cells: one for ASCII and two for other Unicode characters.
- One result per screen and at most ten results per query.
- A **96-byte UTF-8 query limit** and a **512-byte text response limit**.
- The decoder borrows the response buffer without heap allocation. Networking, the display driver, and fonts have separate memory costs.
- One proposed image at a time, up to 128 × 80 pixels. An eight-row RGB565 stripe would need **2,048 bytes**; image transport is not implemented yet.
- Search, results, text, images, links, and temporary back navigation. No bookmarks, browsing history, restored reading position, or Codex approval UI.

| Button | Result screen | Reader screen |
| --- | --- | --- |
| D-pad up / down | Previous / next result | Reserved |
| D-pad left / right | Reserved | Previous / next text page |
| A | Open selected document | Reserved for later link/image actions |
| B | Back | Back to results |
| Start | Query entry | Query entry |
| Select | Home | Home |

These actions are implemented in the client core, not yet connected to physical button events. The query keyboard is pending; English and Japanese input remain in scope.

## Run host checks

Use stable Rust and Cargo. Host checks do not require an ESP32 toolchain.

```sh
# Search the original fixture documents.
cargo run -- examples/documents.json 'chromatic wifi'

# Build and decode a result packet, then open its first reader page.
cargo run --example chromatic -- examples/documents.json 'chromatic wifi'

# Run all search and protocol checks.
cargo test --workspace
cargo check -p chromatic-client
cargo clippy --workspace --all-targets -- -D warnings
```

The Chromatic example runs entirely on the host and prints that distinction. `examples/documents.json` contains original test text and nonexistent `.invalid` URLs, including Japanese fixtures. It is not a crawled corpus or evidence of hardware browsing.

## Repository layout

```text
src/                         Host search index, CLI, and packet formatting
crates/chromatic-client/     Dependency-free, no_std decoder and button actions
examples/                    Original fixtures and host protocol check
devices/chromatic/           Target profile and firmware integration plan
docs/architecture.md         Device/service boundary and collection design
docs/chromatic-protocol.md   PCS1 text format and response limits
docs/roadmap.md              Hardware-first implementation milestones
```

## Hardware evidence and limits

Earlier work on one Chromatic DevDay unit verified Wi-Fi, native SNTP, and certificate-validated HTTPS fetching. That firmware and its verification artifacts are outside this repository. Those checks do not verify Pocket Search, image display, or battery-only operation.

The development configuration used ESP32-U4WDH at 160 MHz, 4 MiB flash, no enabled PSRAM, ESP-IDF 5.3, and LVGL with RGB565. Available heap before a small HTTPS request was about 58 KiB. These are observations of that unit and firmware, not a universal free-memory guarantee. See the [device profile](devices/chromatic/README.md).

The development Wi-Fi allowed outbound HTTPS but did not establish device-to-computer LAN reachability. Hardware testing needs a verified reachable endpoint; a running local service alone does not prove a Wi-Fi path to the device.

## Next milestone

Build the firmware adapter for query entry, result selection, and text reading, together with the bounded companion API. Complete a physical query → result → text test before expanding the browser experience. Then add collected pages and reduced images. Video is deferred until text and still-image behavior is measured on the device.

See the [architecture](docs/architecture.md), [protocol](docs/chromatic-protocol.md), and [roadmap](docs/roadmap.md). Device credentials, activation codes, NVS dumps, and full-flash backups are not included.

## Open-source development

Repository-authored code, documentation, and original fixtures are licensed under **GPL-3.0-only**. See [LICENSE](LICENSE) and [notices](NOTICE.md). Dependencies retain their own licenses. This is an independent project, not an official ModRetro product. The repository is prepared for public development; changing GitHub visibility is a separate publishing action.

Contributions should keep the device path bounded and justify performance changes with reproducible measurements. See [contributing](CONTRIBUTING.md), [security reporting](SECURITY.md), and [performance notes](docs/performance.md). The initial optimization replaces nested posting trees, candidate intersections, and full-hit formatting with compact vectors, rarest-term planning, and bounded top-k selection. Client decoding remains allocation-free and uses a single combined text-validation pass.

```sh
cargo bench --locked --bench performance > performance.json
```

Checked-in timings are host-only measurements on Apple M4, not physical Chromatic speed claims. The ESP32 CPU/heap probe has a target compile check; device execution, Rust Xtensa integration, and actual profile selection remain pending.

## CI, CD, and versions

Development integrates through **`develop`**, then promotes to **`main`**. Stable releases use annotated **`vX.Y.Z`** tags matching the single workspace version. Development package names include the base version and commit and cannot be published as stable releases.

Hosted CI checks versions, Rust behavior, and release-gate unit tests. Trusted branch pushes and tags invoke physical Chromatic qualification; only a real native Wi-Fi E2E result for the exact firmware/candidate permits release publication. Missing or skipped hardware tests block the release. The lab runner, native application, driver, and fixture origin are currently pending, so no physical E2E pass or firmware release is claimed.

See [branches and release packages](docs/releases.md) and [physical E2E lab setup](docs/hardware-e2e.md). Packages carry source, companion and native image hashes, build identity, validated evidence, and checksums. Publication promotes tested assets without rebuilding them.
