# Memory and CPU performance

Optimize the physical Chromatic interaction first: predictable buffer sizes, zero allocations in the client core, integer-only parsing, bounded work, and a responsive UI. Measure the companion separately. A faster host search does not prove a faster ESP32 decoder, TLS transfer, or screen update.

## Implemented changes

The index stores sorted contiguous posting vectors instead of a tree of separately allocated document/frequency nodes. Query planning starts with the rarest posting list and checks other terms without building intersected candidate sets. IDF is calculated once per query term. A bounded top-k list stores only document IDs and scores; title, URL, and snippet strings are created only for selected results. Stable score/URL/document-order ranking is preserved.

The client decoder combines cell-width and control-character validation in one Unicode pass after UTF-8 validation. It borrows a capped packet buffer, has no allocator or external dependencies, forbids unsafe code, and performs no floating-point work. Its 512-byte receive-buffer limit excludes the separate packet view, TLS, fonts, display buffers, and UI state.

## Host benchmark

```sh
cargo bench --locked --bench performance > performance.json
```

The harness uses 3,000 deterministic original fixture documents, three queries, ten requested results, ten warm-up operations, and five timed samples. Allocation measurements are separate single operations. Timing samples disable atomic live-byte accounting; allocator delegation and a flag check remain. All temporary objects in a timed operation are dropped before returning. The benchmark is single-threaded and has no network or disk work in the measured queries.

Memory metrics count bytes requested from the System allocator. They are not RSS, allocator bookkeeping, firmware heap, or complete process memory. `peak_live_bytes_above_start` is the temporary live-byte increase above the state before a single operation. `live_bytes_delta_at_return` includes retained output capacity. Index construction starts after the original document corpus has been allocated, so its delta measures added index structures and excludes that pre-existing corpus. No result is a promise for an arbitrary corpus.

The checked-in comparison uses Apple M4, macOS/aarch64, rustc 1.95.0, opt-level 3, thin LTO, one codegen unit, overflow checks enabled, and default portable host CPU flags. Raw before/after JSON and unchanged top-ten IDs are in [benchmarks](benchmarks/2026-10-09-host-comparison.json). Timing varies with the environment; use the recorded min/median/max rather than a single fastest run.

| Host query | Before (µs) | After (µs) | Speedup | Temporary peak bytes before → after |
| --- | ---: | ---: | ---: | ---: |
| chromatic | 2096.3 | 136.5 | 15.4× | 1,577,195 → 5,032 |
| chromatic network | 171.0 | 18.9 | 9.0× | 93,357 → 5,094 |
| Japanese query | 137.0 | 13.2 | 10.4× | 57,410 → 2,964 |

Index construction retained 2,142,121 added bytes before and 945,401 after, beyond the preallocated corpus. Client decoder allocation count and live-byte delta are both zero. These are measurements of the stated host fixture workload.

The reproducible unoptimized implementation plus the final benchmark harness is commit `0e157a57b022994163b4096fdb2f7861a3d9e591`. Run that revision and the optimized revision with identical commands and build settings, sequentially on the same machine. Do not run other builds or benchmarks during timed samples.

## CPU-specific builds

The companion's portable release profile uses opt-level 3, thin LTO, one codegen unit, and overflow checks. For a local companion binary used only on the build machine, `RUSTFLAGS='-C target-cpu=native'` is an optional measured experiment, not the portable distribution default. No native-CPU result is claimed by the checked-in benchmark.

The MCU target is the inspected ESP32 Xtensa platform at 160 MHz. `chromatic-size` and `chromatic-speed` profiles allow comparison of size-oriented and speed-oriented code generation with the actual target toolchain:

```sh
# Host-only compile checks for profile configuration; not ESP32 firmware builds.
cargo build -p chromatic-client --profile chromatic-size
cargo build -p chromatic-client --profile chromatic-speed
```

