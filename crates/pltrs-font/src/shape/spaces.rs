//! HarfBuzz's fallback for Unicode spaces a font lacks: they are drawn with
//! the font's space glyph and given a conventional width
//! (`_hb_ot_shape_fallback_spaces`).

use crate::face::Face;
use crate::scale::Scaler;

/// The width rule for a substituted space.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SpaceWidth {
    /// The space glyph's own advance.
    Space,
    /// One `n`-th of an em.
    EmFraction(i32),
    /// 4/18 of an em (medium mathematical space).
    FourEighteenthsEm,
    /// The advance of the first available digit.
    Figure,
    /// The advance of `.` (or `,`).
    Punctuation,
    /// Half the space advance.
    Narrow,
}

/// The fallback rule for space characters (general category Zs).
pub(crate) fn fallback_width(c: char) -> Option<SpaceWidth> {
    Some(match c {
        ' ' | '\u{A0}' => SpaceWidth::Space,
        '\u{2001}' | '\u{2003}' | '\u{3000}' => SpaceWidth::EmFraction(1),
        '\u{2000}' | '\u{2002}' => SpaceWidth::EmFraction(2),
        '\u{2004}' => SpaceWidth::EmFraction(3),
        '\u{2005}' => SpaceWidth::EmFraction(4),
        '\u{2009}' => SpaceWidth::EmFraction(5),
        '\u{2006}' => SpaceWidth::EmFraction(6),
        '\u{200A}' => SpaceWidth::EmFraction(16),
        '\u{205F}' => SpaceWidth::FourEighteenthsEm,
        '\u{2007}' => SpaceWidth::Figure,
        '\u{2008}' => SpaceWidth::Punctuation,
        '\u{202F}' => SpaceWidth::Narrow,
        _ => return None,
    })
}

/// The advance HarfBuzz gives a substituted space whose space glyph has
/// advance `space_advance`.
pub(crate) fn advance(
    width: SpaceWidth,
    face: &Face<'_>,
    scaler: &Scaler<'_>,
    space_advance: i32,
) -> i32 {
    let em = scaler.em(face).0;
    let glyph_advance = |chars: &[char]| {
        chars
            .iter()
            .find_map(|&c| face.glyph_index(c))
            .map_or(space_advance, |glyph| scaler.advance(face, glyph))
    };
    match width {
        SpaceWidth::Space => space_advance,
        SpaceWidth::EmFraction(n) => ((em + i64::from(n / 2)) / i64::from(n)) as i32,
        SpaceWidth::FourEighteenthsEm => (em * 4 / 18) as i32,
        SpaceWidth::Figure => glyph_advance(&['0', '1', '2', '3', '4', '5', '6', '7', '8', '9']),
        SpaceWidth::Punctuation => glyph_advance(&['.', ',']),
        SpaceWidth::Narrow => space_advance / 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_spaces() {
        assert_eq!(fallback_width('\u{2009}'), Some(SpaceWidth::EmFraction(5)));
        assert_eq!(fallback_width('\u{202F}'), Some(SpaceWidth::Narrow));
        assert_eq!(fallback_width('a'), None);
    }
}
