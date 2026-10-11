![Pocket Search logo: a pixel document and search lens beside the wordmark](assets/pocket-search-logo.png)

# Pocket Search for ModRetro Chromatic

A small, experimental **on-device web collector, search engine and text reader** for the ModRetro Chromatic MCU. The current native mode fetches public HTTPS pages directly, follows a bounded set of links, builds its own Rust index in RAM, and searches and reads that retained text on the console. **A Mac, companion search service, temporary tunnel and USB page proxy are not part of the native data path.**

The first autonomous profile starts at Stanford's historical AI questions page and stays within its `whatisai/node*.html` pages. It admits at most four documents, not the entire Internet. The repository also retains optional host tools for broader crawling, DOM extraction, image preparation and browser development. Those tools do not expand the native profile automatically.

This is MCU/menu firmware, not a Game Boy cartridge ROM or a replacement Rust OS. Full reproducible firmware packaging and qualified physical E2E remain release requirements.

## Architecture

```text
Public HTTPS site
        │ native DNS, clock sync, verified TLS
        ▼
Chromatic MCU
  bounded crawl frontier → streaming HTML + semantic tree → RAM inverted index
  buttons → local Rust ranking → local reader pagination → 160 × 144 LCD
```

Rust owns the restricted link frontier, text extraction, fixed-capacity term/document masks, ranking, PCS1 results and PFW1 semantic reader formatting. The existing ESP-IDF adapter owns HTTPS, cancellation, Wi-Fi and the display. Collection happens on the first submitted query after boot or a refresh. Later searches and reader navigation open no sockets; they use the device's retained collection, even if Wi-Fi is temporarily unavailable. Restart discards that RAM index and the next search collects again.

| Component | Autonomous native mode | Current limit |
| --- | --- | --- |
| Crawler | Direct verified HTTPS, same-profile link discovery, one request at a time, robots policy check | One fixed public host/path profile, eight frontier entries, four retained pages, 90-second collection deadline |
| HTML extraction | Streaming text/title and a bounded semantic tree: headings, paragraphs, nested lists, links and image slots; script/style/comment exclusion | Compact parser, not HTML5 layout, JavaScript or complete CSS layout |
| Index / search | Allocation-free inverted term hashes with document masks; exact matching verifies candidates; a full table falls back to bounded text scanning | 256 term slots, ASCII word matching and non-ASCII substring matching; not the host BM25/Japanese tokenizer |
| Reader | Results, A to open, up/down line scrolling, left/right screen jumps and B to return, all generated on the MCU | 2,560 retained UTF-8 bytes, 48 typography transitions and 24 semantic nodes per document; visible limit markers |
| Typography resources | Basic text color, solid background, pixel margin/padding/border CSS; one cached external sheet and static TrueType ASCII atlas | One <=3 KiB sheet / eight selectors, raw TTF <=24 KiB; no WOFF2, shaping or proportional font metrics |
| Images / video | Native image position/source/alt placeholders; earlier companion-mode still-image experiment remains in the host tools and evidence | Autonomous image decoding/resizing and video are not implemented |

See [semantic layout and CSS scope](docs/native-flow-layout.md), [native typography](docs/typography.md), [on-device scope and verification](docs/on-device-search.md), [architecture](docs/architecture.md), and the [product roadmap](docs/roadmap.md).

## Designed for the handheld

- **160 × 144 pixels**, a title, a 160 × 92 flow reading viewport and button hints.
- One result per screen; every match in the admitted four-page collection is reachable.
- Headings at **16/14 px**, body at **12 px**, annotations at **10 px**, inline bold and paragraph gaps. Up to eight variable-height rows fit each validated viewport.
- **96-byte UTF-8 queries** and **512-byte locally generated text packets**.
- A fixed **22,272-byte opaque Rust state region**, plus a **2,049-byte robots buffer**. TLS, task stacks, fonts and the 15 KiB display band have separate costs.
- No bookmarks, browsing history, restored reading position or Codex approval UI.
- Source URLs are displayed in results. Unsupported active content is treated as data and never executed.

