# SPDX-License-Identifier: GPL-3.0-only
"""Every locked registry dependency needs versioned source and intact notices."""
import difflib
import gzip
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
        self.assertTrue(metadata['local_modifications'])
        upstream=gzip.decompress((ROOT/metadata['upstream_archive']).read_bytes())
        self.assertEqual(hashlib.sha256(upstream).hexdigest(),metadata['upstream_sha256'])
        patch=''.join(difflib.unified_diff(upstream.decode().splitlines(keepends=True),source.decode().splitlines(keepends=True),fromfile='a/stb_image.h',tofile='b/stb_image.h')).encode()
        self.assertEqual(patch,(base/'stb-image.patch').read_bytes())
        self.assertEqual(hashlib.sha256(patch).hexdigest(),metadata['patch_sha256'])
        text=(base/'LICENSE-MIT.txt').read_text()
        self.assertIn('Copyright (c) 2017 Sean Barrett',text)
        self.assertIn(text.rstrip(),source.decode())

    def test_vendored_woff2_brotli_and_ofl_font_fixtures(self):
        base=ROOT/'devices/chromatic/components/pocket_search_ui/third_party'
        info=json.loads((base/'sources.json').read_text())
        self.assertEqual(info['woff2']['commit'],'fb9c3379f2605b10f3e8f1d9636664ab5576775c')
        self.assertEqual(info['brotli']['commit'],'392b261debb0f478811ddacb98b118da36b06158')
        actual={str(p.relative_to(base)) for p in base.rglob('*') if p.is_file() and p.name!='sources.json'}
        self.assertEqual(actual,set(info['files_sha256']))
        for name,sha in info['files_sha256'].items():self.assertEqual(hashlib.sha256((base/name).read_bytes()).hexdigest(),sha)
        for name in ['woff2','brotli']:self.assertIn('Permission is hereby granted',(base/name/'LICENSE').read_text())
        fonts=ROOT/'tests/fixtures/webfonts';provenance=json.loads((fonts/'provenance.json').read_text())
        self.assertEqual(provenance['license'],'OFL-1.1');self.assertIn('SIL OPEN FONT LICENSE',(fonts/'OFL.txt').read_text())
        for name,sha in provenance['files'].items():self.assertEqual(hashlib.sha256((fonts/name).read_bytes()).hexdigest(),sha)
