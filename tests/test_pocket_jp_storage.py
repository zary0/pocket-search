# SPDX-License-Identifier: GPL-3.0-only
import hashlib,json,re,unittest
from pathlib import Path
class PocketJPStorage(unittest.TestCase):
    def test_all_original_pixels_and_coverage_are_lossless(self):
        root=Path(__file__).resolve().parents[1]/"devices/chromatic/firmware/overlay/components/fonts"
        text=(root/"pocket_jp_data.h").read_text();arrays={name:list(map(int,filter(None,re.sub(r"\s+","",body).split(",")))) for name,body in re.findall(r"(\w+)\[\] = \{(.*?)\};",text,re.S)}
        reference=json.loads((root/"pocket_jp_storage.json").read_text());covered=[];pixels=bytearray()
        for c in range(65536):
            if not arrays["presence"][c//8]&(1<<(c%8)):continue
            page=c//256;index=arrays["ranks"][page]+sum(b.bit_count() for b in arrays["presence"][page*32:c//8])+(arrays["presence"][c//8]&((1<<(c%8))-1)).bit_count()
            self.assertEqual(index,len(covered));covered.append(c);bit=arrays["checkpoints"][index//8];glyph=bytearray()
            for _ in range((index%8+1)*18):
                node=0;depth=0
                while node>=0:
                    self.assertLess(depth,13);node=arrays["nodes"][node*2+((arrays["pixels"][bit//8]>>(7-bit%8))&1)];bit+=1;depth+=1
                glyph.append(-node-1)
            pixels.extend(glyph[-18:])
        self.assertEqual(len(covered),reference["glyphs"])
        self.assertEqual(hashlib.sha256(pixels).hexdigest(),reference["uncompressed_pixels_sha256"])
        self.assertEqual(hashlib.sha256(b"".join(c.to_bytes(2,"little") for c in covered)).hexdigest(),reference["codepoints_sha256"])
