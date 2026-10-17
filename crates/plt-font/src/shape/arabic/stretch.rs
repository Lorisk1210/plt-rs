//! The `stch` feature (HarfBuzz's `record_stch` / `apply_stch`): glyphs a
//! font's `stch` lookup multiplied are tiles that are repeated to span the
//! width of the rest of the word, e.g. for Syriac abbreviation marks.

use crate::face::Face;
use crate::layout::buffer::{Buffer, GlyphInfo, GlyphPos};
use crate::scale::Scaler;
use crate::shape::category::GeneralCategory;
use crate::types::Direction;

/// `GlyphInfo::aux` value of a tile drawn once.
const FIXED: u8 = 1;
/// `GlyphInfo::aux` value of a tile that may repeat.
const REPEATING: u8 = 2;
/// Most glyphs one stretched sequence may produce.
const MAX_GLYPHS: i64 = 256;

fn is_tile(info: &GlyphInfo) -> bool {
    matches!(info.aux, FIXED | REPEATING)
}

/// HarfBuzz's `HB_ARABIC_GENERAL_CATEGORY_IS_WORD`.
fn is_word(category: GeneralCategory) -> bool {
    use GeneralCategory::{
        CurrencySymbol, DecimalNumber, EnclosingMark, LetterNumber, MathSymbol, ModifierLetter,
        ModifierSymbol, NonspacingMark, OtherLetter, OtherNumber, OtherSymbol, PrivateUse,
        SpacingMark, Unassigned,
    };
    matches!(
        category,
        Unassigned
            | PrivateUse
            | ModifierLetter
            | OtherLetter
            | SpacingMark
            | EnclosingMark
            | NonspacingMark
            | DecimalNumber
            | LetterNumber
            | OtherNumber
            | CurrencySymbol
            | ModifierSymbol
            | MathSymbol
            | OtherSymbol
    )
}

/// Marks the glyphs `stch` multiplied: odd components repeat, even ones
/// are fixed.
pub(super) fn record(buffer: &mut Buffer) {
    for info in buffer.info.iter_mut().filter(|info| info.multiplied) {
        info.aux = if info.lig_comp() % 2 == 1 {
            REPEATING
        } else {
            FIXED
        };
    }
}

/// How many extra copies each repeating tile gets, how much consecutive
/// copies overlap, and the width left to center the tiles in.
fn tiling(
    total: i64,
    fixed: i64,
    repeating: i64,
    n_fixed: i64,
    n_repeating: i64,
) -> (i64, i64, i64) {
    let mut remaining = total - fixed;
    let mut copies = 0;
    if remaining > repeating && repeating > 0 {
        copies = remaining / repeating - 1;
    }
    let mut overlap = 0;
    if remaining - repeating * (copies + 1) > 0 && n_repeating > 0 {
        copies += 1;
        let excess = (copies + 1) * repeating - remaining;
        if excess > 0 {
            overlap = excess / (copies * n_repeating);
            remaining = 0;
        }
    }
    let base = n_fixed + n_repeating;
    let max_copies = if n_repeating > 0 && base < MAX_GLYPHS {
        (MAX_GLYPHS - base) / n_repeating
    } else {
        0
    };
    (copies.min(max_copies), overlap, remaining)
}

/// Replaces each run of tiles by enough copies to span the preceding
/// letters of its word (the buffer is in visual order).
pub(super) fn apply(face: &Face<'_>, scaler: &Scaler, direction: Direction, buffer: &mut Buffer) {
    if !buffer.info.iter().any(is_tile) {
        return;
    }
    let rtl = direction == Direction::RightToLeft;
    if !rtl {
        buffer.reverse();
    }
    let (info, mut pos) = (buffer.info.clone(), buffer.pos.clone());
    let advance = |glyph| i64::from(scaler.advance(face, glyph));
    // Built back to front, like HarfBuzz's in-place pass.
    let mut out: Vec<(GlyphInfo, GlyphPos)> = Vec::with_capacity(info.len());
    let mut i = info.len();
    while i > 0 {
        if !is_tile(&info[i - 1]) {
            out.push((info[i - 1], pos[i - 1]));
            i -= 1;
            continue;
        }
        let end = i;
        let (mut fixed, mut repeating, mut n_fixed, mut n_repeating) = (0, 0, 0, 0);
        while i > 0 && is_tile(&info[i - 1]) {
            i -= 1;
            if info[i].aux == FIXED {
                fixed += advance(info[i].glyph);
                n_fixed += 1;
            } else {
                repeating += advance(info[i].glyph);
                n_repeating += 1;
            }
        }
        let start = i;
        let total: i64 = info[..start]
            .iter()
            .zip(&pos[..start])
            .rev()
            .take_while(|(g, _)| {
                !is_tile(g) && (g.is_default_ignorable() || is_word(g.chars.category))
            })
            .map(|(_, p)| i64::from(p.x_advance))
            .sum();
        let (copies, overlap, remaining) = tiling(total, fixed, repeating, n_fixed, n_repeating);
        let mut x_offset = remaining / 2;
        for k in (start..end).rev() {
            let width = advance(info[k].glyph);
            let repeat = if info[k].aux == REPEATING {
                1 + copies
            } else {
                1
            };
            pos[k].x_advance = 0;
            for n in 0..repeat {
                if rtl {
                    x_offset -= width;
                    if n > 0 {
                        x_offset += overlap;
                    }
                }
                pos[k].x_offset = x_offset as i32;
                out.push((info[k], pos[k]));
                if !rtl {
                    x_offset += width;
                    if n > 0 {
                        x_offset -= overlap;
                    }
                }
            }
        }
    }
    out.reverse();
    (buffer.info, buffer.pos) = out.into_iter().unzip();
    if !rtl {
        buffer.reverse();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::buffer::CharProps;
    use crate::types::GlyphId;

    #[test]
    fn tile_counts() {
        // 1000 units to fill with a 100-unit fixed and a 400-unit repeating
        // tile: two extra copies overlapping by 150 units.
        assert_eq!(tiling(1000, 100, 400, 1, 1), (2, 150, 0));
        // Exact fits keep the remaining width as HarfBuzz does.
        assert_eq!(tiling(1000, 100, 300, 1, 1), (2, 0, 900));
        assert_eq!(tiling(700, 100, 300, 1, 1), (1, 0, 600));
        // Nothing to fill.
        assert_eq!(tiling(0, 100, 300, 1, 1), (0, 0, -100));
        assert_eq!(tiling(1000, 100, 0, 1, 0), (0, 0, 900));
        assert!(is_word(GeneralCategory::OtherLetter));
        assert!(!is_word(GeneralCategory::SpaceSeparator));
    }

    #[test]
    fn recording_multiplied_glyphs() {
        let mut buffer = Buffer::new(vec![
            GlyphInfo::new(GlyphId(1), 0, CharProps::default(), 1);
            3
        ]);
        buffer.info[1].multiplied = true;
        buffer.info[1].set_lig_props_for_mark(0, 1);
        buffer.info[2].multiplied = true;
        buffer.info[2].set_lig_props_for_mark(0, 2);
        record(&mut buffer);
        let marks: Vec<u8> = buffer.info.iter().map(|i| i.aux).collect();
        assert_eq!(marks, vec![0, REPEATING, FIXED]);
    }
}
