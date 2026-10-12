#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Invoke an operator-installed lab driver; never synthesize physical passes."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
from pipeline import ROOT, Invalid, attach_firmware, evidence, readiness, require, sha, verify_candidate


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    args = parser.parse_args()
    readiness(ROOT)
    driver = Path(os.environ["CHROMATIC_LAB_DRIVER"])
    require(driver.is_absolute() and driver.is_file() and os.access(driver, os.X_OK), "Lab driver must be an installed executable")
    require(not driver.resolve().is_relative_to(ROOT), "Lab driver must live outside repository-controlled files")
    require(sha(driver) == os.environ["CHROMATIC_LAB_DRIVER_SHA256"], "Installed lab driver digest mismatch")
    manifest = verify_candidate(args.candidate)
    require(not args.evidence.exists(), "Evidence output must be new")
    args.evidence.mkdir(parents=True)
    with tempfile.TemporaryDirectory(prefix="chromatic-lab-") as temp:
        build = Path(temp)
        # Private build logs stay local; only selected sanitized evidence is uploaded.
        for command, extra, timeout in [
            ("build", ["--source", str(ROOT), "--commit", manifest["commit"], "--version", manifest["version"], "--output", str(build)], 1200),
            ("e2e", ["--candidate", str(args.candidate.resolve()), "--origin", os.environ["CHROMATIC_TEST_ORIGIN"], "--output", str(args.evidence.resolve())], 3600),
        ]:
            with (build / (command + ".private.log")).open("wb") as log:
                result = subprocess.run([str(driver), command, *extra], stdout=log, stderr=subprocess.STDOUT, timeout=timeout)
            require(result.returncode == 0, "Lab driver phase failed: " + command)
            if command == "build":
                attach_firmware(args.candidate, build / "application.bin", json.loads((build / "firmware.json").read_text()))
        policy = json.loads((ROOT / "ci/hardware-policy.json").read_text())
        sanitized = evidence(args.candidate, args.evidence, policy)
        # Remove unsupported report fields and unlisted files before upload.
        (args.evidence / "report.json").write_text(json.dumps(sanitized, sort_keys=True, indent=2) + "\n")
        allowed = {"report.json", "junit.xml", *sanitized["frames"]}
        require({p.name for p in args.evidence.iterdir()} <= allowed, "Unlisted evidence files cannot be uploaded")


if __name__ == "__main__":
    try:
        main()
    except (Invalid, KeyError, OSError, ValueError, subprocess.TimeoutExpired) as error:
        raise SystemExit(str(error))
