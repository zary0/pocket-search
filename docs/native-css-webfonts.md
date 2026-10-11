# Native CSS and web-font subset

The `0.13.21-css` increment keeps collection, indexing, text layout and font rasterization on Chromatic. It adds a deliberately bounded typography cascade and static TrueType subset. It does **not** reproduce Chrome, complete CSS layout or arbitrary web fonts. No companion search/render service or page data over USB is involved.

## Page and resource pipeline

The first streamed HTML pass discovers the first eligible `<link rel="stylesheet">` in the head. Relative links resolve against the page URL; absolute HTTPS paths are supported. The adapter closes that connection, fetches a stylesheet, parses its bounded rules and optional `@font-face`, then loads a supported font. The most recently parsed stylesheet is reused only when its complete resolved URL matches; another URL invalidates that cache. Parsed rules stay disabled during the discovery pass and become active only during the explicit reparse. Stylesheets with no supported typography rules and no usable face do not cause a second HTML request. It reparses a second independently complete, verified HTML response with the cascade before publishing the document. This trades one extra HTML request for avoiding a retained DOM or per-element selector cache. Links remain deduplicated. Every request shares the 90-second collection deadline; page starts remain separated by at least one second.

A temporary document snapshot (at most 3 KiB) preserves the first complete page if the second response fails. Resource errors leave readable built-in typography. Cancellation discards an unpublished page. The same original HTML must pass all existing source/robots/status/type/body checks on either successful pass. The second pass can observe a changed source page: this is not an atomic server snapshot.

Resources use exact public authorities: `www-formal.stanford.edu`, Google Fonts CSS paths on `fonts.googleapis.com`, and font assets on `fonts.gstatic.com`. HTTPS certificate and hostname validation remain enabled. Redirects, compression, userinfo, custom ports, arbitrary hosts and private-address URL literals are not admitted. This is a source-specific resource policy, not a general DNS/IP reclassification system. It must be revisited before expanding source profiles.

The current Stanford AI profile links a 344-byte stylesheet with nine `SMALL.class`/`BIG.class` font-size rules. The bounded parser retains its first eight rules and reuses them across the four collected pages. Those pages do not request a web face; ordinary article text retains the built-in font. The explicit public-font diagnostic establishes the direct font path; host integration tests additionally exercise selector application to original styled HTML. Neither is evidence of native conformance on arbitrary styled sites.

## Supported typography

| Feature | Current treatment |
| --- | --- |
| External stylesheet | First eligible head link; <=3,072 identity-encoded UTF-8 bytes, no imports |
| Selectors | At most eight simple tag, `.class`, `tag.class`, `#id` or `*` selectors; names <=24 bytes; comma lists consume individual rule slots |
| Cascade | ID > tag.class > class > tag > universal, source order, property-wise `!important`, inline precedence, supported-tag inheritance |
| Sizes | Pixel values and `xx-small`, `x-small`, `small`, `smaller`, `medium`, `large`, `larger`, `x-large`, `xx-large` normalize to 10/12/14/16 px |
| Weight | `normal`, `bold`, `bolder`, 100–500 normal / 600–900 bold; bold is synthesized |
| Family | The first parsed face's family can select the native atlas; other families fall back |
| Face source | One `url(...)`, optional `format('truetype')`, URL <=255 bytes |
| Display | Existing 144 × 92 text viewport, paragraph gaps, word wrapping and row scrolling |

CSS applies only to the parser's supported style-bearing tags (`h1`–`h6`, `b`, `strong`, annotations, `font`, `span`, `p`, `div`, `blockquote`, `body`, `a`, `li`, `big`). Unsupported selectors, nested rules, comments inside declaration bodies and unsupported units/properties are skipped. This is not an HTML5 DOM, complete CSS error recovery, media-query evaluator, font shorthand parser or full selector engine. Embedded `<style>` text is still excluded rather than evaluated.

## Bounded TrueType path

