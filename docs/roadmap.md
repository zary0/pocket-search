# Hardware-first roadmap

## 0. Chromatic development foundation

- [x] Make Chromatic the primary platform and document the MCU/service boundary.
- [x] Provide English README files and architecture documentation.
- [x] Implement Japanese/English search and ranking on the host.
- [x] Implement bounded text packet formatting for the 160 × 144 target.
- [x] Implement an allocation-free `no_std` decoder and button action mapping.
- [x] Check pagination, malformed packets, query limits, and host/client compatibility.
- [x] Add GPL-3.0-only licensing and English contribution/security guidance for public development.
- [x] Add repeatable host timing/allocation measurements and compact bounded search selection.
- [x] Cross-compile an ESP32 CPU-cycle/heap probe; physical execution is pending.

These checks run on a host. They are not a firmware build or physical browsing result.

## 1. First physical search and text loop

- [ ] Build an Xtensa-compatible client core and firmware adapter with existing display, buttons, fonts, Wi-Fi, SNTP, and TLS configuration.
- [ ] Add bounded HTTP routes for one immutable document collection.
- [ ] Verify a reachable HTTPS endpoint on the actual Wi-Fi network.
- [ ] Implement query entry, result navigation, text pagination, loading, timeout, and error screens.
- [ ] Verify query → result → text physically; record packet sizes, timing, free heap, and largest free block.
- [ ] Measure pinned-core decode cycles and stack use, then select the actual Xtensa size/speed profile.
- [ ] Verify repeated navigation and reconnection without losing responsiveness.
- [ ] Verify battery-only operation separately from USB-powered diagnostic testing.

Initial fixture tests must be visibly identified. They do not complete the real-web milestone.

## 2. Real collected pages

- [ ] Add a robots-aware, size/rate/depth-limited public HTTPS crawler and HTML extractor.
- [ ] Preserve source URLs, collection scope, timestamps, links, and original image references.
- [ ] Add deduplication, persistence, and corpus generation handling before replacing an active index.
- [ ] Query actual collected pages on the handheld and inspect full source information.
- [ ] Confirm query-dependent results from the actual index, including honest zero-result responses.

## 3. Still images

- [ ] Fetch registered site images and resize them on the companion.
- [ ] Define and validate bounded RGB565 stripe transport.
- [ ] Display one reduced image and return to text with B.
- [ ] Measure TLS plus image/display peak memory and handle interrupted transfers.

## 4. Reading and quality

- [ ] Refine Japanese input, wrapping, font fallback, search quality, links, and book text import.
- [ ] Keep temporary back navigation in memory only.
- [ ] Evaluate externally prepared low-resolution video after still-image acceptance.

Bookmarks, browsing history, launch-time reading restoration, and the Codex approval application are outside the product scope.
