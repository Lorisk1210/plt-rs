//! matplotlib's `_mathtext_data` tables that its mathtext font classes
//! read (`latex_to_bakoma`, `tex2uni`, `stix_virtual_fonts`,
//! `stix_glyph_fixes`), for pltrs (Python)'s copy of those classes, which a
//! program's subclass of them calls into (see
//! [`program_fonts`](super::program_fonts)). They are the tables pltrs
//! typesets with, so the two cannot differ.

use super::data::{self, VirtualFont};

/// A run of code points a STIX virtual font moves elsewhere, as
/// matplotlib's `(start, end, font, target)`: the first and last (inclusive)
/// code points, the TeX font of the glyphs, and the code point `start`
/// maps to.
pub type VirtualRange = (u32, u32, String, u32);

/// A STIX virtual font's ranges: the same for every font class, or by
/// font class (matplotlib's list or dictionary).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VirtualFontRanges {
    /// One list for every font class.
    Single(Vec<VirtualRange>),
    /// A list per font class.
    ByClass(Vec<(String, Vec<VirtualRange>)>),
}

/// matplotlib's `latex_to_bakoma`: each symbol's `BaKoMa` font (`cmr10`,
/// ...) and character code.
///
/// ```
/// let table = pltrs_text::mathtext::tables::latex_to_bakoma();
/// assert!(table.contains(&(r"\sum", "cmex10", 0x58)));
/// ```
#[must_use]
pub fn latex_to_bakoma() -> Vec<(&'static str, &'static str, u32)> {
    data::latex_to_bakoma_table()
        .iter()
        .map(|&(symbol, file, code)| (symbol, file.family(), code))
        .collect()
}

/// matplotlib's `tex2uni`: TeX symbol names, without the backslash, and
/// their code points.
///
/// ```
/// let table = pltrs_text::mathtext::tables::tex2uni();
/// assert!(table.contains(&("alpha", 0x3b1)));
/// ```
#[must_use]
pub fn tex2uni() -> Vec<(&'static str, u32)> {
    data::tex2uni_table().to_vec()
}

/// matplotlib's `stix_glyph_fixes`: the code points whose STIX glyphs are
/// swapped, and the code each takes instead.
#[must_use]
pub fn stix_glyph_fixes() -> Vec<(u32, u32)> {
    data::STIX_GLYPH_FIXES.to_vec()
}

/// matplotlib's `stix_virtual_fonts`: the alphabets the STIX fonts keep
/// at other code points, by TeX font name (`bb`, `cal`, `frak`, ...).
///
/// ```
/// use pltrs_text::mathtext::tables::{VirtualFontRanges, stix_virtual_fonts};
/// let fonts = stix_virtual_fonts();
/// let (_, cal) = fonts.iter().find(|(name, _)| name == "cal").expect("cal");
/// assert!(matches!(cal, VirtualFontRanges::Single(_)));
/// ```
#[must_use]
pub fn stix_virtual_fonts() -> Vec<(String, VirtualFontRanges)> {
    let ranges = |ranges: &[data::VirtualRange]| -> Vec<VirtualRange> {
        ranges
            .iter()
            .map(|range| (range.start, range.end, range.font.to_string(), range.target))
            .collect()
    };
    data::virtual_fonts()
        .iter()
        .map(|(name, font)| {
            let table = match font {
                VirtualFont::Single(single) => VirtualFontRanges::Single(ranges(single)),
                VirtualFont::ByClass(classes) => VirtualFontRanges::ByClass(
                    classes
                        .iter()
                        .map(|(class, by)| (class.to_string(), ranges(by)))
                        .collect(),
                ),
            };
            (name.to_string(), table)
        })
        .collect()
}
