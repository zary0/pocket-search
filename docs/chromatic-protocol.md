# PCS1 reading-scene protocol

PCS1 is the implemented host/client text format for the Chromatic reader. The optional API transports packets; the autonomous adapter generates them locally. Kinds 3–5 add image descriptors, registered links and headings for companion-mode scenes. [PCI1](dom-images.md#pci1-image-stripes) carries prepared image pixels separately. Full provenance UI and qualified physical E2E remain pending.

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

In the autonomous native reader, kind 2 positions represent overlapping seven-row text windows. `page_index` is the zero-based top wrapped line; `page_count` is the number of valid window positions, not the number of disjoint screens. It is `max(1, total_lines - 7 + 1)`, including any visible text-limit marker. Up/down changes position by one; the C adapter uses left/right to jump seven positions, clamping to `0..page_count-1`. The final window contains the end of the retained text without empty overscroll. Search-result indices retain their existing meaning. The optional companion still produces disjoint reading scenes; packet layout, validation and C ABI are unchanged. See [autonomous reader controls](on-device-search.md#controls-and-storage).

## Verification boundary

Host tests round-trip Japanese text across reader pages, enforce row/packet limits, map navigation, and reject truncated, malformed, oversized, trailing, and invalid-UTF-8 packets. These host tests do not establish target behavior. Separate [experimental evidence](search-results.md) records actual Xtensa decoding and native Wi-Fi transport with synthetic controls; physical-buttons/USB-video E2E and peak TLS memory remain pending.

The current C ABI uses a 48-byte `PS_View` including the link target, and a 20-byte `PS_ImageView` for borrowed stripe offsets. Build both sides together; the earlier text-only image used a 44-byte view. Unknown kinds fail closed in older clients. Kind 3 is a scene descriptor, not pixels: fetch `/v1/chromatic/image` for that exact document/page and reject incomplete or mismatched image streams.
