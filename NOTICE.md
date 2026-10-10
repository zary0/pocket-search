# Project and third-party notices

Copyright (C) 2026 Pocket Search contributors.

Repository-authored code, documentation, artwork, and original test fixtures are licensed under **GNU GPL version 3 only**, identified as `GPL-3.0-only`. See [LICENSE](LICENSE). This program is distributed without warranty, subject to that license. The Pocket Search logo and its generation record are in [assets](assets/README.md).

Rust dependencies keep their own licenses. No third-party dependency is relicensed by this project. The locked dependency inventory, selected license texts, and external firmware prerequisites are in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md). The `chromatic-client` crate currently has no external dependencies.

ModRetro Chromatic names identify the target device. This project is independent of ModRetro. No commercial game, official firmware image, FPGA image, font, or full-flash device backup is bundled. The experimental band display driver follows the QSPI bus settings and framebuffer addressing in ModRetro’s GPL-3.0 MCU transport, based on upstream commit `63ed3b62b16768ad2415c9e3b865b438ca40fcc6`. See the [upstream source and license](https://github.com/ModRetro/oss-chromatic-console-mcu/tree/63ed3b62b16768ad2415c9e3b865b438ca40fcc6). The driver, measurement component, and integration instructions are provided as source under GPL-3.0-only. If upstream MCU code or assets are imported later, preserve their applicable licenses and notices and include corresponding source and build instructions as required.
