#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Preferred source for the original lab JPEG; Pillow is generation-only.

JPEG encoder versions can change bytes. Commit and review the resulting fixture
and asset.json together; the server always serves those exact committed bytes.
"""
import argparse
from PIL import Image


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("output", help="A new output file; existing files are preserved")
    args = parser.parse_args()
    image = Image.new("RGB", (192, 128))
    colors = ((232, 32, 32), (32, 192, 32), (32, 32, 232))
    for y in range(128):
        for x in range(192):
            image.putpixel((x, y), colors[x // 64])
    with open(args.output, "xb") as output:
        image.save(output, format="JPEG", quality=90, subsampling=0,
                   optimize=False, progressive=False)


if __name__ == "__main__":
    main()
