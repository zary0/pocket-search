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

The installed development application is `0.14.9`, source
`91022671bb21e60cf627466c1d367e14aae9f2e0`, SHA-256
`2c7f3231565ac221bf365bc51f3a85dc38c82a87d02b9abed47c6987f0fbad7b`,
2,015,824 bytes. Application-only readback verifies the same bytes; the
partition table, NVS, saved four-document corpus and FPGA are preserved.
The [sanitized verification record](evidence/native-images/verification.json)
binds this candidate to source/pixel hashes, errors, memory and endurance.

A USB-powered run completes **1,810.676 seconds and 160 native images**:
54 baseline JPEG, 53 PNG and 53 progressive JPEG. Every image matches its
original source hash and decoded-pixel hash, returns to the same retained text,
and is followed by down/up scrolling. There are zero unexpected errors/resets
and zero storage failures; four documents remain at generation 13. Post-back
sampled heap spans 88,456–90,060 bytes (first 90,060, last 89,136); this describes
that run and does not prove absence of every possible leak.

All three formats also pass loading cancellation, immediate cached-text return,
retry and back. A deliberately disconnected PNG returns error 1; reconnecting
and pressing A succeeds through native HTTPS/decode. Cancelled images return
error 9 with zero published image bytes. Cached-back console observations are
774–785 ms including diagnostic round trips; these are not physical-panel latency.

| Original source | Wire / RGB565 bytes | Private decode peak | Median decode | TLS / decode phase minimum free heap |
| --- | --- | --- | --- | --- |
| Raggett portrait, baseline JPEG, 64 x 80 output | 1,902 / 10,240 | 14,336 B | 19.158 ms | 38,448 / 60,500 B |
| W3C PNG logo, 72 x 48 | 1,848 / 6,912 | 21,440 B | 17.962 ms | 39,040 / 53,924 B |
| Raggett boat, progressive JPEG, 110 x 75 | 3,650 / 16,500 | 54,436 B | 61.836 ms | 37,812 / 19,228 B |

The phase minima use the conservative allocator method described above. They
include concurrent runtime tasks, and cannot be relabelled as exact TLS-private
peaks. Decode-private peak excludes immutable wire bytes and preallocated stack.
The minimum observed worker stack headroom is 2,844 bytes. HTTPS request maxima
are 2.321–2.608 seconds for these source files; this is not a universal site limit.

Original files are fetched directly from:

- `https://www.w3.org/People/Raggett/dsr-46c.jpg`
- `https://www.w3.org/assets/logos/w3c-2025-transitional/w3c-72x48.png`
- `https://www.w3.org/People/Raggett/rib.jpg`

The PNG output matches a separately calculated Pillow RGB565 reference.
Progressive pixels match the pinned host component-plane decoder reference;
that reference is never delivered to the console. Source photos and runtime
configuration are not bundled in the numeric evidence. The operator confirmed
both logo and boat on the `0.14.8` LCD; their `0.14.9` decoded bytes are unchanged.

The first `0.14.8` repeated-image trial fails after eight successful images:
progressive decode exhausts the 16 KiB runtime-headroom guard, while text remains
readable. `0.14.9` defers each pixel plane until its coefficients can be released,
reducing this source's private peak from 58,978 to 54,436 bytes without changing
pixels or weakening either cap. The unsuccessful trial is retained separately
and never counted as a successful endurance run. Unknown or incompletely finished
progressive planes are rejected before any output sampling.

This qualifies scoped USB native image development. Battery power, independently
actuated physical buttons, physical USB video, arbitrary/malformed native-origin
acceptance and the dedicated isolated release lab remain separate requirements.
`CHROMATIC_E2E_READY` stays false; no release qualification is claimed.
