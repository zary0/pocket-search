# Complete native MCU build

Pocket Search builds the complete ESP32 MCU application from this checkout and
pinned public dependency sources. The project no longer requires the author's
modified SDK directory, absolute paths, or an unpublished display/font adapter.
This builds an application, not a complete factory-flash dump or an FPGA image.
Existing bootloader, FPGA, board identity and Wi-Fi settings are separate.

## Supported environment

- ESP32-U4WDH at 160 MHz, 4 MiB flash, no PSRAM; existing 2 MiB application region
  starting at `0x10000`, NVS at `0x9000`, board version 4.6.
- ESP-IDF 5.3.0 and LVGL 8.4.0 at the commits in
  [inputs.json](../devices/chromatic/firmware/inputs.json).
- Espressif Rust `v1.97.0.0`, `xtensa-esp32-none-elf`, `chromatic-size` profile;
  matching Rust core source is required for `-Zbuild-std=core`.
- Python 3.12+, Git, the ESP-IDF native build prerequisites and an internet
  connection to obtain public dependencies. Supported tool installer hosts are
  Linux x86_64/arm64 and macOS arm64.

The dependency installer creates a **new disposable directory** and verifies the
official Rust compiler/source archive SHA-256 values before installation. It
checks out exact MCU, IDF, LVGL and recursive submodule revisions. It neither
changes an existing tool installation nor accesses a device.

```sh
python3 scripts/setup_chromatic_firmware.py /tmp/pocket-tools
export IDF_TOOLS_PATH=/tmp/pocket-tools/idf-tools
. /tmp/pocket-tools/idf/export.sh
python3 scripts/build_chromatic_firmware.py \
  --upstream /tmp/pocket-tools/mcu \
  --idf /tmp/pocket-tools/idf \
  --lvgl /tmp/pocket-tools/lvgl \
  --output /tmp/pocket-native \
  --cargo /tmp/pocket-tools/rust/bin/cargo \
  --rustc /tmp/pocket-tools/rust/bin/rustc \
  --idf-python "$IDF_PYTHON_ENV_PATH/bin/python"
```

The source checkout must be clean. `--allow-dirty` supports local development,
labels the result explicitly and cannot establish a release candidate's identity.
Changed tracked sources, hidden submodule edits, wrong revisions, escaping
symlinks, existing destinations and images larger than 2 MiB are rejected.
Untracked vendor license files, device backups, NVS, credentials, Git metadata
and local build products are not staged.

Outputs include `application.bin` and `firmware.json`, with the project version,
source commit, compiler identity, source hashes and application checksum. Private
local build logs are kept outside the repository. Build tools may print generic
full-flash commands: **those are not this project's deployment procedure**.
Installation remains application-only, with device identity, recovery, partition,
settings and read-back verification. No install command is run by either script.

ESP-IDF reproducible-build mode removes build dates and path macros. Same-source
builds are compared before claiming byte reproducibility; builds on another host
or with another toolchain must retain their own checksum and verification.

## CI and release boundary

Hosted CI uses Ubuntu 24.04 and includes `native-firmware` on pull requests and develop/main pushes. The explicit OS label avoids a moving `ubuntu-latest` baseline; tool/source pins remain mandatory.
It builds in an isolated GitHub-hosted machine with read-only repository access.
Its artifact contains build identity metadata only; binaries remain outside public
CI artifacts until distribution and physical qualification are complete. Require
both `ci` and `native-firmware` in branch rules. `main` also requires the separate
physical qualification check.

The public build overlay retains ModRetro's GPL license and provenance. The
compiled Pocket JP bitmap is derived from Noto Sans JP and retains OFL-1.1, its
copyright and modified font name; it is not relabeled GPL. ESP-IDF's radio/PHY
libraries and their upstream licenses remain external pinned prerequisites.
Gowin tools, vendor licenses, FPGA images and device state are not part of this
MCU build. This document does not qualify hardware operation or a firmware release.
