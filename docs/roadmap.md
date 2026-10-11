# Product-first Chromatic roadmap

The outcome is a small public-web collector, search engine and reader that runs
on Chromatic without a Mac, companion search service or temporary tunnel.
Completion requires the actual installed candidate and physical acceptance;
a checked implementation item does not qualify a firmware release.

## P0 — Reproducible firmware and trustworthy physical operation

- [x] Allocation-free Rust streaming extraction, fixed frontier, inverted index,
  exact matching, locally generated results and bounded semantic reading windows.
- [x] Native Wi-Fi, SNTP, verified HTTPS, robots checks and bounded sequential
  requests. No host page proxy or external search API in autonomous mode.
- [x] Complete MCU application source overlay, fonts, display/button integration,
  public pinned dependencies, checksum-verified tool installer and clean build.
  Two macOS `0.14.0` builds match; isolated Linux native CI passes. See
  [build procedure and distribution boundary](native-firmware-build.md).
- [x] Correct cancellation retry: a cancelled robots request does not cache a
  denial. Unpublished cancelled pages remain resumable. Transfer/format errors
  retain readable content and free temporary image allocations.
- [ ] Verify **the latest candidate** on battery: cold start, saved corpus without
  Wi-Fi, reconnection, fresh collection, input, scrolling and image/back behavior.
  The earlier operator-confirmed battery experiment does not qualify new changes.
- [ ] Finish latest physical controls/video, no-result, disconnected, timeout,
  cancellation, malformed/oversized source and long-duration acceptance.
  MCU render-buffer readback and synthetic button injection are development
  evidence, not physical USB video or independent button/panel observation.
- [ ] Provision a dedicated isolated runner, trusted installed lab driver,
  deterministic native HTTPS fixture origin and actual capture/control access.
  Keep `CHROMATIC_E2E_READY=false` until verified. Never execute contributor PR
  code on the maintainer's daily-use machine.
- [ ] Bind candidate/source/readback and all required physical E2E evidence,
  with no failed/skipped scenarios, before main promotion or any `vX.Y.Z` release.

The [physical lab contract](hardware-e2e.md) remains a hard release boundary.
Hosted CI checks actual complete MCU compilation and behavior, not a fabricated
hardware pass. The project is still private; this work does not change visibility,
register a personal machine as a runner or automatically merge a PR.

## P1 — Useful sources and durable local content

- [x] Maintainer-reviewed source configuration and a generated dependency-free
  registry; select Stanford AI, W3C Web or W3C People on the device.
- [x] Full admitted URL, profile, acquisition/check UTC time, status, received/
  retained bytes and text-limit information, with paged source details.
- [x] Validated explicit snapshot format, two commit-last flash banks, ownership
  and CRC checks, previous-bank fallback, index rebuild and retained crawl cursor.
  Foreign storage is never automatically erased. Queries/history are not stored.
- [x] Persist completed pages, requeue cancelled unpublished pages, load/search
  a completed corpus offline, and resume an incomplete frontier.
- [x] Explicit conditional update using ETag/Last-Modified. Keep a 304's original
  text/provenance; replace only complete 200 content; preserve old data on failure.
- [ ] Qualify latest restart/resume/conditional-update behavior physically and
  test actual power interruption during flash writes; host truncation/torn-header
  tests do not prove physical brownout recovery.
- [ ] Expand reviewed sites and resource/DNS/robots policy with native adversarial
  acceptance. Arbitrary URLs, a durable cross-domain frontier and Internet-wide
  indexing are outside this bounded four-document increment.
- [ ] Add measured complete-page storage and deliberate stable-format migrations.
  The current 2,560-byte body limit remains visible and is not complete HTML storage.

See [retained-content implementation and limits](native-retained-content.md).

## P2 — Native source pixels and staged browser fidelity

- [x] Resolve the visible image source; directly acquire a bounded verified HTTPS
  baseline JPEG on the selected public authority; decode with the ESP32 ROM path,
  resize to RGB565, show a separate viewer and release temporary buffers on exit.
- [ ] Qualify actual source pixels, cancellation/retry/back, TLS/decode peak memory
  and long-duration image use on the latest candidate. A host-prepared image never
  substitutes for native acquisition/decoding.
- [ ] Add independently bounded native PNG and progressive-JPEG support. GIF,
  SVG, WebP, animation and video remain later format work.
- [x] Retain headings, paragraphs, lists, links and image slots; reflow to 160 px;
  preserve supported size/weight/color/box styles and vertical line scrolling.
- [x] Basic external CSS plus four bounded embedded simple rules, with cross-page
  isolation and streaming/chunk-boundary tests. Persist supported computed styles.
- [ ] Extend selector/document-order behavior and measured layout incrementally;
  flex, grid, table layout and complete HTML5/CSS conformance are not implemented.
- [ ] Add WOFF2, downloaded Japanese faces and shaping only with bounded native
  decoders, font licensing and measured memory/cancellation acceptance. Built-in
  Pocket JP and static TrueType ASCII are the present font paths.

## P3 — Physical feedback and measured efficiency

- [x] FPGA menu-button click RTL and official-source integration, leaf simulation
  and licensed Gowin build with no reported timing violations.
- [ ] Verify complete clock-domain behavior, original FPGA recovery, deployment,
  speaker/headphone/USB audio and mute/volume behavior on the physical console.
  The custom FPGA image is not installed; no physical click is claimed.
- [x] Cache immutable layout-position counts at publish/load and invalidate/rebuild
  after replacement; remove repeated whole-page counting from navigation.
- [ ] Record latest CPU, heap/largest block, stack, TLS/image and drawing timings
  together; compare before/after under identical source and navigation workloads.
- [ ] Extend endurance testing and optimize measured bottlenecks without weakening
  certificate, content, storage and release validation. Assess task consolidation
  or bare-metal changes against equivalent working networking/recovery behavior.

## Separate future work and historical evidence

Japanese input ergonomics, user-supplied book import and relevance improvements
can follow the working bounded reader. Bookmarks, browsing history, launch-time
reading restoration and the Codex approval app remain outside product scope.

The optional Rust companion crawler/API/browser and host image preparation are
retained development tools. Their broader corpus does not expand native scope.
Historical [standalone search](on-device-search.md), [typography](typography.md),
[semantic flow](native-flow-layout.md), [companion DOM/images](dom-images.md), and
[display measurements](performance.md) retain their original revision and evidence
boundaries. Release policy stays `feature/fix` -> `develop` -> qualified `main` ->
annotated `vX.Y.Z`, with review and CI required and no automatic merge.
