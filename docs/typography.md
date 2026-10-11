# Native reader typography

The autonomous `0.13.21-css` reader preserves a bounded set of heading, body, annotation, inline weight and paragraph distinctions from fetched HTML. Rust extracts styles while streaming the original page, wraps text to measured glyph advances, and generates validated PSR1 viewports. The MCU draws those glyphs through LVGL on the 160 × 144 display. Collection, search and rendering require no companion service.

## Supported source structure

| Source | Native treatment |
| --- | --- |
| `h1` | 16 px, bold, separate block |
| `h2`, `h3` | 14 px, bold, separate block |
| `h4`–`h6` | 12 px, bold, separate block |
| Ordinary body | 12 px |
| `strong`, `b` | Bold within the current size; nesting restores the previous style |
| `small`, `sub`, `sup`, `figcaption`, `aside`, `footer`, `caption` | 10 px; annotations use a quieter gray |
| `p`, `blockquote`, heading boundaries | Paragraph separation, including a six-pixel empty row |
| `br`, `div`, list and table boundaries | Line separation |
| Legacy `font size` | Maps supported numeric/relative sizes to the four native sizes |
| Inline `font-size` / `font-weight` on recognized style-bearing tags | Maps supported pixel sizes and size keywords to native sizes; bold/bolder or numeric weight 600–900 enables bold; normal/100–500 removes it |

Whitespace inside a paragraph is normalized. Paragraph breaks remain distinct from ordinary line breaks. An omitted `</p>` is closed when a new supported block starts. Unsupported elements contribute plain visible text when otherwise allowed; scripts, style contents and comments remain excluded.

These are normalized semantic distinctions, not a full reproduction of the original browser design. The first bounded external stylesheet and a single static TrueType web face are now supported; see [CSS and web-font scope](native-css-webfonts.md). JavaScript, arbitrary CSS layout, italic, Unicode shaping and superscript/subscript vertical positioning remain unsupported. Query entry and missing-glyph coverage retain their existing limits.

## Layout and drawing

The body occupies **144 × 92 pixels** at `(8, 34)`, leaving the existing title, status and button hints visible. Rows use their tallest glyph's size plus two pixels of line spacing. Mixed sizes share the row baseline; paragraph gaps are six pixels high. Up to eight rows fit a viewport, further constrained by the 512-byte packet budget. Wrapping preserves UTF-8 boundaries and uses whole words where possible; a long word can wrap between glyphs.

ASCII advances are half the normalized size, other Unicode scalars advance one size, and bold adds one pixel. Thus a 16 px bold heading has nine-pixel Latin advances and seventeen-pixel non-ASCII advances. Rust validates the complete row width before drawing. The native font wrappers use the same advances.

The renderer scales the existing bitmap font and adds one horizontal pixel for bold. The renderer can also use a downloaded 867-byte ASCII atlas. It adds no framebuffer or full-page text cache. Its sixteen single-glyph scratch/cache entries use 656 bytes. The borrowed C rich view is 112 bytes. These figures exclude the existing font, LVGL object, GUI/network stacks and display band.

Down advances one wrapped row; Up reverses it. Left/right jump seven row positions. The last valid position packs the retained tail against both pixel and packet limits, preventing empty overscroll or unreachable text. B returns to the selected search result. This is discrete row scrolling, not smooth pixel animation.

## Bounded storage and validation

Each document retains at most 2,560 UTF-8 body bytes and 48 style transitions. The parser's nesting stack has 16 frames. Both are held inside the 16,640-byte opaque Rust state region (16,416 bytes used). Exceeding style capacity falls back to body style for subsequent content, preserves retained text, and adds `[Style limit reached]`; text truncation retains its separate `[Text limit reached]` marker. No dynamic allocation is introduced in the Rust core.

[PSR1](chromatic-protocol.md#psr1-styled-reader-windows) has its own magic/version; PCS1 result and companion formats are unchanged. The decoder checks UTF-8, styles, dimensions, row continuity, packet consumption and navigation metadata before native drawing. Borrowed offsets remain valid only while the current packet buffer is retained. Invalid replies do not replace the displayed document.

## Verification

Portable tests cover every streaming chunk boundary of styled HTML, nested restoration, omitted paragraph closures, style overflow, mixed-language wrapping, worst-case UTF-8 packet budgets, reachable retained tails, every truncated diagnostic packet, malformed geometry/styles, and the C borrowing boundary. The earlier semantic-typography image and measured native observations are recorded in [sanitized typography evidence](evidence/typography/verification.json). The external-resource increment has [separate evidence](native-css-webfonts.md).

The `search_ui typography` diagnostic displays original calibration text only while an already verified native article is open. It includes a large heading, body text, one inline bold word, a paragraph gap and a small annotation, visibly labeled `TEST`. It is never inserted into the search collection; B restores the original article position. Calibration/render-buffer readback establishes the MCU drawing path, not a camera observation of the LCD, battery acceptance or release E2E.
