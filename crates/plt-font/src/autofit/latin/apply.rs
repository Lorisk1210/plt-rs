//! Hinting one glyph with the latin writing system
//! (`af_latin_hints_init`, `af_latin_hints_apply`).

use super::diacritics::Side;
use super::edges::{compute_blue_edges, compute_edges};
use super::hint::hint_edges;
use super::metrics::{LatinBlue, LatinMetrics, ScaledLatin, blue_flag};
use super::segments::{compute_segments, link_segments};
use super::separation::{prepare_tildes, separate_diacritics};
use crate::autofit::AutohintTarget;
use crate::autofit::adjust::adjustment;
use crate::autofit::hints::{Dimension, FontOutline, GlyphHints, edge_flag, scaler_flag};
use crate::autofit::tables::adjust::{
    IGNORE_CAPITAL_BOTTOM, IGNORE_CAPITAL_TOP, IGNORE_SMALL_BOTTOM, IGNORE_SMALL_TOP,
    NO_HEIGHT_CHECK,
};
use crate::fixed::mul_fix;
use crate::types::GlyphId;

/// The hints for a latin glyph at `metrics`' size: no horizontal hinting
/// for the light and LCD targets and for italic faces.
pub(crate) fn hints_init(
    metrics: &ScaledLatin,
    target: AutohintTarget,
    italic: bool,
) -> GlyphHints {
    let mut scaler_flags = 0;
    if matches!(target, AutohintTarget::Light | AutohintTarget::Lcd) || italic {
        scaler_flags |= scaler_flag::NO_HORIZONTAL;
    }
    let mut hints = GlyphHints::new(
        metrics.axis[Dimension::Horizontal.index()].scale,
        metrics.axis[Dimension::Vertical.index()].scale,
        scaler_flags,
    );
    hints.other_flags = target.hinter_flags();
    hints
}

/// The glyph being hinted.
pub(crate) struct LatinGlyph<'a> {
    pub(crate) glyph: GlyphId,
    pub(crate) outline: &'a FontOutline,
    /// A non-base character (no blue zones).
    pub(crate) nonbase: bool,
    /// Whether the script hints vertical edges top to bottom.
    pub(crate) top_to_bottom: bool,
}

/// Hints `glyph` into `hints` (initialized by [`hints_init`]).
pub(crate) fn hints_apply(
    hints: &mut GlyphHints,
    metrics: &LatinMetrics,
    scaled: &ScaledLatin,
    glyph: &LatinGlyph<'_>,
) {
    let units_per_em = metrics.units_per_em;
    hints.reload(glyph.outline, units_per_em);
    if hints.do_horizontal() {
        detect_features(hints, metrics, Dimension::Horizontal, glyph.top_to_bottom);
    }
    let flags = metrics
        .reverse_charmap
        .as_ref()
        .and_then(|map| map.code_point(glyph.glyph))
        .map_or(0, adjustment);
    prepare_tildes(hints, flags);
    detect_features(hints, metrics, Dimension::Vertical, glyph.top_to_bottom);
    let mut accent_height_limit = 0;
    if flags != 0 {
        let blues = &scaled.axis[Dimension::Vertical.index()].blues;
        let capital = base_blues(blues, true);
        let small = base_blues(blues, false);
        if flags & NO_HEIGHT_CHECK == 0 {
            accent_height_limit = if let Some((top, bottom)) = small {
                2 * (top.overshoot.cur - bottom.overshoot.cur) / 3
            } else if let Some((top, bottom)) = capital {
                (top.overshoot.cur - bottom.overshoot.cur) / 2
            } else {
                let height = i64::from(u32::from(units_per_em) * 4 / 10);
                mul_fix(height, scaled.axis[Dimension::Vertical.index()].scale)
            };
        }
        for (base, top_flag, bottom_flag) in [
            (capital, IGNORE_CAPITAL_TOP, IGNORE_CAPITAL_BOTTOM),
            (small, IGNORE_SMALL_TOP, IGNORE_SMALL_BOTTOM),
        ] {
            let Some((top, bottom)) = base else {
                continue;
            };
            if flags & top_flag != 0 {
                ignore_blues(hints, Side::Top, top, bottom);
            }
            if flags & bottom_flag != 0 {
                ignore_blues(hints, Side::Bottom, top, bottom);
            }
        }
    }
    if !glyph.nonbase {
        compute_blue_edges(hints, scaled, units_per_em);
    }
    for dim in Dimension::BOTH {
        if !hints.hints(dim) {
            continue;
        }
        hint_edges(hints, scaled, dim, glyph.top_to_bottom);
        hints.align_edge_points(dim);
        hints.align_strong_points(dim);
        hints.align_weak_points(dim);
        if dim == Dimension::Vertical {
            separate_diacritics(hints, flags, accent_height_limit);
        }
    }
}

/// Segments, stem links and edges of `dim`
/// (`af_latin_hints_detect_features`).
fn detect_features(
    hints: &mut GlyphHints,
    metrics: &LatinMetrics,
    dim: Dimension,
    top_to_bottom: bool,
) {
    let widths = &metrics.widths[dim.index()];
    compute_segments(hints, dim, metrics.units_per_em);
    link_segments(hints, dim, widths.used(), metrics.units_per_em);
    compute_edges(hints, dim, widths.edge_distance_threshold, top_to_bottom);
}

/// The active top and bottom blue zones of capital (or small) base
/// characters (`af_latin_get_base_glyph_blues`), when both exist.
fn base_blues(blues: &[LatinBlue], capital: bool) -> Option<(LatinBlue, LatinBlue)> {
    let find = |flag: u8| {
        let flag = flag | blue_flag::ACTIVE;
        blues.iter().find(|blue| blue.flags & flag == flag).copied()
    };
    let (top, bottom) = if capital {
        (blue_flag::TOP, blue_flag::BOTTOM)
    } else {
        (blue_flag::ADJUSTMENT, blue_flag::BOTTOM_SMALL)
    };
    find(top).zip(find(bottom))
}

/// Keeps edges beyond the base character on `side` (by a seventh of its
/// height plus a quarter pixel) away from blue zones
/// (`af_latin_ignore_top`, `af_latin_ignore_bottom`).
fn ignore_blues(hints: &mut GlyphHints, side: Side, top: LatinBlue, bottom: LatinBlue) {
    let height = top.overshoot.cur - bottom.overshoot.cur;
    for edge in &mut hints.axis_mut(Dimension::Vertical).edges {
        let beyond = match side {
            Side::Top => edge.pos > top.overshoot.cur + height / 7 + 16,
            Side::Bottom => edge.pos < bottom.overshoot.cur - height / 7 - 16,
        };
        if beyond {
            edge.flags |= edge_flag::NO_BLUE;
        }
    }
}
