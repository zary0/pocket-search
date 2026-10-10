# SPDX-License-Identifier: GPL-3.0-only
"""Staging refusal cases, not FPGA build or physical deployment tests."""
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from scripts import prepare_chromatic_click as staging


class FPGAStagingBoundaries(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = self.root / "source"
        self.source.mkdir()
        self.output = self.root / "staged"

    def test_existing_destination_preserved_before_source_access(self):
        self.output.mkdir()
        note = self.output / "keep.txt"
        note.write_text("existing operator output")
        with patch.object(staging, "git") as git:
            with self.assertRaisesRegex(ValueError, "new directory"):
                staging.prepare(self.source, self.output)
            git.assert_not_called()
        self.assertEqual(note.read_text(), "existing operator output")

    def test_nested_and_symlink_destinations_refused(self):
        alias = self.root / "alias"
        alias.symlink_to(self.source, target_is_directory=True)
        for output in (self.source / "staged", alias / "staged", self.root):
            with self.subTest(output=output), patch.object(staging, "git") as git:
                with self.assertRaises(ValueError):
                    staging.prepare(self.source, output)
                git.assert_not_called()
        self.assertFalse((self.source / "staged").exists())

    def test_different_revision_and_dirty_sources_leave_no_output(self):
        with patch.object(staging, "git", return_value="0" * 40):
            with self.assertRaisesRegex(ValueError, "commit"):
                staging.prepare(self.source, self.output)
        with patch.object(staging, "git", side_effect=[staging.PIN["commit"], " M esp32t/src/top.v"]):
            with self.assertRaisesRegex(ValueError, "modifications"):
                staging.prepare(self.source, self.output)
        self.assertFalse(self.output.exists())

    def test_uninitialized_or_wrong_submodule_refused(self):
        for status in ("-" + staging.PIN["gameboy_mister_commit"] + " core\n",
                       "+" + staging.PIN["gameboy_mister_commit"] + " core\n",
                       " " + "0" * 40 + " core\n"):
            with self.subTest(status=status):
                with patch.object(staging, "git", side_effect=[staging.PIN["commit"], "", status.strip()]):
                    with patch.object(staging.subprocess, "check_output", return_value=status):
                        with self.assertRaises(ValueError):
                            staging.prepare(self.source, self.output)
        self.assertFalse(self.output.exists())

    def test_dirty_submodule_refused_even_when_git_configuration_hides_it(self):
        def git(path, *args):
            return subprocess.check_output([
                "git", "-C", str(path), "-c", "user.name=Fixture",
                "-c", "user.email=fixture@example.invalid", "-c", "commit.gpgsign=false",
                "-c", "core.hooksPath=/dev/null", *args,
            ], text=True, stderr=subprocess.PIPE).strip()

        module = self.root / "module"
        module.mkdir()
        git(module, "init", "-q")
        (module / "core.v").write_text("original pinned source\n")
        git(module, "add", "core.v")
        git(module, "commit", "-qm", "Original fixture")
        git(self.source, "init", "-q")
        git(self.source, "-c", "protocol.file.allow=always", "submodule", "add",
            "--quiet", str(module), "core")
        git(self.source, "commit", "-qm", "Pin fixture submodule")
        commit = git(self.source, "rev-parse", "HEAD")
        (self.source / "core/core.v").write_text("modified tracked source\n")
        for mode in ("all", "dirty"):
            with self.subTest(ignore_mode=mode):
                git(self.source, "config", "submodule.core.ignore", mode)
                # Establish that the ordinary check hides this real change.
                self.assertEqual(git(self.source, "status", "--porcelain", "--untracked-files=no"), "")
                with patch.dict(staging.PIN, {"commit": commit}):
                    with self.assertRaisesRegex(ValueError, "tracked modifications"):
                        staging.prepare(self.source, self.output)
                self.assertFalse(self.output.exists())


if __name__ == "__main__":
    unittest.main()
