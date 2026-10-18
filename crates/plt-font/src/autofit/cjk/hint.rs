//! CJK grid fitting (`af_cjk_compute_stem_width`, `af_hint_normal_stem`,
//! `af_cjk_hint_edges`, `af_cjk_align_edge_points`).

use crate::autofit::hints::{Dimension, Edge, GlyphHints, edge_flag, other_flag, point_flag};
use crate::autofit::latin::stems::strong_width;
use crate::autofit::widths::AxisWidths;
use crate::fixed::{mul_div, pix_floor};

/// The largest gaps and shift of light-mode stems
/// (`AF_LIGHT_MODE_MAX_HORZ_GAP`, `..._VERT_GAP`, `..._DELTA_ABS`).
const LIGHT_MAX_HORZ_GAP: i64 = 9;
const LIGHT_MAX_VERT_GAP: i64 = 15;
const LIGHT_MAX_DELTA: i64 = 14;

/// The fitted width of a CJK stem (`af_cjk_compute_stem_width`). The
/// standard widths' scaled values are zero (FreeType never scales them).
fn stem_width(hints: &GlyphHints, widths: &AxisWidths, dim: Dimension, width: i64) -> i64 {
    if !hints.has(other_flag::STEM_ADJUST) {
        return width;
    }
    let vertical = dim == Dimension::Vertical;
    let mut dist = width.abs();
    let snap = if vertical {
        other_flag::VERT_SNAP
    } else {
        other_flag::HORZ_SNAP
    };
    if hints.has(snap) {
        dist = strong_width(hints, widths.used(), vertical, dist, false);
    } else if let Some(standard) = widths.used().first().filter(|s| (dist - s.cur).abs() < 40) {
        dist = standard.cur.max(48);
    } else if dist < 54 {
        dist += (54 - dist) / 2;
    } else if dist < 3 * 64 {
        let delta = dist & 63;
        dist &= -64;
        dist += match delta {
            10..22 => 10,
            42..54 => 54,
            _ => delta,
        };
    }
    if width < 0 { -dist } else { dist }
}

/// Places `stem` at the fitted distance from `base`
/// (`af_cjk_align_linked_edge`).
fn align_linked_edge(
    hints: &mut GlyphHints,
    widths: &AxisWidths,
    dim: Dimension,
    base: usize,
    stem: usize,
) {
    let edges = &hints.axis(dim).edges;
    let (base_edge, stem_edge) = (edges[base], edges[stem]);
    let fitted = stem_width(hints, widths, dim, stem_edge.opos - base_edge.opos);
    hints.axis_mut(dim).edges[stem].pos = base_edge.pos + fitted;
}

/// Fits the stem `edge`-`edge2`, its original centre shifted by `anchor`,
/// moving it by as little as possible to put its edges on the grid;
/// returns the shift (`af_hint_normal_stem`).
fn hint_normal_stem(
    hints: &mut GlyphHints,
    widths: &AxisWidths,
    dim: Dimension,
    edge: usize,
    edge2: usize,
    anchor: i64,
) -> i64 {
    let edges = &hints.axis(dim).edges;
    let (e1, e2) = (edges[edge], edges[edge2]);
    let adjust = hints.has(other_flag::STEM_ADJUST);
    let threshold = if adjust {
        64
    } else {
        let both_round = e1.flags & e2.flags & edge_flag::ROUND != 0;
        let gap = match (dim, both_round) {
            (Dimension::Vertical, true) => LIGHT_MAX_HORZ_GAP,
            (Dimension::Horizontal, true) => LIGHT_MAX_VERT_GAP,
            (Dimension::Vertical, false) => LIGHT_MAX_HORZ_GAP / 3,
            (Dimension::Horizontal, false) => LIGHT_MAX_VERT_GAP / 3,
        };
        64 - gap
    };
    let cur_len = stem_width(hints, widths, dim, e2.opos - e1.opos);
    let org_center = i64::midpoint(e1.opos, e2.opos) + anchor;
    let mut cur_pos1 = org_center - cur_len / 2;
    let mut delta = stem_shift(cur_pos1, cur_len, threshold);
    if !adjust {
        delta = delta.clamp(-LIGHT_MAX_DELTA, LIGHT_MAX_DELTA);
    }
    cur_pos1 += delta;
    let edges = &mut hints.axis_mut(dim).edges;
    if e1.opos < e2.opos {
        edges[edge].pos = cur_pos1;
        edges[edge2].pos = cur_pos1 + cur_len;
    } else {
        edges[edge].pos = cur_pos1 + cur_len;
        edges[edge2].pos = cur_pos1;
    }
    delta
}

