# Native physical-button click

**Status: implemented RTL, passing leaf simulation and a licensed full Gowin build with zero reported timing violations. The custom bitstream has not been installed, and no physical speaker or recovery test has passed.** This work does not satisfy the native release gate.

The sound is generated on the Chromatic FPGA. The MCU firmware's named I2S pins do not provide an audio receiver in the pinned FPGA design: `I2S_BCLK` carries menu state. Do not drive those pins to try to play audio. The codec and USB audio already receive 16-bit stereo PCM from the FPGA's emulator; the integration adds a short click before both existing consumers.

## Behavior

- Physical A, B, Start, Select and D-pad rising edges click while the MCU menu is open. This includes keyboard cursor movement, text entry and other system menus; it is not limited to successful text edits in Pocket Search.
- The side Menu button, MCU-injected buttons and normal game controls do not trigger clicks. Held keys produce one click; simultaneous keys produce one click.
- A burst lasts at most 100,664 gClk cycles (approximately 12 ms at the nominal 8,388,608 Hz). Its 2,048 Hz square wave decays linearly in quantized steps, from a peak magnitude of 786 to zero. The final 127 cycles have zero amplitude.
- Accepted presses are separated by at least 251,659 cycles (approximately 30 ms). Faster edges are discarded without a queue or an extended burst.
- Closing the menu, software mute, apparent power-off or reset cancels the added tone. Unmuting or entering a menu with a held key does not replay it.
- Signed saturation prevents arithmetic wraparound when adding the tone to existing PCM. Inactive feedback passes each original channel through bit for bit.

The existing codec remains responsible for the volume wheel, headphone detection and its normal mute behavior. No codec initialization, pin, clock or constraint is changed. USB receives the mixed PCM before the existing codec path; this does not newly apply the physical volume wheel to USB audio. Actual speaker/headphone loudness and balance still need physical testing.

## Resource scope

The source declares 56 state bits: 8 previous-button bits, one arming bit, a 17-bit burst counter, an 18-bit cooldown counter, an 11-bit phase counter and one polarity bit. It uses integer logic and two saturating PCM adders. There is no waveform ROM, sample buffer, heap allocation, MCU task, audio-file asset or new UART command. This is a source-level count, not a fitted FPGA resource or power measurement. The measured full-project fit is recorded below; physical power and clock-domain qualification remain unverified.

## Simulate

Install Icarus Verilog and run:

```sh
python3 scripts/test_chromatic_click.py
python3 scripts/test_chromatic_click.py --wav /tmp/pocket-click-reference.wav
```

The optional WAV is a 16,384 Hz mono reference sampled from the tested RTL, with three separated clicks. It is **not a native speaker recording**. The test checks attenuation, bounded duration, waveform frequency, simultaneous/held keys, cooldown, signed clipping, cancellation, reset and unchanged game PCM. CI runs the same simulation, then stages the exact official sources and elaborates the patched top with vendor IP black-boxed. That elaboration checks syntax and the new module's connections; it does not simulate the codec/USB, establish clock-domain correctness of the complete vendor design, or generate a board image.

## Prepare the official FPGA project

