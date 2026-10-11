# Chromatic reading protocols

PCS1 is the implemented host/client text format for the Chromatic reader. The optional API transports PCS1 packets; the autonomous adapter generates PCS1 results and PSR1 styled reading windows locally. Kinds 3–5 add image descriptors, registered links and headings for companion-mode scenes. [PCI1](dom-images.md#pci1-image-stripes) carries prepared image pixels separately. Full provenance UI and qualified physical E2E remain pending.

## Limits

| Item | Limit |
| --- | --- |
| Full packet | 512 bytes including header |
| Query | 96 UTF-8 bytes |
| Title | 24 cells, at most 48 UTF-8 bytes |
| Body | At most seven lines |
| Each line | 24 cells, at most 48 UTF-8 bytes |
| Search results | One per response; count bounded by the loaded collection and unsigned 16-bit metadata |
| Page index / count | Unsigned 16-bit; count must be nonzero |

ASCII uses one conservative cell; every other Unicode scalar uses two. Preserve UTF-8 boundaries. The decoder rejects control characters; the host replaces them with spaces, while body newlines delimit rows. This policy is not Unicode shaping or a font coverage guarantee.

## Header

The header is 20 bytes. Multibyte integers are little-endian.

| Offset | Bytes | Field |
| --- | --- | --- |
| 0 | 4 | ASCII magic `PCS1` |
| 4 | 1 | Kind: `1` result, `2` text, `3` image descriptor, `4` link, `5` heading |
| 5 | 1 | Flags: bit 0 previous, bit 1 next; other bits zero |
| 6 | 1 | Body line count, 0–7 |
| 7 | 1 | Title byte length |
| 8 | 4 | Document ID, unsigned 32-bit |
| 12 | 2 | Zero-based result/page index |
| 14 | 2 | Result/page count |
| 16 | 2 | Payload byte length, excluding header |
| 18 | 2 | Reserved, must be zero |

Flags must agree with index and count. All non-result IDs must be nonzero. Result ID zero is an empty-result screen with no open action.

Payload: kind 4 starts with a four-byte little-endian registered target ID (0 = unavailable, 1–256 = registered document), included in the payload length. Other kinds have no target prefix. Then UTF-8 title, then each body line as `one-byte length + UTF-8 bytes`. No terminators or padding. The supplied slice must equal the payload length plus 20, and parsing must consume it exactly.

## Host behavior

`SearchIndex::chromatic_result(query, index)` formats a ranked hit. `SearchIndex::chromatic_page(document_id, page_index)` formats a stored body page. Invalid IDs, out-of-range pages, oversized queries, and bodies needing more than 65,535 pages return errors. Empty results produce an explicit screen rather than a fabricated hit.

Long titles end in an ellipsis. Long result excerpts end with `A: Read full page`; opening retrieves the full stored body through pagination. URLs remain complete on the host even when a result screen cannot fit them. A dedicated source-details view is pending.

IDs are one-based positions in the loaded collection, valid only while it is unchanged. Keep a service corpus immutable or add a generation check before allowing live updates.

## Client behavior

The caller owns and caps the receive buffer. After complete receipt, `Packet::decode` borrows it without copying text or allocating. Keep it alive during drawing/action dispatch, then discard it on the next screen. Reject malformed packets before changing the screen. The native adapter supplies bounded HTTPS receipt, loading/errors, request correlation and rendering. Its localized unsafe C ABI is separate from the safe decoder core.

Up/down navigate within bounds for results and reading scenes; left/right also navigate non-result scenes. A opens a nonzero result ID or registered link target; B goes back, Start requests query entry, and Select requests home. The core returns actions; the native adapter supplies the operation and GUI update.

The earlier `0.13.18-scroll` image used overlapping seven-row PCS1 windows. The current autonomous reader uses PSR1, described below. Its position is still the top wrapped line; up/down moves one and left/right jumps seven, clamped at both ends. Search-result indices and optional companion disjoint PCS1 scenes retain their existing meaning. PCS1 layout and its C ABI are unchanged. See [autonomous reader controls](on-device-search.md#controls-and-storage).

## Verification boundary

Host tests round-trip Japanese text across reader pages, enforce row/packet limits, map navigation, and reject truncated, malformed, oversized, trailing, and invalid-UTF-8 packets. These host tests do not establish target behavior. Separate [experimental evidence](search-results.md) records actual Xtensa decoding and native Wi-Fi transport with synthetic controls; physical-buttons/USB-video E2E and peak TLS memory remain pending.

The current C ABI uses a 48-byte `PS_View` including the link target, and a 20-byte `PS_ImageView` for borrowed stripe offsets. Build both sides together; the earlier text-only image used a 44-byte view. Unknown kinds fail closed in older clients. Kind 3 is a scene descriptor, not pixels: fetch `/v1/chromatic/image` for that exact document/page and reject incomplete or mismatched image streams.

## PSR1 styled reader windows

PSR1 is the autonomous reader format introduced in `0.13.19-type`. The request still uses local reader operation kind 2, but the response magic is `PSR1`. It is not a new PCS1 kind and must be decoded separately. Existing PCS1/PCI1 ABI layouts remain intact; `PS_RichView` is a new 112-byte borrowed C view.

The packet is at most 512 bytes. Its 20-byte header uses little-endian multibyte values:

| Offset | Bytes | Field |
| --- | --- | --- |
| 0 | 4 | ASCII `PSR1` |
| 4 | 1 | Format version: 1 (built-in styles) or 2 (web-font bit allowed) |
| 5 | 1 | Previous/next flags, matching position/count |
| 6 | 1 | Row count, 1–8 |
| 7 | 1 | Title bytes, at most 48 and 24 conservative cells |
| 8 | 4 | Nonzero document ID |
| 12 | 2 | Zero-based top wrapped-row position |
| 14 | 2 | Nonzero valid position count |
| 16 | 2 | Exact payload byte count |
| 18 | 2 | Reserved zero |

Payload is the UTF-8 title followed by each row's four-byte descriptor `[y, height, text_bytes, glyph_count]`, then its UTF-8 text and `ceil(glyph_count / 2)` packed style bytes. The low nibble describes the first scalar; the high nibble describes the second. An unused final high nibble must be zero. Styles 0–7 use bits 0–1 for sizes `[12, 16, 14, 10]` and bit 2 for bold. Version 1 rejects bit 3. Version 2 permits bit 3 to select the already validated native web-font atlas; unsupported/missing glyphs use the built-in font. Font selection does not change advances, packet geometry or the borrowed C ABI. No trailing bytes are valid.

Each row allows at most 64 text bytes and 28 Unicode scalars. Controls are forbidden inside text. The first row starts at y=0; each later row begins at the prior bottom. Nonempty row height must equal the tallest style size plus two; empty rows have height six and no styles. Rows must fit width 144 and total height 92. ASCII advances by half its size, other scalars by its full size, with one extra pixel for bold. The decoder recomputes width and height from all glyphs rather than trusting descriptors.

Position count is derived from the final retained rows fitting the pixel, eight-row and packet budgets. It is not a fixed `total_rows - 7` formula. Up/down change the top row by one; left/right jump seven. The final position includes the retained tail and any text/style limit markers. The formatter returns failure for an out-of-range position rather than fabricating a page.

See [native typography](typography.md) for source interpretation, drawing, memory and verification boundaries. PSR1 does not carry arbitrary CSS, executable code, fonts or image pixels.
