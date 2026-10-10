![Pocket Search logo: a pixel document and search lens beside the wordmark](assets/pocket-search-logo.png)

# Pocket Search for ModRetro Chromatic

A small web search and reading application designed around the **ModRetro Chromatic's 160 × 144 display, physical buttons, and ESP32 MCU**. Enter a query on the handheld, select a result, and read the site's text and reduced images over Wi-Fi.

Chromatic is the primary platform. Host tools develop the search service and verify the device protocol. The application targets the console's MCU firmware and menu interface; a Game Boy cartridge ROM is not the deployment target for this Wi-Fi reader.

**Status: development foundation, not an installed browser.** Implemented: Rust search core, bounded text packet encoder, allocation-free client decoder, and button action mapping. Pending: crawling, HTTP API, Rust firmware integration, and image display. A separate native C/LVGL platform probe has been installed and measured physically: its selected prototype cuts the draw buffer from 45 to 15 KiB and improves small updates, with a full-redraw latency tradeoff. It does not include the Rust reader or verify browsing. See [measured results](docs/performance.md#physical-lvgl-display-comparison).

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
| Device client | `no_std` decoder/button actions on the host; separate C/LVGL search home and bounded keyboard prototype | Rust Xtensa build, decoder/transport integration, results/reader, physical search E2E |
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

These result/reader actions are implemented in the client core but are not connected to a physical search flow. A separate native C UI provides the search home and a bounded hiragana/Latin keyboard; its search service remains unconnected. See [search UI status](docs/search-ui.md) and the [browser preview](ui/README.md).

## Runtime and development environment

| Layer | Environment and validation status |
| --- | --- |
| Host CLI and search core | Native Rust 2024 program. Validated locally on macOS / Apple M4 / aarch64 with Rust and Cargo 1.95.0, and in hosted Ubuntu / x86_64 CI with stable Rust. Other host platforms are not verified |
| Build and release tooling | Rust/Cargo for builds; Git and Python 3.11+ for version checks, packaging, and release-gate tests. The compiled CLI runs on its matching host OS/architecture |
| Chromatic development hardware | One previously inspected DevDay unit: ESP32-U4WDH / Xtensa at 160 MHz, 4 MiB flash, a 2 MiB application partition, and no enabled PSRAM. Display: 160 × 144 RGB565 |
| Native firmware integration target | ESP-IDF 5.3 with FreeRTOS, the existing Wi-Fi/network/TLS stack, and LVGL display integration. Pocket Search's firmware adapter and Rust Xtensa build remain pending |
| Portable device core | Dependency-free `no_std` Rust with no allocator or OS bindings. It decodes bounded packets and maps buttons; networking, fonts, and rendering belong to the pending firmware adapter |
| Network/service runtime | A DNS-resolvable HTTPS service reachable from the device, with native clock synchronization and certificate/hostname validation. The companion HTTP API and public endpoint are not implemented yet |

The manifest declares Rust 1.85 as its minimum version; that minimum is not separately checked in CI. Current validation uses the toolchains above. The hardware values describe the earlier development unit and firmware, not a tested Pocket Search image or guaranteed resources on every Chromatic. See the [device profile](devices/chromatic/README.md) and [lab requirements](docs/hardware-e2e.md).

The initial firmware plan keeps ESP-IDF/FreeRTOS and integrates the small Rust core. A `no_std` core does not make the complete Wi-Fi firmware bare metal. Optimize task stacks, network/TLS buffers, and incremental drawing against real-device measurements before considering a replacement platform; see the [performance trade-offs](docs/performance.md#rtos-footprint-and-bare-metal).

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

## Hardware evidence and limits

Earlier work on one Chromatic DevDay unit verified Wi-Fi, native SNTP, and certificate-validated HTTPS fetching. That firmware and its verification artifacts are outside this repository. Those checks do not verify Pocket Search, image display, or battery-only operation.

Available heap on that earlier unit before a small HTTPS request was about 58 KiB. This is an observation of that unit and firmware, not a universal free-memory guarantee. Its hardware/software environment is summarized above; see the [device profile](devices/chromatic/README.md) for exact measurements.

The development Wi-Fi allowed outbound HTTPS but did not establish device-to-computer LAN reachability. Hardware testing needs a verified reachable endpoint; a running local service alone does not prove a Wi-Fi path to the device.

## Next milestone

Build the firmware adapter for query entry, result selection, and text reading, together with the bounded companion API. Complete a physical query → result → text test before expanding the browser experience. Then add collected pages and reduced images. Video is deferred until text and still-image behavior is measured on the device.

See the [architecture](docs/architecture.md), [protocol](docs/chromatic-protocol.md), and [roadmap](docs/roadmap.md). Device credentials, activation codes, NVS dumps, and full-flash backups are not included.

## Open-source development

Repository-authored code, documentation, artwork, and original fixtures are licensed under **GPL-3.0-only**. See [LICENSE](LICENSE), [project notice](NOTICE.md), and [third-party notices](THIRD_PARTY_NOTICES.md). Dependencies retain their own licenses. This is an independent project, not an official ModRetro product. The repository is prepared for public development; changing GitHub visibility is a separate publishing action.

Contributions should keep the device path bounded and justify performance changes with reproducible measurements. See [contributing](CONTRIBUTING.md), [security reporting](SECURITY.md), and [performance notes](docs/performance.md). The initial optimization replaces nested posting trees, candidate intersections, and full-hit formatting with compact vectors, rarest-term planning, and bounded top-k selection. Client decoding remains allocation-free and uses a single combined text-validation pass.

```sh
cargo bench --locked --bench performance > performance.json
```

The Rust search and decoder timings are host-only measurements on Apple M4. A separate C/LVGL display probe measures native rendering on one physical console; see [display integration](docs/chromatic-rendering.md) and [performance results](docs/performance.md). The CPU-cycle probe has a target compile check; physical Rust decoder execution, Xtensa integration, and profile selection remain pending.

## CI, CD, and versions

Development integrates through **`develop`**, then promotes to **`main`**. Stable releases use annotated **`vX.Y.Z`** tags matching the single workspace version. Development package names include the base version and commit and cannot be published as stable releases.

Hosted CI checks versions, Rust behavior, bounded C/JavaScript input, and release-gate unit tests. Develop pushes record physical E2E as **not run** when the lab is explicitly disabled; once enabled they invoke qualification automatically. Main pushes, manual qualification, and release tags always require the real physical gate. Only a native Wi-Fi E2E result for the exact firmware/candidate permits release publication; missing or skipped hardware tests block it. The lab runner, complete native application, driver, and fixture origin are pending, so no physical E2E pass or firmware release is claimed.

See [branches and release packages](docs/releases.md) and [physical E2E lab setup](docs/hardware-e2e.md). Packages carry source, companion and native image hashes, build identity, validated evidence, and checksums. Publication promotes tested assets without rebuilding them.

See [GitHub governance](docs/github-governance.md) for review ownership, branch-protection payloads, dependency updates, and the current status of repository security controls. Configuration files describe the intended rules; GitHub settings must be enabled and read back before claiming enforcement.

## Security

Read the [security policy](SECURITY.md) for supported versions, confidential reporting, disclosure coordination, device credentials, and the service/device/CI trust boundaries. The policy distinguishes implemented packet and release checks from pending networking and crawler protections.

There is no production release or deployed public service yet. GitHub private vulnerability reporting is not currently available for this private repository. Until a confidential channel is enabled and verified, request a private reporting contact without posting vulnerability details, credentials, or device dumps in an issue. The security policy explains the current reporting route and public-release prerequisites.
