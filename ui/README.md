# Search UI preview

A light search-first interface with one centered wordmark, a rounded query field, and one search action. No account, approval, bookmarks, or history screen is included.

From the repository root:

```sh
python3 -m http.server 8828 --bind 127.0.0.1 --directory ui
```

Open http://127.0.0.1:8828/. If the earlier compatibility preview already owns that port, reuse it: its entry/assets now point to this directory. Do not start a competing server.

The service is not connected. A valid submission reports that state; it does not open Google or fabricate results. Queries stay in the current page's memory. The 96-byte UTF-8 limit matches PCS1. The optional canvas/dialog is a layout and input preview, not a physical device feed; host font rendering differs from the device.

The browser field accepts normal text input. The button preview uses A to enter/select a character, B to erase/back, Start to submit, and Select to change character groups. Hiragana, Latin letters, digits, and symbols are available; no Japanese kanji conversion is provided. The native home uses Select for existing Wi-Fi configuration.

See [native UI and verification boundary](../docs/search-ui.md). Test the bounded input with `node --test tests/test_search_input.mjs`; the MCU input helper also has a sanitizer check in hosted CI.