/// The shift that puts a stem from `cur_pos1`, `cur_len` long, on the
/// grid (the core of `af_hint_normal_stem`).
fn stem_shift(cur_pos1: i64, cur_len: i64, threshold: i64) -> i64 {
    let cur_pos2 = cur_pos1 + cur_len;
    let mut d_off1 = cur_pos1 - pix_floor(cur_pos1);
    let mut d_off2 = cur_pos2 - pix_floor(cur_pos2);
    let mut u_off1 = 64 - d_off1;
    let mut u_off2 = 64 - d_off2;
    if d_off1 == 0 || d_off2 == 0 {
        return 0;
    }
    if cur_len <= threshold {
        if d_off2 < cur_len {
            return if u_off1 <= d_off2 { u_off1 } else { -d_off2 };
        }
        return 0;
    }
    if threshold < 64
        && (d_off1 >= threshold
            || u_off1 >= threshold
            || d_off2 >= threshold
            || u_off2 >= threshold)
    {
        return 0;
    }
    let mut offset = cur_len & 63;
    if offset < 32 {
        if u_off1 <= offset || d_off2 <= offset {
            return 0;
        }
    } else {
        offset = 64 - threshold;
    }
    d_off1 = threshold - u_off1;
    u_off1 -= offset;
    u_off2 = threshold - d_off2;
    d_off2 -= offset;
    if d_off1 <= u_off1 {
        u_off1 = -d_off1;
    }
    if d_off2 <= u_off2 {
        u_off2 = -d_off2;
    }
    if u_off1.abs() <= u_off2.abs() {
        u_off1
    } else {
        u_off2
    }
}

/// Grid-fits the edges of `dim` (`af_cjk_hint_edges`): blue edges, then
/// stems (skipping those too close to the previous one), the symmetry of
/// `m`-like glyphs, serifs, and the rest interpolated.
pub(crate) fn hint_edges(hints: &mut GlyphHints, widths: &AxisWidths, dim: Dimension) {
    let mut anchor = align_blue_edges(hints, widths, dim);
    let mut skipped = align_stems(hints, widths, dim, &mut anchor);
    if dim == Dimension::Horizontal {
        keep_m_symmetric(&mut hints.axis_mut(dim).edges);
    }
    if skipped == 0 {
        return;
    }
    let axis = hints.axis_mut(dim);
    for edge in 0..axis.edges.len() {
        if axis.edges[edge].flags & edge_flag::DONE != 0 {
            continue;
        }
        if let Some(serif) = axis.edges[edge].serif {
            axis.align_serif(serif, edge);
            axis.edges[edge].flags |= edge_flag::DONE;
            skipped -= 1;
        }
    }
    if skipped == 0 {
        return;
    }
    let edges = &mut axis.edges;
    for edge in 0..edges.len() {
        if edges[edge].flags & edge_flag::DONE == 0 {
            interpolate(edges, edge);
        }
    }
}

/// Snaps edges in blue zones (or linked to one); returns the first edge
/// handled, the anchor.
fn align_blue_edges(hints: &mut GlyphHints, widths: &AxisWidths, dim: Dimension) -> Option<usize> {
    let mut anchor = None;
    for edge in 0..hints.axis(dim).edges.len() {
        let edges = &mut hints.axis_mut(dim).edges;
        if edges[edge].flags & edge_flag::DONE != 0 {
            continue;
        }
        let link = edges[edge].link;
        let (edge1, edge2, blue) = if let Some(blue) = edges[edge].blue {
            (edge, link, blue)
        } else if let Some((other, blue)) = link.and_then(|l| edges[l].blue.map(|b| (l, b))) {
            (other, Some(edge), blue)
        } else {
            continue;
        };
        edges[edge1].pos = blue;
        edges[edge1].flags |= edge_flag::DONE;
        if let Some(edge2) = edge2.filter(|&e| edges[e].blue.is_none()) {
            align_linked_edge(hints, widths, dim, edge1, edge2);
            hints.axis_mut(dim).edges[edge2].flags |= edge_flag::DONE;
        }
        anchor = anchor.or(Some(edge));
    }
    anchor
}

