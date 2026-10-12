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

    def test_vendored_native_jpeg_decoder(self):
        base=ROOT/'devices/chromatic/components/pocket_search_ui/vendor'
        metadata=json.loads((base/'stb-image.json').read_text())
        self.assertEqual(metadata['commit'],'013ac3beddff3dbffafd5177e7972067cd2b5083')
        source=(base/'stb_image.h').read_bytes()
        self.assertEqual(hashlib.sha256(source).hexdigest(),metadata['sha256'])
        self.assertFalse(metadata['local_modifications'])
        text=(base/'LICENSE-MIT.txt').read_text()
        self.assertIn('Copyright (c) 2017 Sean Barrett',text)
        self.assertIn(text.rstrip(),source.decode())
