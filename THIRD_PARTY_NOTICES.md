# Third-party notices

This inventory describes the dependencies locked in `Cargo.lock` and the upstream basis of the display transport. Project-authored material remains GPL-3.0-only; third-party material retains its own terms. See [project notice](NOTICE.md) and [GPL license](LICENSE).

## Rust dependencies

Dependency source is downloaded by Cargo rather than vendored here. The host companion includes HTTPS acquisition, robots.txt matching, SHA-256 provenance, HTML5 parsing, URL normalization, and PNG/JPEG preparation; procedural macros and platform dependencies are also inventoried. The device crates do not inherit these host dependencies. For packages offering MIT, this distribution selects MIT; Unicode notices remain required where stated. Other packages retain ISC, Apache-2.0, CDLA-Permissive-2.0, Unicode-3.0, Zlib, BSD-3-Clause or MPL-2.0 as listed below. These dependencies are unmodified.

Exact locked package archives (including MPL source) are available from the versioned crates.io links in [the machine-readable inventory](third_party/dependencies.json). `cargo fetch --locked` retrieves those sources using the checksums in `Cargo.lock`. Recipients of the companion can obtain the unmodified MPL-covered source through those versioned archive links. Preserve upstream notices and source availability when distributing binaries. The `selectors` archive omits a standalone license file; its source notice and the canonical Mozilla MPL-2.0 text are included explicitly.

