//! The latin grid-fitting of one dimension's edges (`af_latin_hint_edges`):
//! blue zone edges first, then stems relative to an anchor, the symmetry
//! of `m`-like glyphs, and finally serifs and lone edges.

use super::metrics::ScaledLatin;
use super::stems::{align_linked_edge, stem_width};
use crate::autofit::hints::{Dimension, Edge, GlyphHints, edge_flag};
use crate::fixed::{mul_div, pix_round};

/// Grid-fits the edges of `dim`.
pub(crate) fn hint_edges(
    hints: &mut GlyphHints,
    metrics: &ScaledLatin,
    dim: Dimension,
    top_to_bottom: bool,
) {
    let top_to_bottom = dim == Dimension::Vertical && top_to_bottom;
    let mut anchor = None;
    if dim == Dimension::Vertical {
        align_blue_edges(hints, metrics, dim, &mut anchor);
    }
    let has_non_stem_edges = align_stems(hints, metrics, dim, top_to_bottom, &mut anchor);
    if dim == Dimension::Horizontal {
        keep_m_symmetric(hints);
    }
    if has_non_stem_edges || anchor.is_none() {
        align_remaining(hints, dim, &mut anchor);
    }
}

/// Snaps edges in blue zones (or linked to one) to the zone, and their
/// stem partners at the fitted distance.
fn align_blue_edges(
    hints: &mut GlyphHints,
    metrics: &ScaledLatin,
    dim: Dimension,
    anchor: &mut Option<usize>,
) {
    for edge in 0..hints.axis(dim).edges.len() {
        let edges = &mut hints.axis_mut(dim).edges;
        if edges[edge].flags & edge_flag::DONE != 0 {
            continue;
        }
        let mut edge2 = edges[edge].link;
        // A stem with a neutral and a non-neutral zone (or two neutral
        // ones) keeps one only.
        if let Some(other) = edge2
            && edges[edge].blue.is_some()
            && edges[other].blue.is_some()
        {
            if edges[other].flags & edge_flag::NEUTRAL != 0 {
                edges[other].blue = None;
                edges[other].flags &= !edge_flag::NEUTRAL;
            } else if edges[edge].flags & edge_flag::NEUTRAL != 0 {
                edges[edge].blue = None;
                edges[edge].flags &= !edge_flag::NEUTRAL;
            }
        }
        let (edge1, blue) = if let Some(blue) = edges[edge].blue {
            (edge, blue)
        } else if let Some(blue) = edge2.and_then(|other| edges[other].blue) {
            let other = edge2.replace(edge).expect("a linked edge");
            (other, blue)
        } else {
            continue;
        };
        edges[edge1].pos = blue;
        edges[edge1].flags |= edge_flag::DONE;
        if let Some(edge2) = edge2.filter(|&e| edges[e].blue.is_none()) {
            align_linked_edge(hints, metrics, dim, edge1, edge2);
            hints.axis_mut(dim).edges[edge2].flags |= edge_flag::DONE;
        }
        if anchor.is_none() {
            *anchor = Some(edge);
        }
    }
}

/// The fitted centre of a stem shorter than 1.5 pixels whose original
/// centre is `center` (26.6) and fitted length `len`: half a pixel off the
/// grid for thin stems, a little off-centre for the others.
fn small_stem_center(center: i64, len: i64) -> i64 {
    let (u_off, d_off) = if len <= 64 { (32, 32) } else { (38, 26) };
    let pos = pix_round(center);
    let error1 = (center - (pos - u_off)).abs();
    let error2 = (center - (pos + d_off)).abs();
    if error1 < error2 {
        pos - u_off
    } else {
        pos + d_off
    }
}

