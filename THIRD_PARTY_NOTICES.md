# Third-party notices

This inventory describes the dependencies locked in `Cargo.lock` and the upstream basis of the display transport. Project-authored material remains GPL-3.0-only; third-party material retains its own terms. See [project notice](NOTICE.md) and [GPL license](LICENSE).

## Rust dependencies

Dependency source is downloaded by Cargo rather than vendored here. The host companion includes HTML5 parsing, URL normalization, and PNG/JPEG preparation; procedural macros and platform dependencies are also inventoried. The device crates do not inherit these host dependencies. For packages offering MIT, this distribution selects MIT; Unicode notices remain required where stated. Other packages retain ISC, Unicode-3.0, Zlib, BSD-3-Clause or MPL-2.0 as listed below. These dependencies are unmodified.

Exact locked package archives (including MPL source) are available from the versioned crates.io links in [the machine-readable inventory](third_party/dependencies.json). `cargo fetch --locked` retrieves those sources using the checksums in `Cargo.lock`. Recipients of the companion can obtain the unmodified MPL-covered source through those versioned archive links. Preserve upstream notices and source availability when distributing binaries. The `selectors` archive omits a standalone license file; its source notice and the canonical Mozilla MPL-2.0 text are included explicitly.

| Package / source | Locked version | Upstream license expression | Selected terms | Included text |
| --- | --- | --- | --- | --- |
| [adler2](https://crates.io/crates/adler2/2.0.1) | 2.0.1 | `0BSD OR MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/adler2-2.0.1/LICENSE-MIT) + [LICENSE-0BSD](third_party/licenses/adler2-2.0.1/LICENSE-0BSD) |
| [autocfg](https://crates.io/crates/autocfg/1.5.1) | 1.5.1 | `Apache-2.0 OR MIT` | `MIT` | [LICENSE-MIT](third_party/licenses/autocfg-1.5.1/LICENSE-MIT) |
| [bitflags](https://crates.io/crates/bitflags/2.13.2) | 2.13.2 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/bitflags-2.13.2/LICENSE-MIT) |
| [bytemuck](https://crates.io/crates/bytemuck/1.25.2) | 1.25.2 | `Zlib OR Apache-2.0 OR MIT` | `MIT` | [LICENSE-MIT](third_party/licenses/bytemuck-1.25.2/LICENSE-MIT) |
| [byteorder-lite](https://crates.io/crates/byteorder-lite/0.1.0) | 0.1.0 | `Unlicense OR MIT` | `MIT` | [LICENSE-MIT](third_party/licenses/byteorder-lite-0.1.0/LICENSE-MIT) |
| [cfg-if](https://crates.io/crates/cfg-if/1.0.5) | 1.0.5 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/cfg-if-1.0.5/LICENSE-MIT) |
| [crc32fast](https://crates.io/crates/crc32fast/1.5.2) | 1.5.2 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/crc32fast-1.5.2/LICENSE-MIT) |
| [cssparser](https://crates.io/crates/cssparser/0.37.0) | 0.37.0 | `MPL-2.0` | `MPL-2.0` | [LICENSE](third_party/licenses/cssparser-0.37.0/LICENSE) |
| [cssparser-macros](https://crates.io/crates/cssparser-macros/0.7.1) | 0.7.1 | `MPL-2.0` | `MPL-2.0` | [LICENSE](third_party/licenses/cssparser-macros-0.7.1/LICENSE) |
| [derive_more](https://crates.io/crates/derive_more/2.1.1) | 2.1.1 | `MIT` | `MIT` | [LICENSE](third_party/licenses/derive_more-2.1.1/LICENSE) |
| [derive_more-impl](https://crates.io/crates/derive_more-impl/2.1.1) | 2.1.1 | `MIT` | `MIT` | [LICENSE](third_party/licenses/derive_more-impl-2.1.1/LICENSE) |
| [displaydoc](https://crates.io/crates/displaydoc/0.2.7) | 0.2.7 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/displaydoc-0.2.7/LICENSE-MIT) |
| [dtoa](https://crates.io/crates/dtoa/1.0.11) | 1.0.11 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/dtoa-1.0.11/LICENSE-MIT) |
| [dtoa-short](https://crates.io/crates/dtoa-short/0.3.5) | 0.3.5 | `MPL-2.0` | `MPL-2.0` | [LICENSE](third_party/licenses/dtoa-short-0.3.5/LICENSE) |
| [ego-tree](https://crates.io/crates/ego-tree/0.11.0) | 0.11.0 | `ISC` | `ISC` | [LICENSE](third_party/licenses/ego-tree-0.11.0/LICENSE) |
| [fastrand](https://crates.io/crates/fastrand/2.5.0) | 2.5.0 | `Apache-2.0 OR MIT` | `MIT` | [LICENSE-MIT](third_party/licenses/fastrand-2.5.0/LICENSE-MIT) |
| [fdeflate](https://crates.io/crates/fdeflate/0.3.7) | 0.3.7 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/fdeflate-0.3.7/LICENSE-MIT) + [SOURCE-NOTICE.txt](third_party/licenses/fdeflate-0.3.7/SOURCE-NOTICE.txt) |
| [flate2](https://crates.io/crates/flate2/1.1.10) | 1.1.10 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/flate2-1.1.10/LICENSE-MIT) |
| [form_urlencoded](https://crates.io/crates/form_urlencoded/1.2.2) | 1.2.2 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/form_urlencoded-1.2.2/LICENSE-MIT) |
| [html5ever](https://crates.io/crates/html5ever/0.39.0) | 0.39.0 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/html5ever-0.39.0/LICENSE-MIT) |
| [icu_collections](https://crates.io/crates/icu_collections/2.1.1) | 2.1.1 | `Unicode-3.0` | `Unicode-3.0` | [LICENSE](third_party/licenses/icu_collections-2.1.1/LICENSE) |
| [icu_locale_core](https://crates.io/crates/icu_locale_core/2.1.1) | 2.1.1 | `Unicode-3.0` | `Unicode-3.0` | [LICENSE](third_party/licenses/icu_locale_core-2.1.1/LICENSE) |
| [icu_normalizer](https://crates.io/crates/icu_normalizer/2.1.1) | 2.1.1 | `Unicode-3.0` | `Unicode-3.0` | [LICENSE](third_party/licenses/icu_normalizer-2.1.1/LICENSE) |
| [icu_normalizer_data](https://crates.io/crates/icu_normalizer_data/2.1.1) | 2.1.1 | `Unicode-3.0` | `Unicode-3.0` | [LICENSE](third_party/licenses/icu_normalizer_data-2.1.1/LICENSE) |
| [icu_properties](https://crates.io/crates/icu_properties/2.1.2) | 2.1.2 | `Unicode-3.0` | `Unicode-3.0` | [LICENSE](third_party/licenses/icu_properties-2.1.2/LICENSE) |
| [icu_properties_data](https://crates.io/crates/icu_properties_data/2.1.2) | 2.1.2 | `Unicode-3.0` | `Unicode-3.0` | [LICENSE](third_party/licenses/icu_properties_data-2.1.2/LICENSE) |
| [icu_provider](https://crates.io/crates/icu_provider/2.1.1) | 2.1.1 | `Unicode-3.0` | `Unicode-3.0` | [LICENSE](third_party/licenses/icu_provider-2.1.1/LICENSE) |
| [idna](https://crates.io/crates/idna/1.1.0) | 1.1.0 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/idna-1.1.0/LICENSE-MIT) |
| [idna_adapter](https://crates.io/crates/idna_adapter/1.2.1) | 1.2.1 | `Apache-2.0 OR MIT` | `MIT` | [LICENSE-MIT](third_party/licenses/idna_adapter-1.2.1/LICENSE-MIT) |
| [image](https://crates.io/crates/image/0.25.10) | 0.25.10 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/image-0.25.10/LICENSE-MIT) |
| [itoa](https://crates.io/crates/itoa/1.0.18) | 1.0.18 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/itoa-1.0.18/LICENSE-MIT) |
| [libc](https://crates.io/crates/libc/0.2.190) | 0.2.190 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/libc-0.2.190/LICENSE-MIT) |
| [litemap](https://crates.io/crates/litemap/0.8.3) | 0.8.3 | `Unicode-3.0` | `Unicode-3.0` | [LICENSE](third_party/licenses/litemap-0.8.3/LICENSE) |
| [lock_api](https://crates.io/crates/lock_api/0.4.14) | 0.4.14 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/lock_api-0.4.14/LICENSE-MIT) |
| [log](https://crates.io/crates/log/0.4.34) | 0.4.34 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/log-0.4.34/LICENSE-MIT) |
| [markup5ever](https://crates.io/crates/markup5ever/0.39.0) | 0.39.0 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/markup5ever-0.39.0/LICENSE-MIT) |
| [memchr](https://crates.io/crates/memchr/2.8.3) | 2.8.3 | `Unlicense OR MIT` | `MIT` | [LICENSE-MIT](third_party/licenses/memchr-2.8.3/LICENSE-MIT) |
| [miniz_oxide](https://crates.io/crates/miniz_oxide/0.8.9) | 0.8.9 | `MIT OR Zlib OR Apache-2.0` | `MIT` | [LICENSE-MIT.md](third_party/licenses/miniz_oxide-0.8.9/LICENSE-MIT.md) |
| [miniz_oxide](https://crates.io/crates/miniz_oxide/0.9.1) | 0.9.1 | `MIT OR Zlib OR Apache-2.0` | `MIT` | [LICENSE-MIT.md](third_party/licenses/miniz_oxide-0.9.1/LICENSE-MIT.md) |
| [moxcms](https://crates.io/crates/moxcms/0.8.1) | 0.8.1 | `BSD-3-Clause OR Apache-2.0` | `BSD-3-Clause` | [LICENSE.md](third_party/licenses/moxcms-0.8.1/LICENSE.md) |
| [new_debug_unreachable](https://crates.io/crates/new_debug_unreachable/1.0.6) | 1.0.6 | `MIT` | `MIT` | [LICENSE-MIT](third_party/licenses/new_debug_unreachable-1.0.6/LICENSE-MIT) |
| [num-traits](https://crates.io/crates/num-traits/0.2.19) | 0.2.19 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/num-traits-0.2.19/LICENSE-MIT) |
| [parking_lot](https://crates.io/crates/parking_lot/0.12.5) | 0.12.5 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/parking_lot-0.12.5/LICENSE-MIT) |
| [parking_lot_core](https://crates.io/crates/parking_lot_core/0.9.12) | 0.9.12 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/parking_lot_core-0.9.12/LICENSE-MIT) |
| [percent-encoding](https://crates.io/crates/percent-encoding/2.3.2) | 2.3.2 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/percent-encoding-2.3.2/LICENSE-MIT) |
| [phf](https://crates.io/crates/phf/0.13.1) | 0.13.1 | `MIT` | `MIT` | [LICENSE](third_party/licenses/phf-0.13.1/LICENSE) |
| [phf_codegen](https://crates.io/crates/phf_codegen/0.13.1) | 0.13.1 | `MIT` | `MIT` | [LICENSE](third_party/licenses/phf_codegen-0.13.1/LICENSE) |
| [phf_generator](https://crates.io/crates/phf_generator/0.13.1) | 0.13.1 | `MIT` | `MIT` | [LICENSE](third_party/licenses/phf_generator-0.13.1/LICENSE) |
| [phf_macros](https://crates.io/crates/phf_macros/0.13.1) | 0.13.1 | `MIT` | `MIT` | [LICENSE](third_party/licenses/phf_macros-0.13.1/LICENSE) |
| [phf_shared](https://crates.io/crates/phf_shared/0.13.1) | 0.13.1 | `MIT` | `MIT` | [LICENSE](third_party/licenses/phf_shared-0.13.1/LICENSE) |
| [png](https://crates.io/crates/png/0.18.1) | 0.18.1 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/png-0.18.1/LICENSE-MIT) |
| [potential_utf](https://crates.io/crates/potential_utf/0.1.6) | 0.1.6 | `Unicode-3.0` | `Unicode-3.0` | [LICENSE](third_party/licenses/potential_utf-0.1.6/LICENSE) |
| [precomputed-hash](https://crates.io/crates/precomputed-hash/0.1.1) | 0.1.1 | `MIT` | `MIT` | [LICENSE](third_party/licenses/precomputed-hash-0.1.1/LICENSE) |
| [proc-macro2](https://crates.io/crates/proc-macro2/1.0.107) | 1.0.107 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/proc-macro2-1.0.107/LICENSE-MIT) |
| [pxfm](https://crates.io/crates/pxfm/0.1.30) | 0.1.30 | `BSD-3-Clause OR Apache-2.0` | `BSD-3-Clause` | [LICENSE.md](third_party/licenses/pxfm-0.1.30/LICENSE.md) |
| [quote](https://crates.io/crates/quote/1.0.47) | 1.0.47 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/quote-1.0.47/LICENSE-MIT) |
| [redox_syscall](https://crates.io/crates/redox_syscall/0.5.18) | 0.5.18 | `MIT` | `MIT` | [LICENSE](third_party/licenses/redox_syscall-0.5.18/LICENSE) |
| [rustc-hash](https://crates.io/crates/rustc-hash/2.1.3) | 2.1.3 | `Apache-2.0 OR MIT` | `MIT` | [LICENSE-MIT](third_party/licenses/rustc-hash-2.1.3/LICENSE-MIT) |
| [rustc_version](https://crates.io/crates/rustc_version/0.4.1) | 0.4.1 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/rustc_version-0.4.1/LICENSE-MIT) |
| [scopeguard](https://crates.io/crates/scopeguard/1.2.0) | 1.2.0 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/scopeguard-1.2.0/LICENSE-MIT) |
| [scraper](https://crates.io/crates/scraper/0.27.0) | 0.27.0 | `ISC` | `ISC` | [LICENSE](third_party/licenses/scraper-0.27.0/LICENSE) |
| [selectors](https://crates.io/crates/selectors/0.38.0) | 0.38.0 | `MPL-2.0` | `MPL-2.0` | [LICENSE-MPL-2.0](third_party/licenses/selectors-0.38.0/LICENSE-MPL-2.0) + [SOURCE-NOTICE.txt](third_party/licenses/selectors-0.38.0/SOURCE-NOTICE.txt) |
| [semver](https://crates.io/crates/semver/1.0.28) | 1.0.28 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/semver-1.0.28/LICENSE-MIT) |
| [serde](https://crates.io/crates/serde/1.0.229) | 1.0.229 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/serde-1.0.229/LICENSE-MIT) |
| [serde_core](https://crates.io/crates/serde_core/1.0.229) | 1.0.229 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/serde_core-1.0.229/LICENSE-MIT) |
| [serde_derive](https://crates.io/crates/serde_derive/1.0.229) | 1.0.229 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/serde_derive-1.0.229/LICENSE-MIT) |
| [serde_json](https://crates.io/crates/serde_json/1.0.151) | 1.0.151 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/serde_json-1.0.151/LICENSE-MIT) |
| [servo_arc](https://crates.io/crates/servo_arc/0.4.3) | 0.4.3 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/servo_arc-0.4.3/LICENSE-MIT) + [SOURCE-NOTICE.txt](third_party/licenses/servo_arc-0.4.3/SOURCE-NOTICE.txt) |
| [simd-adler32](https://crates.io/crates/simd-adler32/0.3.10) | 0.3.10 | `MIT` | `MIT` | [LICENSE.md](third_party/licenses/simd-adler32-0.3.10/LICENSE.md) |
| [siphasher](https://crates.io/crates/siphasher/1.0.4) | 1.0.4 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/siphasher-1.0.4/LICENSE-MIT) + [COPYING](third_party/licenses/siphasher-1.0.4/COPYING) |
| [smallvec](https://crates.io/crates/smallvec/1.16.3) | 1.16.3 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/smallvec-1.16.3/LICENSE-MIT) |
| [stable_deref_trait](https://crates.io/crates/stable_deref_trait/1.2.1) | 1.2.1 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/stable_deref_trait-1.2.1/LICENSE-MIT) |
| [string_cache](https://crates.io/crates/string_cache/0.9.0) | 0.9.0 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/string_cache-0.9.0/LICENSE-MIT) |
| [string_cache_codegen](https://crates.io/crates/string_cache_codegen/0.6.1) | 0.6.1 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/string_cache_codegen-0.6.1/LICENSE-MIT) |
| [syn](https://crates.io/crates/syn/2.0.119) | 2.0.119 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/syn-2.0.119/LICENSE-MIT) |
| [syn](https://crates.io/crates/syn/3.0.6) | 3.0.6 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/syn-3.0.6/LICENSE-MIT) |
| [synstructure](https://crates.io/crates/synstructure/0.14.0) | 0.14.0 | `MIT` | `MIT` | [LICENSE](third_party/licenses/synstructure-0.14.0/LICENSE) |
| [tendril](https://crates.io/crates/tendril/0.5.1) | 0.5.1 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/tendril-0.5.1/LICENSE-MIT) |
| [tinystr](https://crates.io/crates/tinystr/0.8.4) | 0.8.4 | `Unicode-3.0` | `Unicode-3.0` | [LICENSE](third_party/licenses/tinystr-0.8.4/LICENSE) |
| [unicode-ident](https://crates.io/crates/unicode-ident/1.0.26) | 1.0.26 | `(MIT OR Apache-2.0) AND Unicode-3.0` | `MIT AND Unicode-3.0` | [LICENSE-MIT](third_party/licenses/unicode-ident-1.0.26/LICENSE-MIT) + [LICENSE-UNICODE](third_party/licenses/unicode-ident-1.0.26/LICENSE-UNICODE) |
| [url](https://crates.io/crates/url/2.5.8) | 2.5.8 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/url-2.5.8/LICENSE-MIT) |
| [utf8_iter](https://crates.io/crates/utf8_iter/1.0.4) | 1.0.4 | `Apache-2.0 OR MIT` | `MIT` | [LICENSE-MIT](third_party/licenses/utf8_iter-1.0.4/LICENSE-MIT) |
| [web_atoms](https://crates.io/crates/web_atoms/0.2.6) | 0.2.6 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/web_atoms-0.2.6/LICENSE-MIT) |
| [windows-link](https://crates.io/crates/windows-link/0.2.1) | 0.2.1 | `MIT OR Apache-2.0` | `MIT` | [license-mit](third_party/licenses/windows-link-0.2.1/license-mit) |
| [writeable](https://crates.io/crates/writeable/0.6.4) | 0.6.4 | `Unicode-3.0` | `Unicode-3.0` | [LICENSE](third_party/licenses/writeable-0.6.4/LICENSE) |
| [yoke](https://crates.io/crates/yoke/0.8.3) | 0.8.3 | `Unicode-3.0` | `Unicode-3.0` | [LICENSE](third_party/licenses/yoke-0.8.3/LICENSE) |
| [yoke-derive](https://crates.io/crates/yoke-derive/0.8.4) | 0.8.4 | `Unicode-3.0` | `Unicode-3.0` | [LICENSE](third_party/licenses/yoke-derive-0.8.4/LICENSE) |
| [zerofrom](https://crates.io/crates/zerofrom/0.1.8) | 0.1.8 | `Unicode-3.0` | `Unicode-3.0` | [LICENSE](third_party/licenses/zerofrom-0.1.8/LICENSE) |
| [zerofrom-derive](https://crates.io/crates/zerofrom-derive/0.1.8) | 0.1.8 | `Unicode-3.0` | `Unicode-3.0` | [LICENSE](third_party/licenses/zerofrom-derive-0.1.8/LICENSE) |
| [zerotrie](https://crates.io/crates/zerotrie/0.2.5) | 0.2.5 | `Unicode-3.0` | `Unicode-3.0` | [LICENSE](third_party/licenses/zerotrie-0.2.5/LICENSE) |
| [zerovec](https://crates.io/crates/zerovec/0.11.8) | 0.11.8 | `Unicode-3.0` | `Unicode-3.0` | [LICENSE](third_party/licenses/zerovec-0.11.8/LICENSE) |
| [zerovec-derive](https://crates.io/crates/zerovec-derive/0.11.6) | 0.11.6 | `Unicode-3.0` | `Unicode-3.0` | [LICENSE](third_party/licenses/zerovec-derive-0.11.6/LICENSE) |
| [zlib-rs](https://crates.io/crates/zlib-rs/0.6.8) | 0.6.8 | `Zlib` | `Zlib` | [LICENSE](third_party/licenses/zlib-rs-0.6.8/LICENSE) |
| [zmij](https://crates.io/crates/zmij/1.0.23) | 1.0.23 | `MIT` | `MIT` | [LICENSE-MIT](third_party/licenses/zmij-1.0.23/LICENSE-MIT) |
| [zune-core](https://crates.io/crates/zune-core/0.5.3) | 0.5.3 | `MIT OR Apache-2.0 OR Zlib` | `MIT` | [LICENSE-MIT](third_party/licenses/zune-core-0.5.3/LICENSE-MIT) |
| [zune-jpeg](https://crates.io/crates/zune-jpeg/0.5.15) | 0.5.15 | `MIT OR Apache-2.0 OR Zlib` | `MIT` | [LICENSE-MIT](third_party/licenses/zune-jpeg-0.5.15/LICENSE-MIT) |

Additional source attribution is preserved in `servo_arc`'s source notice, `siphasher`'s COPYING and `adler2`'s 0BSD notice alongside their unmodified MIT texts. The locked `fdeflate` and Serde packages supply MIT grants without copyright-holder/year lines; their bundled grants are verbatim, with versioned source available above. Do not invent copyright statements absent from the upstream archive.

`chromatic-client` has no external crate dependencies. `chromatic-ffi` depends only on that project crate. Both are project-authored and licensed GPL-3.0-only.

## ModRetro display transport

The QSPI transaction settings and framebuffer addressing in `devices/chromatic/components/pocket_band_display` follow the ModRetro Chromatic MCU transport at [commit 63ed3b62b16768ad2415c9e3b865b438ca40fcc6](https://github.com/ModRetro/oss-chromatic-console-mcu/tree/63ed3b62b16768ad2415c9e3b865b438ca40fcc6), whose [license](https://github.com/ModRetro/oss-chromatic-console-mcu/blob/63ed3b62b16768ad2415c9e3b865b438ca40fcc6/LICENSE) is GNU GPL v3. Pocket Search's bounded band driver replaces the full-frame buffering and transaction handling; its source and modifications are GPL-3.0-only. Preserve applicable upstream notices when importing further code.

No official firmware binary, FPGA image, commercial game, device backup, or third-party font is included. ModRetro and Chromatic identify the target device; the project is independent of ModRetro.

## External build and runtime prerequisites

ESP-IDF 5.3, FreeRTOS, LVGL 8.4.0, the Xtensa C toolchain, esp-rs Rust/LLVM toolchain (installed with espup 0.17.1), Rust core/compiler-builtins used by build-std, and a suitable licensed font are external prerequisites of the experimental MCU integration. Their source/binaries are not bundled in the current packages. A future native application distribution must inventory the exact linked components and assets, include their required notices, and provide corresponding source/build instructions before release. Do not treat this list as a completed firmware distribution review.

Third-party GitHub Actions execute in CI and are not part of the release application. Their upstream source and pinned revisions are recorded in `.github/workflows`; dependency updates keep those pins reviewable.
