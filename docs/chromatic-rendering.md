# Chromatic display measurement and integration

The experimental C components measure the existing LVGL display transport on physical hardware. They are separate from the Rust client decoder and do not implement a web reader, image transport, or release qualification. The band driver currently depends on the render probe for its timing hooks.

## Target and transport

Use the inspected ESP32 target, ESP-IDF 5.3, LVGL 8.4.0, and RGB565 at 160 × 144. The measured development firmware is based on ModRetro MCU commit `63ed3b62b16768ad2415c9e3b865b438ca40fcc6`, with earlier Wi-Fi and font integration outside this repository. It runs a GUI task on core 1 at a fixed 160 MHz with `-Og`; these are not production Rust optimization results.

The upstream MCU uses a 46,080-byte full framebuffer and 45 transfers of 1,024 bytes. The experimental driver supports a single 16-row buffer (5,120 bytes and five SPI descriptors) or 48-row buffer (15,360 bytes and fifteen descriptors). It rounds dirty rectangles to full-width bands at the selected height. Each 16-row unit is exactly five complete transfers. The FPGA's [QSPI memory port](https://github.com/ModRetro/oss-chromatic-console-fpga/blob/main/esp32t/src/rtl/BSP/mem_system_top.sv) uses a fixed 1,024-byte burst: do not substitute short final transfers or an arbitrary stripe height. The portable planner rejects short and unaligned bands.

Retain the original board pins and QSPI configuration: 40 MHz, QIO data, half duplex, 11 command bits, 32 address bits, three dummy bits, and three CS cycles before/after. The write command is `0x400 | 1023`; addresses are byte offsets into the 46,080-byte framebuffer. The SPI device must have one owner. The driver retires completed transactions before reusing descriptors and signals LVGL only after the final DMA completion for each band.

## Integration

Copy `pocket_render_probe` and `pocket_band_display` from `devices/chromatic/components` into a matching ESP-IDF application's components directory. Add both to the application's `REQUIRES`. Select `POCKET_DISPLAY_ROWS=16` (the default) or `48` through CMake, for example `idf.py -DPOCKET_DISPLAY_ROWS=48 build`. The PUBLIC compile definition also sets the caller’s buffer constants. Do not use a cartridge ROM flashing tool for this MCU application.

For the stock comparison, retain its full buffer, direct mode, full refresh, SPI queue size, and transport. Add `pocket_render_probe_flush_start(46080, lv_disp_flush_is_last(driver))` before queuing a frame, and `pocket_render_probe_flush_end()` in the final SPI completion ISR before `lv_disp_flush_ready()`. Add `driver.render_start_cb = pocket_render_probe_render_start` and initialize the probe after registering the display:

```c
pocket_render_probe_init(display, font, "stock-full", 46080);
```

For the band comparison, remove the old full framebuffer, descriptors, send function, flush callback, and completion callback; leaving them allocated defeats the RAM reduction. Configure the device queue to `POCKET_BAND_MAX_TRANSFERS` and `post_cb` to `pocket_band_display_spi_done`. Initialize the LVGL driver resolution before registering it:

```c
lv_disp_drv_init(&driver);
driver.hor_res = 160;
driver.ver_res = 144;
driver.render_start_cb = pocket_render_probe_render_start;
pocket_band_display_init(spi, &draw_buffer, &driver, capture_observer);
display = lv_disp_drv_register(&driver);
pocket_render_probe_init(display, font, "stripe16", POCKET_BAND_BUFFER_BYTES);
// Use "stripe48" when POCKET_DISPLAY_ROWS=48.
```

`capture_observer` may be `NULL`. It runs synchronously on the GUI task before DMA; its color pointer is valid only during that call. Normal drawing uses no second framebuffer. The measured fixture uses an existing `pocket_jp` font with 14-pixel line height and zero extra line spacing. That earlier font integration is not bundled here. Keep the same font, configuration, platform, and fixture for both modes; substituting a font changes results.

Both modes call `pocket_render_probe_step()` immediately before `lv_timer_handler()` on the GUI task. Surround the handler with `esp_timer_get_time()` and pass its elapsed microseconds to `pocket_render_probe_handler_time()`. Do not call LVGL from the console task. Register all diagnostic commands before starting the console. Keep the CPU frequency, task pinning, GUI scheduling, tick configuration, Wi-Fi connection, and compiler flags unchanged between modes.

## Read-only diagnostics