The original safe Rust rasterizer accepts raw static TrueType `0x00010000` containers up to **24,576 bytes**. It reads bounded `head`, `maxp`, `hhea`, `hmtx`, `loca`, `glyf` and Unicode cmap format-4 tables. It accepts at most 24 tables, 16 cmap records, 256 segments and 512 glyphs. Each usable glyph is limited to 128 points, 16 contours, 1,024 skipped instruction bytes and 64 intersections per scanline. It does not execute font instructions. Quadratic curves use eight fixed subdivisions, integer scanline intersections and nonzero winding.

Only ASCII 32–126 is rasterized into **95 × 9 bitmap bytes + 12 presence bytes = 867 bytes**. Missing, composite or over-complex glyphs use the built-in font. Non-ASCII glyphs keep the existing built-in coverage. Font metrics normalize to a six-by-twelve cell, scaled to the four native sizes; proportional advances, kerning, hinting, antialiasing, italic, shaping, variable/color fonts, CFF, WOFF and WOFF2 are unsupported. This preserves supported letter shapes, not exact desktop font metrics.

One successful face is immutable for the boot session. Its atlas is published with acquire/release synchronization; GUI code never reads a partly built or freed atlas. A refresh does not replace an atlas still referenced by a displayed packet. A different face falls back until restart. Raw font memory is allocated after the TLS handshake, never above the source cap, and freed before a following socket. The font and index are volatile; no flash cache is introduced.

PSR1 version 2 enables the font-selection nibble bit while retaining the same row widths, packet layout and 112-byte borrowed C view. Version 1 rejects that bit. PCS1 remains unchanged. See [protocol](chromatic-protocol.md#psr1-styled-reader-windows).

## Diagnostics and verification boundaries

`search_local fonts` explicitly fetches a small VT323 CSS/TrueType subset from Google Fonts; it does not collect or index diagnostic articles. Once the font is loaded and a verified real article is open, `search_ui webfont` shows original calibration text visibly marked `TEST`. B returns to the original article. `search_local status` reports resource attempts, atlas readiness, source bytes and glyph count; `search_ui typography` retains the built-in calibration.

The source font is [VT323 under OFL 1.1](https://github.com/google/fonts/blob/main/ofl/vt323/OFL.txt), obtained at runtime and not bundled. The parser/rasterizer and mathematical test outlines are project-authored GPL-3.0-only code; no new dependency or third-party parser is added.

Portable tests cover source specificity/order/importance/inheritance, unsupported rules, resolver failures, every HTML chunk boundary through discovery/reparse, failed second-fetch restoration, exact mathematical holes/curves, truncated/mutated font tables, unsupported containers/composites and resource authority lookalikes. This is bounded regression coverage, not completed fuzz qualification. Native observations and exact installed image identity are recorded in [sanitized evidence](evidence/css-webfonts/verification.json). Render-buffer captures prove MCU drawing; they are not a camera observation of the panel, operator input/battery acceptance or qualified physical E2E.

## Inspected development-unit measurements

The installed application is **1,937,040 bytes**, leaving **160,112 bytes** in the established 2 MiB app partition. Its source identity, archive/image hashes, installation readback and preserved settings are in the evidence file. The opaque Rust state uses **16,416 of 16,640 reserved bytes**; the 656-byte glyph scratch/cache and 867-byte atlas are separate from that region and from existing GUI/TLS/display costs.

One direct collection retained four pages in **36.811 seconds**, including native clock/HTTPS work. Local result generation took **4.656 ms** in that sample. The stylesheet was requested once, with **three cache hits**; later reader requests transferred zero wire bytes. The VT323 diagnostic fetched **17,948 raw font bytes**, produced **34 supported glyphs**, and completed its two resource requests in **2.581 seconds**. Font acquisition sampled **20,388 minimum free heap bytes**, **13,312 minimum largest-block bytes**, and a **3,312-byte stack high-water free margin** in the 10,240-byte worker stack.

These are individual observed samples, not performance guarantees or a complete heap/stack peak trace. Collection under real network conditions still dominates latency. Font rendering CPU time was not measured separately. The smaller persistent atlas and three avoided stylesheet connections are concrete resource reductions; broader memory, complex-font and battery/endurance acceptance remains pending.
