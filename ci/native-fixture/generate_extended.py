#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Original decoder fixtures; no source photographs are converted or bundled."""
from pathlib import Path
import hashlib
import json
import struct
import zlib
from PIL import Image

ROOT = Path(__file__).resolve().parent

def chunk(kind, data):
    return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))

def png(width, height, depth, color, rows, extra=b'', split=False):
    compressed = zlib.compress(b''.join(rows), 9)
    parts = [compressed[i:i+7] for i in range(0, len(compressed), 7)] if split else [compressed]
    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, depth, color, 0, 0, 0)) + extra + b''.join(chunk(b'IDAT', part) for part in parts) + chunk(b'IEND', b'')

def main():
    colors = [(255,0,0,255), (0,255,0,128), (0,0,255,0), (255,255,0,64), (0,255,255,255), (0,0,0,255)]
    row = bytes(v for x in range(192) for v in colors[x//32])
    rows=[];previous=bytes(len(row))
    for y in range(32):
        method=y%5;filtered=bytearray()
        for x,value in enumerate(row):
            a=row[x-4] if x>=4 else 0;b=previous[x];c=previous[x-4] if x>=4 else 0
            p=a+b-c;dist=[abs(p-a),abs(p-b),abs(p-c)];paeth=[a,b,c][dist.index(min(dist))]
            predictor=[0,a,b,(a+b)//2,paeth][method];filtered.append((value-predictor)&255)
        rows.append(bytes([method])+filtered);previous=row
    assets={'bars.png':png(192,32,8,6,rows,split=True)}
    palette=bytes([255,0,0,0,0,255]);alpha=bytes([255,0]);packed=bytes([0x01]*8+[0])
    assets['palette.png']=png(17,9,4,3,[b'\0'+packed]*9,chunk(b'PLTE',palette)+chunk(b'tRNS',alpha))
    jpeg=Image.new('RGB',(64,128));pixels=jpeg.load()
    for y in range(128):
        for x in range(64):pixels[x,y]=colors[(x//8)%6][:3]
    jpeg.save(ROOT/'bars-progressive.jpg',format='JPEG',quality=90,subsampling=2,progressive=True)
    for name,data in assets.items():(ROOT/name).write_bytes(data)
    names=['bars.png','palette.png','bars-progressive.jpg']
    metadata={'schema_version':1,'provenance':'Original deterministic color/filter/alpha fixtures authored for Pocket Search; no third-party photographs','license':'GPL-3.0-only','source':'ci/native-fixture/generate_extended.py','assets':{name:{'bytes':(ROOT/name).stat().st_size,'sha256':hashlib.sha256((ROOT/name).read_bytes()).hexdigest()} for name in names}}
    (ROOT/'extended-assets.json').write_text(json.dumps(metadata,indent=2)+'\n')

if __name__=='__main__':main()
