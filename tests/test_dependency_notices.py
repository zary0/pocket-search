# SPDX-License-Identifier: GPL-3.0-only
"""Every locked registry dependency needs versioned source and intact notices."""
import hashlib
import json
from pathlib import Path
import tomllib
import unittest
ROOT=Path(__file__).resolve().parents[1]
class DependencyNotices(unittest.TestCase):
    def test_locked_inventory_and_notice_hashes(self):
        locked={(p['name'],p['version']):p['checksum'] for p in tomllib.loads((ROOT/'Cargo.lock').read_text())['package'] if p.get('source','').startswith('registry+')}
        inventory=json.loads((ROOT/'third_party/dependencies.json').read_text())
        self.assertEqual(len(inventory),len(locked))
        self.assertEqual({(p['name'],p['version']):p['checksum'] for p in inventory},locked)
        for p in inventory:
            self.assertEqual(p['source'],f"https://crates.io/api/v1/crates/{p['name']}/{p['version']}/download")
            self.assertTrue(p['files'])
            for f in p['files']:
                path=Path(f['path']);self.assertFalse(path.is_absolute());self.assertNotIn('..',path.parts)
                self.assertEqual(hashlib.sha256((ROOT/path).read_bytes()).hexdigest(),f['sha256'])
