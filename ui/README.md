# Search and reader preview

A white search home, query-dependent result titles/URLs/snippets, and a paginated semantic reader with headings, paragraphs, images and registered links. No account, approval, bookmarks or history screen is included.

From the repository root:

```sh
cargo run --locked -- serve examples/documents.json 127.0.0.1:8828 --html examples/html-manifest.json
```

Open http://127.0.0.1:8828/. Reuse the existing owned preview port rather than starting competing services. A plain static file server cannot serve the search API. UI files are compiled into the companion binary; rebuild/restart it after editing those assets.

The optional HTML manifest adds original DOM articles, the project logo and an original color-grid image. The current collection contains original sample articles and nonexistent `.invalid` URLs. Results come from the Rust index; this is not real-web or Google search. Queries and navigation stay in current-page memory, with no application history or storage. Requests reach the companion service; an HTTPS deployment provider may observe them.

The field accepts normal text input, capped at 96 UTF-8 bytes. Result titles open the stored article; previous/next buttons read its <=512-byte PCS1 pages. The optional 160 × 144 canvas/button dialog uses the same packets, with Up/Down to select a result, A to read, Left/Right for pages, B to return, and Start for input. Image scenes decode the same bounded PCI1 bytes as the native client; the browser never inserts source HTML or executes its scripts. A opens registered links, with at most four temporary back entries. It is a browser layout/input preview, not a physical device feed; host fonts differ from the device.

The button keyboard provides hiragana, Latin letters, digits and symbols without kanji conversion. The native home retains Select for Wi-Fi configuration; result/reader Select returns home.

See [search transport and evidence](../docs/search-results.md). Run `node --test tests/test_search_input.mjs tests/test_pcs1.mjs tests/test_images.mjs` for UTF-8 input and packet bounds; hosted CI also checks the native input helper with sanitizers.