Both profiles retain overflow checks, thin LTO, and one codegen unit. The future Xtensa build must select the actual ESP32 target and toolchain; these host commands do not do so. An installed Xtensa C compiler does not provide the Rust Xtensa compiler. Neither profile is declared the device winner until code size, cycles, and full firmware behavior have been measured. Rust/Cargo profile controls are described in the [Cargo reference](https://doc.rust-lang.org/cargo/reference/profiles.html), and CPU flags in the [rustc reference](https://doc.rust-lang.org/rustc/codegen-options/index.html).

## ESP32 measurement component

`devices/chromatic/components/pocket_perf_probe` is an ESP-IDF 5.3 component for the inspected ESP32 target. Supply a small synchronous callback, such as the future decoder adapter, from a task pinned to one core with a fixed CPU frequency. Warm it up before measurement and keep network, logging, allocation, and blocking out of the callback. The probe itself allocates no memory and uses integer counters.

Reports contain per-call CPU-cycle minimum/maximum/total, elapsed microseconds, start/end frequency, core ID, before/after free bytes and largest blocks, and a since-boot heap low-water mark. Cycle timings include interruption overhead and counter reads. The since-boot minimum is not the callback's peak memory. A 25 ms batch budget is checked between calls; it cannot interrupt a blocking callback. Only complete `ESP_OK` reports are valid comparisons.

The component compiled with the existing ESP32 Xtensa GCC 13.2 toolchain and inspected ESP-IDF 5.3 configuration using warnings as errors. Its object had 389 bytes of text and no object data/BSS; that excludes linked SDK routines, call-stack use, and the caller's report buffer. It has not been linked into or run on the device. The Rust client has not yet been cross-built or installed either.

When hardware integration is ready, record firmware revision, target/toolchain, CPU frequency, core pinning, build profile, packet corpus, cycles, heap and largest-block measurements, stack high-water mark, TLS peak costs, transfer time, and visible render time. Keep CPU-only decode, verified HTTPS, and display latency as separate measurements before reporting end-to-end performance.

## Performance acceptance

Preserve ranked results, complete text pagination, UTF-8 and packet validation, time synchronization, and certificate verification. Report improvements together with allocation, retained-memory, flash-size, and latency tradeoffs. Use explicit device measurements to justify hardware-specific optimizations rather than extrapolating Apple M4 timings to Chromatic.

## RTOS footprint and bare metal

Removing a scheduler and application task stacks can reduce firmware overhead when the same work fits a simpler event loop. A bare-metal rewrite does not guarantee lower memory use or faster end-to-end behavior: Wi-Fi, TCP/IP, TLS, interrupts, and display buffers still need an implementation and resources. The portable `no_std` core already has no allocator or OS dependency; that property does not describe the complete firmware.

The initial target remains ESP-IDF 5.3/FreeRTOS with existing device drivers. Espressif documents task/stack and network/TLS buffer tuning in its [RAM optimization guide](https://docs.espressif.com/projects/esp-idf/en/v5.3/esp32/api-guides/performance/ram-usage.html), and task-based networking in the [Wi-Fi model](https://docs.espressif.com/projects/esp-idf/en/v5.3/esp32/api-guides/wifi.html#esp32-wi-fi-programming-model). Our engineering choice is to measure and reduce that footprint first:

- Consolidate application work where practical and size stacks from worst-case high-water measurements with headroom.
- Bound concurrent requests and network/TLS buffers; measure both memory and throughput rather than assuming smaller buffers are faster.
- Use incremental drawing and the proposed image stripes to avoid another full framebuffer; drawing and image transport remain unimplemented.
- Keep certificate checks, UTF-8 validation, and recovery behavior intact.

Compare linked image size, peak/free heap, largest free block, task stack use, decoder cycles, verified HTTPS latency, and visible rendering under identical scenarios. No complete ESP-IDF-versus-bare-metal build or physical performance comparison exists yet. Replacing the platform requires a supported Wi-Fi/TLS integration and equivalent physical E2E, not just a smaller core binary.
