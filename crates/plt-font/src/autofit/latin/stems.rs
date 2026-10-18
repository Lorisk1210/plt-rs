//! Latin stem widths (`af_latin_compute_stem_width`) and the placement of
//! a stem's second edge (`af_latin_align_linked_edge`).

use super::metrics::ScaledLatin;
use crate::autofit::hints::{Dimension, GlyphHints, edge_flag, other_flag};
use crate::autofit::widths::{Width, snap_width};

/// The grid-fitted width of a stem `width` (26.6, signed) long whose base
/// edge moved by `base_delta`. Very thin stems, and all stems when stem
/// adjustment is off, keep their width.
pub(crate) fn stem_width(
    hints: &GlyphHints,
    metrics: &ScaledLatin,
    dim: Dimension,
    width: i64,
    base_delta: i64,
    base_flags: u8,
    stem_flags: u8,
) -> i64 {
    let axis = &metrics.axis[dim.index()];
    if !hints.has(other_flag::STEM_ADJUST) || axis.extra_light {
        return width;
    }
    let vertical = dim == Dimension::Vertical;
    let dist = width.abs();
    let snap = if vertical {
        other_flag::VERT_SNAP
    } else {
        other_flag::HORZ_SNAP
    };
    let dist = if hints.has(snap) {
        strong_width(hints, &axis.widths, vertical, dist, true)
    } else {
        smooth_width(
            metrics, dim, width, base_delta, base_flags, stem_flags, dist,
        )
    };
    if width < 0 { -dist } else { dist }
}

/// Light quantization of a stem width (the smooth hinting process).
fn smooth_width(
    metrics: &ScaledLatin,
    dim: Dimension,
    width: i64,
    base_delta: i64,
    base_flags: u8,
    stem_flags: u8,
    mut dist: i64,
) -> i64 {
    let axis = &metrics.axis[dim.index()];
    // Leave the widths of serifs alone.
    if stem_flags & edge_flag::SERIF != 0 && dim == Dimension::Vertical && dist < 3 * 64 {
        return dist;
    }
    if base_flags & edge_flag::ROUND != 0 {
        if dist < 80 {
            dist = 64;
        }
    } else if dist < 56 {
        dist = 56;
    }
    let Some(standard) = axis.widths.first() else {
        return dist;
    };
    if (dist - standard.cur).abs() < 40 {
        return standard.cur.max(48);
    }
    if dist < 3 * 64 {
        let delta = dist & 63;
        dist &= -64;
        dist += match delta {
            10..32 => 10,
            32..54 => 54,
            _ => delta,
        };
        return dist;
    }
    // When the base edge's rounding and the stem's go the same way, the
    // stem gets shorter by an amount decreasing with the size, so that
    // outlines do not collide at small sizes.
    let mut base = 0;
    if (width > 0 && base_delta > 0) || (width < 0 && base_delta < 0) {
        let ppem = i64::from(metrics.x_ppem);
        if ppem < 10 {
            base = base_delta;
        } else if ppem < 30 {
            base = base_delta * (30 - ppem) / 20;
        }
        base = base.abs();
    }
    (dist - base + 32) & !63
}

/// Snapping of a stem width to whole pixels (the strong hinting process),
/// shared with the CJK hinter. With `limit_distortion` (latin), stems
/// between 3/4 and 2 pixels only round when that moves them less than a
/// quarter pixel.
pub(crate) fn strong_width(
    hints: &GlyphHints,
    widths: &[Width],
    vertical: bool,
    dist: i64,
    limit_distortion: bool,
) -> i64 {
    let original = dist;
    let dist = snap_width(widths, dist);
    if vertical {
        // Always round stem heights to whole pixels.
        return if dist >= 64 { (dist + 16) & !63 } else { 64 };
    }
    if hints.has(other_flag::MONO) {
        return if dist < 64 { 64 } else { (dist + 32) & !63 };
    }
    if dist < 48 {
        return (dist + 64) >> 1;
    }
    if dist < 128 {
        let rounded = (dist + 22) & !63;
        if limit_distortion && (rounded - original).abs() >= 16 {
            return if original < 48 {
                (original + 64) >> 1
            } else {
                original
            };
        }
        return rounded;
    }
    (dist + 32) & !63
}

/// Places `stem_edge` at the fitted distance from `base_edge`
/// (`af_latin_align_linked_edge`).
pub(crate) fn align_linked_edge(
    hints: &mut GlyphHints,
    metrics: &ScaledLatin,
    dim: Dimension,
    base: usize,
    stem: usize,
) {
    let edges = &hints.axis(dim).edges;
    let (base_edge, stem_edge) = (edges[base], edges[stem]);
    let dist = stem_edge.opos - base_edge.opos;
    let base_delta = base_edge.pos - base_edge.opos;
    let fitted = stem_width(
        hints,
        metrics,
        dim,
        dist,
        base_delta,
        base_edge.flags,
        stem_edge.flags,
    );
    hints.axis_mut(dim).edges[stem].pos = base_edge.pos + fitted;
}
