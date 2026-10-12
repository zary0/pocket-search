// SPDX-License-Identifier: GPL-3.0-only
//! Bounded horizontal Japanese kana clusters. No script VM, GSUB or GPOS execution.
pub fn compose(base: char, mark: char) -> Option<char> {
    let b = base as u32;
    let voiced = matches!(
        b,
        0x304b
            | 0x304d
            | 0x304f
            | 0x3051
            | 0x3053
            | 0x3055
            | 0x3057
            | 0x3059
            | 0x305b
            | 0x305d
            | 0x305f
            | 0x3061
            | 0x3064
            | 0x3066
            | 0x3068
            | 0x306f
            | 0x3072
            | 0x3075
            | 0x3078
            | 0x307b
            | 0x309d
            | 0x30ab
            | 0x30ad
            | 0x30af
            | 0x30b1
            | 0x30b3
            | 0x30b5
            | 0x30b7
            | 0x30b9
            | 0x30bb
            | 0x30bd
            | 0x30bf
            | 0x30c1
            | 0x30c4
            | 0x30c6
            | 0x30c8
            | 0x30cf
            | 0x30d2
            | 0x30d5
            | 0x30d8
            | 0x30db
            | 0x30fd
    );
    let semi = matches!(
        b,
        0x306f | 0x3072 | 0x3075 | 0x3078 | 0x307b | 0x30cf | 0x30d2 | 0x30d5 | 0x30d8 | 0x30db
    );
    let value = match mark as u32 {
        0x3099 if voiced => b + 1,
        0x309a if semi => b + 2,
        0x3099 if b == 0x3046 => 0x3094,
        0x3099 if b == 0x30a6 => 0x30f4,
        0x3099 if (0x30ef..=0x30f2).contains(&b) => b + 8,
        _ => return None,
    };
    char::from_u32(value)
}
/// Normalize only supported BMP kana clusters into bounded UTF-8; offsets after
/// the pair refer to the normalized output. Caller commits this before layout.
pub fn normalize<'a>(input: &str, out: &'a mut [u8]) -> Option<&'a str> {
    let mut length = 0;
    for c in input.chars() {
        if matches!(c, '\u{3099}' | '\u{309a}') && length >= 3 {
            if let Ok(last) = core::str::from_utf8(&out[length - 3..length]) {
                if let Some(value) = last.chars().next().and_then(|base| compose(base, c)) {
                    let mut bytes = [0; 4];
                    out[length - 3..length]
                        .copy_from_slice(value.encode_utf8(&mut bytes).as_bytes());
                    continue;
                }
            }
        }
        let mut bytes = [0; 4];
        let encoded = c.encode_utf8(&mut bytes);
        if encoded.len() > out.len() - length {
            return None;
        }
        out[length..length + encoded.len()].copy_from_slice(encoded.as_bytes());
        length += encoded.len();
    }
    core::str::from_utf8(&out[..length]).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn kana_clusters() {
        for (a, m, b) in [
            ('か', '\u{3099}', 'が'),
            ('は', '\u{309a}', 'ぱ'),
            ('カ', '\u{3099}', 'ガ'),
            ('ハ', '\u{309a}', 'パ'),
            ('ウ', '\u{3099}', 'ヴ'),
            ('ワ', '\u{3099}', 'ヷ'),
        ] {
            assert_eq!(compose(a, m), Some(b));
        }
        assert_eq!(compose('日', '\u{3099}'), None);
        assert_eq!(compose('か', '\u{309a}'), None);
    }
}
