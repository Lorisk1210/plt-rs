//! Glyph names to characters, by the Adobe Glyph List specification
//! (fontTools' `agl.toUnicode`, which matplotlib's `dviread` uses to tell
//! the characters of TeX's glyphs).

use std::collections::HashMap;
use std::sync::OnceLock;

/// Adobe's `glyphlist.txt` (AGL 2.0): `name;XXXX[ XXXX...]` lines.
const GLYPH_LIST: &str = include_str!("../data/glyphlist.txt");

/// The code points of every name of the glyph list, read once.
fn glyph_list() -> &'static HashMap<&'static str, Vec<char>> {
    static LIST: OnceLock<HashMap<&'static str, Vec<char>>> = OnceLock::new();
    LIST.get_or_init(|| {
        GLYPH_LIST
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .filter_map(|line| {
                let (name, values) = line.split_once(';')?;
                let chars = values
                    .split(' ')
                    .map(|hex| u32::from_str_radix(hex, 16).ok().and_then(char::from_u32))
                    .collect::<Option<Vec<char>>>()?;
                Some((name, chars))
            })
            .collect()
    })
}

/// The characters a glyph name stands for (fontTools' `toUnicode`):
/// everything from the first `.` on is dropped, the rest is split at `_`,
/// and each part is a name of the Adobe Glyph List, `uniXXXX...` (groups
/// of four hexadecimal digits outside the surrogates) or `uXXXX` to
/// `uXXXXXX`; a part that is none of these stands for nothing. An empty
/// string means the name stands for no character.
pub fn glyph_name_to_unicode(name: &str) -> String {
    let base = name.split_once('.').map_or(name, |(base, _)| base);
    base.split('_').flat_map(component_chars).collect()
}

/// The characters of one part of a glyph name.
fn component_chars(component: &str) -> Vec<char> {
    if let Some(chars) = glyph_list().get(component) {
        return chars.clone();
    }
    uni_chars(component)
        .or_else(|| u_char(component).map(|c| vec![c]))
        .unwrap_or_default()
}

/// Whether `digits` is a non-empty run of uppercase hexadecimal digits.
fn is_upper_hex(digits: &str) -> bool {
    !digits.is_empty()
        && digits
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
}

/// `uni` followed by groups of four uppercase hexadecimal digits, none a
/// surrogate.
fn uni_chars(component: &str) -> Option<Vec<char>> {
    let digits = component.strip_prefix("uni")?;
    if !is_upper_hex(digits) || digits.len() % 4 != 0 {
        return None;
    }
    digits
        .as_bytes()
        .chunks(4)
        .map(|group| {
            let group = std::str::from_utf8(group).ok()?;
            char::from_u32(u32::from_str_radix(group, 16).ok()?)
        })
        .collect()
}

/// `u` followed by four to six uppercase hexadecimal digits naming a
/// Unicode scalar value.
fn u_char(component: &str) -> Option<char> {
    let digits = component.strip_prefix('u')?;
    if !is_upper_hex(digits) || !(4..=6).contains(&digits.len()) {
        return None;
    }
    char::from_u32(u32::from_str_radix(digits, 16).ok()?)
}

#[cfg(test)]
mod tests {
    use super::glyph_name_to_unicode as to_unicode;

    #[test]
    fn names_of_the_glyph_list_are_their_characters() {
        assert_eq!(to_unicode("A"), "A");
        assert_eq!(to_unicode("pi"), "\u{3c0}");
        assert_eq!(to_unicode("nahiragana"), "\u{306a}");
        assert_eq!(to_unicode("dalethatafpatah"), "\u{5d3}\u{5b2}");
    }

    #[test]
    fn suffixes_are_dropped_and_ligatures_split() {
        assert_eq!(to_unicode("longs_t.oldstyle"), "\u{17f}t");
        assert_eq!(to_unicode("f_f_i"), "ffi");
        assert_eq!(to_unicode("A.sc"), "A");
        assert_eq!(to_unicode(".notdef"), "");
    }

    #[test]
    fn uni_and_u_names_give_their_code_points() {
        assert_eq!(to_unicode("uni20AC"), "\u{20ac}");
        assert_eq!(to_unicode("uni20AC0041"), "\u{20ac}A");
        assert_eq!(to_unicode("u1F600"), "\u{1f600}");
        // Lowercase digits, surrogates, odd lengths and values beyond
        // Unicode stand for nothing.
        assert_eq!(to_unicode("uni20ac"), "");
        assert_eq!(to_unicode("uniD800"), "");
        assert_eq!(to_unicode("uni20A"), "");
        assert_eq!(to_unicode("u110000"), "");
        assert_eq!(to_unicode("uD800"), "");
    }

    #[test]
    fn unknown_names_stand_for_nothing() {
        assert_eq!(to_unicode("integraldisplay"), "");
        assert_eq!(to_unicode("parenleftBig"), "");
        assert_eq!(to_unicode(""), "");
    }
}
