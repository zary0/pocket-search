# Native search result presentation

The `0.14.10` native result screen makes the page title and its retained-text
excerpt the main content. It shows one selected result at a time on the
160 x 144 display:

- A compact query field and result number at the top.
- A blue, bold title, wrapping to at most two 20-cell lines.
- Three excerpt lines for a two-line title, or four for a one-line title.
- A small gray authority label instead of a multi-line complete URL.
- Up/Down to select, A to read, B to return, Start to edit the query.

Select still opens complete admitted URL and acquisition details. Shortening the
visible source does not change the document identity, source policy or open
action. Long titles and excerpts end with an explicit ellipsis. The excerpt is
retained page text near the first matching query word, preferring a nearby
sentence or paragraph boundary. ASCII matching uses the ranker's word boundaries
and case folding; non-ASCII matching preserves UTF-8 boundaries. Title-only
matches use the beginning of the retained body. No model-generated summary or
additional network request is involved.

## Packet and memory scope

PCS1 framing, per-line validation, pagination, IDs and the Rust/C ABI stay
unchanged. The autonomous adapter marks replies as local. Successful native
result packets use the existing title field for the first title line, line 0
for a possibly empty title continuation, line 1 for the authority, and lines 2
onward for the excerpt. The native result renderer applies this convention only
to local result replies with a nonzero document ID. Empty results and optional
companion packets keep their existing presentation and action validation.

Excerpt construction uses fixed stack buffers; it adds no persistent document
fields, storage migration, dynamic allocation or dependency. The index, ranking,
retained content, article layout and image pipeline are unchanged. The optional
browser canvas follows the same visual hierarchy from companion metadata; it
remains an approximate host preview, not physical LCD evidence.

Host checks cover query-context selection beyond navigation, ASCII whole-word
and case behavior, title-only matches, Japanese UTF-8, valid PCS1 decoding and
article access. Native candidate/readback, actual framebuffer observations and
operator observations must be recorded separately; older image endurance does
not qualify a new complete firmware release.
