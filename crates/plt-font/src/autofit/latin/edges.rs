//! Edges from segments and their blue zones (`af_latin_hints_compute_edges`,
//! `af_latin_hints_compute_blue_edges`).

use super::metrics::{ScaledLatin, blue_flag};
use crate::autofit::hints::{AxisHints, Dimension, Direction, Edge, GlyphHints, edge_flag};
use crate::fixed::{div_fix, mul_fix};

/// Groups the segments of `dim` into edges, sorted by position, and
/// computes each edge's roundness, stem link and serif.
/// `edge_distance_threshold` (font units) is the style's; `top_to_bottom`
/// orders vertical edges downwards.
pub(crate) fn compute_edges(
    hints: &mut GlyphHints,
    dim: Dimension,
    edge_distance_threshold: i64,
    top_to_bottom: bool,
) {
    let scale = hints.scale(dim);
    let top_to_bottom = dim == Dimension::Vertical && top_to_bottom;
    // Segments shorter than a pixel or wider than half a pixel are
    // ignored (the length threshold applies to vertical segments only).
    let length_threshold = match dim {
        Dimension::Horizontal => div_fix(64, hints.y_scale),
        Dimension::Vertical => 0,
    };
    let width_threshold = div_fix(32, scale);
    // At most a quarter pixel apart.
    let threshold = div_fix(mul_fix(edge_distance_threshold, scale).min(64 / 4), scale);
    let axis = hints.axis_mut(dim);
    axis.edges.clear();
    for seg in 0..axis.segments.len() {
        let segment = axis.segments[seg];
        let height = i64::from(segment.height);
        if height < length_threshold
            || i64::from(segment.delta) > width_threshold
            || segment.dir == Direction::NONE
        {
            continue;
        }
        // Serifs smaller than 1.5 pixels are ignored.
        if segment.serif.is_some() && 2 * height < 3 * length_threshold {
            continue;
        }
        let found = axis.edges.iter().position(|edge| {
            (i64::from(segment.pos) - i64::from(edge.fpos)).abs() < threshold
                && edge.dir == segment.dir
        });
        add_to_edge(axis, seg, found, scale, top_to_bottom);
    }
    // One-point segments without a direction join an existing edge.
    for seg in 0..axis.segments.len() {
        let segment = axis.segments[seg];
        if segment.dir != Direction::NONE {
            continue;
        }
        let found = axis
            .edges
            .iter()
            .position(|edge| (i64::from(segment.pos) - i64::from(edge.fpos)).abs() < threshold);
        if found.is_some() {
            add_to_edge(axis, seg, found, scale, top_to_bottom);
        }
    }
    axis.link_edge_segments();
    for edge in 0..axis.edges.len() {
        let (mut round, mut straight) = (0, 0);
        let members: Vec<usize> = axis.edge_segments(edge).collect();
        for seg in members {
            let segment = axis.segments[seg];
            if segment.flags & edge_flag::ROUND != 0 {
                round += 1;
            } else {
                straight += 1;
            }
            let serif_edge = segment.serif.and_then(|serif| axis.segments[serif].edge);
            let is_serif = serif_edge.is_some_and(|serif_edge| serif_edge != edge);
            let link_edge = segment.link.and_then(|link| axis.segments[link].edge);
            if link_edge.is_none() && !is_serif {
                continue;
            }
            let (other, mut edge2) = if is_serif {
                (segment.serif, axis.edges[edge].serif)
            } else {
                (segment.link, axis.edges[edge].link)
            };
            let other = other.expect("a serif or link segment");
            let other_edge = axis.segments[other].edge;
            edge2 = match edge2 {
                Some(current) => {
                    let edge_delta = (i64::from(axis.edges[edge].fpos)
                        - i64::from(axis.edges[current].fpos))
                    .abs();
                    let seg_delta =
                        (i64::from(segment.pos) - i64::from(axis.segments[other].pos)).abs();
                    if seg_delta < edge_delta {
                        other_edge
                    } else {
                        Some(current)
                    }
                }
                None => other_edge,
            };
            if is_serif {
                axis.edges[edge].serif = edge2;
                if let Some(edge2) = edge2 {
                    axis.edges[edge2].flags |= edge_flag::SERIF;
                }
            } else {
                axis.edges[edge].link = edge2;
            }
        }
        let edge = &mut axis.edges[edge];
        edge.flags = if round > 0 && round >= straight {
            edge_flag::ROUND
        } else {
            0
        };
        if edge.serif.is_some() && edge.link.is_some() {
            edge.serif = None;
        }
    }
}

/// Adds segment `seg` to edge `found`, or to a new edge at its position.
fn add_to_edge(
    axis: &mut AxisHints,
    seg: usize,
    found: Option<usize>,
    scale: i64,
    top_to_bottom: bool,
) {
    let segment = axis.segments[seg];
    match found {
        None => {
            axis.insert_edge(
                Edge::new(seg, segment.dir, segment.pos, scale),
                top_to_bottom,
            );
            axis.segments[seg].edge_next = seg;
        }
        Some(edge) => {
            let (first, last) = (axis.edges[edge].first, axis.edges[edge].last);
            axis.segments[seg].edge_next = first;
            axis.segments[last].edge_next = seg;
            axis.edges[edge].last = seg;
        }
    }
}

/// Aligns the horizontal edges to the closest active blue zone within
/// reach: top zones take edges against the major direction, bottom zones
/// edges along it, neutral zones both; round edges may snap to the
/// overshoot.
pub(crate) fn compute_blue_edges(hints: &mut GlyphHints, metrics: &ScaledLatin, units_per_em: u16) {
    let vertical = &metrics.axis[Dimension::Vertical.index()];
    let scale = vertical.scale;
    let axis = hints.axis_mut(Dimension::Vertical);
    let major_dir = axis.major_dir;
    for edge in &mut axis.edges {
        if edge.flags & edge_flag::NO_BLUE != 0 {
            continue;
        }
        let mut best_dist = mul_fix(i64::from(units_per_em) / 40, scale).min(64 / 2);
        let mut best: Option<(i64, bool)> = None;
        for blue in vertical
            .blues
            .iter()
            .filter(|blue| blue.flags & blue_flag::ACTIVE != 0)
        {
            let is_top = blue.flags & (blue_flag::TOP | blue_flag::SUB_TOP) != 0;
            let is_neutral = blue.flags & blue_flag::NEUTRAL != 0;
            let is_major_dir = edge.dir == major_dir;
            if !(is_top ^ is_major_dir || is_neutral) {
                continue;
            }
            let fpos = i64::from(edge.fpos);
            let dist = mul_fix((fpos - blue.reference.org).abs(), scale);
            if dist < best_dist {
                best_dist = dist;
                best = Some((blue.reference.fit, is_neutral));
            }
            if edge.flags & edge_flag::ROUND != 0 && dist != 0 && !is_neutral {
                let is_under_ref = fpos < blue.reference.org;
                if is_top ^ is_under_ref {
                    let dist = mul_fix((fpos - blue.overshoot.org).abs(), scale);
                    if dist < best_dist {
                        best_dist = dist;
                        best = Some((blue.overshoot.fit, is_neutral));
                    }
                }
            }
        }
        if let Some((fit, neutral)) = best {
            edge.blue = Some(fit);
            if neutral {
                edge.flags |= edge_flag::NEUTRAL;
            }
        }
    }
}
