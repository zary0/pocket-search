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

Device-to-computer LAN reachability was not established. An externally reachable HTTPS service is a candidate for testing; none is deployed by this repository.

## What is portable today

`crates/chromatic-client` provides a dependency-free `no_std` decoder for a single PCS1 text packet and button action mapping. It borrows at most one 512-byte buffer. It has no allocator, framebuffer, network stack, font renderer, OS bindings, or unsafe code.

`src/chromatic.rs` prepares text on the host for the target's row and byte limits. The host example checks encoder/decoder compatibility with original fixtures. It does not connect to a console.

## Firmware integration still required

1. Build the core for the actual ESP32 Xtensa toolchain.
2. Connect existing display, button, and font drivers through a defined C ABI or native Rust integration.
3. Preserve Wi-Fi settings, native SNTP, and certificate verification.
4. Use a capped receive buffer; reject oversized HTTP bodies before decoding.
5. Translate actions into requests and UI state changes.
6. Inspect text layout, Unicode fallback, controls, TLS memory costs, and errors physically.

The platform source is [ModRetro's Chromatic MCU repository](https://github.com/ModRetro/oss-chromatic-console-mcu). Existing firmware and font code are not copied here. Future imported or modified upstream source must retain its licensing and notices.

## Deployment boundary

Use ModRetro plugin discovery and physical capture to identify the intended device and inspect actual screens. Cartridge ROM play/flash is separate from MCU deployment. A streamed emulator demo does not verify this native Wi-Fi client.

A future MCU installer needs the intended device, exact image, verified recovery backup, preserved settings, and read-back verification. This repository supplies no MCU image or flashing command. Do not substitute cartridge flashing for MCU application deployment.

Never commit activation codes, passwords, tokens, NVS contents, device-specific backups, or full-flash dumps. Store sanitized measurements and distinguish host checks, streamed demos, USB diagnostics, and physical Wi-Fi operation.

See the [protocol](../../docs/chromatic-protocol.md) and [hardware-first roadmap](../../docs/roadmap.md).

## CPU and memory measurements

The Rust client stays integer-only, dependency-free, allocation-free, and bounded. Compare the `chromatic-size` and `chromatic-speed` build profiles on the actual Xtensa toolchain before choosing a firmware setting. The current Rust build checks are host-only.

`components/pocket_perf_probe` provides an ESP-IDF CPU-cycle and heap probe for the ESP32 configuration. It was cross-compiled with the existing target C compiler and SDK, but has not been linked or executed physically. Use a pinned task and fixed frequency with a bounded synchronous callback. Its measurements include interrupts; network and display tests need separate end-to-end timing. See [performance notes](../../docs/performance.md) for the exact scope and reproducible host results.
