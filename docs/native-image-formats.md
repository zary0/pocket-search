# Bounded native image formats

The `0.14.9` reader fetches the original selected image using native Wi-Fi,
SNTP, certificate-verified HTTPS and the selected authority's robots policy.
No Mac image converter, search service or tunnel participates. MIME must match
JPEG/PNG magic. Redirects, compressed HTTP bodies and over-limit responses fail
without replacing retained text. Only one image job runs at a time.

## Independent envelopes

| Format | Native decoder | Source envelope | Memory envelope |
| --- | --- | --- | --- |
| Baseline JPEG | ESP32 ROM TinyJPEG | Single scan, 8-bit, 1/3 components, at most 1024 x 1024 | 4 KiB decoder pool plus RGB565 output |
| Progressive JPEG | Pinned private stb_image JPEG component-plane implementation | SOF2, 8-bit, 1/3 components, each dimension at most 128, at most 9,216 pixels, at most 32 scans; restricted sampling/tables and approximation order | 64 KiB aggregate private live allocation including headers and output; pixel planes deferred until conversion; completed coefficient planes freed immediately |
| PNG | ESP32 ROM miniz inflater plus original chunk/filter/palette code | Non-interlaced; each dimension at most 256, at most 8,192 pixels; gray/indexed depth 1/2/4/8, RGB/gray-alpha/RGBA depth 8; filtered bytes at most 32 KiB | Same independent 64 KiB aggregate private allocation cap; exact filtered output extent; no whole-image RGB copy |

Every format shares a 24 KiB wire cap, aspect-preserving nearest-neighbor resize,
RGB565 output at most 160 x 80, and a five-second cooperative decode deadline.
The 64 KiB extended-decoder cap excludes immutable source bytes, worker stack,
Wi-Fi/TLS/LVGL and allocator metadata outside the explicitly counted headers.
Each new extended allocation checks the current heap snapshot for 16 KiB of
runtime headroom; other runtime tasks still allocate concurrently, and
fragmentation can still reject an otherwise admitted image. These limits are
maximum admission rules, not a guarantee that every matching file will fit.

PNG verifies every chunk CRC and ordering before allocation. It requires exact
zlib output length and checksum, reconstructs all five filters, validates every
palette index, and composites transparency against white. It rejects 16-bit,
Adam7, APNG, unknown critical chunks, trailing bytes and malformed data. Ancillary
profiles/text are neither expanded nor interpreted. Progressive JPEG reconstructs
all scans before sampling component planes, uses nearest chroma sampling, and
rejects arithmetic/12-bit/four-component encodings. EXIF orientation, ICC/gamma
color reproduction and animation are not implemented. GIF, SVG, WebP and video
remain later format work.

Cancellation, invalid input, allocation failure and deadline expiry free owned
temporary buffers. B returns to cached text without waiting for a closing image
socket; A can retry after cancellation or failure. A successful inline image uses
the same slot cache and expansion/back behavior as baseline JPEG.

## Dependency and host checks

The vendored JPEG source and small progressive-memory patch are pinned by
commit and SHA-256 in `vendor/stb-image.json`; see [third-party notices](../THIRD_PARTY_NOTICES.md).
Only JPEG is compiled. PNG follows the [W3C PNG specification](https://www.w3.org/TR/png-3/)
using the pinned ESP-IDF ROM miniz API. Private decoder allocation is serialized.

CI exercises original RGBA/filter/multi-IDAT and palette/transparency fixtures,
progressive JPEG, exact PNG RGB565 output, every source truncation, allocation
failure, cooperative cancellation and deadline, malformed CRC/Adler/filter/length,
format/dimension rejection and deterministic entropy/chunk mutations under
AddressSanitizer/UndefinedBehaviorSanitizer. The host PNG backend uses zlib for
inflate; its internal memory is not the native ROM inflater's memory. Host tests
never qualify native heap behavior, actual source pixels or long-duration use.
The original lab source can serve `png` and `progressive` modes without preparing
pixels for the console. See [fixture boundary](native-fixture.md).

## Native measurement and qualification

`search_ui image baseline|png|progressive` is a development diagnostic available
only from a verified retained reader. It selects fixed original W3C files through
the same authority/robots, verified HTTPS and decode path as article images;
it does not accept a caller URL or insert a diagnostic image into a different
article's semantic slot. The viewer explicitly identifies the diagnostic source.
B returns to the original article. A retries a failed diagnostic.

`SEARCH_NET` records source and RGB565 SHA-256, output dimensions, wire bytes,
decode time, exact decoder-private live high-water bytes and allocation count.
Separate ESP-IDF allocator-recorded minimum-free windows cover TLS acquisition
and decoding. The sum of per-region minima is a **conservative lower bound** on
phase free heap, not an exact simultaneous whole-device peak. It includes other
runtime tasks. Periodic sampled minima remain separate. Do not overlap these
windows with the render probe or capture: the SDK watermark monitor is global
and does not enforce ownership. Decoder-private peak excludes TLS and stack.

Latest-candidate installation/readback, native source/pixel verification,
cancellation/retry/back and a real thirty-minute repeated native image run are
required before checking the image qualification item. USB diagnostic controls
and MCU framebuffer readback do not qualify battery power, independently actuated
physical buttons or physical USB-video E2E. The complete release gate stays closed
until the [physical lab contract](hardware-e2e.md) is satisfied.

### Latest measured candidate

The installed development application is `0.14.8`, source
`adefabe855f20da22eedddd52976371480ec9cab`, SHA-256
`f2774f0c59b83b359e83c3255ad57f24d3beab4ebbb367435563ff147f45391d`,
2,015,584 bytes. Application-only readback verifies the same bytes; the
partition table, NVS, saved four-document corpus and FPGA are preserved.
The current development measurements use USB power and diagnostic GUI commands.
The complete physical/battery release gate is still closed.

These `0.14.8` measurements precede the allocation-order fix; they do not qualify
`0.14.9`. A repeated run exhausted the 16 KiB headroom guard on a progressive
image after eight successful cycles; text stayed intact. The new candidate
defers progressive pixel planes and releases completed coefficient planes
one at a time, preserving output pixels and both memory caps. Native
qualification remains pending until a complete run.

The original 72 x 48 W3C PNG is fetched directly from
`https://www.w3.org/assets/logos/w3c-2025-transitional/w3c-72x48.png`
(1,848 bytes), rather than its redirecting legacy URL. Native ROM inflate/filter
processing takes 17,821 microseconds and emits 6,912 RGB565 bytes. Source SHA-256
`590608bf256237f22d84bbab2a0a1afc0d4f20db6350bbac2c6841f1f3e13b01` and
pixel SHA-256
`a3e1770cb1d556ccecd8489809ae9f47c31f05a91854355da20f3aac7ac9f502`
match the original source and a separately calculated Pillow RGB565 reference.
Decoder-private peak is 21,440 bytes; allocator-recorded phase free-heap lower
bounds are 43,132 bytes for TLS and 57,496 bytes for decoding. MCU framebuffer
readback shows the source logo in the labelled image viewer; this is not USB
video or independent physical-panel observation.
