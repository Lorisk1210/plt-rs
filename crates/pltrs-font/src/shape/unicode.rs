//! Per-character Unicode properties as HarfBuzz records them for shaping
//! (`_hb_glyph_info_set_unicode_props`): general category, modified
//! combining class, default-ignorable kind and grapheme continuation.

use crate::layout::buffer::{CharProps, Ignorable};
use crate::shape::category::GeneralCategory;
use crate::shape::unicode_data::COMBINING_CLASSES;

/// HarfBuzz's `_hb_modified_combining_class` for the classes it changes:
/// Hebrew, Arabic and Syriac points, Telugu, Thai, Lao and Tibetan vowel
/// signs, reordered to match how fonts expect them.
fn modify(class: u8) -> u8 {
    match class {
        // Hebrew points.
        10 => 22,
        11 => 15,
        12 => 16,
        13 => 17,
        14 => 23,
        15 => 18,
        16 => 19,
        17 => 20,
        18 => 21,
        19 => 14,
        20 => 24,
        21 => 12,
        22 => 25,
        23 => 13,
        24 => 10,
        25 => 11,
        // Arabic: shadda before the vowel marks.
        27..=32 => class + 1,
        33 => 27,
        // Telugu length marks, Thai sara u, Tibetan vowel signs.
        84 => 4,
        91 => 5,
        103 => 3,
        130 => 132,
        132 => 131,
        _ => class,
    }
}

/// HarfBuzz's `modified_combining_class`.
pub(crate) fn modified_combining_class(c: char) -> u8 {
    match c {
        // Tai Tham SAKOT after tone marks; Tibetan PADMA after vowels.
        '\u{1A60}' | '\u{0FC6}' => 254,
        // Tibetan TSA -PHRU before U+0F74.
        '\u{0F39}' => 127,
        _ => {
            let code = u32::from(c);
            let index = COMBINING_CLASSES.partition_point(|&(start, _, _)| start <= code);
            match COMBINING_CLASSES.get(index.wrapping_sub(1)) {
                Some(&(_, end, class)) if code <= end => modify(class),
                _ => 0,
            }
        }
    }
}

/// HarfBuzz's `is_default_ignorable`.
pub(crate) fn is_default_ignorable(c: char) -> bool {
    matches!(
        u32::from(c),
        0x00AD
            | 0x034F
            | 0x061C
            | 0x17B4..=0x17B5
            | 0x180B..=0x180E
            | 0x200B..=0x200F
            | 0x202A..=0x202E
            | 0x2060..=0x206F
            | 0xFE00..=0xFE0F
            | 0xFEFF
            | 0xFFF0..=0xFFF8
            | 0x1D173..=0x1D17A
            | 0xE0000..=0xE0FFF
    )
}

/// The shaping properties of `c`.
pub(crate) fn char_props(c: char) -> CharProps {
    let category = GeneralCategory::of(c);
    let ignorable = is_default_ignorable(c).then_some(match c {
        '\u{200C}' => Ignorable::Zwnj,
        '\u{200D}' => Ignorable::Zwj,
        '\u{180B}'..='\u{180D}' | '\u{180F}' | '\u{E0020}'..='\u{E007F}' | '\u{034F}' => {
            Ignorable::Hidden
        }
        _ => Ignorable::Other,
    });
    CharProps {
        category,
        combining_class: if category.is_mark() {
            modified_combining_class(c)
        } else {
            0
        },
        ignorable,
        continuation: category.is_mark(),
    }
}

fn is_regional_indicator(c: char) -> bool {
    matches!(c, '\u{1F1E6}'..='\u{1F1FF}')
}

/// HarfBuzz's grapheme continuation flags: marks, emoji modifiers, ZWJ,
/// the second of a pair of regional indicators, halfwidth sound marks and
/// tag characters continue the preceding character's grapheme.
pub(crate) fn continuations(chars: &[char]) -> Vec<bool> {
    let mut flags: Vec<bool> = Vec::with_capacity(chars.len());
    for (index, &c) in chars.iter().enumerate() {
        let continues = GeneralCategory::of(c).is_mark()
            || matches!(
                c,
                '\u{1F3FB}'..='\u{1F3FF}' | '\u{200D}' | '\u{FF9E}' | '\u{FF9F}'
                    | '\u{E0020}'..='\u{E007F}'
            )
            || (is_regional_indicator(c)
                && index > 0
                && is_regional_indicator(chars[index - 1])
                && !flags[index - 1]);
        flags.push(continues);
    }
    flags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combining_classes() {
        assert_eq!(modified_combining_class('\u{301}'), 230);
        assert_eq!(modified_combining_class('\u{323}'), 220);
        assert_eq!(modified_combining_class('a'), 0);
        // Arabic: shadda (33) sorts before fatha (30 -> 31).
        assert_eq!(modified_combining_class('\u{651}'), 27);
        assert_eq!(modified_combining_class('\u{64E}'), 31);
        // Hebrew: dagesh (21) before sheva (10 -> 22).
        assert_eq!(modified_combining_class('\u{5BC}'), 12);
        assert_eq!(modified_combining_class('\u{5B0}'), 22);
        assert_eq!(modified_combining_class('\u{0F39}'), 127);
        assert_eq!(modified_combining_class('\u{1A60}'), 254);
    }

    #[test]
    fn properties() {
        let zwj = char_props('\u{200D}');
        assert_eq!(zwj.ignorable, Some(Ignorable::Zwj));
        assert_eq!(char_props('\u{200C}').ignorable, Some(Ignorable::Zwnj));
        assert_eq!(char_props('\u{34F}').ignorable, Some(Ignorable::Hidden));
        assert_eq!(char_props('\u{AD}').ignorable, Some(Ignorable::Other));
        assert_eq!(char_props('a').ignorable, None);
        let acute = char_props('\u{301}');
        assert!(acute.is_nonspacing_mark() && acute.combining_class == 230);
        assert!(is_default_ignorable('\u{200D}') && !is_default_ignorable(' '));
    }

    #[test]
    fn grapheme_continuations() {
        let text: Vec<char> = "a\u{301}\u{1F1E6}\u{1F1E8}\u{1F1E6}x\u{200D}"
            .chars()
            .collect();
        assert_eq!(
            continuations(&text),
            vec![false, true, false, true, false, false, true]
        );
    }
}
