# Contributing

Pocket Search is intended for public open-source development under **GPL-3.0-only**. Contributions to this repository use that license. Keep existing copyright, SPDX, and third-party notices intact. The repository is an independent project, not an official ModRetro product.

## Development priorities

Chromatic is the primary target: 160 × 144 pixels, physical buttons, bounded memory, and a responsive MCU UI. Keep the device core dependency-free and allocation-free. Crawling, the corpus, ranking, and image preparation belong to the companion service.

Prefer measured changes with a clear behavior or resource benefit. Do not trade UTF-8 validation, packet bounds, certificate validation, result correctness, or source attribution for a faster number. Avoid CPU intrinsics or unsafe device code without an actual target measurement and a portable reference path.

## Checks

```sh
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo check --locked -p chromatic-client
cargo bench --locked --bench performance
python3 scripts/pipeline.py version
python3 -m unittest discover -s tests -v
cc -std=c11 -Wall -Wextra -Werror -fsanitize=address,undefined tests/test_band_plan.c -o /tmp/test-band-plan
/tmp/test-band-plan
cc -std=c11 -Wall -Wextra -Werror -fsanitize=address,undefined -DPOCKET_BAND_ROWS=48 tests/test_band_plan.c -o /tmp/test-band-plan48
/tmp/test-band-plan48
```

For performance work, compare the same corpus, queries, compiler, optimization profile, architecture, and CPU settings. Record before/after JSON, ranked result equivalence, requested allocations, peak live allocation delta, and median timings. Host timings do not prove Xtensa device performance. See [performance notes](docs/performance.md) and the [physical display probe](docs/chromatic-rendering.md).

## Pull requests

Explain the problem, resulting behavior, validation, and measured tradeoffs. Keep firmware, protocol, service, and documentation changes reviewable. Add meaningful boundary or regression checks when behavior changes; ordinary documentation edits do not need redundant tests. Use English in README files and contributor documentation. Japanese fixture data and input support remain welcome.

Target `develop` with feature/fix PRs. `main` promotion and `vX.Y.Z` releases require the [release policy](docs/releases.md), including actual physical qualification. Release-gate unit fixtures do not satisfy the physical requirement.

Do not include credentials, activation codes, NVS dumps, physical-device identifiers, full-flash backups, or private site content. Clearly label fixture tests, host benchmarks, streamed emulator demos, USB diagnostics, and actual device measurements. Hardware writes require the existing device-specific deployment workflow.
