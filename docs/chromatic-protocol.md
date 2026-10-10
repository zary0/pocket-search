# PCS1 text protocol

PCS1 is the implemented host/client text format for the initial Chromatic reader. The experimental API and native adapter transport and render these packets. Image stripes, links, collected-page metadata and qualified physical E2E remain pending.

## Limits

| Item | Limit |
| --- | --- |
| Full packet | 512 bytes including header |
| Query | 96 UTF-8 bytes |
| Title | 24 cells, at most 48 UTF-8 bytes |
| Body | At most seven lines |
| Each line | 24 cells, at most 48 UTF-8 bytes |
| Search results | At most ten, one per response |
| Page index / count | Unsigned 16-bit; count must be nonzero |

ASCII uses one conservative cell; every other Unicode scalar uses two. Preserve UTF-8 boundaries. The decoder rejects control characters; the host replaces them with spaces, while body newlines delimit rows. This policy is not Unicode shaping or a font coverage guarantee.

## Header

The header is 20 bytes. Multibyte integers are little-endian.

| Offset | Bytes | Field |
| --- | --- | --- |
| 0 | 4 | ASCII magic `PCS1` |
| 4 | 1 | Kind: `1` result, `2` reader |
| 5 | 1 | Flags: bit 0 previous, bit 1 next; other bits zero |
| 6 | 1 | Body line count, 0–7 |
| 7 | 1 | Title byte length |
| 8 | 4 | Document ID, unsigned 32-bit |
| 12 | 2 | Zero-based result/page index |
| 14 | 2 | Result/page count |
| 16 | 2 | Payload byte length, excluding header |
| 18 | 2 | Reserved, must be zero |

Flags must agree with index and count. Reader IDs must be nonzero. Result ID zero is an empty-result screen with no open action.

Payload: UTF-8 title, then each body line as `one-byte length + UTF-8 bytes`. No terminators or padding. The supplied slice must equal the payload length plus 20, and parsing must consume it exactly.

## Host behavior

`SearchIndex::chromatic_result(query, index)` formats a ranked hit. `SearchIndex::chromatic_page(document_id, page_index)` formats a stored body page. Invalid IDs, out-of-range pages, oversized queries, and bodies needing more than 65,535 pages return errors. Empty results produce an explicit screen rather than a fabricated hit.

Long titles end in an ellipsis. Long result excerpts end with `A: Read full page`; opening retrieves the full stored body through pagination. URLs remain complete on the host even when a result screen cannot fit them. A dedicated source-details view is pending.

IDs are one-based positions in the loaded collection, valid only while it is unchanged. Keep a service corpus immutable or add a generation check before allowing live updates.

## Client behavior

The caller owns and caps the receive buffer. After complete receipt, `Packet::decode` borrows it without copying text or allocating. Keep it alive during drawing/action dispatch, then discard it on the next screen. Reject malformed packets before changing the screen. The native adapter supplies bounded HTTPS receipt, loading/errors, request correlation and rendering. Its localized unsafe C ABI is separate from the safe decoder core.

Result up/down and reader left/right navigate only within bounds. A opens a nonzero result ID; B goes back, Start requests query entry, and Select requests home. The core returns actions; the native adapter turns them into Wi-Fi requests and GUI updates.

## Verification boundary

Host tests round-trip Japanese text across reader pages, enforce row/packet limits, map navigation, and reject truncated, malformed, oversized, trailing, and invalid-UTF-8 packets. These host tests do not establish target behavior. Separate [experimental evidence](search-results.md) records actual Xtensa decoding and native Wi-Fi transport with synthetic controls; physical-buttons/USB-video E2E and peak TLS memory remain pending.
