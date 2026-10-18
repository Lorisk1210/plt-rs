//! Moving the outline points after the edges are grid-fitted: segment
//! points onto their edges, strong points interpolated between edges (the
//! TrueType `IP` instruction) and weak points between their touched
//! neighbours (`IUP`).

use std::cmp::Ordering;

use super::{Dimension, GlyphHints, Point, point_flag};
use crate::fixed::{div_fix, mul_fix};

impl GlyphHints {
    /// Puts the points of every segment that belongs to an edge onto the
    /// edge (`af_glyph_hints_align_edge_points`).
    pub(crate) fn align_edge_points(&mut self, dim: Dimension) {
        let axis = &self.axis[dim.index()];
        for segment in &axis.segments {
            let Some(edge) = segment.edge else {
                continue;
            };
            let pos = axis.edges[edge].pos;
            let mut point = segment.first;
            loop {
                let p = &mut self.points[point];
                match dim {
                    Dimension::Horizontal => {
                        p.x = pos;
                        p.flags |= point_flag::TOUCH_X;
                    }
                    Dimension::Vertical => {
                        p.y = pos;
                        p.flags |= point_flag::TOUCH_Y;
                    }
                }
                if point == segment.last {
                    break;
                }
                point = p.next;
            }
        }
    }

    /// Interpolates the untouched strong points between the edges around
    /// them (`af_glyph_hints_align_strong_points`).
    pub(crate) fn align_strong_points(&mut self, dim: Dimension) {
        let touch = touch_flag(dim);
        let edge_count = self.axis[dim.index()].edges.len();
        if edge_count == 0 {
            return;
        }
        for index in 0..self.points.len() {
            let point = self.points[index];
            if point.flags & (touch | point_flag::WEAK_INTERPOLATION) != 0 {
                continue;
            }
            let (fu, ou) = match dim {
                Dimension::Vertical => (i64::from(point.fy), point.oy),
                Dimension::Horizontal => (i64::from(point.fx), point.ox),
            };
            let u = self.strong_position(dim, fu, ou);
            let point = &mut self.points[index];
            match dim {
                Dimension::Horizontal => point.x = u,
                Dimension::Vertical => point.y = u,
            }
            point.flags |= touch;
        }
    }

    /// The aligned position of a strong point at font-unit coordinate
    /// `fu` (scaled `ou`).
    fn strong_position(&mut self, dim: Dimension, fu: i64, ou: i64) -> i64 {
        let edges = &self.axis[dim.index()].edges;
        let (first, last) = (edges[0], edges[edges.len() - 1]);
        if i64::from(first.fpos) - fu >= 0 {
            return first.pos - (first.opos - ou);
        }
        if fu - i64::from(last.fpos) >= 0 {
            return last.pos + (ou - last.opos);
        }
        // Find the enclosing edges: linearly for few edges, else by a
        // binary search.
        let count = edges.len();
        let min = if count <= 8 {
            let nn = edges
                .iter()
                .position(|edge| i64::from(edge.fpos) >= fu)
                .unwrap_or(count);
            if edges.get(nn).is_some_and(|edge| i64::from(edge.fpos) == fu) {
                return edges[nn].pos;
            }
            nn
        } else {
            let (mut min, mut max) = (0, count);
            while min < max {
                let mid = usize::midpoint(max, min);
                match fu.cmp(&i64::from(edges[mid].fpos)) {
                    Ordering::Less => max = mid,
                    Ordering::Greater => min = mid + 1,
                    Ordering::Equal => return edges[mid].pos,
                }
            }
            min
        };
        let before = min - 1;
        let scale = self.edge_scale(dim, before, min);
        let before = self.axis[dim.index()].edges[before];
        before.pos + mul_fix(fu - i64::from(before.fpos), scale)
    }

    /// Interpolates every untouched point from the touched points around
    /// it on its contour (`af_glyph_hints_align_weak_points`).
    pub(crate) fn align_weak_points(&mut self, dim: Dimension) {
        let touch = touch_flag(dim);
        for point in &mut self.points {
            (point.u, point.v) = match dim {
                Dimension::Horizontal => (point.x, point.ox),
                Dimension::Vertical => (point.y, point.oy),
            };
        }
        for contour in 0..self.contours.len() {
            let first_point = self.contours[contour];
            let end_point = self.points[first_point].prev;
            self.interpolate_contour(first_point, end_point, touch);
        }
        for point in &mut self.points {
            match dim {
                Dimension::Horizontal => point.x = point.u,
                Dimension::Vertical => point.y = point.u,
            }
        }
    }

    /// Weak point interpolation of the contour `first_point..=end_point`.
    fn interpolate_contour(&mut self, first_point: usize, end_point: usize, touch: u16) {
        let points = &mut self.points;
        let touched = |points: &[Point], index: usize| points[index].flags & touch != 0;
        let Some(first_touched) = (first_point..=end_point).find(|&p| touched(points, p)) else {
            return;
        };
        let mut point = first_touched;
        let last_touched = loop {
            // Skip touched neighbours.
            while point < end_point && touched(points, point + 1) {
                point += 1;
            }
            let last_touched = point;
            match (point + 1..=end_point).find(|&p| touched(points, p)) {
                Some(next) => {
                    iup_interpolate(points, last_touched + 1, next - 1, last_touched, next);
                    point = next;
                }
                None => break last_touched,
            }
        };
        if last_touched == first_touched {
            iup_shift(points, first_point, end_point, first_touched);
        } else {
            if last_touched < end_point {
                iup_interpolate(
                    points,
                    last_touched + 1,
                    end_point,
                    last_touched,
                    first_touched,
                );
            }
            if first_touched > 0 {
                iup_interpolate(
                    points,
                    first_point,
                    first_touched - 1,
                    last_touched,
                    first_touched,
                );
            }
        }
    }
}

/// The touch flag of `dim`.
const fn touch_flag(dim: Dimension) -> u16 {
    match dim {
        Dimension::Horizontal => point_flag::TOUCH_X,
        Dimension::Vertical => point_flag::TOUCH_Y,
    }
}

/// Shifts the points `p1..=p2` (except `reference`) by the distance
/// `reference` moved (`af_iup_shift`).
fn iup_shift(points: &mut [Point], p1: usize, p2: usize, reference: usize) {
    let delta = points[reference].u - points[reference].v;
    if delta == 0 {
        return;
    }
    for (index, point) in points.iter_mut().enumerate().take(p2 + 1).skip(p1) {
        if index != reference {
            point.u = point.v + delta;
        }
    }
}

/// Interpolates the points `p1..=p2` between the references `ref1` and
/// `ref2`, as the TrueType `IUP` instruction does (`af_iup_interp`).
fn iup_interpolate(points: &mut [Point], p1: usize, p2: usize, ref1: usize, ref2: usize) {
    if p1 > p2 {
        return;
    }
    let (mut ref1, mut ref2) = (points[ref1], points[ref2]);
    if ref1.v > ref2.v {
        std::mem::swap(&mut ref1, &mut ref2);
    }
    let (v1, v2, u1, u2) = (ref1.v, ref2.v, ref1.u, ref2.u);
    let (d1, d2) = (u1 - v1, u2 - v2);
    let scale = (u1 != u2 && v1 != v2).then(|| div_fix(u2 - u1, v2 - v1));
    for point in &mut points[p1..=p2] {
        let u = point.v;
        point.u = if u <= v1 {
            u + d1
        } else if u >= v2 {
            u + d2
        } else {
            scale.map_or(u1, |scale| u1 + mul_fix(u - v1, scale))
        };
    }
}
