# Retained sources, updates and native images

The `0.14.6` increment keeps collection, storage, search, layout and baseline JPEG
processing on the ESP32. No companion service, tunnel, host image preparation or
USB page proxy participates. It is an experimental increment; physical release
qualification, current battery acceptance and a dedicated CI lab remain required.

See [scoped native CPU, memory, rendering and image measurements](native-device-measurements.md)
for installed-image identity and the actual development observations.

## Source configuration and controls

Edit `devices/chromatic/sources.json`, run
`python3 scripts/generate_chromatic_sources.py`, review both files and rebuild.
The generated dependency-free registry supports one to eight named profiles.
The default profiles are Stanford AI, W3C Web and W3C People. Seeds must be short
public HTTPS URLs in an explicit directory on one authority; the selected scope
admits either numbered `node*.html` leaves or bounded ASCII HTML paths. This is
maintainer-reviewed configuration, not unrestricted URL entry or an Internet-wide
crawler. Query strings, IP literals, credentials, ports, escapes and parent paths
are rejected. Newly added authorities still need native resource and adversarial
network acceptance; a public-looking name alone does not establish safe DNS.

- Home **Left** opens sources; Up/Down selects, A applies and B returns.
- A enters a query and Start searches. Completed collections are reused.
- Home **Up** checks existing documents for updates. Home **Right** clears the
  logical collection and the next query starts a new crawl.
- Results: Up/Down selects, A reads, Select opens source information.
- Reader: Up/Down scrolls one line, Left/Right seven lines, A opens a visible
  supported image before a visible retained link, Select opens source information,
  and B returns. Image loading B cancels; image B returns to the cached text.
- Source information: Up/Down pages the complete admitted URL, profile, HTTP
  acquisition status, UTC fetch/check time, received and retained byte counts,
  and any text limit. B restores the prior result/reader position.

Switching profiles clears the active corpus and persists the new selection.
There are still at most four documents, eight frontier entries, 2,560 retained
UTF-8 body bytes, 48 style transitions and 24 semantic nodes per document.
Retained text and complete original HTML are different: overflow is visible,
and raw complete pages are not stored. No query, history or reading position is
written to flash.

## Durable storage and resumption

A runtime data partition reserves `0x210000..0x230000` in the established 4 MiB
flash layout. It does not rewrite the existing partition table. ESP-IDF checks
partition overlap and flash capacity. Before first use, every byte must be erased;
a foreign or damaged ownership record disables storage without erasing it.
The operator must preserve a device recovery backup and verify this region before
installation. A region that is not empty is not automatically disposable.

An ownership sector and two 32 KiB banks retain up to 18,432 serialized bytes.
The format stores only explicit little-endian fields and initialized UTF-8 data,
not struct padding, pointers, parser stacks or hash-table memory. It includes
published text/styles/semantic references, validators, provenance and the bounded
frontier/cursor. Each bank has payload CRC, checked length, generation and a
commit-last header. A failed/torn new bank leaves the previous one intact.
On load the core validates all strings, URLs, counts, style ranges and tree/resource
indices before publishing documents, then rebuilds the index and layout counts.
A corrupt newer bank falls back to a valid older one. Corrupt ownership fails
closed; it requires deliberate operator recovery, not an automatic erase.

Successful page completion persists progress. A cancelled unpublished page is
requeued. On restart, a complete collection is searchable without a socket;
a partial collection can resume on a query with Wi-Fi and can be searched offline.
Flash failure leaves the RAM collection usable and increments `store_failures`.
`persistent_index` in diagnostics means the storage region is available; a
nonzero generation and successful restart load establish actual persistence.

Clear is logical, not secure erasure: the older bank can retain public content
until overwritten. This development format is versioned; incompatible snapshots
are rejected and must be recollected. No stable storage migration guarantee exists.

## Conditional updates