The source integration is pinned to [ModRetro FPGA commit a01aa375147943cbcd4a41f41585220d55e0c28a](https://github.com/ModRetro/oss-chromatic-console-fpga/tree/a01aa375147943cbcd4a41f41585220d55e0c28a), including Gameboy MiSTer submodule commit `44f43d5797cf6e67480c156b47974d6f1ca9ce4a`. Inputs and critical SHA-256 values are in [upstream.json](../devices/chromatic/fpga/upstream.json).

```sh
git clone https://github.com/ModRetro/oss-chromatic-console-fpga.git /tmp/chromatic-fpga-upstream
git -C /tmp/chromatic-fpga-upstream checkout --detach a01aa375147943cbcd4a41f41585220d55e0c28a
git -C /tmp/chromatic-fpga-upstream submodule update --init --recursive
python3 scripts/prepare_chromatic_click.py /tmp/chromatic-fpga-upstream /tmp/chromatic-fpga-click
python3 scripts/test_chromatic_click.py --staged /tmp/chromatic-fpga-click
```

The preparer refuses a different revision, modified tracked source, uninitialized/mismatched submodules, missing project files or an existing destination. It copies only tracked sources into a separate tree; it preserves upstream license files and does not copy Git metadata, untracked local files or license credentials. It modifies six files: `top.v`, `system_monitor.sv`, `evt1_x2.gprj`, the added `pocket_ui_click.sv`, `build_options.tcl` and `evt1_x2_process_config.json`. The last two changes remove the obsolete disabled `gen_text_timing_rpt` option (SUG1220-2.1) and use the documented timing-prioritized placement algorithm (`Place_Option: "2"`). The upstream timing-driven routing setting stays enabled, with route option 1; pin, clock and timing constraints are unchanged. Enabling the unsupported standalone text report remains an error.

The custom version is `0x1392`: upstream major 18/minor 14 with the debug/version bit set. This distinguishes the modified 14-bit identity, as requested by the upstream custom-modification instructions. It is not an official ModRetro release or a Pocket Search `vX.Y.Z` release. The generated source manifest records exact output hashes and explicitly reports `bitstream_built: false` and `hardware_tested: false`; it is not a build or release qualification manifest.

## Licensed build evidence

The [sanitized vendor-build evidence](evidence/native-click/vendor-build.json) records three complete builds with Gowin 1.9.12.04 on macOS arm64, using the same target, routing setting, pins and timing constraints. The unmodified upstream baseline has zero reported violations. Adding clicks with the original placement algorithm caused 12 reset-recovery violations in the PSRAM interface, with worst recovery slack of -0.122 ns; these appear in the vendor's setup-violation summary. Timing-prioritized placement removes those violations without changing the audio RTL or relaxing constraints. The final report has zero setup/hold violation endpoints and minimum reported recovery slack of +1.162 ns.

| Full-project resource | Upstream baseline | Click build, timing placement |
| --- | ---: | ---: |
| Logic | 16,233 / 23,040 | 16,412 / 23,040 |
| Registers | 8,525 / 23,595 | 8,579 / 23,595 |
| CLS | 10,150 / 11,520 | 10,026 / 11,520 |
| BSRAM | 44 / 56 | 44 / 56 |
| DSP | 11.5 / 28 | 11.5 / 28 |

These are whole-project results with different placement settings, not isolated mixer cost or console RAM measurements. The final build took 259.58 seconds and produced a 902,035-byte bitstream with SHA-256 `9fc7cf1a772d18aec9e42f8f288bd79b3b93e33a40cf02c36adf836cb736cf0c`. The private vendor license, proprietary tools, complete vendor reports and bitstream are not bundled in this repository. Source and report hashes are retained for comparison.

A successful compiler exit and positive frequency summary alone do not qualify a build. Clock-relationship warnings and complete clock-domain behavior still require review; recovery and physical speaker/headphone/USB tests remain open. This evidence is a local vendor build, not hosted CI, a console recording, or release E2E acceptance.

## Complete the target build and physical test

The pinned upstream design specifies **Gowin FPGA Designer 1.9.12.03 with a valid vendor license**. This development machine now has the official **Gowin 1.9.12.04 macOS arm64** tool and a working machine-bound license. The tested compiler version is recorded separately from the upstream recommendation. Icarus and open-source generic synthesis do not replace the complete Gowin build or its proprietary IP. In an appropriately licensed environment, run the staged project build:

```sh
cd /tmp/chromatic-fpga-click/esp32t
gw_sh build.tcl
```

Before deployment, inspect the complete build's target, errors, warnings, resource and timing reports, and record bitstream hashes. Deployment then needs the intended device and a verified recovery procedure for that exact FPGA image. This repository's MCU application-only installer and the plugin's cartridge-ROM flash are different workflows; neither installs this RTL. No FPGA flashing command or distributable bitstream is provided here.

Physical acceptance must cover keyboard and navigation feedback, all eight buttons, held/simultaneous/rapid input, wheel minimum/maximum, software mute, speaker/headphones, game PCM, menu transitions, reset, USB audio/video and settings preservation. Record the installed image identity and real capture separately from simulation. The upstream update tool's restoration behavior has not been tested with this custom image.
