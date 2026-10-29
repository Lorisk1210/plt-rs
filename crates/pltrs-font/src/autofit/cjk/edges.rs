//! CJK stems, edges and blue edges (`af_cjk_hints_link_segments`,
//! `af_cjk_hints_compute_edges`, `af_cjk_hints_compute_blue_edges`).

use super::CjkBlue;
use crate::autofit::hints::{
    AxisHints, Dimension, Edge, GlyphHints, Segment, edge_flag, segment_distance,
};
use crate::autofit::latin::segments::latin_constant;
use crate::fixed::{div_fix, mul_fix};

/// Links facing segments into stems, preferring near and long overlaps,
/// then turns the ends of strokes that widen into serifs (or unlinks
/// them).
pub(crate) fn link_segments(hints: &mut GlyphHints, dim: Dimension, units_per_em: u16) {
    let dist_threshold = div_fix(64 * 3, hints.scale(dim));
    let axis = hints.axis_mut(dim);
    let major_dir = axis.major_dir;
    let segments = &mut axis.segments;
    let len_threshold = latin_constant(units_per_em, 8);
    for seg1 in 0..segments.len() {
        if segments[seg1].dir != major_dir {
            continue;
        }
        for seg2 in 0..segments.len() {
            let (a, b) = (segments[seg1], segments[seg2]);
            if seg2 == seg1 || !a.dir.is_opposite(b.dir) {
                continue;
            }
            let dist = i64::from(b.pos) - i64::from(a.pos);
            if dist < 0 {
                continue;
            }
            let len =
                i64::from(a.max_coord.min(b.max_coord)) - i64::from(a.min_coord.max(b.min_coord));
            if len < len_threshold {
                continue;
            }
            for (this, other) in [(seg1, seg2), (seg2, seg1)] {
                let segment = &mut segments[this];
                if dist * 8 < segment.score * 9
                    && (dist * 8 < segment.score * 7 || segment.len < len)
                {
                    segment.score = dist;
                    segment.len = len;
                    segment.link = Some(other);
                }
            }
        }
    }
    mark_serifs(segments, dist_threshold);
    for seg1 in 0..segments.len() {
        let Some(seg2) = segments[seg1].link else {
            continue;
        };
        if segments[seg2].link != Some(seg1) {
            segments[seg1].link = None;
            if segments[seg2].score < dist_threshold
                || segments[seg1].score < segments[seg2].score * 4
            {
                segments[seg1].serif = segments[seg2].link;
            }
        }
    }
}

/// Strokes wider at one end: the narrow stem `seg1`-`link1` inside a
/// wider one `seg2`-`link2` makes the outer segments serifs when it is
/// much longer, else it is unlinked.
fn mark_serifs(segments: &mut [crate::autofit::hints::Segment], dist_threshold: i64) {
    for seg1 in 0..segments.len() {
        let Some(link1) = segments[seg1].link else {
            continue;
        };
        if segments[link1].link != Some(seg1) || segments[link1].pos <= segments[seg1].pos {
            continue;
        }
        if segments[seg1].score >= dist_threshold {
            continue;
        }
        for seg2 in 0..segments.len() {
            if segments[seg2].pos > segments[seg1].pos || seg1 == seg2 {
                continue;
            }
            let Some(link2) = segments[seg2].link else {
                continue;
            };
            if segments[link2].link != Some(seg2) || segments[link2].pos < segments[link1].pos {
                continue;
            }
            if segments[seg1].pos == segments[seg2].pos
                && segments[link1].pos == segments[link2].pos
            {
                continue;
            }
            let (score1, score2) = (segments[seg1].score, segments[seg2].score);
            if score2 <= score1 || score1 * 4 <= score2 {
                continue;
            }
            if segments[seg1].len >= segments[seg2].len * 3 {
                for segment in segments.iter_mut() {
                    if segment.link == Some(seg2) {
                        segment.link = None;
                        segment.serif = Some(link1);
                    } else if segment.link == Some(link2) {
                        segment.link = None;
                        segment.serif = Some(seg1);
                    }
                }
            } else {
                segments[seg1].link = None;
                segments[link1].link = None;
                break;
            }
        }
    }
}