| Package / source | Locked version | Upstream license expression | Selected terms | Included text |
| --- | --- | --- | --- | --- |
| [adler2](https://crates.io/crates/adler2/2.0.1) | 2.0.1 | `0BSD OR MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/adler2-2.0.1/LICENSE-MIT) + [LICENSE-0BSD](third_party/licenses/adler2-2.0.1/LICENSE-0BSD) |
| [atomic-waker](https://crates.io/crates/atomic-waker/1.1.2) | 1.1.2 | `Apache-2.0 OR MIT` | `MIT` | [LICENSE-APACHE](third_party/licenses/atomic-waker-1.1.2/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/atomic-waker-1.1.2/LICENSE-MIT) + [LICENSE-THIRD-PARTY](third_party/licenses/atomic-waker-1.1.2/LICENSE-THIRD-PARTY) |
| [autocfg](https://crates.io/crates/autocfg/1.5.1) | 1.5.1 | `Apache-2.0 OR MIT` | `MIT` | [LICENSE-MIT](third_party/licenses/autocfg-1.5.1/LICENSE-MIT) |
| [base64](https://crates.io/crates/base64/0.22.1) | 0.22.1 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/base64-0.22.1/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/base64-0.22.1/LICENSE-MIT) |
| [base64](https://crates.io/crates/base64/0.23.1) | 0.23.1 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/base64-0.23.1/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/base64-0.23.1/LICENSE-MIT) |
| [bitflags](https://crates.io/crates/bitflags/2.13.2) | 2.13.2 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/bitflags-2.13.2/LICENSE-MIT) |
| [block-buffer](https://crates.io/crates/block-buffer/0.10.4) | 0.10.4 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/block-buffer-0.10.4/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/block-buffer-0.10.4/LICENSE-MIT) |
| [bumpalo](https://crates.io/crates/bumpalo/3.20.3) | 3.20.3 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/bumpalo-3.20.3/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/bumpalo-3.20.3/LICENSE-MIT) |
| [bytemuck](https://crates.io/crates/bytemuck/1.25.2) | 1.25.2 | `Zlib OR Apache-2.0 OR MIT` | `MIT` | [LICENSE-MIT](third_party/licenses/bytemuck-1.25.2/LICENSE-MIT) |
| [byteorder-lite](https://crates.io/crates/byteorder-lite/0.1.0) | 0.1.0 | `Unlicense OR MIT` | `MIT` | [LICENSE-MIT](third_party/licenses/byteorder-lite-0.1.0/LICENSE-MIT) |
| [bytes](https://crates.io/crates/bytes/1.12.1) | 1.12.1 | `MIT` | `MIT` | [LICENSE](third_party/licenses/bytes-1.12.1/LICENSE) |
| [cc](https://crates.io/crates/cc/1.7.0) | 1.7.0 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/cc-1.7.0/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/cc-1.7.0/LICENSE-MIT) |
| [cfg-if](https://crates.io/crates/cfg-if/1.0.5) | 1.0.5 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/cfg-if-1.0.5/LICENSE-MIT) |
| [cfg_aliases](https://crates.io/crates/cfg_aliases/0.2.2) | 0.2.2 | `MIT` | `MIT` | [LICENSE](third_party/licenses/cfg_aliases-0.2.2/LICENSE) + [NOTICES.md](third_party/licenses/cfg_aliases-0.2.2/NOTICES.md) + [COPYRIGHT-TECTONIC.md](third_party/licenses/cfg_aliases-0.2.2/COPYRIGHT-TECTONIC.md) |
| [chacha20](https://crates.io/crates/chacha20/0.10.2) | 0.10.2 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/chacha20-0.10.2/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/chacha20-0.10.2/LICENSE-MIT) |
| [cpufeatures](https://crates.io/crates/cpufeatures/0.2.17) | 0.2.17 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/cpufeatures-0.2.17/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/cpufeatures-0.2.17/LICENSE-MIT) |
| [cpufeatures](https://crates.io/crates/cpufeatures/0.3.1) | 0.3.1 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/cpufeatures-0.3.1/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/cpufeatures-0.3.1/LICENSE-MIT) |
| [crc32fast](https://crates.io/crates/crc32fast/1.5.2) | 1.5.2 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/crc32fast-1.5.2/LICENSE-MIT) |
| [crypto-common](https://crates.io/crates/crypto-common/0.1.7) | 0.1.7 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/crypto-common-0.1.7/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/crypto-common-0.1.7/LICENSE-MIT) |
| [cssparser](https://crates.io/crates/cssparser/0.37.0) | 0.37.0 | `MPL-2.0` | `MPL-2.0` | [LICENSE](third_party/licenses/cssparser-0.37.0/LICENSE) |
| [cssparser-macros](https://crates.io/crates/cssparser-macros/0.7.1) | 0.7.1 | `MPL-2.0` | `MPL-2.0` | [LICENSE](third_party/licenses/cssparser-macros-0.7.1/LICENSE) |
| [derive_more](https://crates.io/crates/derive_more/2.1.1) | 2.1.1 | `MIT` | `MIT` | [LICENSE](third_party/licenses/derive_more-2.1.1/LICENSE) |
| [derive_more-impl](https://crates.io/crates/derive_more-impl/2.1.1) | 2.1.1 | `MIT` | `MIT` | [LICENSE](third_party/licenses/derive_more-impl-2.1.1/LICENSE) |
| [digest](https://crates.io/crates/digest/0.10.7) | 0.10.7 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/digest-0.10.7/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/digest-0.10.7/LICENSE-MIT) |
| [displaydoc](https://crates.io/crates/displaydoc/0.2.7) | 0.2.7 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/displaydoc-0.2.7/LICENSE-MIT) |
| [dtoa](https://crates.io/crates/dtoa/1.0.11) | 1.0.11 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/dtoa-1.0.11/LICENSE-MIT) |
| [dtoa-short](https://crates.io/crates/dtoa-short/0.3.5) | 0.3.5 | `MPL-2.0` | `MPL-2.0` | [LICENSE](third_party/licenses/dtoa-short-0.3.5/LICENSE) |
| [ego-tree](https://crates.io/crates/ego-tree/0.11.0) | 0.11.0 | `ISC` | `ISC` | [LICENSE](third_party/licenses/ego-tree-0.11.0/LICENSE) |
| [fastrand](https://crates.io/crates/fastrand/2.5.0) | 2.5.0 | `Apache-2.0 OR MIT` | `MIT` | [LICENSE-MIT](third_party/licenses/fastrand-2.5.0/LICENSE-MIT) |
| [fdeflate](https://crates.io/crates/fdeflate/0.3.7) | 0.3.7 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/fdeflate-0.3.7/LICENSE-MIT) + [SOURCE-NOTICE.txt](third_party/licenses/fdeflate-0.3.7/SOURCE-NOTICE.txt) |
| [find-msvc-tools](https://crates.io/crates/find-msvc-tools/0.1.14) | 0.1.14 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/find-msvc-tools-0.1.14/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/find-msvc-tools-0.1.14/LICENSE-MIT) |
| [flate2](https://crates.io/crates/flate2/1.1.10) | 1.1.10 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/flate2-1.1.10/LICENSE-MIT) |
| [form_urlencoded](https://crates.io/crates/form_urlencoded/1.2.2) | 1.2.2 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/form_urlencoded-1.2.2/LICENSE-MIT) |
| [futures-channel](https://crates.io/crates/futures-channel/0.3.34) | 0.3.34 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/futures-channel-0.3.34/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/futures-channel-0.3.34/LICENSE-MIT) |
| [futures-core](https://crates.io/crates/futures-core/0.3.34) | 0.3.34 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/futures-core-0.3.34/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/futures-core-0.3.34/LICENSE-MIT) |
| [futures-io](https://crates.io/crates/futures-io/0.3.34) | 0.3.34 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/futures-io-0.3.34/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/futures-io-0.3.34/LICENSE-MIT) |
| [futures-sink](https://crates.io/crates/futures-sink/0.3.34) | 0.3.34 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/futures-sink-0.3.34/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/futures-sink-0.3.34/LICENSE-MIT) |
| [futures-task](https://crates.io/crates/futures-task/0.3.34) | 0.3.34 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/futures-task-0.3.34/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/futures-task-0.3.34/LICENSE-MIT) |
| [futures-util](https://crates.io/crates/futures-util/0.3.34) | 0.3.34 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/futures-util-0.3.34/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/futures-util-0.3.34/LICENSE-MIT) |
| [generic-array](https://crates.io/crates/generic-array/0.14.7) | 0.14.7 | `MIT` | `MIT` | [LICENSE](third_party/licenses/generic-array-0.14.7/LICENSE) |
| [getrandom](https://crates.io/crates/getrandom/0.2.17) | 0.2.17 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/getrandom-0.2.17/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/getrandom-0.2.17/LICENSE-MIT) |
| [getrandom](https://crates.io/crates/getrandom/0.4.3) | 0.4.3 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/getrandom-0.4.3/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/getrandom-0.4.3/LICENSE-MIT) |
| [html5ever](https://crates.io/crates/html5ever/0.39.0) | 0.39.0 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/html5ever-0.39.0/LICENSE-MIT) |
| [http](https://crates.io/crates/http/1.5.0) | 1.5.0 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/http-1.5.0/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/http-1.5.0/LICENSE-MIT) |
| [http-body](https://crates.io/crates/http-body/1.1.0) | 1.1.0 | `MIT` | `MIT` | [LICENSE](third_party/licenses/http-body-1.1.0/LICENSE) |
| [http-body-util](https://crates.io/crates/http-body-util/0.1.5) | 0.1.5 | `MIT` | `MIT` | [LICENSE](third_party/licenses/http-body-util-0.1.5/LICENSE) |
| [httparse](https://crates.io/crates/httparse/1.10.1) | 1.10.1 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/httparse-1.10.1/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/httparse-1.10.1/LICENSE-MIT) |
| [hyper](https://crates.io/crates/hyper/1.12.0) | 1.12.0 | `MIT` | `MIT` | [LICENSE](third_party/licenses/hyper-1.12.0/LICENSE) |
| [hyper-rustls](https://crates.io/crates/hyper-rustls/0.27.10) | 0.27.10 | `Apache-2.0 OR ISC OR MIT` | `MIT` | [LICENSE-APACHE](third_party/licenses/hyper-rustls-0.27.10/LICENSE-APACHE) + [LICENSE-ISC](third_party/licenses/hyper-rustls-0.27.10/LICENSE-ISC) + [LICENSE-MIT](third_party/licenses/hyper-rustls-0.27.10/LICENSE-MIT) |
| [hyper-util](https://crates.io/crates/hyper-util/0.1.21) | 0.1.21 | `MIT` | `MIT` | [LICENSE](third_party/licenses/hyper-util-0.1.21/LICENSE) |
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
| [ipnet](https://crates.io/crates/ipnet/2.12.2) | 2.12.2 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/ipnet-2.12.2/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/ipnet-2.12.2/LICENSE-MIT) |
| [itoa](https://crates.io/crates/itoa/1.0.18) | 1.0.18 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/itoa-1.0.18/LICENSE-MIT) |
| [js-sys](https://crates.io/crates/js-sys/0.3.106) | 0.3.106 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/js-sys-0.3.106/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/js-sys-0.3.106/LICENSE-MIT) |
| [libc](https://crates.io/crates/libc/0.2.190) | 0.2.190 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/libc-0.2.190/LICENSE-MIT) |
| [litemap](https://crates.io/crates/litemap/0.8.3) | 0.8.3 | `Unicode-3.0` | `Unicode-3.0` | [LICENSE](third_party/licenses/litemap-0.8.3/LICENSE) |
| [lock_api](https://crates.io/crates/lock_api/0.4.14) | 0.4.14 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/lock_api-0.4.14/LICENSE-MIT) |
| [log](https://crates.io/crates/log/0.4.34) | 0.4.34 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/log-0.4.34/LICENSE-MIT) |
| [lru-slab](https://crates.io/crates/lru-slab/0.1.3) | 0.1.3 | `MIT OR Apache-2.0 OR Zlib` | `MIT` | [LICENSE-APACHE](third_party/licenses/lru-slab-0.1.3/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/lru-slab-0.1.3/LICENSE-MIT) + [LICENSE-ZLIB](third_party/licenses/lru-slab-0.1.3/LICENSE-ZLIB) |
| [markup5ever](https://crates.io/crates/markup5ever/0.39.0) | 0.39.0 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/markup5ever-0.39.0/LICENSE-MIT) |
| [memchr](https://crates.io/crates/memchr/2.8.3) | 2.8.3 | `Unlicense OR MIT` | `MIT` | [LICENSE-MIT](third_party/licenses/memchr-2.8.3/LICENSE-MIT) |
| [miniz_oxide](https://crates.io/crates/miniz_oxide/0.8.9) | 0.8.9 | `MIT OR Zlib OR Apache-2.0` | `MIT` | [LICENSE-MIT.md](third_party/licenses/miniz_oxide-0.8.9/LICENSE-MIT.md) |
| [miniz_oxide](https://crates.io/crates/miniz_oxide/0.9.1) | 0.9.1 | `MIT OR Zlib OR Apache-2.0` | `MIT` | [LICENSE-MIT.md](third_party/licenses/miniz_oxide-0.9.1/LICENSE-MIT.md) |
| [mio](https://crates.io/crates/mio/1.2.4) | 1.2.4 | `MIT` | `MIT` | [LICENSE](third_party/licenses/mio-1.2.4/LICENSE) |
| [moxcms](https://crates.io/crates/moxcms/0.8.1) | 0.8.1 | `BSD-3-Clause OR Apache-2.0` | `BSD-3-Clause` | [LICENSE.md](third_party/licenses/moxcms-0.8.1/LICENSE.md) |
| [new_debug_unreachable](https://crates.io/crates/new_debug_unreachable/1.0.6) | 1.0.6 | `MIT` | `MIT` | [LICENSE-MIT](third_party/licenses/new_debug_unreachable-1.0.6/LICENSE-MIT) |
| [num-traits](https://crates.io/crates/num-traits/0.2.19) | 0.2.19 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/num-traits-0.2.19/LICENSE-MIT) |
| [once_cell](https://crates.io/crates/once_cell/1.21.4) | 1.21.4 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/once_cell-1.21.4/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/once_cell-1.21.4/LICENSE-MIT) |
| [parking_lot](https://crates.io/crates/parking_lot/0.12.5) | 0.12.5 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/parking_lot-0.12.5/LICENSE-MIT) |
| [parking_lot_core](https://crates.io/crates/parking_lot_core/0.9.12) | 0.9.12 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/parking_lot_core-0.9.12/LICENSE-MIT) |
| [percent-encoding](https://crates.io/crates/percent-encoding/2.3.2) | 2.3.2 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/percent-encoding-2.3.2/LICENSE-MIT) |
| [phf](https://crates.io/crates/phf/0.13.1) | 0.13.1 | `MIT` | `MIT` | [LICENSE](third_party/licenses/phf-0.13.1/LICENSE) |
| [phf_codegen](https://crates.io/crates/phf_codegen/0.13.1) | 0.13.1 | `MIT` | `MIT` | [LICENSE](third_party/licenses/phf_codegen-0.13.1/LICENSE) |
| [phf_generator](https://crates.io/crates/phf_generator/0.13.1) | 0.13.1 | `MIT` | `MIT` | [LICENSE](third_party/licenses/phf_generator-0.13.1/LICENSE) |
| [phf_macros](https://crates.io/crates/phf_macros/0.13.1) | 0.13.1 | `MIT` | `MIT` | [LICENSE](third_party/licenses/phf_macros-0.13.1/LICENSE) |
| [phf_shared](https://crates.io/crates/phf_shared/0.13.1) | 0.13.1 | `MIT` | `MIT` | [LICENSE](third_party/licenses/phf_shared-0.13.1/LICENSE) |
| [pin-project-lite](https://crates.io/crates/pin-project-lite/0.2.17) | 0.2.17 | `Apache-2.0 OR MIT` | `MIT` | [LICENSE-APACHE](third_party/licenses/pin-project-lite-0.2.17/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/pin-project-lite-0.2.17/LICENSE-MIT) |
| [png](https://crates.io/crates/png/0.18.1) | 0.18.1 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/png-0.18.1/LICENSE-MIT) |
| [potential_utf](https://crates.io/crates/potential_utf/0.1.6) | 0.1.6 | `Unicode-3.0` | `Unicode-3.0` | [LICENSE](third_party/licenses/potential_utf-0.1.6/LICENSE) |
| [precomputed-hash](https://crates.io/crates/precomputed-hash/0.1.1) | 0.1.1 | `MIT` | `MIT` | [LICENSE](third_party/licenses/precomputed-hash-0.1.1/LICENSE) |
| [proc-macro2](https://crates.io/crates/proc-macro2/1.0.107) | 1.0.107 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/proc-macro2-1.0.107/LICENSE-MIT) |
| [pxfm](https://crates.io/crates/pxfm/0.1.30) | 0.1.30 | `BSD-3-Clause OR Apache-2.0` | `BSD-3-Clause` | [LICENSE.md](third_party/licenses/pxfm-0.1.30/LICENSE.md) |
| [quinn](https://crates.io/crates/quinn/0.11.12) | 0.11.12 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/quinn-0.11.12/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/quinn-0.11.12/LICENSE-MIT) |
| [quinn-proto](https://crates.io/crates/quinn-proto/0.11.19) | 0.11.19 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/quinn-proto-0.11.19/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/quinn-proto-0.11.19/LICENSE-MIT) |
| [quinn-udp](https://crates.io/crates/quinn-udp/0.5.16) | 0.5.16 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/quinn-udp-0.5.16/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/quinn-udp-0.5.16/LICENSE-MIT) |
| [quote](https://crates.io/crates/quote/1.0.47) | 1.0.47 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/quote-1.0.47/LICENSE-MIT) |
| [r-efi](https://crates.io/crates/r-efi/6.0.0) | 6.0.0 | `MIT OR Apache-2.0 OR LGPL-2.1-or-later` | `MIT` | [AUTHORS](third_party/licenses/r-efi-6.0.0/AUTHORS) |
| [rand](https://crates.io/crates/rand/0.10.3) | 0.10.3 | `MIT OR Apache-2.0` | `MIT` | [COPYRIGHT](third_party/licenses/rand-0.10.3/COPYRIGHT) + [LICENSE-APACHE](third_party/licenses/rand-0.10.3/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/rand-0.10.3/LICENSE-MIT) |
| [rand_core](https://crates.io/crates/rand_core/0.10.1) | 0.10.1 | `MIT OR Apache-2.0` | `MIT` | [COPYRIGHT](third_party/licenses/rand_core-0.10.1/COPYRIGHT) + [LICENSE-APACHE](third_party/licenses/rand_core-0.10.1/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/rand_core-0.10.1/LICENSE-MIT) |
| [rand_pcg](https://crates.io/crates/rand_pcg/0.10.2) | 0.10.2 | `MIT OR Apache-2.0` | `MIT` | [COPYRIGHT](third_party/licenses/rand_pcg-0.10.2/COPYRIGHT) + [LICENSE-APACHE](third_party/licenses/rand_pcg-0.10.2/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/rand_pcg-0.10.2/LICENSE-MIT) |
| [redox_syscall](https://crates.io/crates/redox_syscall/0.5.18) | 0.5.18 | `MIT` | `MIT` | [LICENSE](third_party/licenses/redox_syscall-0.5.18/LICENSE) |
| [reqwest](https://crates.io/crates/reqwest/0.12.28) | 0.12.28 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/reqwest-0.12.28/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/reqwest-0.12.28/LICENSE-MIT) |
| [ring](https://crates.io/crates/ring/0.17.14) | 0.17.14 | `Apache-2.0 AND ISC` | `Apache-2.0 AND ISC` | [LICENSE](third_party/licenses/ring-0.17.14/LICENSE) + [LICENSE-BoringSSL](third_party/licenses/ring-0.17.14/LICENSE-BoringSSL) + [LICENSE-other-bits](third_party/licenses/ring-0.17.14/LICENSE-other-bits) + [LICENSE-APACHE](third_party/licenses/ring-0.17.14/src/polyfill/once_cell/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/ring-0.17.14/src/polyfill/once_cell/LICENSE-MIT) + [LICENSE](third_party/licenses/ring-0.17.14/third_party/fiat/LICENSE) |
| [robotstxt](https://crates.io/crates/robotstxt/0.3.0) | 0.3.0 | `Apache-2.0` | `Apache-2.0` | [LICENSE](third_party/licenses/robotstxt-0.3.0/LICENSE) |
| [rustc-hash](https://crates.io/crates/rustc-hash/2.1.3) | 2.1.3 | `Apache-2.0 OR MIT` | `MIT` | [LICENSE-MIT](third_party/licenses/rustc-hash-2.1.3/LICENSE-MIT) |
| [rustc_version](https://crates.io/crates/rustc_version/0.4.1) | 0.4.1 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/rustc_version-0.4.1/LICENSE-MIT) |
| [rustls](https://crates.io/crates/rustls/0.23.45) | 0.23.45 | `Apache-2.0 OR ISC OR MIT` | `MIT` | [LICENSE-APACHE](third_party/licenses/rustls-0.23.45/LICENSE-APACHE) + [LICENSE-ISC](third_party/licenses/rustls-0.23.45/LICENSE-ISC) + [LICENSE-MIT](third_party/licenses/rustls-0.23.45/LICENSE-MIT) |
| [rustls-pki-types](https://crates.io/crates/rustls-pki-types/1.15.1) | 1.15.1 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/rustls-pki-types-1.15.1/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/rustls-pki-types-1.15.1/LICENSE-MIT) |
| [rustls-webpki](https://crates.io/crates/rustls-webpki/0.103.15) | 0.103.15 | `ISC` | `ISC` | [LICENSE](third_party/licenses/rustls-webpki-0.103.15/LICENSE) |
| [rustversion](https://crates.io/crates/rustversion/1.0.23) | 1.0.23 | `MIT OR Apache-2.0` | `Apache-2.0` | [LICENSE-APACHE](third_party/licenses/rustversion-1.0.23/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/rustversion-1.0.23/LICENSE-MIT) + [ATTRIBUTION.md](third_party/licenses/rustversion-1.0.23/ATTRIBUTION.md) |
| [ryu](https://crates.io/crates/ryu/1.0.23) | 1.0.23 | `Apache-2.0 OR BSL-1.0` | `Apache-2.0` | [LICENSE-APACHE](third_party/licenses/ryu-1.0.23/LICENSE-APACHE) + [LICENSE-BOOST](third_party/licenses/ryu-1.0.23/LICENSE-BOOST) |
| [scopeguard](https://crates.io/crates/scopeguard/1.2.0) | 1.2.0 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/scopeguard-1.2.0/LICENSE-MIT) |
| [scraper](https://crates.io/crates/scraper/0.27.0) | 0.27.0 | `ISC` | `ISC` | [LICENSE](third_party/licenses/scraper-0.27.0/LICENSE) |
| [selectors](https://crates.io/crates/selectors/0.38.0) | 0.38.0 | `MPL-2.0` | `MPL-2.0` | [LICENSE-MPL-2.0](third_party/licenses/selectors-0.38.0/LICENSE-MPL-2.0) + [SOURCE-NOTICE.txt](third_party/licenses/selectors-0.38.0/SOURCE-NOTICE.txt) |
| [semver](https://crates.io/crates/semver/1.0.28) | 1.0.28 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/semver-1.0.28/LICENSE-MIT) |
| [serde](https://crates.io/crates/serde/1.0.229) | 1.0.229 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/serde-1.0.229/LICENSE-MIT) |
| [serde_core](https://crates.io/crates/serde_core/1.0.229) | 1.0.229 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/serde_core-1.0.229/LICENSE-MIT) |
| [serde_derive](https://crates.io/crates/serde_derive/1.0.229) | 1.0.229 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/serde_derive-1.0.229/LICENSE-MIT) |
| [serde_json](https://crates.io/crates/serde_json/1.0.151) | 1.0.151 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/serde_json-1.0.151/LICENSE-MIT) |
| [serde_urlencoded](https://crates.io/crates/serde_urlencoded/0.7.1) | 0.7.1 | `MIT/Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/serde_urlencoded-0.7.1/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/serde_urlencoded-0.7.1/LICENSE-MIT) |
| [servo_arc](https://crates.io/crates/servo_arc/0.4.3) | 0.4.3 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/servo_arc-0.4.3/LICENSE-MIT) + [SOURCE-NOTICE.txt](third_party/licenses/servo_arc-0.4.3/SOURCE-NOTICE.txt) |
| [sha2](https://crates.io/crates/sha2/0.10.9) | 0.10.9 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/sha2-0.10.9/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/sha2-0.10.9/LICENSE-MIT) |
| [shlex](https://crates.io/crates/shlex/2.0.1) | 2.0.1 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/shlex-2.0.1/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/shlex-2.0.1/LICENSE-MIT) |
| [simd-adler32](https://crates.io/crates/simd-adler32/0.3.10) | 0.3.10 | `MIT` | `MIT` | [LICENSE.md](third_party/licenses/simd-adler32-0.3.10/LICENSE.md) |
| [siphasher](https://crates.io/crates/siphasher/1.0.4) | 1.0.4 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/siphasher-1.0.4/LICENSE-MIT) + [COPYING](third_party/licenses/siphasher-1.0.4/COPYING) |
| [slab](https://crates.io/crates/slab/0.4.12) | 0.4.12 | `MIT` | `MIT` | [LICENSE](third_party/licenses/slab-0.4.12/LICENSE) |
| [smallvec](https://crates.io/crates/smallvec/1.16.3) | 1.16.3 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/smallvec-1.16.3/LICENSE-MIT) |
| [socket2](https://crates.io/crates/socket2/0.6.5) | 0.6.5 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/socket2-0.6.5/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/socket2-0.6.5/LICENSE-MIT) |
| [stable_deref_trait](https://crates.io/crates/stable_deref_trait/1.2.1) | 1.2.1 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/stable_deref_trait-1.2.1/LICENSE-MIT) |
| [string_cache](https://crates.io/crates/string_cache/0.9.0) | 0.9.0 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/string_cache-0.9.0/LICENSE-MIT) |
| [string_cache_codegen](https://crates.io/crates/string_cache_codegen/0.6.1) | 0.6.1 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/string_cache_codegen-0.6.1/LICENSE-MIT) |
| [subtle](https://crates.io/crates/subtle/2.6.1) | 2.6.1 | `BSD-3-Clause` | `BSD-3-Clause` | [LICENSE](third_party/licenses/subtle-2.6.1/LICENSE) |
| [syn](https://crates.io/crates/syn/2.0.119) | 2.0.119 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/syn-2.0.119/LICENSE-MIT) |
| [syn](https://crates.io/crates/syn/3.0.6) | 3.0.6 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/syn-3.0.6/LICENSE-MIT) |
| [sync_wrapper](https://crates.io/crates/sync_wrapper/1.0.2) | 1.0.2 | `Apache-2.0` | `Apache-2.0` | [LICENSE](third_party/licenses/sync_wrapper-1.0.2/LICENSE) |
| [synstructure](https://crates.io/crates/synstructure/0.14.0) | 0.14.0 | `MIT` | `MIT` | [LICENSE](third_party/licenses/synstructure-0.14.0/LICENSE) |
| [tendril](https://crates.io/crates/tendril/0.5.1) | 0.5.1 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/tendril-0.5.1/LICENSE-MIT) |
| [thiserror](https://crates.io/crates/thiserror/2.0.21) | 2.0.21 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/thiserror-2.0.21/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/thiserror-2.0.21/LICENSE-MIT) |
| [thiserror-impl](https://crates.io/crates/thiserror-impl/2.0.21) | 2.0.21 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/thiserror-impl-2.0.21/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/thiserror-impl-2.0.21/LICENSE-MIT) |
| [tinystr](https://crates.io/crates/tinystr/0.8.4) | 0.8.4 | `Unicode-3.0` | `Unicode-3.0` | [LICENSE](third_party/licenses/tinystr-0.8.4/LICENSE) |
| [tinyvec](https://crates.io/crates/tinyvec/1.13.3) | 1.13.3 | `Zlib OR Apache-2.0 OR MIT` | `MIT` | [LICENSE-APACHE.md](third_party/licenses/tinyvec-1.13.3/LICENSE-APACHE.md) + [LICENSE-MIT.md](third_party/licenses/tinyvec-1.13.3/LICENSE-MIT.md) + [LICENSE-ZLIB.md](third_party/licenses/tinyvec-1.13.3/LICENSE-ZLIB.md) + [COPYRIGHT.md](third_party/licenses/tinyvec-1.13.3/COPYRIGHT.md) |
| [tokio](https://crates.io/crates/tokio/1.53.2) | 1.53.2 | `MIT` | `MIT` | [LICENSE](third_party/licenses/tokio-1.53.2/LICENSE) |
| [tokio-rustls](https://crates.io/crates/tokio-rustls/0.26.6) | 0.26.6 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/tokio-rustls-0.26.6/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/tokio-rustls-0.26.6/LICENSE-MIT) |
| [tower](https://crates.io/crates/tower/0.5.3) | 0.5.3 | `MIT` | `MIT` | [LICENSE](third_party/licenses/tower-0.5.3/LICENSE) |
| [tower-http](https://crates.io/crates/tower-http/0.6.11) | 0.6.11 | `MIT` | `MIT` | [LICENSE](third_party/licenses/tower-http-0.6.11/LICENSE) |
| [tower-layer](https://crates.io/crates/tower-layer/0.3.3) | 0.3.3 | `MIT` | `MIT` | [LICENSE](third_party/licenses/tower-layer-0.3.3/LICENSE) |
| [tower-service](https://crates.io/crates/tower-service/0.3.3) | 0.3.3 | `MIT` | `MIT` | [LICENSE](third_party/licenses/tower-service-0.3.3/LICENSE) |
| [tracing](https://crates.io/crates/tracing/0.1.44) | 0.1.44 | `MIT` | `MIT` | [LICENSE](third_party/licenses/tracing-0.1.44/LICENSE) |
| [tracing-core](https://crates.io/crates/tracing-core/0.1.36) | 0.1.36 | `MIT` | `MIT` | [LICENSE](third_party/licenses/tracing-core-0.1.36/LICENSE) + [LICENSE](third_party/licenses/tracing-core-0.1.36/src/spin/LICENSE) |
| [try-lock](https://crates.io/crates/try-lock/0.2.5) | 0.2.5 | `MIT` | `MIT` | [LICENSE](third_party/licenses/try-lock-0.2.5/LICENSE) |
| [typenum](https://crates.io/crates/typenum/1.20.1) | 1.20.1 | `MIT OR Apache-2.0` | `MIT` | [LICENSE](third_party/licenses/typenum-1.20.1/LICENSE) + [LICENSE-APACHE](third_party/licenses/typenum-1.20.1/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/typenum-1.20.1/LICENSE-MIT) |
| [unicode-ident](https://crates.io/crates/unicode-ident/1.0.26) | 1.0.26 | `(MIT OR Apache-2.0) AND Unicode-3.0` | `MIT AND Unicode-3.0` | [LICENSE-MIT](third_party/licenses/unicode-ident-1.0.26/LICENSE-MIT) + [LICENSE-UNICODE](third_party/licenses/unicode-ident-1.0.26/LICENSE-UNICODE) |
| [untrusted](https://crates.io/crates/untrusted/0.9.0) | 0.9.0 | `ISC` | `ISC` | [LICENSE.txt](third_party/licenses/untrusted-0.9.0/LICENSE.txt) |
| [url](https://crates.io/crates/url/2.5.8) | 2.5.8 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/url-2.5.8/LICENSE-MIT) |
| [utf8_iter](https://crates.io/crates/utf8_iter/1.0.4) | 1.0.4 | `Apache-2.0 OR MIT` | `MIT` | [LICENSE-MIT](third_party/licenses/utf8_iter-1.0.4/LICENSE-MIT) |
| [version_check](https://crates.io/crates/version_check/0.9.5) | 0.9.5 | `MIT/Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/version_check-0.9.5/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/version_check-0.9.5/LICENSE-MIT) |
| [want](https://crates.io/crates/want/0.3.2) | 0.3.2 | `MIT` | `MIT` | [LICENSE](third_party/licenses/want-0.3.2/LICENSE) |
| [wasi](https://crates.io/crates/wasi/0.11.1+wasi-snapshot-preview1) | 0.11.1+wasi-snapshot-preview1 | `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | `MIT` | [LICENSE-APACHE](third_party/licenses/wasi-0.11.1+wasi-snapshot-preview1/LICENSE-APACHE) + [LICENSE-Apache-2.0_WITH_LLVM-exception](third_party/licenses/wasi-0.11.1+wasi-snapshot-preview1/LICENSE-Apache-2.0_WITH_LLVM-exception) + [LICENSE-MIT](third_party/licenses/wasi-0.11.1+wasi-snapshot-preview1/LICENSE-MIT) |
| [wasm-bindgen](https://crates.io/crates/wasm-bindgen/0.2.129) | 0.2.129 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/wasm-bindgen-0.2.129/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/wasm-bindgen-0.2.129/LICENSE-MIT) |
| [wasm-bindgen-futures](https://crates.io/crates/wasm-bindgen-futures/0.4.79) | 0.4.79 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/wasm-bindgen-futures-0.4.79/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/wasm-bindgen-futures-0.4.79/LICENSE-MIT) |
| [wasm-bindgen-macro](https://crates.io/crates/wasm-bindgen-macro/0.2.129) | 0.2.129 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/wasm-bindgen-macro-0.2.129/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/wasm-bindgen-macro-0.2.129/LICENSE-MIT) |
| [wasm-bindgen-macro-support](https://crates.io/crates/wasm-bindgen-macro-support/0.2.129) | 0.2.129 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/wasm-bindgen-macro-support-0.2.129/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/wasm-bindgen-macro-support-0.2.129/LICENSE-MIT) |
| [wasm-bindgen-shared](https://crates.io/crates/wasm-bindgen-shared/0.2.129) | 0.2.129 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/wasm-bindgen-shared-0.2.129/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/wasm-bindgen-shared-0.2.129/LICENSE-MIT) |
| [web-sys](https://crates.io/crates/web-sys/0.3.106) | 0.3.106 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/web-sys-0.3.106/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/web-sys-0.3.106/LICENSE-MIT) |
| [web-time](https://crates.io/crates/web-time/1.1.0) | 1.1.0 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-APACHE](third_party/licenses/web-time-1.1.0/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/web-time-1.1.0/LICENSE-MIT) |
| [web_atoms](https://crates.io/crates/web_atoms/0.2.6) | 0.2.6 | `MIT OR Apache-2.0` | `MIT` | [LICENSE-MIT](third_party/licenses/web_atoms-0.2.6/LICENSE-MIT) |
| [webpki-roots](https://crates.io/crates/webpki-roots/1.0.9) | 1.0.9 | `CDLA-Permissive-2.0` | `CDLA-Permissive-2.0` | [LICENSE](third_party/licenses/webpki-roots-1.0.9/LICENSE) |
| [windows-link](https://crates.io/crates/windows-link/0.2.1) | 0.2.1 | `MIT OR Apache-2.0` | `MIT` | [license-mit](third_party/licenses/windows-link-0.2.1/license-mit) |
| [windows-sys](https://crates.io/crates/windows-sys/0.52.0) | 0.52.0 | `MIT OR Apache-2.0` | `MIT` | [license-apache-2.0](third_party/licenses/windows-sys-0.52.0/license-apache-2.0) + [license-mit](third_party/licenses/windows-sys-0.52.0/license-mit) |
| [windows-sys](https://crates.io/crates/windows-sys/0.61.2) | 0.61.2 | `MIT OR Apache-2.0` | `MIT` | [license-apache-2.0](third_party/licenses/windows-sys-0.61.2/license-apache-2.0) + [license-mit](third_party/licenses/windows-sys-0.61.2/license-mit) |
| [windows-targets](https://crates.io/crates/windows-targets/0.52.6) | 0.52.6 | `MIT OR Apache-2.0` | `MIT` | [license-apache-2.0](third_party/licenses/windows-targets-0.52.6/license-apache-2.0) + [license-mit](third_party/licenses/windows-targets-0.52.6/license-mit) |
| [windows_aarch64_gnullvm](https://crates.io/crates/windows_aarch64_gnullvm/0.52.6) | 0.52.6 | `MIT OR Apache-2.0` | `MIT` | [license-apache-2.0](third_party/licenses/windows_aarch64_gnullvm-0.52.6/license-apache-2.0) + [license-mit](third_party/licenses/windows_aarch64_gnullvm-0.52.6/license-mit) |
| [windows_aarch64_msvc](https://crates.io/crates/windows_aarch64_msvc/0.52.6) | 0.52.6 | `MIT OR Apache-2.0` | `MIT` | [license-apache-2.0](third_party/licenses/windows_aarch64_msvc-0.52.6/license-apache-2.0) + [license-mit](third_party/licenses/windows_aarch64_msvc-0.52.6/license-mit) |
| [windows_i686_gnu](https://crates.io/crates/windows_i686_gnu/0.52.6) | 0.52.6 | `MIT OR Apache-2.0` | `MIT` | [license-apache-2.0](third_party/licenses/windows_i686_gnu-0.52.6/license-apache-2.0) + [license-mit](third_party/licenses/windows_i686_gnu-0.52.6/license-mit) |
| [windows_i686_gnullvm](https://crates.io/crates/windows_i686_gnullvm/0.52.6) | 0.52.6 | `MIT OR Apache-2.0` | `MIT` | [license-apache-2.0](third_party/licenses/windows_i686_gnullvm-0.52.6/license-apache-2.0) + [license-mit](third_party/licenses/windows_i686_gnullvm-0.52.6/license-mit) |
| [windows_i686_msvc](https://crates.io/crates/windows_i686_msvc/0.52.6) | 0.52.6 | `MIT OR Apache-2.0` | `MIT` | [license-apache-2.0](third_party/licenses/windows_i686_msvc-0.52.6/license-apache-2.0) + [license-mit](third_party/licenses/windows_i686_msvc-0.52.6/license-mit) |
| [windows_x86_64_gnu](https://crates.io/crates/windows_x86_64_gnu/0.52.6) | 0.52.6 | `MIT OR Apache-2.0` | `MIT` | [license-apache-2.0](third_party/licenses/windows_x86_64_gnu-0.52.6/license-apache-2.0) + [license-mit](third_party/licenses/windows_x86_64_gnu-0.52.6/license-mit) |
| [windows_x86_64_gnullvm](https://crates.io/crates/windows_x86_64_gnullvm/0.52.6) | 0.52.6 | `MIT OR Apache-2.0` | `MIT` | [license-apache-2.0](third_party/licenses/windows_x86_64_gnullvm-0.52.6/license-apache-2.0) + [license-mit](third_party/licenses/windows_x86_64_gnullvm-0.52.6/license-mit) |
| [windows_x86_64_msvc](https://crates.io/crates/windows_x86_64_msvc/0.52.6) | 0.52.6 | `MIT OR Apache-2.0` | `MIT` | [license-apache-2.0](third_party/licenses/windows_x86_64_msvc-0.52.6/license-apache-2.0) + [license-mit](third_party/licenses/windows_x86_64_msvc-0.52.6/license-mit) |
| [writeable](https://crates.io/crates/writeable/0.6.4) | 0.6.4 | `Unicode-3.0` | `Unicode-3.0` | [LICENSE](third_party/licenses/writeable-0.6.4/LICENSE) |
| [yoke](https://crates.io/crates/yoke/0.8.3) | 0.8.3 | `Unicode-3.0` | `Unicode-3.0` | [LICENSE](third_party/licenses/yoke-0.8.3/LICENSE) |
| [yoke-derive](https://crates.io/crates/yoke-derive/0.8.4) | 0.8.4 | `Unicode-3.0` | `Unicode-3.0` | [LICENSE](third_party/licenses/yoke-derive-0.8.4/LICENSE) |
| [zerofrom](https://crates.io/crates/zerofrom/0.1.8) | 0.1.8 | `Unicode-3.0` | `Unicode-3.0` | [LICENSE](third_party/licenses/zerofrom-0.1.8/LICENSE) |
| [zerofrom-derive](https://crates.io/crates/zerofrom-derive/0.1.8) | 0.1.8 | `Unicode-3.0` | `Unicode-3.0` | [LICENSE](third_party/licenses/zerofrom-derive-0.1.8/LICENSE) |
| [zeroize](https://crates.io/crates/zeroize/1.9.1) | 1.9.1 | `Apache-2.0 OR MIT` | `MIT` | [LICENSE-APACHE](third_party/licenses/zeroize-1.9.1/LICENSE-APACHE) + [LICENSE-MIT](third_party/licenses/zeroize-1.9.1/LICENSE-MIT) |
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

No official firmware binary, FPGA image, commercial game or device backup is included. The native MCU build overlay includes a compiled Pocket JP font bitmap under OFL-1.1, as described below. ModRetro and Chromatic identify the target device; the project is independent of ModRetro.

## External build and runtime prerequisites

The optional native click integration follows the PCM wiring and project/version fields in [ModRetro's FPGA source at commit a01aa375147943cbcd4a41f41585220d55e0c28a](https://github.com/ModRetro/oss-chromatic-console-fpga/tree/a01aa375147943cbcd4a41f41585220d55e0c28a), under its [GNU GPL v3 license](third_party/licenses/modretro-fpga/LICENSE). The repository-authored click module and staging modifications are GPL-3.0-only. The preparer retains upstream and submodule license files in its separate local source tree. The complete official FPGA project, vendor IP and MiSTer source/binaries are external prerequisites, not bundled release assets. A distributable FPGA image would need its own complete source, licensing and build review.

Icarus Verilog is an external simulation tool. The pinned upstream design specifies Gowin FPGA Designer 1.9.12.03; the complete design was built here with Gowin FPGA Designer 1.9.12.04. An appropriate vendor license is required. Neither the tools nor vendor license credentials are bundled. See [native click build and validation boundaries](docs/native-click.md).

ESP-IDF 5.3, FreeRTOS, LVGL 8.4.0, the Xtensa C toolchain, esp-rs Rust/LLVM toolchain (installed with espup 0.17.1), Rust core/compiler-builtins used by build-std, are external prerequisites of the experimental MCU integration. Their source/binaries are not bundled in the current packages. A future native application distribution must inventory the exact linked components and assets, include their required notices, and provide corresponding source/build instructions before release. Do not treat this list as a completed firmware distribution review.

Third-party GitHub Actions execute in CI and are not part of the release application. Their upstream source and pinned revisions are recorded in `.github/workflows`; dependency updates keep those pins reviewable.

The HTTPS stack remains host-only. Ring requires Apache-2.0 **and** ISC and includes its BoringSSL/other-bits and nested notices verbatim. WebPKI root data retains CDLA-Permissive-2.0. `robotstxt` is an unmodified Rust port of Google's robots.txt matcher, used only for crawling rules; it does not provide Google search results. Additional upstream grants/notices are retained even when an alternative MIT grant is selected.

Supplemental attribution files preserve the Tectonic source-header copyright used by cfg_aliases and tinyvec's copyright from its bundled Zlib grant. Rustversion's upstream MIT file contains no explicit copyright line; its original bytes are preserved, author metadata is retained, and this distribution selects the declared Apache-2.0 alternative.

## Runtime web-font diagnostic

The bounded CSS parser and static TrueType subset rasterizer are original project code under GPL-3.0-only; no font parsing library or font binary is vendored or linked. The optional native diagnostic directly requests a VT323 subset from Google Fonts. VT323 is copyright 2011 The VT323 Project Authors and remains [SIL Open Font License 1.1](https://github.com/google/fonts/blob/main/ofl/vt323/OFL.txt), not GPL. Its runtime-generated glyph atlas is volatile and is not bundled in the firmware or repository. The repository's mathematical font fixtures are original outlines. Any future distribution of downloaded font files or derived font data must retain the applicable copyright and OFL text; do not relabel those assets as project GPL code.

## Reproducible native MCU overlay

`devices/chromatic/firmware/overlay` retains the MCU integration based on ModRetro commit `63ed3b62b16768ad2415c9e3b865b438ca40fcc6` under its GPL v3 license. Repository modifications are GPL-3.0-only; upstream grants remain intact. The builder obtains complete pinned public MCU, ESP-IDF and LVGL sources, including their licenses. It copies tracked source only. No complete vendor firmware, private configuration, NVS or backup is imported.

The compiled `components/fonts/pocket_jp.c` bitmap is a modified Noto Sans JP font named **Pocket JP**, copyright 2014-2021 Adobe, Reserved Font Name 'Source'. It remains **OFL-1.1**, with the original notice in `components/fonts/POCKET-FONT-LICENSE.txt`. The generator renders 12-pixel glyphs at weight 500 to a 1-bit bitmap. The original font input SHA-256 is in `devices/chromatic/firmware/inputs.json`; derived bitmap bytes are included as compilable source. These font assets are not GPL code. Runtime downloads of other web fonts retain their separate licenses.

See [complete native MCU build](docs/native-firmware-build.md) for compiler/dependency pins, supported hosts and application-only build/deployment boundaries. A native build pass does not replace physical E2E or complete distribution qualification.

## ESP32 ROM JPEG path

The native baseline JPEG adapter calls the existing ESP32 ROM TinyJPEG API declared in pinned ESP-IDF `esp32/rom/tjpgd.h`. That header attributes TinyJPEG to ChaN, 2012. The ROM decoder is resident device code; no JPEG decoder implementation or third-party source photograph is vendored by Pocket Search. The repository-authored admission policy, resizing callback and adapter are GPL-3.0-only. Preserve the pinned upstream ESP-IDF header notices and platform licensing with distributed corresponding source. This use does not establish general decoder conformance or a security audit.
