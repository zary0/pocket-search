#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Stage pinned Chromatic FPGA sources with native feedback; never build or flash."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
FPGA = ROOT / "devices/chromatic/fpga"
PIN = json.loads((FPGA / "upstream.json").read_text())
MODULE = "esp32t/src/rtl/BSP/pocket_ui_click.sv"


def git(source, *args):
    return subprocess.check_output(["git", "-C", str(source), *args], text=True).strip()


def sha(data):
    return hashlib.sha256(data).hexdigest()


def replace_once(text, old, new):
    if text.count(old) != 1:
        raise ValueError("FPGA integration anchor is missing or ambiguous")
    return text.replace(old, new, 1)


def integrate(top, monitor, project):
    """Transform only the PCM wiring, project source list, and custom version."""
    top = replace_once(top, "    wire [15:0] left, right;",
                       "    wire [15:0] left, right;\n    wire [15:0] game_left, game_right;")
    # The EMU's audio output is the sole original PCM producer. Codec and USB
    # retain their existing left/right ports and now consume the mixed wires.
    top = replace_once(top, "        // audio\n        .left(left),\n        .right(right),",
                       "        // audio\n        .left(game_left),\n        .right(game_right),")
    insertion = """
    // Pocket Search physical menu feedback (GPL-3.0-only).
    pocket_ui_click u_pocket_ui_click(
        .clk(gClk),
        .reset_n(lock_o),
        .menu_open(~menuDisabled),
        .software_mute(system_control[0]),
        .appear_off(appear_off),
        .buttons({BTN_DPAD_DOWN_filtered, BTN_DPAD_LEFT_filtered,
                  BTN_DPAD_RIGHT_filtered, BTN_DPAD_UP_filtered,
                  BTN_A_filtered, BTN_B_filtered, BTN_SEL_filtered,
                  BTN_START_filtered}),
        .game_left(game_left),
        .game_right(game_right),
        .left(left),
        .right(right)
    );

"""
    top = replace_once(top, "    reg UART_TXD;", insertion + "    reg UART_TXD;")
    monitor = replace_once(monitor, "        1'd0,  // 1 bit debug,",
                           "        1'd1,  // custom Pocket Search RTL: debug/version bit")
    anchor = '        <File path="src/rtl/BSP/aud_system_top_8_16.sv" type="file.verilog" enable="1"/>'
    project = replace_once(project, anchor, anchor + '\n        <File path="src/rtl/BSP/pocket_ui_click.sv" type="file.verilog" enable="1"/>')
    return top, monitor, project


def prepare(source, output):
    source, output = source.resolve(), output.absolute()
    # Resolve the parent too, so a symlink cannot place output inside source.
    output = output.parent.resolve() / output.name
    if output.exists() or output.is_symlink():
        raise ValueError("Output must be a new directory; nothing is overwritten")
    if output.is_relative_to(source) or source.is_relative_to(output):
        raise ValueError("Source and output must be separate trees")
    if git(source, "rev-parse", "HEAD") != PIN["commit"]:
        raise ValueError("Source commit does not match the pinned official FPGA revision")
    if git(source, "status", "--porcelain", "--untracked-files=no"):
        raise ValueError("Official source or a submodule has tracked modifications")
    modules = git(source, "submodule", "status", "--recursive")
    # git() strips leading whitespace, so inspect raw submodule status here.
    raw_modules = subprocess.check_output(["git", "-C", str(source), "submodule", "status", "--recursive"], text=True)
    if any(not line.startswith(" ") for line in raw_modules.splitlines()):
        raise ValueError("Initialize all submodules at their pinned commits first")
    if PIN["gameboy_mister_commit"] not in modules:
        raise ValueError("Unexpected Gameboy MiSTer revision")
    for relative, expected in PIN["sha256"].items():
        if sha((source / relative).read_bytes()) != expected:
            raise ValueError("Pinned source hash mismatch: " + relative)
    names = subprocess.check_output(["git", "-C", str(source), "ls-files", "--recurse-submodules", "-z"]).decode().split("\0")
    names = [name for name in names if name]
    for name in names:
        path = source / name
        if not path.resolve().is_relative_to(source) or path.is_symlink() or not path.is_file():
            raise ValueError("Unsupported tracked source path: " + name)
    top, monitor, project = integrate(
        (source / "esp32t/src/top.v").read_text(),
        (source / "esp32t/src/rtl/BSP/system_monitor.sv").read_text(),
        (source / "esp32t/evt1_x2.gprj").read_text())
    # SUG1220-2.1 removed this reporting switch from set_option. The pinned
    # IDE config disables it; retain that choice and fail if it is enabled.
    options = (source / "esp32t/build_options.tcl").read_text()
    options = replace_once(options,
        "    Generate_Plain_Text_Timing_Report gen_text_timing_rpt\n", "")
    options = replace_once(options,
        "# The GW5A CLI rejects gen_ibis even when disabled.",
        "# SUG1220-2.1 removed gen_text_timing_rpt. Preserve the disabled IDE\n"
        "# choice without issuing an obsolete option; fail if a text report is requested.\n"
        "if {[ide_option Generate_Plain_Text_Timing_Report]} {\n"
        "    error \"Standalone text timing reports require a supported reporting command\"\n"
        "}\n# The GW5A CLI rejects gen_ibis even when disabled.")
    # SUG1220-2.1 defines placement option 2 as timing-prioritized. The
    # original option 0 prioritizes compiler speed. Keep routing and all
    # clocks/pins unchanged while fitting the extra audio logic.
    process_config = (source / "esp32t/impl/evt1_x2_process_config.json").read_text()
    process_config = replace_once(process_config,
        '"Place_Option": "0"', '"Place_Option": "2"')
    changed = {
        "esp32t/src/top.v": top.encode(),
        "esp32t/src/rtl/BSP/system_monitor.sv": monitor.encode(),
        "esp32t/evt1_x2.gprj": project.encode(),
        MODULE: (FPGA / "pocket_ui_click.sv").read_bytes(),
        "esp32t/build_options.tcl": options.encode(),
        "esp32t/impl/evt1_x2_process_config.json": process_config.encode(),
    }
    for entry in ET.fromstring(project).findall("./FileList/File"):
        if entry.attrib.get("enable") == "1":
            relative = "esp32t/" + entry.attrib["path"]
            if relative not in names and relative not in changed:
                raise ValueError("Missing enabled project source: " + relative)
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".pocket-fpga-", dir=output.parent) as directory:
        staged = Path(directory) / "source"
        staged.mkdir()
        # Only tracked sources, including pinned submodules. No .git, local
        # build products, license credentials, untracked files, or backups.
        for name in names:
            destination = staged / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes((source / name).read_bytes())
        for name, data in changed.items():
            (staged / name).write_bytes(data)
        manifest = {
            "schema_version": 1,
            "upstream_commit": PIN["commit"],
            "gameboy_mister_commit": PIN["gameboy_mister_commit"],
            "custom_fpga_version": PIN["custom_version"],
            "changed_sha256": {name: sha(data) for name, data in changed.items()},
            "source_only": True, "bitstream_built": False, "hardware_tested": False,
        }
        (staged / "pocket-click-manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
        staged.rename(output)
    return manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path, help="Clean official checkout with initialized submodules")
    parser.add_argument("output", type=Path, help="New, separate source directory")
    args = parser.parse_args()
    try:
        manifest = prepare(args.source, args.output)
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"Cannot stage FPGA: {error}\n")
    print(json.dumps(manifest, indent=2))


if __name__ == "__main__":
    main()
