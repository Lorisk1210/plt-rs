//! HarfBuzz's Hebrew shaper (`hb-ot-shaper-hebrew.cc`): presentation-form
//! composition for fonts without `GPOS` mark positioning, and a mark
//! reordering fix for patah/qamats followed by sheva/hiriq and meteg.

use crate::shape::normalize::{ClusterChar, merge_clusters};

/// Presentation forms with dagesh of U+05D0..=U+05EA (0: none encoded).
const DAGESH_FORMS: [u32; 27] = [
    0xFB30, 0xFB31, 0xFB32, 0xFB33, 0xFB34, 0xFB35, 0xFB36, 0, 0xFB38, 0xFB39, 0xFB3A, 0xFB3B,
    0xFB3C, 0, 0xFB3E, 0, 0xFB40, 0xFB41, 0, 0xFB43, 0xFB44, 0, 0xFB46, 0xFB47, 0xFB48, 0xFB49,
    0xFB4A,
];

/// HarfBuzz's modified combining classes the reordering looks at.
mod class {
    pub(super) const PATAH: u8 = 20;
    pub(super) const QAMATS: u8 = 21;
    pub(super) const SHEVA: u8 = 22;
    pub(super) const HIRIQ: u8 = 23;
    pub(super) const METEG: u8 = 25;
    pub(super) const BELOW: u8 = 220;
}

/// The presentation form composing `a` and `b` that Unicode normalization
/// excludes but old fonts need (used when canonical composition fails and
/// the font has no `GPOS` mark positioning).
pub(crate) fn compose(a: char, b: char) -> Option<char> {
    let a = u32::from(a);
    let composed = match (u32::from(b), a) {
        (0x05B4, 0x05D9) => 0xFB1D,
        (0x05B7, 0x05F2) => 0xFB1F,
        (0x05B7, 0x05D0) => 0xFB2E,
        (0x05B8, 0x05D0) => 0xFB2F,
        (0x05B9, 0x05D5) => 0xFB4B,
        (0x05BC, 0x05D0..=0x05EA) => DAGESH_FORMS[(a - 0x05D0) as usize],
        (0x05BC, 0xFB2A) | (0x05C1, 0xFB49) => 0xFB2C,
        (0x05BC, 0xFB2B) | (0x05C2, 0xFB49) => 0xFB2D,
        (0x05BF, 0x05D1) => 0xFB4C,
        (0x05BF, 0x05DB) => 0xFB4D,
        (0x05BF, 0x05E4) => 0xFB4E,
        (0x05C1, 0x05E9) => 0xFB2A,
        (0x05C2, 0x05E9) => 0xFB2B,
        _ => 0,
    };
    char::from_u32(composed).filter(|&c| c != '\0')
}

/// HarfBuzz's `reorder_marks_hebrew`: in the canonically ordered run of
/// marks `start..end` of `chars`, sheva or hiriq after patah or qamats
/// moves after a following meteg (or other below mark).
pub(crate) fn reorder_marks(chars: &mut [ClusterChar], start: usize, end: usize) {
    for i in start + 2..end {
        let classes = [i - 2, i - 1, i].map(|k| chars[k].props.combining_class);
        if matches!(classes[0], class::PATAH | class::QAMATS)
            && matches!(classes[1], class::SHEVA | class::HIRIQ)
            && matches!(classes[2], class::METEG | class::BELOW)
        {
            merge_clusters(chars, i - 1, i + 1);
            chars.swap(i - 1, i);
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presentation_forms() {
        assert_eq!(compose('\u{5E9}', '\u{5C1}'), Some('\u{FB2A}'));
        assert_eq!(compose('\u{FB49}', '\u{5C2}'), Some('\u{FB2D}'));
        assert_eq!(compose('\u{5D1}', '\u{5BC}'), Some('\u{FB31}'));
        assert_eq!(compose('\u{5D7}', '\u{5BC}'), None);
        assert_eq!(compose('\u{5D5}', '\u{5B9}'), Some('\u{FB4B}'));
        assert_eq!(compose('a', '\u{5B9}'), None);
    }

    #[test]
    fn meteg_reordering() {
        // QAMATS, HIRIQ, METEG: the hiriq moves after the meteg.
        let mut marks: Vec<ClusterChar> = "\u{5B8}\u{5B4}\u{5BD}"
            .chars()
            .map(|c| ClusterChar::new(c, 0, 0))
            .collect();
        reorder_marks(&mut marks, 0, 3);
        let order: String = marks.iter().map(|m| m.c).collect();
        assert_eq!(order, "\u{5B8}\u{5BD}\u{5B4}");
    }
}
