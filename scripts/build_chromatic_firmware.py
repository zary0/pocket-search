#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Build the complete native MCU app from pinned public sources; never install it."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
FIRMWARE = ROOT / "devices/chromatic/firmware"
PIN = json.loads((FIRMWARE / "inputs.json").read_text())


def git(root, *args):
    return subprocess.check_output(["git", "-C", str(root), *args], text=True).strip()


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def clean_checkout(path, expected):
    if git(path, "rev-parse", "HEAD") != expected:
        raise ValueError("Pinned source revision mismatch: " + str(path))
    if git(path, "status", "--porcelain", "--untracked-files=no", "--ignore-submodules=none"):
        raise ValueError("Tracked source or submodule modifications: " + str(path))
    raw = subprocess.check_output(["git", "-C", str(path), "submodule", "status", "--recursive"], text=True)
    if any(not line.startswith(" ") for line in raw.splitlines()):
        raise ValueError("Initialize all pinned submodules: " + str(path))


def tracked_copy(source, destination):
    names = subprocess.check_output(["git", "-C", str(source), "ls-files", "--recurse-submodules", "-z"]).decode().split("\0")
    for name in filter(None, names):
        path = source / name
        if path.is_symlink() or not path.is_file() or not path.resolve().is_relative_to(source):
            raise ValueError("Unsupported source path: " + name)
        target = destination / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, target)


def stage(upstream, lvgl, output):
    upstream, lvgl = upstream.resolve(), lvgl.resolve()
    output = output.absolute().parent.resolve() / output.name
    if output.exists() or output.is_symlink():
        raise ValueError("Output must be a new directory")
    for source in (upstream, lvgl, ROOT):
        if output.is_relative_to(source) or source.is_relative_to(output):
            raise ValueError("Build output and source trees must be separate")
    clean_checkout(upstream, PIN["mcu"]["commit"])
    clean_checkout(lvgl, PIN["lvgl"]["commit"])
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".pocket-firmware-", dir=output.parent) as temporary:
        staged = Path(temporary) / "firmware"
        staged.mkdir()
        tracked_copy(upstream, staged)
        tracked_copy(lvgl, staged / "components/lvgl")
        # Declared text sources only. No local SDK build products or device state.
        overlay = FIRMWARE / "overlay"
        for path in overlay.rglob("*"):
            if path.is_symlink():
                raise ValueError("Symlink in firmware overlay")
            if path.is_file():
                target = staged / path.relative_to(overlay)
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(path, target)
        components = ROOT / "devices/chromatic/components"
        for name in ("pocket_search_ui", "pocket_band_display", "pocket_render_probe", "pocket_perf_probe"):
            shutil.copytree(components / name, staged / "components" / name)
        shutil.copyfile(FIRMWARE / "sdkconfig.defaults", staged / "sdkconfig.defaults")
        staged.rename(output)
    return output


def build(args):
    idf = args.idf.resolve()
    clean_checkout(idf, PIN["idf"]["commit"])
    if not args.allow_dirty and git(ROOT, "status", "--porcelain", "--untracked-files=normal", "--ignore-submodules=none"):
        raise ValueError("Pocket Search checkout must be clean; --allow-dirty is development-only")
    commit = git(ROOT, "rev-parse", "HEAD")
    version = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    compiler = subprocess.check_output([str(args.rustc), "--version"], text=True).strip()
    if compiler != PIN["rust"]["rustc"]:
        raise ValueError("Rust Xtensa compiler identity mismatch: " + compiler)
    project = stage(args.upstream, args.lvgl, args.output)
    config = (project / "sdkconfig.defaults").read_text()
    (project / "sdkconfig.defaults").write_text(config + f'CONFIG_APP_PROJECT_VER="{version}"\n')
    native = project / "rust-target"
    environment = dict(os.environ, RUSTC=str(args.rustc.resolve()), CARGO_TARGET_DIR=str(native), IDF_PATH=str(idf), IDF_COMPONENT_MANAGER="0")
    # Prevent caller-provided flags or target overrides from changing the fixed profile.
    for name in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "CARGO_BUILD_TARGET"):
        environment.pop(name, None)
    sysroot = subprocess.check_output([str(args.rustc), "--print", "sysroot"], text=True).strip()
    cargo_home = Path(os.environ.get("CARGO_HOME", str(Path.home() / ".cargo")))
    remaps = [(ROOT, "/pocket-search"), (Path(sysroot) / "lib/rustlib/src/rust", "/rust-src"), (cargo_home, "/cargo")]
    rust_flags = ["--remap-path-prefix=" + str(source) + "=" + target for source, target in remaps]
    environment["CARGO_ENCODED_RUSTFLAGS"] = "\x1f".join(rust_flags)
    rust_command = [str(args.cargo.resolve()), "build", "--locked", "-Zbuild-std=core", "--target", PIN["rust"]["target"], "-p", "chromatic-ffi", "--features", "native", "--profile", PIN["rust"]["profile"]]
    with (project / "rust-build.log").open("w") as log:
        subprocess.run(rust_command, cwd=ROOT, env=environment, stdout=log, stderr=subprocess.STDOUT, check=True)
    archive = native / PIN["rust"]["target"] / PIN["rust"]["profile"] / "libchromatic_ffi.a"
    # Retain the virtualenv executable path: resolving its symlink loses the
    # environment containing ESP-IDF's Python dependencies.
    command = [str(args.idf_python.absolute()), str(idf / "tools/idf.py"), "-B", "build", "-D", "POCKET_RUST_ARCHIVE=" + str(archive), "-D", "IDF_TARGET=esp32", "build"]
    with (project / "idf-build.log").open("w") as log:
        subprocess.run(command, cwd=project, env=environment, stdout=log, stderr=subprocess.STDOUT, check=True)
    image = project / "build/pocket_search.bin"
    if not image.is_file() or image.stat().st_size > PIN["flash"]["application_bytes"]:
        raise ValueError("Native image missing or exceeds the existing 2 MiB app partition")
    inputs = {}
    for directory in ("main", "components", "cmake"):
        for path in (project / directory).rglob("*"):
            if path.is_file():
                inputs[str(path.relative_to(project))] = sha(path)
    for name in ("CMakeLists.txt", "sdkconfig.defaults", "partitions-wifi.csv"):
        inputs[name] = sha(project / name)
    report = {"schema_version": 1, "project": "pocket-search", "target": "esp32", "commit": commit, "version": version, "sha256": sha(image), "application_bytes": image.stat().st_size, "deployment": "application-only", "development_dirty": args.allow_dirty, "installed": False, "physical_qualified": False, "rustc": compiler, "rust_archive_sha256": sha(archive), "rust_path_remapping": True, "pinned_inputs": PIN, "source_sha256": inputs}
    shutil.copyfile(image, project / "application.bin")
    (project / "firmware.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(json.dumps({k: report[k] for k in ("version", "sha256", "application_bytes", "development_dirty", "physical_qualified")}))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("upstream", "idf", "lvgl", "output", "cargo", "rustc", "idf-python"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--allow-dirty", action="store_true", help="Development only; never release evidence")
    args = parser.parse_args()
    try:
        build(args)
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"Native build refused/failed: {error}. Inspect local build logs.\n")


if __name__ == "__main__":
    main()