/// Aligns the remaining stems, keeping their order; returns whether
/// edges without a stem link remain.
fn align_stems(
    hints: &mut GlyphHints,
    metrics: &ScaledLatin,
    dim: Dimension,
    top_to_bottom: bool,
    anchor: &mut Option<usize>,
) -> bool {
    let mut has_non_stem_edges = false;
    for edge in 0..hints.axis(dim).edges.len() {
        let edges = &hints.axis(dim).edges;
        if edges[edge].flags & edge_flag::DONE != 0 {
            continue;
        }
        let Some(edge2) = edges[edge].link else {
            has_non_stem_edges = true;
            continue;
        };
        // Should not happen, but better safe than sorry.
        if edges[edge2].blue.is_some() {
            align_linked_edge(hints, metrics, dim, edge2, edge);
            hints.axis_mut(dim).edges[edge].flags |= edge_flag::DONE;
            continue;
        }
        let (e1, e2) = (edges[edge], edges[edge2]);
        let org_len = e2.opos - e1.opos;
        let cur_len = stem_width(hints, metrics, dim, org_len, 0, e1.flags, e2.flags);
        let edges = &mut hints.axis_mut(dim).edges;
        match *anchor {
            None => {
                // The first stem: centre short ones specially.
                if cur_len < 96 {
                    let center = small_stem_center(e1.opos + (org_len >> 1), cur_len);
                    edges[edge].pos = center - cur_len / 2;
                    edges[edge2].pos = edges[edge].pos + cur_len;
                } else {
                    edges[edge].pos = pix_round(e1.opos);
                }
                *anchor = Some(edge);
                edges[edge].flags |= edge_flag::DONE;
                align_linked_edge(hints, metrics, dim, edge, edge2);
            }
            Some(anchor) => {
                let a = edges[anchor];
                let org_pos = a.pos + (e1.opos - a.opos);
                let org_center = org_pos + (org_len >> 1);
                if e2.flags & edge_flag::DONE != 0 {
                    edges[edge].pos = e2.pos - cur_len;
                } else if cur_len < 96 {
                    let center = small_stem_center(org_center, cur_len);
                    edges[edge].pos = center - cur_len / 2;
                    edges[edge2].pos = center + cur_len / 2;
                } else {
                    let pos1 = pix_round(org_pos);
                    let delta1 = (pos1 + (cur_len >> 1) - org_center).abs();
                    let pos2 = pix_round(org_pos + org_len) - cur_len;
                    let delta2 = (pos2 + (cur_len >> 1) - org_center).abs();
                    edges[edge].pos = if delta1 < delta2 { pos1 } else { pos2 };
                    edges[edge2].pos = edges[edge].pos + cur_len;
                }
                edges[edge].flags |= edge_flag::DONE;
                edges[edge2].flags |= edge_flag::DONE;
                keep_order(edges, edge, top_to_bottom);
            }
        }
    }
    has_non_stem_edges
}

/// Moves `edge` back to its predecessor's position when it went past it,
/// unless its stem would (almost) vanish.
fn keep_order(edges: &mut [crate::autofit::hints::Edge], edge: usize, top_to_bottom: bool) {
    if edge == 0 {
        return;
    }
    let before = edges[edge - 1].pos;
    let pos = edges[edge].pos;
    let crossed = if top_to_bottom {
        pos > before
    } else {
        pos < before
    };
    if crossed
        && edges[edge]
            .link
            .is_some_and(|link| (edges[link].pos - before).abs() > 16)
    {
        edges[edge].pos = before;
    }
}

/// Keeps the three stems of a lowercase `m` (six or twelve vertical
/// edges) evenly spaced.
fn keep_m_symmetric(hints: &mut GlyphHints) {
    let edges = &mut hints.axis_mut(Dimension::Horizontal).edges;
    let (edge1, edge2, edge3) = match edges.len() {
        6 => (0, 2, 4),
        12 => (1, 5, 9),
        _ => return,
    };
    let dist1 = edges[edge2].opos - edges[edge1].opos;
    let dist2 = edges[edge3].opos - edges[edge2].opos;
    if (dist1 - dist2).abs() >= 8 {
        return;
    }
    let delta = edges[edge3].pos - (2 * edges[edge2].pos - edges[edge1].pos);
    edges[edge3].pos -= delta;
    let link = edges[edge3].link;
    if let Some(link) = link {
        edges[link].pos -= delta;
    }
    // Move the serifs along with the stem.
    if edges.len() == 12 {
        edges[8].pos -= delta;
        edges[11].pos -= delta;
    }
    edges[edge3].flags |= edge_flag::DONE;
    if let Some(link) = link {
        edges[link].flags |= edge_flag::DONE;
    }
}

