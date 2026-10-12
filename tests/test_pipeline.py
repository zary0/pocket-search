# SPDX-License-Identifier: GPL-3.0-only
"""Unit fixtures validate release gates; these are NOT physical E2E tests."""
import copy
import os
from unittest.mock import patch
import json
from pathlib import Path
import struct
import subprocess
import tarfile
import tempfile
import unittest
import zlib
from scripts import pipeline as p


class ReleaseGateTests(unittest.TestCase):
    def test_readiness_requires_complete_build_and_explicit_lab_configuration(self):
        with patch.dict(os.environ, {'CHROMATIC_E2E_READY':'false'},clear=True):
            with self.assertRaises(p.Invalid):p.readiness(p.ROOT)
        configured={'CHROMATIC_E2E_READY':'true','CHROMATIC_LAB_DRIVER':'/opt/pocket-lab/driver','CHROMATIC_LAB_DRIVER_SHA256':'0'*64,'CHROMATIC_TEST_ORIGIN':'https://www.w3.org'}
        with patch.dict(os.environ,configured,clear=True):
            p.readiness(p.ROOT)
            with tempfile.TemporaryDirectory() as folder:
                with self.assertRaises(p.Invalid):p.readiness(Path(folder))
        for origin in ('http://fixture.example.org','https://fixture.example.org','https://www.w3.org:8443',
                       'https://user@www.w3.org','https://127.0.0.1','https://www.w3.org/?key=private'):
            configured['CHROMATIC_TEST_ORIGIN']=origin
            with self.subTest(origin=origin), patch.dict(os.environ,configured,clear=True):
                with self.assertRaises(p.Invalid):p.readiness(p.ROOT)

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
        (self.root / "assets").mkdir()
        (self.root / "docs").mkdir()
        (self.root / "examples").mkdir()
        (self.root / "ui").mkdir()
        (self.root / "devices/chromatic").mkdir(parents=True)
        (self.root / "third_party/licenses/unit-dependency").mkdir(parents=True)
        for name in ("LICENSE", "NOTICE.md", "THIRD_PARTY_NOTICES.md", "third_party/licenses/unit-dependency/LICENSE-MIT", "README.md", "CONTRIBUTING.md", "SECURITY.md", "devices/chromatic/README.md", "docs/releases.md", "assets/README.md", "ui/README.md", "ui/index.html", "examples/html-manifest.json", "third_party/dependencies.json"):
            (self.root / name).write_text("UNIT FIXTURE, not a release or physical result\n")
        (self.root / "assets/unit-logo.png").write_bytes(b"UNIT FIXTURE artwork bytes, not a physical capture")
        (self.root / "README.md").write_text('![Unit logo](assets/unit-logo.png)\n[Security](SECURITY.md)\n[Release policy](docs/releases.md)\n')
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
        (self.frames / "junit.xml").write_text('<testsuite tests="' + str(len(p.CASES)) + '">' + ''.join('<testcase name="' + case + '" />' for case in sorted(p.CASES)) + '</testsuite>')
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

    def test_companion_includes_notices_artwork_and_security_without_private_notes(self):
        (self.root / "assets/private-note.txt").write_text("UNTRACKED UNIT FIXTURE, must not be packaged")
        (self.root / "third_party/licenses/private-note.txt").write_text("UNTRACKED UNIT FIXTURE, must not be packaged")
        (self.root / "ui/private-note.txt").write_text("UNTRACKED UNIT FIXTURE, must not be packaged")
        self.make()
        manifest = p.verify_candidate(self.folder)
        entry = next(asset for asset in manifest["assets"] if asset["kind"] == "companion")
        prefix = "pocket-search-" + manifest["label"] + "/"
        with tarfile.open(self.folder / entry["name"]) as archive:
            names = {item.name for item in archive.getmembers()}
            for name in ["README.md", "SECURITY.md", "THIRD_PARTY_NOTICES.md", "third_party/licenses/unit-dependency/LICENSE-MIT", "assets/unit-logo.png", "assets/README.md", "docs/releases.md", "ui/README.md", "ui/index.html", "examples/html-manifest.json", "third_party/dependencies.json"]:
                with self.subTest(name=name):
                    self.assertIn(prefix + name, names)
                    self.assertEqual(archive.extractfile(prefix + name).read(), (self.root / name).read_bytes())
            self.assertNotIn(prefix + "assets/private-note.txt", names)
            self.assertNotIn(prefix + "third_party/licenses/private-note.txt", names)
            self.assertNotIn(prefix + "ui/private-note.txt", names)
            self.assertEqual(archive.getmember(prefix + "pocket-search").mode, 0o755)

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

    def test_incomplete_native_acceptance_and_short_endurance_cannot_pass(self):
        self.make(release=True); self.attach()
        original = self.report()
        additions = {"cancellation_retry", "persistent_cold_start", "native_image_processing",
                     "battery_standalone", "endurance_30_minutes"}
        for case in additions:
            report = copy.deepcopy(original)
            del report["cases"][case]
            p.write(self.frames / "report.json", report)
            with self.subTest(case=case), self.assertRaises(p.Invalid):
                p.evidence(self.folder, self.frames, self.policy)
        for key, value in [("endurance_seconds", 1799), ("battery_runtime_seconds", 119),
                           ("unexpected_resets", 1), ("persistent_store_failures", 1),
                           ("native_image_source_bytes", 0), ("native_decoded_image_bytes", 0), ("native_baseline_images_verified", 0),
                           ("native_png_images_verified", 0), ("native_progressive_images_verified", 0),
                           ("native_image_decoder_peak_bytes", 65537)]:
            report = copy.deepcopy(original)
            report["metrics"][key] = value
            p.write(self.frames / "report.json", report)
            with self.subTest(metric=key), self.assertRaises(p.Invalid):
                p.evidence(self.folder, self.frames, self.policy)

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
