# Chromatic MCU target

Pocket Search targets a custom application in the ModRetro Chromatic MCU firmware, using the console's menu display, buttons, and Wi-Fi-capable development hardware. This is a development target, not a claim that stock firmware includes a browser.

## Measured development profile

Earlier work outside this repository inspected and tested one Chromatic DevDay unit:

| Property | Observed value |
| --- | --- |
| MCU | ESP32-U4WDH / Xtensa, 160 MHz |
| Flash | 4 MiB total; 2 MiB application partition in the development firmware |
| PSRAM | Not enabled in the inspected build |
| Display | 160 × 144 pixels, RGB565 through LVGL |
| Platform | ESP-IDF 5.3, based on MCU firmware 0.13.8 |
| Heap before small HTTPS probe | 59,652 bytes; largest block 55,296 bytes |
| Verified transport | Native Wi-Fi DNS, native SNTP, certificate-validated HTTPS |
| Probe result | `https://example.com/`, HTTP 200, complete 577-byte body |

The probe used USB for power, its diagnostic command, and reading the result. Wi-Fi fetched the page and clock without a host HTTP relay or supplied host clock. Battery-only operation was not tested. These observations are specific to that earlier unit and firmware, not Pocket Search measurements or guaranteed memory availability on other consoles.

Device-to-computer LAN reachability was not established. Earlier companion experiments used an authorized temporary HTTPS tunnel. The current autonomous mode directly collects, indexes and searches pages on the MCU and requires neither that tunnel nor a companion service; see [on-device search](../../docs/on-device-search.md).

## DevDay storage verification

