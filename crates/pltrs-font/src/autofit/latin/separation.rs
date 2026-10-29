//! The diacritic adjustments of the latin hinter, driven by the
//! adjustment database: tildes stretched before hinting, and diacritics
//! pushed one pixel clear of their base after it
//! (`af_glyph_hints_apply_vertical_separation_adjustments`).

use super::diacritics::Side;
use crate::autofit::hints::GlyphHints;
use crate::autofit::tables::adjust::{
    DOWN, DOWN2, TILDE_BOTTOM, TILDE_BOTTOM2, TILDE_TOP, TILDE_TOP2, UP, UP2,
};
use crate::fixed::pix_round;

impl Side {
    /// The adjustment flags of the side: pushing the outermost contour,
    /// pushing the next one, and either of them being a tilde.
    const fn flags(self) -> [u16; 4] {
        match self {
            Self::Top => [UP, UP2, TILDE_TOP, TILDE_TOP2],
            Self::Bottom => [DOWN, DOWN2, TILDE_BOTTOM, TILDE_BOTTOM2],
        }
    }
}

/// Stretches and grid-aligns the tildes `flags` announce, before the
/// outline is analysed: inner tildes first (moving the contours beyond
/// them along), then outermost ones.
pub(crate) fn prepare_tildes(hints: &mut GlyphHints, flags: u16) {
    let tildes = [TILDE_TOP, TILDE_BOTTOM, TILDE_TOP2, TILDE_BOTTOM2];
    if tildes.iter().any(|&tilde| flags & tilde != 0) {
        hints.compute_vertical_extrema();
    }
    for side in [Side::Top, Side::Bottom] {
        if flags & side.flags()[3] == 0 {
            continue;
        }
        let contour = hints.second_extreme_contour(side);
        let stretched = hints.stretch_tilde(side, contour);
        let aligned = hints.align_tilde(side, contour);
        let offset = match side {
            Side::Top => stretched + aligned,
            Side::Bottom => stretched - aligned,
        };
        let limit = match side {
            Side::Top => hints.contour_y_minima[contour],
            Side::Bottom => hints.contour_y_maxima[contour],
        };
        hints.move_contours(side, limit, offset);
    }
    for side in [Side::Top, Side::Bottom] {
        if flags & side.flags()[2] != 0 {
            let contour = hints.extreme_contour(side);
            hints.stretch_tilde(side, contour);
            hints.align_tilde(side, contour);
        }
    }
}

/// Pushes the diacritics `flags` asks for one pixel clear of the rest of
/// the hinted outline. Diacritics taller than `accent_height_limit`, or
/// not above (below) the base, are left alone, and stop further
/// adjustment.
pub(crate) fn separate_diacritics(hints: &mut GlyphHints, flags: u16, accent_height_limit: i64) {
    let contours = hints.contours.len();
    let wanted = |side: Side| {
        let [outer, inner, ..] = side.flags();
        (flags & outer != 0 && contours >= 2) || (flags & inner != 0 && contours >= 3)
    };
    let (top, bottom) = (wanted(Side::Top), wanted(Side::Bottom));
    if top || bottom {
        hints.compute_vertical_extrema();
    }
    if top && !separate(hints, Side::Top, flags, accent_height_limit) {
        return;
    }
    if bottom {
        separate(hints, Side::Bottom, flags, accent_height_limit);
    }
}

/// The adjustment of one side; false when FreeType gives up on the glyph.
fn separate(hints: &mut GlyphHints, side: Side, flags: u16, accent_height_limit: i64) -> bool {
    let [outer_flag, inner_flag, tilde_flag, inner_tilde_flag] = side.flags();
    let outer = flags & outer_flag != 0;
    let inner = flags & inner_flag != 0;
    let (tilde, inner_tilde) = (flags & tilde_flag != 0, flags & inner_tilde_flag != 0);
    let accent = if inner {
        hints.second_extreme_contour(side)
    } else {
        hints.extreme_contour(side)
    };
    if !hints.overlaps_horizontally(accent) {
        return false;
    }
    let (accent_min, accent_max) = (
        hints.contour_y_minima[accent],
        hints.contour_y_maxima[accent],
    );
    let accent_height = accent_max.wrapping_sub(accent_min);
    if accent_height > accent_height_limit {
        return false;
    }
    // The gap to the nearest contour on the base side, when under a pixel.
    let mut min_distance = 64;
    for contour in (0..hints.contours.len()).filter(|&c| c != accent) {
        let (min_y, max_y) = (
            hints.contour_y_minima[contour],
            hints.contour_y_maxima[contour],
        );
        let (distance, on_base_side) = match side {
            Side::Top => (accent_min.wrapping_sub(max_y), min_y < accent_min),
            Side::Bottom => (min_y.wrapping_sub(accent_max), max_y > accent_max),
        };
        if distance < 64 && distance < min_distance && on_base_side {
            min_distance = distance;
        }
    }
    let adjustment = 64 - min_distance;
    let mut centering = 0;
    if tilde || inner_tilde {
        let tilde_contour = if outer || inner_tilde {
            accent
        } else {
            hints.extreme_contour(side)
        };
        let tilde_height = hints.contour_y_maxima[tilde_contour]
            .wrapping_sub(hints.contour_y_minima[tilde_contour]);
        // Re-centre a tilde under three pixels that the push would put on
        // the grid.
        let pos = match side {
            Side::Top => {
                let pos = accent_min.wrapping_add(adjustment);
                if inner && tilde {
                    pos + accent_height
                } else {
                    pos
                }
            }
            Side::Bottom => {
                let pos = accent_max.wrapping_sub(adjustment);
                if inner && tilde {
                    pos - accent_height
                } else {
                    pos
                }
            }
        };
        if pos % 64 == 0 && tilde_height < 3 * 64 {
            centering = (pix_round(tilde_height) - tilde_height) / 2;
        }
    }
    let calculated = if (outer && tilde) || (inner && inner_tilde) {
        adjustment + centering
    } else {
        adjustment
    };
    // A delta of 2/64 pixel allows for rounding differences.
    if calculated != 0 && calculated >= -2 && (calculated <= 66 || adjustment <= 66) {
        let height_delta = accent_height / 8;
        let limit = match side {
            Side::Top => accent_min.wrapping_sub(height_delta),
            Side::Bottom => accent_max.wrapping_add(height_delta),
        };
        // The push applies to everything as high (low) as the diacritic,
        // such as the inner contour of a ring.
        hints.move_contours(side, limit, adjustment);
        if inner && tilde {
            let limit = match side {
                Side::Top => limit.wrapping_add(accent_height),
                Side::Bottom => limit.wrapping_sub(accent_height),
            };
            hints.move_contours(side, limit, centering);
        }
    }
    true
}
