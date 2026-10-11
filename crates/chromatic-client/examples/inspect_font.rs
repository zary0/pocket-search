// SPDX-License-Identifier: GPL-3.0-only
//! Inspect a local, operator-supplied TTF using exactly the native bounded rasterizer.
fn main() {
    use std::io::Read;
    let path = std::env::args()
        .nth(1)
        .expect("Usage: inspect_font FILE.ttf");
    let mut bytes = Vec::with_capacity(chromatic_client::webfont::LIMIT + 1);
    std::fs::File::open(path)
        .unwrap()
        .take((chromatic_client::webfont::LIMIT + 1) as u64)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(
        bytes.len() <= chromatic_client::webfont::LIMIT,
        "Font exceeds the native input limit"
    );
    let mut atlas = [0; chromatic_client::webfont::ATLAS_BYTES];
    let n = chromatic_client::webfont::rasterize(&bytes, &mut atlas);
    println!(
        "source_bytes={} rasterized_glyphs={} atlas_bytes={} state_bytes={}",
        bytes.len(),
        n,
        atlas.len(),
        std::mem::size_of::<chromatic_client::local::LocalSearch>()
    );
    for code in *b"APS" {
        let i = usize::from(code - 32);
        println!("glyph={}", code as char);
        for y in 0..12 {
            for x in 0..6 {
                let bit = y * 6 + x;
                print!(
                    "{}",
                    if atlas[i * 9 + bit / 8] & (1 << (7 - bit % 8)) != 0 {
                        '#'
                    } else {
                        '.'
                    }
                );
            }
            println!();
        }
    }
    assert!(n > 0, "Unsupported or invalid font");
}
