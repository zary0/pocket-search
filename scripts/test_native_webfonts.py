#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Host ASan/UBSan qualification of the exact native bounded WOFF2 sources."""
from pathlib import Path
import os
import subprocess
import tempfile
ROOT=Path(__file__).resolve().parents[1]
def main():
    src=ROOT/"devices/chromatic/components/pocket_search_ui"
    b=src/"third_party/brotli/c";w=src/"third_party/woff2"
    sources=[b/"common"/x for x in ("constants.c","context.c","dictionary.c","platform.c","shared_dictionary.c","transform.c")]
    sources += [b/"dec"/x for x in ("bit_reader.c","decode.c","huffman.c","state.c","static_init.c","prefix.c")]
    sources += [w/"src"/x for x in ("woff2_dec.cc","woff2_common.cc","table_tags.cc","variable_length.cc","woff2_out.cc")]+[src/"pocket_woff2.cpp",ROOT/"tests/test_native_webfonts.cc"]
    with tempfile.TemporaryDirectory(prefix="pocket-webfonts-") as tmp:
        objects=[]
        flags=["-O1","-g","-fsanitize=address,undefined","-fno-omit-frame-pointer","-I"+str(src),"-I"+str(b/"include"),"-I"+str(w/"include")]
        for source in sources:
            cpp=source.suffix in (".cc",".cpp");compiler=os.environ.get("CXX" if cpp else "CC","c++" if cpp else "cc");obj=Path(tmp)/(source.stem+".o")
            subprocess.run([compiler]+(["-std=c++17","-fexceptions"] if cpp else ["-std=c11"])+flags+["-c",str(source),"-o",str(obj)],check=True);objects.append(str(obj))
        exe=Path(tmp)/"test-fonts"
        subprocess.run([os.environ.get("CXX","c++"),"-fsanitize=address,undefined"]+objects+["-o",str(exe)],check=True)
        fixtures=ROOT/"tests/fixtures/webfonts"
        subprocess.run([str(exe)]+[str(fixtures/f) for f in ("noto-jp-subset.woff2","noto-jp-reference.ttf","noto-jp-budget-rejection.woff2")],check=True)
if __name__=="__main__":main()
