#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Install pinned public build dependencies in a new disposable directory."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import tarfile
import urllib.request
from build_chromatic_firmware import PIN


def archive(url, expected, destination):
    digest = hashlib.sha256()
    total = 0
    with urllib.request.urlopen(url, timeout=60) as response, destination.open("xb") as output:
        while block := response.read(65536):
            total += len(block)
            if total > 220_000_000:
                raise ValueError("Tool archive exceeds the fixed download budget")
            digest.update(block)
            output.write(block)
    if digest.hexdigest() != expected:
        raise ValueError("Official tool archive checksum mismatch")
    unpacked = destination.parent / (destination.stem + "-unpacked")
    unpacked.mkdir()
    # Python's data filter rejects escaping links, absolute paths and devices.
    with tarfile.open(destination) as bundle:
        bundle.extractall(unpacked, filter="data")
    roots = list(unpacked.iterdir())
    if len(roots) != 1 or not (roots[0] / "install.sh").is_file():
        raise ValueError("Unexpected official Rust installer layout")
    return roots[0]


def checkout(pin, output):
    command = ["git", "-c", "core.hooksPath=/dev/null", "-c", "credential.helper="]
    subprocess.run(command + ["clone", "--filter=blob:none", "--no-checkout", pin["repository"], str(output)], check=True)
    subprocess.run(command + ["-C", str(output), "fetch", "--depth=1", "origin", pin["commit"]], check=True)
    subprocess.run(command + ["-C", str(output), "checkout", "--detach", pin["commit"]], check=True)
    subprocess.run(command + ["-C", str(output), "submodule", "update", "--init", "--recursive", "--depth=1"], check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    if sys.version_info < (3, 12):
        parser.error("Python 3.12+ is required for safe archive extraction")
    output = args.output.absolute()
    if output.exists() or output.is_symlink():
        parser.error("Output must be a new directory; existing tools are never changed")
    hosts = {("Linux", "x86_64"): "x86_64-unknown-linux-gnu", ("Linux", "aarch64"): "aarch64-unknown-linux-gnu", ("Darwin", "arm64"): "aarch64-apple-darwin"}
    host = hosts.get((platform.system(), platform.machine()))
    if host not in PIN["rust"]["archives"]:
        parser.error("No verified compiler archive for this host")
    output.mkdir(parents=True)
    release = PIN["rust"]["release"]
    base = "https://github.com/esp-rs/rust-build/releases/download/" + release + "/"
    number = release.removeprefix("v")
    compiler = archive(base + f"rust-{number}-{host}.tar.xz", PIN["rust"]["archives"][host], output / "rust.tar.xz")
    toolchain = output / "rust"
    subprocess.run(["sh", str(compiler / "install.sh"), "--prefix=" + str(toolchain), "--components=rustc,cargo,rust-std-" + host, "--disable-ldconfig"], check=True)
    source = archive(base + f"rust-src-{number}.tar.xz", PIN["rust"]["source_sha256"], output / "rust-src.tar.xz")
    subprocess.run(["sh", str(source / "install.sh"), "--prefix=" + str(toolchain), "--disable-ldconfig"], check=True)
    for name in ("mcu", "idf", "lvgl"):
        checkout(PIN[name], output / name)
    environment = dict(os.environ, IDF_TOOLS_PATH=str(output / "idf-tools"))
    subprocess.run([sys.executable, str(output / "idf/tools/idf_tools.py"), "install", "--targets", "esp32"], env=environment, check=True)
    subprocess.run([sys.executable, str(output / "idf/tools/idf_tools.py"), "install-python-env"], env=environment, check=True)
    print("Pinned tools installed. Set IDF_TOOLS_PATH=" + str(output / "idf-tools") + " and source " + str(output / "idf/export.sh"))


if __name__ == "__main__":
    main()