/// Fits the stems; returns how many edges were skipped (lone edges and
/// stems too close to the previous one).
fn align_stems(
    hints: &mut GlyphHints,
    widths: &AxisWidths,
    dim: Dimension,
    anchor: &mut Option<usize>,
) -> i64 {
    let mut skipped = 0;
    let mut delta = 0;
    let mut last_stem: Option<i64> = None;
    for edge in 0..hints.axis(dim).edges.len() {
        let edges = &hints.axis(dim).edges;
        if edges[edge].flags & edge_flag::DONE != 0 {
            continue;
        }
        let Some(edge2) = edges[edge].link else {
            skipped += 1;
            continue;
        };
        // Stems too near the previous one are interpolated at the end, so
        // that they do not merge.
        if last_stem.is_some_and(|last| edges[edge].pos < last + 64 || edges[edge2].pos < last + 64)
        {
            skipped += 1;
            continue;
        }
        if edges[edge2].blue.is_some() {
            align_linked_edge(hints, widths, dim, edge2, edge);
            hints.axis_mut(dim).edges[edge].flags |= edge_flag::DONE;
            continue;
        }
        if edge2 < edge {
            align_linked_edge(hints, widths, dim, edge2, edge);
            let edge = &mut hints.axis_mut(dim).edges[edge];
            edge.flags |= edge_flag::DONE;
            last_stem = Some(edge.pos);
            continue;
        }
        if dim == Dimension::Horizontal && anchor.is_none() {
            delta = hint_normal_stem(hints, widths, Dimension::Horizontal, edge, edge2, 0);
        } else {
            hint_normal_stem(hints, widths, dim, edge, edge2, delta);
        }
        *anchor = Some(edge);
        let edges = &mut hints.axis_mut(dim).edges;
        edges[edge].flags |= edge_flag::DONE;
        edges[edge2].flags |= edge_flag::DONE;
        last_stem = Some(edges[edge2].pos);
    }
    skipped
}

/// Keeps the three stems of an `m` evenly spaced (six or twelve vertical
/// edges, each stem's edges adjacent).
fn keep_m_symmetric(edges: &mut [Edge]) {
    let (edge1, edge2, edge3) = match edges.len() {
        6 => (0, 2, 4),
        12 => (1, 5, 9),
        _ => return,
    };
    let span =
        ((edges[edge2].opos - edges[edge1].opos) - (edges[edge3].opos - edges[edge2].opos)).abs();
    let paired = [edge1, edge2, edge3]
        .iter()
        .all(|&e| edges[e].link == Some(e + 1));
    if !paired || span >= 8 {
        return;
    }
    let delta = edges[edge3].pos - (2 * edges[edge2].pos - edges[edge1].pos);
    edges[edge3].pos -= delta;
    edges[edge3 + 1].pos -= delta;
    if edges.len() == 12 {
        edges[8].pos -= delta;
        edges[11].pos -= delta;
    }
    edges[edge3].flags |= edge_flag::DONE;
    edges[edge3 + 1].flags |= edge_flag::DONE;
}

/// Places an undone edge from the nearest done edges around it (without
/// marking it done).
fn interpolate(edges: &mut [Edge], edge: usize) {
    let done = |e: &Edge| e.flags & edge_flag::DONE != 0;
    let before = edges[..edge].iter().rposition(done);
    let after = edges[edge + 1..]
        .iter()
        .position(done)
        .map(|i| edge + 1 + i);
    let shifted = |base: Edge, edge: Edge| base.pos + (edge.opos - base.opos);
    edges[edge].pos = match (before, after) {
        (None, None) => return,
        (None, Some(after)) => shifted(edges[after], edges[edge]),
        (Some(before), None) => shifted(edges[before], edges[edge]),
        (Some(before), Some(after)) => {
            let (b, a, e) = (edges[before], edges[after], edges[edge]);
            if a.fpos == b.fpos {
                b.pos
            } else {
                b.pos
                    + mul_div(
                        i64::from(e.fpos) - i64::from(b.fpos),
                        a.pos - b.pos,
                        i64::from(a.fpos) - i64::from(b.fpos),
                    )
            }
        }
    };
}

/// Moves the points of every edge's segments onto the edge: to its
/// position when stems snap, else by the edge's shift
/// (`af_cjk_align_edge_points`).
pub(crate) fn align_edge_points(hints: &mut GlyphHints, dim: Dimension) {
    let snap = match dim {
        Dimension::Horizontal => other_flag::HORZ_SNAP,
        Dimension::Vertical => other_flag::VERT_SNAP,
    };
    let snapping = hints.has(snap);
    let touch = match dim {
        Dimension::Horizontal => point_flag::TOUCH_X,
        Dimension::Vertical => point_flag::TOUCH_Y,
    };
    let GlyphHints { points, axis, .. } = hints;
    let axis = &axis[dim.index()];
    for (index, edge) in axis.edges.iter().enumerate() {
        for segment in axis.edge_segments(index) {
            let segment = axis.segments[segment];
            let mut point = segment.first;
            loop {
                let p = &mut points[point];
                let coordinate = match dim {
                    Dimension::Horizontal => &mut p.x,
                    Dimension::Vertical => &mut p.y,
                };
                if snapping {
                    *coordinate = edge.pos;
                } else {
                    *coordinate += edge.pos - edge.opos;
                }
                p.flags |= touch;
                if point == segment.last {
                    break;
                }
                point = p.next;
            }
        }
    }
}