/// Whether the serif of `edge` is real: no other edge's segments lie
/// between the serif and its base edge.
fn real_serif(hints: &GlyphHints, dim: Dimension, edge: usize, serif: usize) -> bool {
    let axis = hints.axis(dim);
    let edges = &axis.edges;
    let (top, bottom) = if edges[serif].opos < edges[edge].opos {
        (edge, serif)
    } else {
        (serif, edge)
    };
    // The extent of an edge's first and last segments (whatever the
    // outline's orientation).
    let extent = |e: usize| {
        let ends = [edges[e].first, edges[e].last].map(|seg| axis.segments[seg]);
        let vs = ends.map(|seg| [hints.points[seg.first].v, hints.points[seg.last].v]);
        let vs = vs.as_flattened();
        (
            vs.iter().copied().min().unwrap_or(0),
            vs.iter().copied().max().unwrap_or(0),
        )
    };
    let (top_min, top_max) = extent(top);
    let (bottom_min, bottom_max) = extent(bottom);
    let (min_pos, max_pos) = (top_min.min(bottom_min), top_max.max(bottom_max));
    (bottom + 1..top).all(|e| {
        let (e_min, e_max) = extent(e);
        (e_min < min_pos && e_max < min_pos) || (e_min > max_pos && e_max > max_pos)
    })
}

/// Hints the edges still undone: serifs relative to their base, the
/// first lone edge as the anchor if there is none, others interpolated
/// between their neighbours.
///
/// FreeType then keeps the edge between its neighbours like stems, but
/// only for edges with a stem link, and every such edge is done by now.
fn align_remaining(hints: &mut GlyphHints, dim: Dimension, anchor: &mut Option<usize>) {
    let count = hints.axis(dim).edges.len();
    for edge in 0..count {
        let edges = &hints.axis(dim).edges;
        if edges[edge].flags & edge_flag::DONE != 0 {
            continue;
        }
        let serif = edges[edge].serif;
        let delta = serif.map_or(1000, |serif| (edges[serif].opos - edges[edge].opos).abs());
        if let Some(serif) = serif
            && delta < 64 + 32
            && !real_serif(hints, dim, edge, serif)
        {
            continue;
        }
        let axis = hints.axis_mut(dim);
        match (serif, *anchor) {
            (Some(serif), _) if delta < 64 + 16 => axis.align_serif(serif, edge),
            (_, None) => {
                axis.edges[edge].pos = pix_round(axis.edges[edge].opos);
                *anchor = Some(edge);
            }
            (_, Some(anchor)) => interpolate_lone_edge(&mut axis.edges, edge, anchor),
        }
        axis.edges[edge].flags |= edge_flag::DONE;
    }
}

/// Places a lone edge between the nearest done edges around it, or by
/// rounding its distance to the anchor to half pixels.
fn interpolate_lone_edge(edges: &mut [Edge], edge: usize, anchor: usize) {
    let done = |e: &Edge| e.flags & edge_flag::DONE != 0;
    let before = edges[..edge].iter().rposition(done);
    let after = edges[edge + 1..]
        .iter()
        .position(done)
        .map(|i| edge + 1 + i);
    edges[edge].pos = if let (Some(before), Some(after)) = (before, after) {
        let (b, a) = (edges[before], edges[after]);
        if a.opos == b.opos {
            b.pos
        } else {
            b.pos + mul_div(edges[edge].opos - b.opos, a.pos - b.pos, a.opos - b.opos)
        }
    } else {
        let a = edges[anchor];
        a.pos + ((edges[edge].opos - a.opos + 16) & !31)
    };
}
