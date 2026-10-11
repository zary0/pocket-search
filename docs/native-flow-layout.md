# Native semantic flow layout

The `0.13.23-flow` increment retains a small semantic tree alongside each collected document, applies basic typography/paint/box CSS, and reflows its text into a **160 × 92** viewport on the 160 × 144 Chromatic display. Collection, structure, layout and drawing stay on the MCU. It introduces no rendering server, page proxy, JavaScript engine or new dependency.

## Retained structure

Each of the four documents keeps up to **24 recognized nodes**, a **16-frame** parent stack and a **384-byte** combined link/image reference pool. Nodes preserve source order, parent, text range and computed paint/box values. Supported elements include headings, paragraphs, divisions, quotes, inline bold/size spans, ordered/unordered lists and items, links, figures, captions and images. Common omitted paragraph/item end tags are repaired. This is a compact semantic tree, not an HTML5 DOM with browser error recovery.

Lists retain their nesting and display numbers or bullet markers. Link labels stay in the original text flow with color and underline. A opens the first visible link whose resolved destination is already in the admitted collection; B restores the prior document and scroll position. Other link destinations are retained as metadata but are not fetched outside the existing source profile.

**Image nodes preserve their position, `src`, bounded height and `alt` text. They currently draw a framed `[Image: …]` placeholder, not decoded source pixels.** Images without descriptions use a visible fallback label. Native PNG/JPEG acquisition, decoding and resizing remain autonomous P2 work. The previous companion-prepared image experiment does not establish this capability.

Structure/reference overflow keeps the retained text readable and adds `[Structure limit reached]`. Body and typography limits remain 2,560 UTF-8 bytes and 48 transitions per document, with separate visible markers. Nothing is saved to flash.

## Basic CSS

Inline attributes and the existing first external stylesheet share property-wise specificity, source order and `!important` precedence. Text color and underline inherit; margins, padding, background and borders are not copied into child styles. Transparent descendants reveal the closest painted ancestor.

| Property | Supported subset |
| --- | --- |
| `font-size`, `font-weight`, `font-family` | Existing 10/12/14/16 px, synthesized bold and bounded static TrueType atlas |
| `color` | `#rgb`, `#rrggbb`, integer `rgb(r,g,b)` and 16 CSS color names; RGB565 output |
| `background-color`, `background` | A supported solid color |
| `margin`, `padding` | One to four nonnegative integer pixel values; four individual sides; each side clamps to 12 px |
| `border` | `Npx solid COLOR` or `none`; uniform width clamps to 3 px |
| `border-width`, `border-color` | A uniform pixel width or supported color |
| `text-decoration` | `underline`, `none` |

The resource cap remains 3,072 UTF-8 bytes and eight simple selectors. A viewport has eight foreground/background combinations, including its canvas entry. Palette exhaustion uses the default paint and reports `Color limit` in the controls; it never discards text. Horizontal insets clamp to 60 px per side to preserve a readable width. Nested blocks contribute indentation/insets and ancestor background; the nearest active border paints each fragment. Independent overlapping ancestor borders, margin collapsing and a general box-formatting context are not implemented.

Embedded `<style>` rules, complex selectors, media queries/imports, percentages, `em`/`rem`, negative/automatic margins, floats, flex/grid, positioning, background images/gradients, rounded/dashed borders and complete web-font metrics remain unsupported. CSS is normalized for this small display rather than reproduced at desktop scale.

## Reflow, transport and controls

Rust wraps at the supported glyph advances and Unicode scalar boundaries, preferring word breaks. List depth, margins, padding and borders reduce the available text width. Margin rows remain outside painted boxes. Paragraph background and border fragments continue through wrapped rows; inline colors/backgrounds and underlines travel with individual glyphs. A full document framebuffer is never allocated.

Up/down moves one visual row; left/right jumps seven rows. Each position generates a bounded local PFW1 packet. Both the 92 px height and the 512-byte packet budget determine its last valid position, so the retained tail is reachable and scrolling cannot move beyond it. Title/status and controls remain visible. This is discrete scrolling, without pixel animation.

PFW1 contains a title, up to eight paint entries and eight rows. Each row carries text, per-glyph font/paint codes, text/box geometry and border edges. The Rust decoder validates all UTF-8, offsets, glyph counts, palette indices, geometry, navigation, row continuity and total consumption before exposing borrowed C offsets. The C adapter retains one packet and a 216-byte validated view. PCS1 results and historical PSR1 calibration remain compatible.

The allocation-free Rust state reservation grows from 16,640 to **22,272 bytes**. A failed CSS second fetch preserves a complete first-pass document in a temporary snapshot capped at **4,608 bytes**, then frees it before the next request. TLS, tasks, fonts and display memory have separate costs; this reservation is not a complete system RAM peak measurement. Retained-document indexing borrows its text rather than copying a whole document onto the worker stack.

## Verification boundaries

Portable checks exercise every split of the original [HTML fixture](../examples/fixtures/flow.html), structural parenting and omitted list closures, CSS specificity/importance/shorthands/inheritance, source references, already-collected link navigation, every viewport/tail, structure overflow, mixed UTF-8 and malicious/truncated packet geometry. The native Xtensa archive and ESP-IDF application build must also pass before an app-only installation.

`search_ui layout` displays an original `TEST` diagnostic only while a verified real article is open. Its structure and paint are prepared on the MCU, separately from the search corpus. Up/down and left/right inspect it; B restores the actual article. No fixture page, index or display packet is supplied over USB.

Native measurements and image identity will be recorded separately from operator LCD/button/battery observations. This change does not complete release qualification, physical CI E2E or reproducible distribution packaging.