/// The edge of `segment`'s direction nearest to it within `threshold`
/// whose linked segments stay together with `segment`'s link.
fn nearest_edge(axis: &AxisHints, segment: &Segment, threshold: i64) -> Option<usize> {
    let mut found = None;
    let mut best = 0xFFFF;
    for (index, edge) in axis.edges.iter().enumerate() {
        if edge.dir != segment.dir {
            continue;
        }
        let dist = (i64::from(segment.pos) - i64::from(edge.fpos)).abs();
        if dist >= threshold || dist >= best {
            continue;
        }
        if let Some(link) = segment.link {
            let mut dist2 = 0;
            for member in axis.edge_segments(index) {
                if let Some(link1) = axis.segments[member].link {
                    dist2 = segment_distance(&axis.segments[link], &axis.segments[link1]);
                    if dist2 >= threshold {
                        break;
                    }
                }
            }
            if dist2 >= threshold {
                continue;
            }
        }
        best = dist;
        found = Some(index);
    }
    found
}

/// Groups segments into edges: a segment joins the nearest edge of its
/// direction within the threshold whose linked segments stay together.
pub(crate) fn compute_edges(hints: &mut GlyphHints, dim: Dimension, edge_distance_threshold: i64) {
    let scale = hints.scale(dim);
    let threshold = if mul_fix(edge_distance_threshold, scale) > 64 / 4 {
        div_fix(64 / 4, scale)
    } else {
        edge_distance_threshold
    };
    let axis = hints.axis_mut(dim);
    axis.edges.clear();
    for seg in 0..axis.segments.len() {
        let segment = axis.segments[seg];
        let found = nearest_edge(axis, &segment, threshold);
        match found {
            None => {
                axis.insert_edge(Edge::new(seg, segment.dir, segment.pos, scale), false);
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
            let serif = segment
                .serif
                .filter(|&serif| axis.segments[serif].edge != Some(edge));
            let (other, current) = match (serif, segment.link) {
                (Some(serif), _) => (serif, axis.edges[edge].serif),
                (None, Some(link)) => (link, axis.edges[edge].link),
                (None, None) => continue,
            };
            let other_edge = axis.segments[other].edge;
            let edge2 = match current {
                Some(current) => {
                    let edge_delta = (i64::from(axis.edges[edge].fpos)
                        - i64::from(axis.edges[current].fpos))
                    .abs();
                    if segment_distance(&segment, &axis.segments[other]) < edge_delta {
                        other_edge
                    } else {
                        Some(current)
                    }
                }
                None => other_edge,
            };
            if serif.is_some() {
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

/// Aligns each vertical edge to the nearest point (reference or
/// undershoot, whichever is closer) of an active blue zone of its side.
pub(crate) fn compute_blue_edges(hints: &mut GlyphHints, blues: &[CjkBlue], units_per_em: u16) {
    let scale = hints.y_scale;
    let best_dist0 = mul_fix(i64::from(units_per_em) / 40, scale).min(64 / 2);
    let axis = hints.axis_mut(Dimension::Vertical);
    let major_dir = axis.major_dir;
    for edge in &mut axis.edges {
        let fpos = i64::from(edge.fpos);
        let mut best_dist = best_dist0;
        let mut best = None;
        for blue in blues.iter().filter(|blue| blue.active) {
            if !(blue.top ^ (edge.dir == major_dir)) {
                continue;
            }
            let compare = if (fpos - blue.reference.org).abs() > (fpos - blue.overshoot.org).abs() {
                blue.overshoot
            } else {
                blue.reference
            };
            let dist = mul_fix((fpos - compare.org).abs(), scale);
            if dist < best_dist {
                best_dist = dist;
                best = Some(compare.fit);
            }
        }
        if best.is_some() {
            edge.blue = best;
        }
    }
}