| Button | Home | Results | Reader |
| --- | --- | --- | --- |
| A | Enter a query | Open selected document | Open the first visible already-collected link |
| Start | Submit current query | Query entry | Query entry |
| Up / down | Reserved | Previous / next result | Scroll up / down one text line |
| Left / right | Right clears collection for the next query | Reserved | Jump up / down seven lines |
| B | System menu | Home | Results |
| Select | Wi-Fi setup | Home | Home |

USB diagnostics can queue synthetic input and read the MCU render buffer for development; they do not send pages or search results to the console. Physical-controls and battery-only acceptance are recorded separately from those diagnostics.

## Runtime and development environment

An optional [native button-click implementation](docs/native-click.md) generates short feedback in the FPGA while menus are open. Its RTL simulation and licensed Gowin 1.9.12.04 build pass with zero reported timing violations; physical speaker testing, clock-domain review and recovery qualification remain pending.

| Layer | Environment and validation status |
| --- | --- |
| Chromatic development unit | ESP32-U4WDH / Xtensa at 160 MHz, 4 MiB flash, a 2 MiB application partition, no enabled PSRAM, 160 × 144 RGB565 display |
| Native platform | ESP-IDF 5.3, FreeRTOS, LVGL, native Wi-Fi/SNTP and certificate/hostname validation; experimental operator-local integration |
| Native Rust core | Dependency-free `no_std`, no allocator; local collection/search plus packet/image validation. Xtensa build uses the installed Espressif Rust 1.97 nightly toolchain |
| Optional host tools | Rust 2024, Rust/Cargo 1.95 locally on macOS / Apple M4 / aarch64; hosted Ubuntu / x86_64 CI uses stable Rust |
| Checks / packaging | Git, Python 3.11+, Node.js and Rust/Cargo; physical release evidence remains required |

The native core does not make the entire firmware bare metal. Networking, task stacks and display integration continue to use the established platform. Optimize against real target heap, stack and execution measurements; see [performance](docs/performance.md). Hardware values describe the inspected development unit and are not guarantees for every console.

## Run host checks

```sh
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo build --locked -p chromatic-client --profile chromatic-size
cargo build --locked -p chromatic-client --profile chromatic-speed
```

These validate the portable core, including streaming boundaries, capacity limits, honest empty results and the C boundary. They are not physical device tests.

Optional companion tools remain available:

```sh
cargo run -- crawl examples/crawl-ai.json data/ai-001
cargo run -- serve-crawl data/ai-001 127.0.0.1:8858
cargo run -- serve examples/documents.json 127.0.0.1:8828 --html examples/html-manifest.json
```

The companion admits up to 256 pages, can prepare PNG/JPEG images, and uses a richer HTML5 parser and search model. Its browser preview shows companion behavior. The original fixtures contain invented `.invalid` URLs and are explicitly marked as samples; none are bundled into the autonomous native collection.

## Hardware evidence and limits

The current on-device application is `0.13.21-css`, with semantic font sizes, inline bold, paragraph spacing and up/down line scrolling, plus a bounded external-CSS cascade and a small static TrueType web-font subset. Styles and font metrics are normalized for the small screen; this is not full browser fidelity. See [native CSS/web-font scope](docs/native-css-webfonts.md) and [typography evidence](docs/typography.md). The earlier `0.13.17-local` experiment established autonomous collection: With Mac services and tunnels stopped, it directly collected four public pages and returned four `AI` matches. The operator confirmed battery-only restart, query entry and A to read. One first collection took 18.241 seconds; local ranking/packet generation took 5.042 ms on the inspected console. See [measurements and acceptance limits](docs/on-device-search.md), [standalone evidence](docs/evidence/on-device-search/verification.json) and [scroll verification](docs/evidence/reader-scroll/verification.json).