As checked on 2026-10-12, the inspected unit's 4 MiB MCU flash is verified, but a
populated microSD connector on the DevDay edition is not verified. The
[official DevDay quickstart](https://support.modretro.com/en_us/chromatic-devday-edition-quickstart-guid-By1iOlcMg)
documents the included writable cartridge and developer activation, without an
SD-card storage specification. Those cartridge features do not establish MCU
access to removable storage.

The pinned [public MCU sources](https://github.com/ModRetro/oss-chromatic-console-mcu/tree/63ed3b62b16768ad2415c9e3b865b438ca40fcc6)
and this application expose no SD-card mount or card-identification command.
USB discovery confirms the console's connection, not connector population.
Visual inspection of the specific DevDay board or manufacturer confirmation is
still required; absence of a driver does not prove absence of a connector.
Pocket Search currently uses its owned internal-flash banks for retained content,
and does not claim SD support or any removable-storage capacity.

## What is portable today

`crates/chromatic-client` provides dependency-free `no_std` text/image validation, button action mapping, and the on-device crawl frontier, streaming HTML text, fixed-capacity inverted index and local pagination. Rust owns no allocator, framebuffer, network stack, font renderer or OS bindings. The adapter reserves 16 KiB for opaque local state plus its separate robots buffer; networking and display retain separate costs.

`src/chromatic.rs` prepares text on the host for the target's row and byte limits. The host example checks encoder/decoder compatibility with original fixtures. It does not connect to a console.

## Experimental firmware integration

1. Build the core and C ABI for the actual ESP32 Xtensa toolchain; this now succeeds in the local experimental image.
2. The repository UI/HTTPS adapter links the Rust C ABI to the existing display/button/font drivers. Complete reproducible upstream/font integration is still required.
3. Preserve Wi-Fi settings, native SNTP, and certificate verification.
4. Use a capped receive buffer; reject oversized HTTP bodies before decoding.
5. Translate actions into requests and UI state changes.
6. Inspect text layout, Unicode fallback, controls, TLS memory costs, and errors physically.

The platform source is [ModRetro's Chromatic MCU repository](https://github.com/ModRetro/oss-chromatic-console-mcu). Experimental display/probe components are supplied here; complete existing firmware and font assets are not bundled. Future imported or modified upstream source must retain its licensing and notices.

## Deployment boundary

Use ModRetro plugin discovery and physical capture to identify the intended device and inspect actual screens. Cartridge ROM play/flash is separate from MCU deployment. A streamed emulator demo does not verify this native Wi-Fi client.

A future MCU installer needs the intended device, exact image, verified recovery backup, preserved settings, and read-back verification. This repository supplies no MCU image or flashing command. Do not substitute cartridge flashing for MCU application deployment.

Never commit activation codes, passwords, tokens, NVS contents, device-specific backups, or full-flash dumps. Store sanitized measurements and distinguish host checks, streamed demos, USB diagnostics, and physical Wi-Fi operation.

See the [protocol](../../docs/chromatic-protocol.md) and [hardware-first roadmap](../../docs/roadmap.md).

## Native button audio

The optional [FPGA click implementation](../../docs/native-click.md) creates physical menu-button feedback without MCU audio files or tasks. Its RTL simulation, pinned-source integration and licensed Gowin 1.9.12.04 build pass, with zero reported timing violations. Actual speaker verification and recovery qualification remain pending. It requires a separate FPGA image, not an MCU application update or a cartridge ROM.

## CPU and memory measurements

The Rust client stays integer-only, dependency-free, allocation-free, and bounded. Compare the `chromatic-size` and `chromatic-speed` build profiles on the actual Xtensa toolchain before choosing a firmware setting. Hosted Rust checks are host-only; the separate experimental image has an actual Xtensa build and device execution.

`components/pocket_perf_probe` provides an ESP-IDF CPU-cycle and heap probe for the ESP32 configuration. It was cross-compiled with the existing target C compiler and SDK, but has not been linked or executed physically. Use a pinned task and fixed frequency with a bounded synchronous callback. Its measurements include interrupts; network and display tests need separate end-to-end timing. See [performance notes](../../docs/performance.md) for the exact scope and reproducible host results.

## Experimental display components

`components/pocket_render_probe` measures idle, counter, text-page, and full-background updates in the existing native LVGL platform. `components/pocket_band_display` replaces the full framebuffer with a single 16-row or 48-row DMA buffer and retires completed SPI transactions before descriptor reuse. The portable transfer-boundary test runs in hosted CI; actual target builds and physical measurements are separate evidence.

This is a platform-level C integration. The standalone rendering probe does not connect the Rust decoder or satisfy the physical release gate; the current search component integrates the Rust decoder and on-device collection; the earlier companion sample is separate historical evidence. The newer DOM/image adapter adds bounded RGB565 display; [its evidence](../../docs/dom-images.md) is separate from the display probe. The current standalone collector uses native Wi-Fi and USB diagnostics; its screenshots are actual MCU render-buffer readbacks rather than USB-video captures. See [integration and measurement details](../../docs/chromatic-rendering.md).

The measured text prototype uses 48 rows and an opt-in graphics-only `-O2` build: 15 KiB of draw-buffer storage, 8.97 ms counter updates, and 24.68 ms full-background updates. Stock was 45 KiB and 20.87/20.88 ms respectively. The smaller buffer improves memory and small counter updates; page replacement and full updates remain slower. These are medians of three window means on one development unit, not complete reader or panel-scanout performance. See [raw-backed results](../../docs/performance.md#physical-lvgl-display-comparison).

## Native sample search

`components/pocket_search_ui` now provides a bounded native HTTPS worker, results and reader screens. `crates/chromatic-ffi` is an allocation-free 48-byte scene view and 20-byte image-stripe view of the Rust decoder, compiled for `xtensa-esp32-none-elf` and linked into experimental DOM/image application (the earlier text-only image was `0.13.11-results`). See [commands, build identity and physical evidence](../../docs/search-results.md). The [complete MCU build](../../docs/native-firmware-build.md) now uses pinned public sources and passes hosted native CI. [Retained standalone sources and baseline JPEG](../../docs/native-retained-content.md) and [scoped native measurements](../../docs/native-device-measurements.md) are implemented. Complete failure/mixed-language physical acceptance, distribution review and release qualification remain pending.
