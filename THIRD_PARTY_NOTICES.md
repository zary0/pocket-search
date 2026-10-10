# Third-party notices

This inventory describes the dependencies locked in `Cargo.lock` and the upstream basis of the display transport. Project-authored material remains GPL-3.0-only; third-party material retains its own terms. See [project notice](NOTICE.md) and [GPL license](LICENSE).

## Rust dependencies

Dependency source is downloaded by Cargo rather than vendored here. The companion binary includes compiled dependencies; procedural-macro dependencies are used while building. For dual-licensed packages, this distribution uses the MIT option. `unicode-ident` also requires its Unicode-3.0 notice. The unmodified selected license/copyright texts below are included in source and companion packages. Versions are from the committed lockfile; review and update this inventory and the license files whenever dependencies change.

| Package / upstream | Locked version | Upstream license expression | Use | Included license text |
| --- | --- | --- | --- | --- |
| [itoa](https://github.com/dtolnay/itoa) | 1.0.18 | `MIT OR Apache-2.0` | Companion dependency | [MIT](third_party/licenses/itoa-1.0.18/LICENSE-MIT) |
| [memchr](https://github.com/BurntSushi/memchr) | 2.8.3 | `Unlicense OR MIT` | Companion dependency | [MIT](third_party/licenses/memchr-2.8.3/LICENSE-MIT) |
| [proc-macro2](https://github.com/dtolnay/proc-macro2) | 1.0.107 | `MIT OR Apache-2.0` | Build-time procedural macro | [MIT](third_party/licenses/proc-macro2-1.0.107/LICENSE-MIT) |
| [quote](https://github.com/dtolnay/quote) | 1.0.47 | `MIT OR Apache-2.0` | Build-time procedural macro | [MIT](third_party/licenses/quote-1.0.47/LICENSE-MIT) |
| [serde](https://github.com/serde-rs/serde) | 1.0.229 | `MIT OR Apache-2.0` | Companion dependency | [MIT](third_party/licenses/serde-1.0.229/LICENSE-MIT) |
| [serde_core](https://github.com/serde-rs/serde) | 1.0.229 | `MIT OR Apache-2.0` | Companion dependency | [MIT](third_party/licenses/serde_core-1.0.229/LICENSE-MIT) |
| [serde_derive](https://github.com/serde-rs/serde) | 1.0.229 | `MIT OR Apache-2.0` | Build-time procedural macro | [MIT](third_party/licenses/serde_derive-1.0.229/LICENSE-MIT) |
| [serde_json](https://github.com/serde-rs/json) | 1.0.151 | `MIT OR Apache-2.0` | Companion dependency | [MIT](third_party/licenses/serde_json-1.0.151/LICENSE-MIT) |
| [syn](https://github.com/dtolnay/syn) | 3.0.6 | `MIT OR Apache-2.0` | Build-time procedural macro | [MIT](third_party/licenses/syn-3.0.6/LICENSE-MIT) |
| [unicode-ident](https://github.com/dtolnay/unicode-ident) | 1.0.26 | `(MIT OR Apache-2.0) AND Unicode-3.0` | Build-time procedural macro | [MIT](third_party/licenses/unicode-ident-1.0.26/LICENSE-MIT) + [Unicode](third_party/licenses/unicode-ident-1.0.26/LICENSE-UNICODE) |
| [zmij](https://github.com/dtolnay/zmij) | 1.0.23 | `MIT` | Companion dependency | [MIT](third_party/licenses/zmij-1.0.23/LICENSE-MIT) |

`chromatic-client` has no external crate dependencies. It is project-authored and licensed GPL-3.0-only.

## ModRetro display transport

The QSPI settings, framebuffer addressing, and display initialization sequence in `devices/chromatic/components/pocket_band_display` follow the ModRetro Chromatic MCU transport at [commit 63ed3b62b16768ad2415c9e3b865b438ca40fcc6](https://github.com/ModRetro/oss-chromatic-console-mcu/tree/63ed3b62b16768ad2415c9e3b865b438ca40fcc6), whose [license](https://github.com/ModRetro/oss-chromatic-console-mcu/blob/63ed3b62b16768ad2415c9e3b865b438ca40fcc6/LICENSE) is GNU GPL v3. Pocket Search's bounded band driver replaces the full-frame buffering and transaction handling; its source and modifications are GPL-3.0-only. Preserve applicable upstream notices when importing further code.

No official firmware binary, FPGA image, commercial game, device backup, or third-party font is included. ModRetro and Chromatic identify the target device; the project is independent of ModRetro.

## External build and runtime prerequisites

ESP-IDF 5.3, FreeRTOS, LVGL 8.4.0, the Xtensa toolchain, and a suitable licensed font are external prerequisites of the experimental MCU integration. Their source/binaries are not bundled in the current packages. A future native application distribution must inventory the exact linked components and assets, include their required notices, and provide corresponding source/build instructions before release. Do not treat this list as a completed firmware distribution review.

Third-party GitHub Actions execute in CI and are not part of the release application. Their upstream source and pinned revisions are recorded in `.github/workflows`; dependency updates keep those pins reviewable.
