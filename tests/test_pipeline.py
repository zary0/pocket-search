# SPDX-License-Identifier: GPL-3.0-only
"""Unit fixtures validate release gates; these are NOT physical E2E tests."""
import copy
import json
from pathlib import Path
import struct
import subprocess
import tempfile
import unittest
import zlib
from scripts import pipeline as p


class ReleaseGateTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "repository"
        self.root.mkdir()
        (self.root / "crates/client").mkdir(parents=True)
        (self.root / "ci").mkdir()
        (self.root / "Cargo.toml").write_text('[package]\nname="pocket-search"\nversion.workspace=true\n[workspace]\nmembers=["crates/client"]\n[workspace.package]\nversion="0.1.0"\n')
        (self.root / "crates/client/Cargo.toml").write_text('[package]\nname="chromatic-client"\nversion.workspace=true\n')
        (self.root / "Cargo.lock").write_text('[[package]]\nname="pocket-search"\nversion="0.1.0"\n[[package]]\nname="chromatic-client"\nversion="0.1.0"\n')
        for name in ("LICENSE", "NOTICE.md", "README.md"):
            (self.root / name).write_text("UNIT FIXTURE, not a release or physical result\n")
        self.policy = json.loads((p.ROOT / "ci/hardware-policy.json").read_text())
        p.write(self.root / "ci/hardware-policy.json", self.policy)
        for command in (("init", "-b", "main"), ("config", "user.name", "Unit fixture"), ("config", "user.email", "fixture@example.invalid"), ("add", "."), ("commit", "-m", "unit fixture")):
            subprocess.run(["git", "-C", str(self.root), *command], check=True, capture_output=True)
        self.commit = p.git(self.root, "rev-parse", "HEAD")
        subprocess.run(["git", "-C", str(self.root), "update-ref", "refs/remotes/origin/main", self.commit], check=True)
        self.binary = Path(self.temp.name) / "fixture-binary"
        info = json.dumps({"version": "0.1.0", "commit": self.commit, "protocol_version": 1})
        self.binary.write_text("#!/usr/bin/env python3\nprint(" + repr(info) + ")\n")
        self.binary.chmod(0o755)
        self.folder = Path(self.temp.name) / "candidate"
        self.frames = Path(self.temp.name) / "evidence"

    def make(self, release=False):
        if release:
            subprocess.run(["git", "-C", str(self.root), "tag", "-a", "v0.1.0", "-m", "unit tag"], check=True)
        p.candidate(self.root, self.folder, self.binary, "unit-test-host", "v0.1.0" if release else None)

    def attach(self):
        image = Path(self.temp.name) / "UNIT-FIXTURE.bin"
        image.write_bytes(b"\xe9" + b"UNIT FIXTURE NOT FLASHABLE" * 2)
        p.attach_firmware(self.folder, image, {"project": "pocket-search", "target": "esp32", "commit": self.commit,
            "version": "0.1.0", "sha256": p.sha(image), "deployment": "application-only"})

    def report(self):
        self.frames.mkdir()
        report = {"schema_version": 1, "execution_mode": "physical-native-wifi", "commit": self.commit,
                  "version": "0.1.0", "candidate_sha256": p.sha(self.folder / "candidate.json"),
                  "firmware_sha256": p.verify_candidate(self.folder)["firmware"]["sha256"],
                  "transport_source": "device-wifi", "host_proxy_used": False,
                  "clock_source": "native-sntp", "certificate_validation": True,
                  "frame_source": "physical-usb-video", "cases": {case: "passed" for case in p.CASES},
                  "frames": ["unit-query.png", "unit-results.png", "unit-reader.png"],
                  "metrics": {**self.policy["maximum"], **self.policy["minimum"]}}
        report["readback_sha256"] = report["firmware_sha256"]
        p.write(self.frames / "report.json", report)
        (self.frames / "junit.xml").write_text('<testsuite tests="5">' + ''.join('<testcase name="' + case + '" />' for case in sorted(p.CASES)) + '</testsuite>')
        def chunk(kind, value):
            return struct.pack(">I", len(value)) + kind + value + struct.pack(">I", zlib.crc32(kind + value) & 0xffffffff)
        # Explicit synthetic unit-test PNGs, never uploaded as physical evidence.
        data = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 160, 144, 8, 2, 0, 0, 0))
        for index, name in enumerate(report["frames"]):
            pixels = (b"\0" + bytes([index, 0, 0]) * 160) * 144
            image = data + chunk(b"IDAT", zlib.compress(pixels)) + chunk(b"IEND", b"")
            (self.frames / name).write_bytes(image)
        return report

    def test_shared_version_and_lock_drift(self):
        self.assertEqual(p.version(self.root), "0.1.0")
        (self.root / "Cargo.lock").write_text((self.root / "Cargo.lock").read_text().replace('version="0.1.0"', 'version="0.2.0"', 1))
        with self.assertRaises(p.Invalid): p.version(self.root)

    def test_wrong_tag_and_non_main_commit_are_rejected(self):
        with self.assertRaises(p.Invalid): p.release_tag(self.root, "v0.2.0")
        self.make(release=True)
        subprocess.run(["git", "-C", str(self.root), "update-ref", "refs/remotes/origin/main", self.commit], check=True)
        (self.root / "change").write_text("feature change")
        subprocess.run(["git", "-C", str(self.root), "add", "."], check=True)
        subprocess.run(["git", "-C", str(self.root), "commit", "-m", "feature"], check=True, capture_output=True)
        subprocess.run(["git", "-C", str(self.root), "tag", "-d", "v0.1.0"], check=True, capture_output=True)
        subprocess.run(["git", "-C", str(self.root), "tag", "-a", "v0.1.0", "-m", "non-main unit tag"], check=True)
        with self.assertRaises(p.Invalid): p.release_tag(self.root, "v0.1.0")

    def test_stale_binary_is_rejected(self):
        self.binary.write_text('#!/usr/bin/env python3\nprint(\'{"version":"0.0.0"}\')\n')
        with self.assertRaises(p.Invalid): p.candidate(self.root, self.folder, self.binary, "unit-host")
        self.assertFalse(self.folder.exists())

    def test_dirty_tracked_sources_cannot_form_a_candidate(self):
        (self.root / "README.md").write_text("uncommitted change")
        with self.assertRaises(p.Invalid): p.candidate(self.root, self.folder, self.binary, "unit-host")
        self.assertFalse(self.folder.exists())

    def test_development_candidate_cannot_be_promoted(self):
        self.make()
        self.assertIn("-dev." + self.commit[:12], p.verify_candidate(self.folder)["label"])
        with self.assertRaises(p.Invalid): p.promote(self.root, self.folder, self.frames, Path(self.temp.name) / "release")

    def test_asset_tampering_and_path_escape_are_rejected(self):
        self.make()
        manifest = p.verify_candidate(self.folder)
        source = self.folder / manifest["assets"][0]["name"]
        source.write_bytes(source.read_bytes() + b"changed")
        with self.assertRaises(p.Invalid): p.verify_candidate(self.folder)
        with self.assertRaises(p.Invalid): p.file_in(self.folder, "../outside")

    def test_physical_evidence_rejects_wrong_identity_and_execution_mode(self):
        self.make(release=True); self.attach()
        original = self.report()
        for key, value in [("commit", "0" * 40), ("readback_sha256", "0" * 64), ("candidate_sha256", "0" * 64), ("execution_mode", "emulator"), ("host_proxy_used", True), ("certificate_validation", False)]:
            report = copy.deepcopy(original); report[key] = value
            p.write(self.frames / "report.json", report)
            with self.subTest(key=key), self.assertRaises(p.Invalid): p.evidence(self.folder, self.frames, self.policy)

    def test_skipped_junit_and_nonfinite_budget_cannot_pass(self):
        self.make(release=True); self.attach()
        report = self.report()
        report["metrics"]["decoder_cycles_per_packet"] = float("nan")
        p.write(self.frames / "report.json", report)
        with self.assertRaises(p.Invalid): p.evidence(self.folder, self.frames, self.policy)
        report["metrics"]["decoder_cycles_per_packet"] = 1
        p.write(self.frames / "report.json", report)
        path = self.frames / "junit.xml"
        path.write_text(path.read_text().replace(" />", "><skipped /></testcase>", 1))
        with self.assertRaises(p.Invalid): p.evidence(self.folder, self.frames, self.policy)

    def test_missing_firmware_blocks_physical_qualification(self):
        self.make(release=True)
        with self.assertRaises(p.Invalid): p.evidence(self.folder, self.frames, self.policy)

    def test_promote_keeps_exact_candidate_bytes_and_binds_manifest(self):
        self.make(release=True); self.attach(); self.report()
        manifest = p.verify_candidate(self.folder)
        output = Path(self.temp.name) / "release"
        p.promote(self.root, self.folder, self.frames, output)
        self.assertEqual(p.sha(output / "candidate.json"), p.sha(self.folder / "candidate.json"))
        for entry in manifest["assets"]:
            self.assertEqual(p.sha(output / entry["name"]), entry["sha256"])
        qualified = json.loads((output / "release.json").read_text())
        self.assertTrue(qualified["qualified"])
        self.assertEqual(qualified["candidate_sha256"], p.sha(output / "candidate.json"))
        self.assertTrue((output / "SHA256SUMS").is_file())


if __name__ == "__main__":
    unittest.main()
