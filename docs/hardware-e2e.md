# Physical Chromatic E2E lab

The lab must test the native Pocket Search MCU application through the actual display, controls, and device Wi-Fi. Host fixtures, packet unit tests, PyBoy, and streamed emulator demos cannot qualify a release.

## Current readiness

As checked on 2026-10-10, no lab runner is registered, `CHROMATIC_E2E_READY=false`, the complete Pocket Search MCU firmware project and installed lab driver are not implemented, and no reachable test HTTPS origin is configured. The workflows and release-gate validators exist. The experimental search UI/rendering images have external integration and different application/version identities; they cannot satisfy this pipeline.

Automatic develop pushes record **Physical E2E not run** while the repository variable is explicitly `false`. This keeps the development status separate from a failed test or a fabricated pass. An unset/invalid value runs the strict readiness check and fails. Main pushes, manual dispatch on develop/main, and release tags always invoke qualification, whose readiness and physical evidence gates remain mandatory. Enable `true` only after the prerequisites below are verified; then develop pushes also run physical qualification automatically.

## Dedicated runner

Use an operator-controlled machine with USB access to the intended Chromatic, ESP-IDF and the actual Rust Xtensa toolchain, Python 3.11+, and the firmware deployment/capture dependencies. The pinned Actions use Node.js 24 and require Actions Runner **2.327.1 or newer**. Register it for this repository with the `chromatic` label. Use a dedicated unit and preserve its verified recovery backup and Wi-Fi settings outside the checkout.

Configure the `chromatic-lab` environment for trusted `develop`, `main`, and stable release tags. Do not execute fork PRs or arbitrary contributor refs on the lab machine. Preconfigure physical capture access through the normal ModRetro/OS permission flow; CI cannot silently grant it. Keep the installed driver outside repository-controlled paths.

Repository variables:

| Variable | Meaning |
| --- | --- |
| `CHROMATIC_E2E_READY` | `true` only after the application, runner, driver, and fixture origin are verified |
| `CHROMATIC_LAB_DRIVER` | Absolute installed driver executable path, for example `/opt/pocket-search-lab/driver` |
| `CHROMATIC_LAB_DRIVER_SHA256` | Operator-approved digest of that installed executable |
| `CHROMATIC_TEST_ORIGIN` | Reachable HTTPS fixture origin with a certificate accepted by the device |

These variables do not contain Wi-Fi passwords, activation keys, tokens, or backups. Runner credentials and private device state stay outside the repository and upload artifacts. Actions execute the driver with an argument list and verify its path and hash rather than evaluating a repository-provided shell command.

## Driver contract

`scripts/hardware_runner.py` implements orchestration and validation. The operator-installed driver must implement:

1. `build --source PATH --commit SHA --version X.Y.Z --output PATH`: build the real Pocket Search native application from the exact checkout. Write `application.bin` and `firmware.json` with `project`, `target`, `commit`, `version`, `sha256`, and `deployment: application-only`. CPU target is `esp32`. The image must fit the 2 MiB application budget.
2. `e2e --candidate PATH --origin HTTPS_ORIGIN --output PATH`: select the intended physical device, install/read-back the exact candidate image using the established recovery/settings-preservation workflow, verify its version and digest, then exercise the native application and capture physical USB video. Write sanitized `report.json`, `junit.xml`, and selected PNG captures.

The driver must never emit a synthetic success or substitute a different firmware, device, image, or transport. Its complete implementation is still pending. Build/private logs are kept in temporary local files and are not uploaded.

## Required scenarios

- `native_wifi_verified_https`: device-native clock synchronization and certificate-validated HTTPS to the fixture origin; no host fetch proxy.
- `query_result_open`: submit a known query through device controls, inspect real returned results, select one, and verify the displayed document/source.
- `utf8_pagination`: read all pages of a mixed Japanese/English fixture and verify bounds, text completeness, back navigation, and physical glyph behavior.
- `empty_results_network_errors`: zero-result behavior, timeout/reconnection, and malformed/oversized replies without fabricated content or a frozen UI.
- `cpu_memory_budget`: pinned-core decoder cycle measurements, zero decoder allocations, and free/largest heap checks after warm-up; measure verified HTTPS separately.

The service origin must support deterministic test documents, error injection, request correlation, and a native-device request trace so the driver can verify the path. These fixture service features are not implemented yet. Initial thresholds in `ci/hardware-policy.json` are acceptance targets, not measured achievements.

## Evidence

The report uses schema 1 and `execution_mode: physical-native-wifi`. It must bind `commit`, `version`, `candidate_sha256`, `firmware_sha256`, and `readback_sha256`; state `transport_source: device-wifi`, `host_proxy_used: false`, `clock_source: native-sntp`, `certificate_validation: true`, and `frame_source: physical-usb-video`; list all five passed cases and all policy metrics; and reference at least three distinct physical captures.

JUnit must contain exactly the required scenarios with zero failures, errors, or skips. PNGs must have valid chunk checksums and the Chromatic's 160:144 geometry at integer scale 1–8. Failed, skipped, incomplete, wrong-revision, emulator, host-proxy, invalid-metric, or tampered-asset evidence is rejected. Capture filenames are plain relative filenames; arbitrary paths and unlisted artifacts are rejected.

Parsing JSON and PNG metadata cannot authenticate their physical origin. Trust comes from the controlled runner and hash-approved driver performing actual installation, read-back, native requests, UI checks, and physical capture. `tests/test_pipeline.py` uses explicit synthetic unit fixtures solely to check gate acceptance/rejection. Those fixtures are never physical results or release evidence.

After validation, the pipeline uploads only the candidate and selected evidence. Publication revalidates everything and promotes the same tested bytes without rebuilding. A missing runner or unfinished driver leaves the release blocked.