The probe accepts `render_probe idle`, `counter`, `page`, and `full`. Each uses six warm-up steps followed by 32 steps, spaced at least 100 ms by the native timer. Screen changes abort the case. `show` loads the measurement scene outside a timing window; `restore` returns to its saved screen and deletes the fixture. The console prints one `RENDER_REPORT` per completed case.

The collector also requires the existing development adapter's `fwversion`, `wifi_status`, and `pocket_screen` commands. A different adapter must implement that contract before using this collector. `fwversion` returns the MCU version; the legacy version string is truncated, so every report must independently match the full app-description version. `wifi_status` returns a `CWJSON` object with `state: connected`.

`pocket_screen` returns `POCKETFRAME 160 144 RGB565`, 144 rows of 640 lowercase hexadecimal characters, then `POCKETFRAME_END`. This is MCU render-buffer data, not a USB-video capture. The full-buffer adapter copies a completed render. The band adapter must claim a requested snapshot on the GUI task **before** invalidating the whole scene, then concatenate all bands at `area.y1 * 320`. Signal completion only after the last band. On timeout, the console must not free a snapshot that the GUI task owns. Never copy only the last band buffer or claim a partially initialized frame as complete.

The diagnostic snapshot temporarily allocates another 46,080 bytes. Allocate it only after all measurement windows; exclude this diagnostic memory from normal-operation claims. The collector retries only incomplete read-only captures, at most three times, and preserves completed measurements after a failure. It never flashes firmware, changes Wi-Fi settings, or approves a release.

Install pyserial in a suitable Python environment. After device-specific application installation, recovery/settings checks, and read-back verification, supply that verified image digest:

```sh
python3 scripts/measure_chromatic_render.py \
  --port YOUR_SELECTED_CHROMATIC_PORT \
  --firmware-version YOUR_FULL_APP_VERSION \
  --application-sha256 YOUR_READBACK_VERIFIED_SHA256 \
  --mode stock-full --repeat 3 \
  --output NEW_BASELINE.json --capture NEW_BASELINE.png
# Repeat with each band image, --mode stripe16, stripe48, or stripe48-o2, and new paths.
```

A partial JSON with every sample can resume only its capture using the same arguments plus `--resume-capture`. Verify that the same application is still installed first. Do not overwrite earlier evidence. Keep identifiers, credentials, NVS backups, and raw console logs outside the repository.

## Interpretation

`frame_total_us / frames` measures render-start to final DMA completion, including LVGL work, interrupt and scheduler delays. It does not measure the LCD's visible scanout or input-to-photon latency. `handler_total_us` is wall time including waits, not CPU cycles or utilization. Transfer counts exclude command/address overhead. The window heap low-water mark includes all firmware tasks and background Wi-Fi; the since-boot low-water mark includes earlier boot work. Instant free heap is not a peak-memory measurement. Readback equality verifies render data, not the physical panel or FPGA memory contents.

Retain idle, small counter changes, body-page replacement, and full-screen background changes as distinct workloads. Small dirty regions can become faster while full-screen changes become slower with a single smaller buffer. Report that tradeoff and variability. This probe does not qualify the native Wi-Fi search loop or justify replacing FreeRTOS with bare metal. The physical release gate remains required.

The measured development firmware retains the upstream tick call, which passes the 1,000 µs constant directly to `lv_tick_inc()` although LVGL expects milliseconds. Native `esp_timer` controls this probe's scheduling and timing. Correct that platform issue separately and rerun the comparison before treating the fixture's animation/timer behavior as production behavior; this A/B comparison does not change it.

## Optional graphics compiler experiment

For the inspected application, copy `devices/chromatic/cmake/pocket_graphics.cmake` into the firmware and include it **after** `project()` in the root CMakeLists. Call `pocket_optimize_graphics()` there. It appends `-O2` only to the existing `lvgl`, `fonts`, and `pocket_band_display` targets; confirm that the generated compile commands place it after `-Og`. The probe, network/TLS, assertions, and platform clock settings retain their existing flags. This helper requires those exact component names.

Use `POCKET_DISPLAY_ROWS=48` and probe mode `stripe48-o2` for the recorded experiment. Give the installed image a unique version and hash. Compare pixel readback, timings, heap, linked sizes, and application bytes against the same 48-row `-Og` image before selecting it. This is a native Xtensa C compiler experiment, not a Rust profile measurement, and it does not remove FreeRTOS or certificate checks.
