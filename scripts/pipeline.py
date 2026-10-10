#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Versioned candidates and fail-closed physical E2E qualification."""
import argparse
import gzip
import hashlib
import io
import json
import math
import os
from pathlib import Path
import re
import struct
import subprocess
import tarfile
import tomllib
import xml.etree.ElementTree as ET
import zlib

ROOT = Path(__file__).resolve().parents[1]
SEMVER = r"(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)"
CASES = {
    "native_wifi_verified_https", "query_result_open", "utf8_pagination",
    "empty_results_network_errors", "cpu_memory_budget",
}


class Invalid(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise Invalid(message)


def git(root, *args):
    return subprocess.check_output(["git", "-C", str(root), *args], text=True).strip()


def sha(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def write(path, value):
    path.write_text(json.dumps(value, sort_keys=True, indent=2) + "\n")


def version(root=ROOT):
    manifest = tomllib.loads((root / "Cargo.toml").read_text())
    value = manifest["workspace"]["package"]["version"]
    require(re.fullmatch(SEMVER, value), "Workspace version must be X.Y.Z")
    members = [".", *manifest["workspace"]["members"]]
    names = set()
    for member in members:
        package = tomllib.loads((root / member / "Cargo.toml").read_text())["package"]
        require(package["version"] == {"workspace": True}, "Every package must inherit the workspace version")
        names.add(package["name"])
    lock = tomllib.loads((root / "Cargo.lock").read_text())
    for name in names:
        matches = [p for p in lock["package"] if p["name"] == name and "source" not in p]
        require(len(matches) == 1 and matches[0]["version"] == value, "Cargo.lock version mismatch")
    return value


def release_tag(root, tag):
    require(tag == "v" + version(root), "Tag must exactly match v + workspace version")
    require(git(root, "cat-file", "-t", "refs/tags/" + tag) == "tag", "Release tag must be annotated")
    require(git(root, "rev-parse", tag + "^{commit}") == git(root, "rev-parse", "HEAD"), "Tag/checkout mismatch")
    result = subprocess.run(["git", "-C", str(root), "merge-base", "--is-ancestor", "HEAD", "origin/main"], capture_output=True)
    require(result.returncode == 0, "Release commit must belong to main history")


def file_in(folder, name):
    require(isinstance(name, str) and Path(name).name == name and name not in (".", ".."), "Asset must be a plain filename")
    path = folder / name
    require(path.is_file() and not path.is_symlink(), "Missing or symlinked asset")
    return path


def asset(folder, name, kind):
    path = file_in(folder, name)
    return {"name": name, "kind": kind, "size": path.stat().st_size, "sha256": sha(path)}


def verify_candidate(folder):
    manifest = json.loads(file_in(folder, "candidate.json").read_text())
    require(manifest["schema_version"] == 1, "Unknown candidate schema")
    require(re.fullmatch(r"[0-9a-f]{40}", manifest["commit"]), "Invalid commit")
    require(re.fullmatch(SEMVER, manifest["version"]), "Invalid version")
    expected = "v" + manifest["version"]
    if manifest["channel"] == "development":
        expected += "-dev." + manifest["commit"][:12]
    else:
        require(manifest["channel"] == "release" and manifest["tag"] == expected, "Invalid release channel/tag")
    require(manifest["label"] == expected and manifest["protocol_version"] == 1, "Candidate label/protocol mismatch")
    require(re.fullmatch(r"[A-Za-z0-9_-]+", manifest["host_target"]), "Invalid candidate host target")
    expected_names = {"source": "pocket-search-" + expected + "-source.zip",
                      "companion": "pocket-search-" + expected + "-" + manifest["host_target"] + ".tar.gz",
                      "firmware": "chromatic-" + expected + ".bin"}
    names, kinds = set(), []
    for entry in manifest["assets"]:
        require(entry["name"] not in names, "Duplicate candidate asset")
        names.add(entry["name"])
        kinds.append(entry["kind"])
        require(entry["kind"] in expected_names and entry["name"] == expected_names[entry["kind"]], "Asset filename/version mismatch")
        actual = asset(folder, entry["name"], entry["kind"])
        require(actual == entry, "Candidate checksum or size mismatch")
    require(sorted(kinds) == (["companion", "source"] if manifest["firmware"] is None else ["companion", "firmware", "source"]), "Unexpected or missing candidate asset kinds")
    if manifest["firmware"] is not None:
        firmware = next(a for a in manifest["assets"] if a["kind"] == "firmware")
        require(manifest["firmware"] == {**firmware, "target": "esp32", "deployment": "application-only"}, "Firmware manifest/asset mismatch")
    return manifest


def candidate(root, output, binary, target, tag=None):
    current_version = version(root)
    require(not git(root, "status", "--porcelain", "--untracked-files=no"), "Tracked working tree must be clean")
    commit = git(root, "rev-parse", "HEAD")
    if tag:
        release_tag(root, tag)
    label = tag or f"v{current_version}-dev.{commit[:12]}"
    require(re.fullmatch(r"[A-Za-z0-9_-]+", target), "Invalid host target")
    require(binary.is_file() and binary.stat().st_size > 0, "Missing compiled companion")
    build_info = json.loads(subprocess.check_output([str(binary.resolve()), "--build-info"], text=True))
    require(build_info == {"version": current_version, "commit": commit, "protocol_version": 1}, "Companion binary build/source/version mismatch")
    require(not output.exists(), "Candidate output must be a new directory")
    output.mkdir(parents=True)
    prefix = "pocket-search-" + label
    source_name = prefix + "-source.zip"
    subprocess.run(["git", "-C", str(root), "archive", "--format=zip", "--prefix=" + prefix + "/", "HEAD", "-o", str((output / source_name).resolve())], check=True)
    binary_name = prefix + "-" + target + ".tar.gz"
    timestamp = int(git(root, "show", "-s", "--format=%ct", "HEAD"))
    documentation = ["LICENSE", "NOTICE.md", "THIRD_PARTY_NOTICES.md", "README.md", "CONTRIBUTING.md", "SECURITY.md", "devices/chromatic/README.md"]
    documentation.extend(name for name in git(root, "ls-files", "-z", "--", "docs", "assets", "ui", "third_party/licenses").split("\0") if name)
    with (output / binary_name).open("wb") as stream, gzip.GzipFile(filename="", fileobj=stream, mode="wb", mtime=timestamp) as compressed, tarfile.open(fileobj=compressed, mode="w") as archive:
        for name, path in [("pocket-search", binary), *[(name, root / name) for name in documentation]]:
            require(path.is_file() and not path.is_symlink(), "Missing or symlinked package file: " + name)
            data = path.read_bytes()
            info = tarfile.TarInfo(prefix + "/" + name)
            info.size, info.mtime, info.mode = len(data), timestamp, 0o755 if name == "pocket-search" else 0o644
            archive.addfile(info, io.BytesIO(data))
    manifest = {"schema_version": 1, "version": current_version, "commit": commit,
                "label": label, "channel": "release" if tag else "development", "tag": tag,
                "protocol_version": 1, "host_target": target,
                "assets": [asset(output, source_name, "source"), asset(output, binary_name, "companion")],
                "firmware": None, "phase": "built-candidate"}
    write(output / "candidate.json", manifest)
    return manifest


def attach_firmware(folder, image, metadata):
    manifest = verify_candidate(folder)
    require(manifest["firmware"] is None, "Firmware already attached")
    require(metadata["project"] == "pocket-search" and metadata["target"] == "esp32", "Wrong firmware project/CPU")
    require(metadata["commit"] == manifest["commit"] and metadata["version"] == manifest["version"], "Firmware source/version mismatch")
    require(metadata["sha256"] == sha(image), "Firmware build digest mismatch")
    require(metadata["deployment"] == "application-only", "Only an application image may enter the candidate")
    size = image.stat().st_size
    require(24 <= size <= 2 * 1024 * 1024, "Invalid application image size")
    with image.open("rb") as stream:
        require(stream.read(1) == b"\xe9", "Not an ESP application image")
    name = "chromatic-" + manifest["label"] + ".bin"
    destination = folder / name
    require(not destination.exists(), "Firmware destination already exists")
    destination.write_bytes(image.read_bytes())
    entry = asset(folder, name, "firmware")
    manifest["assets"].append(entry)
    manifest["firmware"] = {**entry, "target": "esp32", "deployment": "application-only"}
    write(folder / "candidate.json", manifest)


def png(path):
    data = path.read_bytes()
    require(len(data) <= 12 * 1024 * 1024 and data[:8] == b"\x89PNG\r\n\x1a\n", "Invalid screenshot")
    offset, dimensions, image_data, kind = 8, None, False, None
    while offset < len(data):
        require(offset + 12 <= len(data), "Truncated PNG chunk")
        length = struct.unpack(">I", data[offset:offset + 4])[0]
        end = offset + 12 + length
        require(end <= len(data), "Truncated PNG data")
        kind, payload = data[offset + 4:offset + 8], data[offset + 8:end - 4]
        crc = struct.unpack(">I", data[end - 4:end])[0]
        require(zlib.crc32(kind + payload) & 0xffffffff == crc, "Invalid PNG CRC")
        if kind == b"IHDR":
            require(dimensions is None and length == 13, "Invalid PNG header")
            dimensions = struct.unpack(">II", payload[:8])
        if kind == b"IDAT":
            image_data = image_data or bool(payload)
        offset = end
        if kind == b"IEND":
            require(length == 0 and end == len(data) and image_data, "Invalid PNG ending")
            break
    require(kind == b"IEND" and dimensions is not None, "Incomplete PNG")
    width, height = dimensions
    require(width % 160 == 0 and height % 144 == 0 and width // 160 == height // 144 and 1 <= width // 160 <= 8, "Wrong physical screen dimensions")


def metric(value):
    require(type(value) in (int, float) and math.isfinite(value) and value >= 0, "Invalid performance metric")
    return value


def evidence(folder, evidence_dir, policy):
    manifest = verify_candidate(folder)
    require(manifest["firmware"] is not None, "Physical qualification requires native firmware")
    report = json.loads(file_in(evidence_dir, "report.json").read_text())
    require(report["schema_version"] == 1 and report["execution_mode"] == "physical-native-wifi", "Emulator/host/stub reports cannot qualify")
    require(report["commit"] == manifest["commit"] and report["version"] == manifest["version"], "E2E revision mismatch")
    require(report["candidate_sha256"] == sha(folder / "candidate.json"), "E2E tested a different candidate")
    require(report["firmware_sha256"] == manifest["firmware"]["sha256"] == report["readback_sha256"], "Installed firmware/read-back mismatch")
    require(report["transport_source"] == "device-wifi" and report["host_proxy_used"] is False, "Native Wi-Fi path required")
    require(report["clock_source"] == "native-sntp" and report["certificate_validation"] is True, "Verified HTTPS and native clock required")
    require(report["frame_source"] == "physical-usb-video", "Physical capture required")
    require(set(report["cases"]) == CASES and all(status == "passed" for status in report["cases"].values()), "Missing, failed, or skipped physical case")
    metrics = report["metrics"]
    for key, limit in policy["maximum"].items():
        require(metric(metrics[key]) <= limit, "Physical maximum budget exceeded: " + key)
    for key, limit in policy["minimum"].items():
        require(metric(metrics[key]) >= limit, "Physical minimum budget missed: " + key)
    frames = report["frames"]
    require(len(frames) >= 3 and len(set(frames)) == len(frames), "At least three distinct captures required")
    frame_hashes = set()
    for name in frames:
        path = file_in(evidence_dir, name)
        png(path)
        frame_hashes.add(sha(path))
    require(len(frame_hashes) >= 3, "Reused screenshot bytes cannot represent three UI stages")
    suite = ET.parse(file_in(evidence_dir, "junit.xml")).getroot()
    cases = list(suite.iter("testcase"))
    require(len(cases) == len(CASES) and {c.attrib["name"] for c in cases} == CASES, "JUnit case mismatch")
    require(all(next(c.iter(kind), None) is None for c in cases for kind in ("failure", "error", "skipped")), "JUnit contains a failed/skipped case")
    for node in suite.iter():
        if node.tag in ("testsuite", "testsuites"):
            require(all(int(node.attrib.get(key, 0)) == 0 for key in ("failures", "errors", "skipped")), "JUnit aggregate failure")
    return {key: report[key] for key in ("schema_version", "execution_mode", "commit", "version", "candidate_sha256", "firmware_sha256", "readback_sha256", "transport_source", "host_proxy_used", "clock_source", "certificate_validation", "frame_source", "cases", "metrics", "frames")}


def promote(root, folder, evidence_dir, output):
    manifest = verify_candidate(folder)
    require(manifest["channel"] == "release", "Development candidates are not releases")
    release_tag(root, manifest["tag"])
    require(manifest["commit"] == git(root, "rev-parse", "HEAD"), "Release checkout/candidate mismatch")
    policy = json.loads((root / "ci/hardware-policy.json").read_text())
    report = evidence(folder, evidence_dir, policy)
    require(not output.exists(), "Release output already exists")
    output.mkdir(parents=True)
    # Promote exact tested bytes; do not rebuild or modify the candidate manifest.
    for entry in manifest["assets"]:
        (output / entry["name"]).write_bytes(file_in(folder, entry["name"]).read_bytes())
    (output / "candidate.json").write_bytes((folder / "candidate.json").read_bytes())
    write(output / "e2e.json", report)
    write(output / "release.json", {"schema_version": 1, "tag": manifest["tag"],
        "version": manifest["version"], "commit": manifest["commit"], "qualified": True,
        "candidate_sha256": sha(folder / "candidate.json"), "e2e_sha256": sha(output / "e2e.json")})
    (output / "junit.xml").write_bytes((evidence_dir / "junit.xml").read_bytes())
    for frame in report["frames"]:
        (output / frame).write_bytes(file_in(evidence_dir, frame).read_bytes())
    checksums = [f"{sha(path)}  {path.name}" for path in sorted(output.iterdir())]
    (output / "SHA256SUMS").write_text("\n".join(checksums) + "\n")


def readiness(root):
    require(os.environ.get("CHROMATIC_E2E_READY") == "true", "Physical runner/adapter not enabled; release remains blocked")
    require((root / "devices/chromatic/firmware/CMakeLists.txt").is_file(), "Pocket Search MCU firmware project is not implemented")
    for key in ("CHROMATIC_LAB_DRIVER", "CHROMATIC_LAB_DRIVER_SHA256", "CHROMATIC_TEST_ORIGIN"):
        require(os.environ.get(key), "Missing lab configuration: " + key)
    require(os.environ["CHROMATIC_TEST_ORIGIN"].startswith("https://"), "Lab origin must use verified HTTPS")


def main():
    parser = argparse.ArgumentParser()
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("version")
    commands.add_parser("readiness")
    package = commands.add_parser("candidate")
    package.add_argument("--output", type=Path, required=True)
    package.add_argument("--binary", type=Path, required=True)
    package.add_argument("--target", required=True)
    package.add_argument("--tag")
    firmware = commands.add_parser("firmware")
    firmware.add_argument("--candidate", type=Path, required=True)
    firmware.add_argument("--image", type=Path, required=True)
    firmware.add_argument("--metadata", type=Path, required=True)
    validate = commands.add_parser("evidence")
    validate.add_argument("--candidate", type=Path, required=True)
    validate.add_argument("--evidence", type=Path, required=True)
    promotion = commands.add_parser("promote")
    promotion.add_argument("--candidate", type=Path, required=True)
    promotion.add_argument("--evidence", type=Path, required=True)
    promotion.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        if args.command == "version": print(version())
        elif args.command == "readiness": readiness(ROOT)
        elif args.command == "candidate": candidate(ROOT, args.output, args.binary, args.target, args.tag)
        elif args.command == "firmware": attach_firmware(args.candidate, args.image, json.loads(args.metadata.read_text()))
        elif args.command == "evidence": evidence(args.candidate, args.evidence, json.loads((ROOT / "ci/hardware-policy.json").read_text()))
        elif args.command == "promote": promote(ROOT, args.candidate, args.evidence, args.output)
    except (Invalid, KeyError, OSError, ValueError, subprocess.CalledProcessError) as error:
        parser.exit(1, str(error) + "\n")


if __name__ == "__main__":
    main()
