#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Run real click RTL simulation. This is not physical device qualification."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import wave

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--iverilog", default="iverilog")
    parser.add_argument("--iverilog-base", type=Path)
    parser.add_argument("--vvp", default="vvp")
    parser.add_argument("--wav", type=Path, help="Write optional RTL reference audio, not hardware audio")
    parser.add_argument("--staged", type=Path, help="Also elaborate the staged top; vendor IP remains black-boxed")
    parser.add_argument("--report", type=Path, help="Write sanitized simulation-only evidence on success")
    args = parser.parse_args()
    for tool in (args.iverilog, args.vvp):
        if not shutil.which(tool):
            parser.error(f"Missing simulation tool: {tool}; install Icarus Verilog")
    with tempfile.TemporaryDirectory(prefix="pocket-click-") as directory:
        temp = Path(directory)
        command = [args.iverilog, "-g2012", "-Wall", "-s", "tb_pocket_ui_click"]
        if args.iverilog_base:
            command += ["-B", str(args.iverilog_base.resolve())]
        command += ["-o", str(temp / "click.vvp"),
                    str(ROOT / "devices/chromatic/fpga/pocket_ui_click.sv"),
                    str(ROOT / "tests/fpga/tb_pocket_ui_click.sv")]
        subprocess.run(command, check=True, timeout=60)
        run = [args.vvp, str(temp / "click.vvp")]
        if args.wav:
            run += ["+pcm=" + str(temp / "reference.pcm")]
        subprocess.run(run, check=True, timeout=60)
        if args.staged:
            manifest = json.loads((args.staged / "pocket-click-manifest.json").read_text())
            pin = json.loads((ROOT / "devices/chromatic/fpga/upstream.json").read_text())
            if manifest["upstream_commit"] != pin["commit"]:
                raise ValueError("Staged source revision mismatch")
            for name, digest in manifest["changed_sha256"].items():
                if hashlib.sha256((args.staged / name).read_bytes()).hexdigest() != digest:
                    raise ValueError("Staged source was changed after preparation")
            staged_module = args.staged / "esp32t/src/rtl/BSP/pocket_ui_click.sv"
            if staged_module.read_bytes() != (ROOT / "devices/chromatic/fpga/pocket_ui_click.sv").read_bytes():
                raise ValueError("Staged module differs from the simulated module")
            top_command = command[:command.index("-s")] + ["-i", "-s", "top"]
            if args.iverilog_base:
                top_command += ["-B", str(args.iverilog_base.resolve())]
            top_command += ["-o", str(temp / "top.vvp"),
                            str(args.staged / "esp32t/src/top.v"), str(staged_module)]
            subprocess.run(top_command, check=True, timeout=60)
            print("PASS: staged top elaboration (vendor IP black-boxed; no timing/fit proof)")
        if args.wav:
            samples = [int(line) for line in (temp / "reference.pcm").read_text().splitlines()]
            args.wav.parent.mkdir(parents=True, exist_ok=True)
            with wave.open(str(args.wav), "wb") as audio:
                audio.setparams((1, 2, 16384, 0, "NONE", "not compressed"))
                audio.writeframes(b"".join(struct.pack("<h", sample) for sample in samples))
            print(f"RTL reference audio (not recorded hardware): {args.wav}")
        if args.report:
            report = {
                "schema_version": 1, "execution_mode": "rtl-simulation",
                "module_sha256": hashlib.sha256((ROOT / "devices/chromatic/fpga/pocket_ui_click.sv").read_bytes()).hexdigest(),
                "leaf_simulation_passed": True,
                "staged_top_elaborated_with_black_boxes": args.staged is not None,
                "bitstream_built": False, "hardware_tested": False,
                "nominal_gclk_hz": 8388608, "burst_cycles": 100664,
                "minimum_retrigger_cycles": 251659, "peak_pcm_magnitude": 786,
            }
            args.report.parent.mkdir(parents=True, exist_ok=True)
            args.report.write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