An explicit update sends a retained ETag, or Last-Modified when there is no ETag,
through the same verified HTTPS and robots policy. A 304 retains text, styles,
source byte count and original fetch time, while recording the check time. A
complete 200 replaces one document and rebuilds its searchable terms. A failed,
cancelled or incomplete response restores the previous document. Each successfully
checked/replaced page is saved independently; an interrupted update is not an
atomic four-page snapshot. Changes are not crawled on every search. There is no
background timer, deletion discovery or web-scale incremental indexing.

## Native image boundary

The device resolves an image from the visible retained semantic slot, checks the
selected public authority and its robots policy, verifies HTTPS and MIME, and
receives at most 24 KiB. It uses the existing ESP32 ROM TinyJPEG decoder after
bounded baseline-envelope/table validation, then writes nearest-neighbor RGB565
pixels at no more than 160 x 80. Raw bytes, the 4 KiB decoder pool and the output
are temporary; leaving/cancelling/error frees them. Decode has a five-second
cooperative deadline. Source pixels are not processed on the Mac.

Supported: bounded, single-scan, eight-bit **baseline JPEG**, one or three
components, dimensions no larger than 1024 x 1024. Progressive JPEG, PNG, GIF,
SVG, WebP, ICC color management, animation and video remain unsupported. Image
failure keeps the text readable and shows a retry/back notice. Images open in a
separate viewer; the text retains the image slot and alt text. This is not inline
browser image compositing. W3C's Tim Berners-Lee photograph was acquired directly
but is progressive and correctly rejected. The W3C People profile supplies a small
baseline source for the native acceptance case.

The ROM envelope tests exercise bounds and rejection, not a general image-decoder
security audit. Keep the small reviewed source scope until malformed entropy and
long-duration native decoding have broader acceptance.

## CSS and performance increment

Alongside the existing eight external rules, the streaming parser retains at most
four complete simple embedded rules of up to 256 bytes. It reuses its tag scratch
buffer; no additional whole stylesheet or DOM allocation is made. Basic sizes,
weight, colors, backgrounds and pixel box declarations apply before body text.
Nested blocks, print-only styles, imports, escapes and embedded faces are skipped.
Embedded rules follow external rules at equal specificity; this bounded subset
does not reproduce arbitrary stylesheet document order or retroactively restyle
already emitted text. Supported computed styles survive persistence.

The reader caches its total layout-position count when publishing/loading an
immutable document. Navigation no longer repeats the complete page-count pass;
its remaining work scans to the requested window and encodes that window. Cache
values are recomputed after a changed document or snapshot load, not trusted from
flash. Keep the actual target timings and memory observations separate from host
unit results. The retained core still fits the fixed 22,272-byte state region.
WOFF2, Japanese downloaded faces, shaping, flex/grid/table layout and general
browser conformance remain further staged work, not implied by these changes.

## Explicit native measurement windows

`search_local compare 1` runs thirty warm comparisons on core 0 between the full-count reference encoder and the optimized encoder for the same retained document/window. It rejects unequal packet bytes and reports CPU frequency, elapsed totals and local PFW decoder cycles. The core is allocator-free; its zero-allocation result comes from that code boundary, not a whole-system allocator trace. The diagnostic never changes the reader or uploads page text.

`render_probe native` measures thirty seconds of the current GUI without installing a synthetic test scene. It reports handler time, actual band/DMA bytes and render-start to final-DMA frame time, local minimum heap and GUI stack headroom. It refuses an existing platform-test scene. This is MCU/driver timing, not LCD scanout latency or physical-video evidence. The historical idle/counter/page/full platform scenes remain separate.

The `0.14.5` display change assigns an immutable menu background source only when its identity changes. LVGL otherwise invalidates the covered screen on every GUI poll, even when reader state is unchanged. Ordinary widget updates, tab transitions, loading/error states and image changes still invalidate and draw normally. Compare actual native idle and navigation windows before claiming the resulting transfer/CPU saving.

The `0.14.6` change also skips only the stock battery meter's occluded paint while Pocket Search's full-screen tab is active. Voltage sampling, charge status and power handling run separately. Other tabs call the original draw function. The background-only change did not stop the repeated frames, because that hidden widget also invalidated the screen every poll; both actual before/after measurements must retain their revision identity.
