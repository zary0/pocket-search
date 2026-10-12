# Bounded native Japanese and WOFF2 fonts

The 0.14.13 development candidate adds direct verified-HTTPS WOFF2 download,
bounded native decompression, BMP Japanese outline rasterization, four-level
coverage and horizontal kana cluster composition. The design uses a quiet white
page, consistent 10/12/14/16-pixel hierarchy and existing paragraph space. Noto
Sans JP is the licensed face used for acceptance; Apple proprietary fonts are
not part of the implementation. This is a staged small-screen font engine,
not complete desktop font or browser fidelity.

## Native path and limits

A supported `@font-face` has one HTTPS `url(...)`, optionally followed by
`format('truetype')` or `format('woff2')`. Existing source-authority, URL-length,
TLS, hostname, MIME, identity-content and redirect restrictions remain in force.
Google Fonts CSS requests negotiate its WOFF2 representation. Only the first
face is used. Downloaded fonts are volatile; no font file or glyph cache is
written to the corpus partition.

| Resource | Independent limit |
| --- | --- |
| Font wire bytes / decoded SFNT / transformed buffer | 24,576 bytes each |
| WOFF2 private heap, including reconstructed SFNT and allocator headers | 65,536 bytes total live; 16,384-byte system free-heap reserve before each allocation |
| Decode and raster deadlines | 5 seconds each, with cooperative cancellation/yield checkpoints |
| Face | Static TrueType outlines only, at most 24 tables and 512 glyphs |
| BMP cmap | Format 4, at most 16 records, 256 segments, 4,096 candidates |
| Raster outline | 256 points, 32 contours, 64 scan intersections, 1,024 skipped instruction bytes |
| Published atlas | 128 sorted Unicode slots, each 16×16 at 2bpp; 8,708 bytes including metadata |
| GUI scratch | 16 style glyphs × 72 bytes; 1,152 bytes, plus small cache keys |
| Native worker stack | 16,384 bytes; separate from decoder heap and fixed atlas |

All upstream WOFF2 vectors, maps and raw arrays use the same bounded allocator.
Brotli uses custom allocation callbacks and processes 256-byte input/output
slices. Its static dictionary remains read-only firmware data. Oversized trees,
windows or font reconstruction allocations fail before exceeding the private
budget, even when the source font itself is small. WOFF2 collections, metadata
and private blocks are rejected. Invalid, variable/CFF/color fonts, unsupported
composite or complex glyphs and absent atlas characters use Pocket JP. A full
atlas admits its first 128 supported scalars; it does not promise a complete
Japanese font.

The Rust rasterizer executes no hinting or font instructions. Integer quadratic
curves use eight subdivisions and 2×4 subpixel coverage, quantized to four levels.
Glyphs use the font em square with fixed side/descent space, preserving Japanese
outline aspect. Area sampling and small-screen optical compensation retain fine
strokes at the supported fixed cells. Kerning, proportional
advances, GSUB/GPOS, Arabic/Indic shaping, vertical writing, variation selectors
and arbitrary Unicode normalization remain unsupported. Horizontal hiragana
and katakana plus combining dakuten/handakuten are composed before retained
text, indexing and layout; a composed kana occupies one full-width cell. The
original glyph's style is retained. Unmatched marks are not discarded.

One successful face is immutable until restart. The worker constructs an
unpublished atlas; cancellation or failure clears it and never sets readiness.
An acquire/release flag publishes only a completed atlas. The GUI draws from
fixed static storage; resource cleanup cannot invalidate its pointers. A second
face falls back until restart. Cancelled/failed acquisition may be retried.

## Storage and build profile

The complete 16,077-character Pocket JP fallback has lossless Huffman storage,
BMP presence/rank indexing and eight-glyph checkpoints. Decoding needs an
18-byte GUI cache and no heap. Its complete original coverage and pixel hashes
are checked in CI; this is compression, not a smaller fallback character set.
C/C++ builds use size optimization with exceptions enabled for bounded allocator
failure unwinding. The existing 2MiB app partition, NVS and corpus layout remain
unchanged; app-only installation is required.

## Licensing and acceptance

Noto Sans JP and VT323 are OFL-1.1. Their font licenses are independent of the
GPL project. Native WOFF2/Brotli code retains MIT notices and pinned sources.
OFL test fonts retain their complete license and provenance. Only independently
licensed fonts may be redistributed; adding an arbitrary site's font to a
release requires checking its rights. See [third-party notices](../THIRD_PARTY_NOTICES.md).

`search_local jfonts` explicitly downloads a small Noto Sans JP WOFF2 subset.
After opening a verified article, `search_ui jpfont` renders original Japanese
calibration content at multiple sizes and B returns to the article. The
diagnostic does not add articles to the index. `search_local jfonts-cancel N`
injects persistent cancellation after N decoder/raster checks, using the same
actual received source bytes. `search_local font-cancel` cancels a running
console diagnostic during transport. Ordinary B cancellation remains available
for user-started page collection.

Host ASan/UBSan tests exercise every allocation failure, cancellation throughout
reconstruction, retry, truncated sources, mutations and independent FontTools
reference tables. Rust tests verify real Japanese reference glyphs, grayscale,
normal-form matching and style/layout boundaries. Hardware acceptance requires
native source/atlas hashes, elapsed time, allocator peak, TLS/whole-phase heap
windows, stack margin, cancel/retry/back and repeated rendering on the exact
installed clean-source candidate. Host-prepared fonts or glyph images never
substitute for that acceptance. Battery power and independent physical-control
release qualification remain separate P0 requirements.
