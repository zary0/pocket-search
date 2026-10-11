# SPDX-License-Identifier: GPL-3.0-only
"""Real Git input-boundary tests; never firmware installation evidence."""
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
from scripts import build_chromatic_firmware as firmware


def git(root, *args):
    return subprocess.check_output(["git", "-C", str(root), "-c", "user.name=Fixture",
        "-c", "user.email=fixture@example.invalid", "-c", "commit.gpgsign=false",
        "-c", "core.hooksPath=/dev/null", *args], text=True, stderr=subprocess.PIPE).strip()


class NativeBuildInputs(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = self.root / "source"
        self.source.mkdir()
        git(self.source, "init", "-q")
        (self.source / "source.c").write_text("original tracked source\n")
        git(self.source, "add", "source.c")
        git(self.source, "commit", "-qm", "Original fixture")
        self.commit = git(self.source, "rev-parse", "HEAD")

    def test_wrong_revision_and_tracked_modifications_refused(self):
        with self.assertRaisesRegex(ValueError, "revision"):
            firmware.clean_checkout(self.source, "0" * 40)
        (self.source / "source.c").write_text("modified source\n")
        with self.assertRaisesRegex(ValueError, "modifications"):
            firmware.clean_checkout(self.source, self.commit)

    def test_only_tracked_source_copied_no_credentials_or_git(self):
        (self.source / "private.lic").write_text("original synthetic non-credential fixture")
        (self.source / "nvs.bin").write_bytes(b"original synthetic state")
        firmware.clean_checkout(self.source, self.commit)
        destination = self.root / "copy"
        firmware.tracked_copy(self.source.resolve(), destination)
        self.assertEqual({p.name for p in destination.iterdir()}, {"source.c"})

    def test_tracked_symlink_refused(self):
        (self.source / "alias").symlink_to("source.c")
        git(self.source, "add", "alias")
        git(self.source, "commit", "-qm", "Unsupported fixture path")
        with self.assertRaisesRegex(ValueError, "Unsupported"):
            firmware.tracked_copy(self.source.resolve(), self.root / "copy")

    def test_existing_and_overlapping_output_preserved(self):
        output = self.root / "existing"
        output.mkdir()
        note = output / "keep"
        note.write_text("operator-owned output")
        for target in (output, self.source / "build", self.root):
            with self.subTest(target=target), patch.object(firmware, "clean_checkout") as check:
                with self.assertRaises(ValueError):
                    firmware.stage(self.source, self.root / "lvgl", target)
                check.assert_not_called()
        self.assertEqual(note.read_text(), "operator-owned output")

    def test_hidden_submodule_edits_and_wrong_submodule_refused(self):
        module = self.root / "module"
        module.mkdir()
        git(module, "init", "-q")
        (module / "core.c").write_text("original module\n")
        git(module, "add", "core.c")
        git(module, "commit", "-qm", "Original module")
        git(self.source, "-c", "protocol.file.allow=always", "submodule", "add", "--quiet", str(module), "core")
        git(self.source, "commit", "-qm", "Pin module")
        commit = git(self.source, "rev-parse", "HEAD")
        (self.source / "core/core.c").write_text("changed module\n")
        for mode in ("all", "dirty"):
            with self.subTest(mode=mode):
                git(self.source, "config", "submodule.core.ignore", mode)
                self.assertEqual(git(self.source, "status", "--porcelain", "--untracked-files=no"), "")
                with self.assertRaisesRegex(ValueError, "modifications"):
                    firmware.clean_checkout(self.source, commit)


if __name__ == "__main__":
    unittest.main()