App-only installation preserves the established partition table, bootloader, FPGA and Wi-Fi settings. The index is volatile; there is no flash cache yet. The source profile, parsing and memory limits are explicit, so the application does not claim general browser support or Internet-wide search.

Earlier [real-site companion evidence](docs/evidence/real-sites/verification.json) verified 13 externally collected pages, native Wi-Fi reader responses and a user-confirmed source image. That evidence describes `0.13.16-web`, whose Mac/tunnel dependency prompted the autonomous mode. It is not evidence of native image decoding or local indexing. Historical [service setup](docs/search-service.md) applies only to that older companion mode.

## Next milestone

Qualify standalone collection, search, reading and errors with physical controls and battery power, then make the complete firmware integration reproducible. Next add configurable public source profiles and measured persistent storage, followed by on-device image decoding/resizing. The priority is autonomous, bounded operation on the console.

## Open-source development

Repository-authored code, documentation, artwork, and original fixtures are licensed under **GPL-3.0-only**. See [LICENSE](LICENSE), [project notice](NOTICE.md), and [third-party notices](THIRD_PARTY_NOTICES.md). Dependencies retain their own licenses. This is an independent project, not an official ModRetro product. The repository is prepared for public development; changing GitHub visibility is a separate publishing action.

Contributions should keep the device path bounded and justify performance changes with reproducible measurements. See [contributing](CONTRIBUTING.md), [security reporting](SECURITY.md), and [performance notes](docs/performance.md). The initial optimization replaces nested posting trees, candidate intersections, and full-hit formatting with compact vectors, rarest-term planning, and bounded top-k selection. Client decoding remains allocation-free and uses a single combined text-validation pass.

```sh
cargo bench --locked --bench performance > performance.json
```

The Rust search and decoder timings are host-only measurements on Apple M4. A separate C/LVGL display probe measures native rendering on one physical console; see [display integration](docs/chromatic-rendering.md) and [performance results](docs/performance.md). The CPU-cycle probe has a target compile check; the experimental search image now executes the Rust decoder on Xtensa. Decoder cycle/peak-memory measurements and profile comparison remain pending.

## CI, CD, and versions

Development integrates through **`develop`**, then promotes to **`main`**. Stable releases use annotated **`vX.Y.Z`** tags matching the single workspace version. Development package names include the base version and commit and cannot be published as stable releases.

Hosted CI checks versions, Rust behavior, bounded C/JavaScript input, and release-gate unit tests. Develop pushes record physical E2E as **not run** when the lab is explicitly disabled; once enabled they invoke qualification automatically. Main pushes, manual qualification, and release tags always require the real physical gate. Only a native Wi-Fi E2E result for the exact firmware/candidate permits release publication; missing or skipped hardware tests block it. The lab runner, complete native application, driver, and fixture origin are pending, so no physical E2E pass or firmware release is claimed.

See [branches and release packages](docs/releases.md) and [physical E2E lab setup](docs/hardware-e2e.md). Packages carry source, companion and native image hashes, build identity, validated evidence, and checksums. Publication promotes tested assets without rebuilding them.

See [GitHub governance](docs/github-governance.md) for review ownership, branch-protection payloads, dependency updates, and the current status of repository security controls. Configuration files describe the intended rules; GitHub settings must be enabled and read back before claiming enforcement.

## Security

Read the [security policy](SECURITY.md) for supported versions, confidential reporting, disclosure coordination, device credentials, and the service/device/CI trust boundaries. The policy distinguishes implemented packet and release checks from implemented experimental transport bounds and pending crawler/production protections.

There is no production release or deployed public service yet. GitHub private vulnerability reporting is not currently available for this private repository. Until a confidential channel is enabled and verified, request a private reporting contact without posting vulnerability details, credentials, or device dumps in an issue. The security policy explains the current reporting route and public-release prerequisites.
