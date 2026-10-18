//! Loading an outline into the hints: scaled points, contour links, the
//! direction of each point's in and out vectors and the weak/strong point
//! classification (`af_glyph_hints_reload`).

use super::{Direction, FontOutline, GlyphHints, Point, point_flag};
use crate::fixed::mul_fix;
use crate::outline::PointKind;

/// `FT_HYPOT`: the "alpha max plus beta min" length approximation.
fn hypot(x: i64, y: i64) -> i64 {
    let (x, y) = (x.wrapping_abs(), y.wrapping_abs());
    if x > y {
        x.wrapping_add(3i64.wrapping_mul(y) >> 3)
    } else {
        y.wrapping_add(3i64.wrapping_mul(x) >> 3)
    }
}

/// Whether the corner between vectors `in` and `out` is flat, that is,
/// one of them dominates (`ft_corner_is_flat`).
fn corner_is_flat(in_x: i64, in_y: i64, out_x: i64, out_y: i64) -> bool {
    let (ax, ay) = (in_x.wrapping_add(out_x), in_y.wrapping_add(out_y));
    let (d_in, d_out, d_hypot) = (hypot(in_x, in_y), hypot(out_x, out_y), hypot(ax, ay));
    d_in.wrapping_add(d_out).wrapping_sub(d_hypot) < (d_hypot >> 4)
}

/// `FT_MSB` of a non-zero value.
fn msb(value: u32) -> i32 {
    31 - value.leading_zeros() as i32
}

/// Whether the outline is filled with PostScript's orientation (outer
/// contours counter-clockwise), from the signed area of the control
/// polygon (`FT_Outline_Get_Orientation`).
fn is_postscript(outline: &FontOutline) -> bool {
    const LIMIT: i64 = 0x100_0000;
    let Some(&first) = outline.points.first() else {
        return false;
    };
    let (mut x_min, mut y_min, mut x_max, mut y_max) = (first.0, first.1, first.0, first.1);
    for &(x, y) in &outline.points {
        (x_min, y_min, x_max, y_max) = (x_min.min(x), y_min.min(y), x_max.max(x), y_max.max(y));
    }
    if x_min == x_max || y_min == y_max {
        return false;
    }
    if x_min < -LIMIT || y_min < -LIMIT || x_max > LIMIT || y_max > LIMIT {
        return false;
    }
    let x_shift = (msb((x_max.abs() | x_min.abs()) as u32) - 14).max(0);
    let y_shift = (msb((y_max - y_min) as u32) - 14).max(0);
    let mut area = 0i64;
    let mut start = 0;
    for &last in &outline.contour_ends {
        let shifted = |(x, y): (i64, i64)| (x >> x_shift, y >> y_shift);
        let mut previous = shifted(outline.points[last]);
        for &point in &outline.points[start..=last] {
            let current = shifted(point);
            area = area.wrapping_add((current.1 - previous.1).wrapping_mul(current.0 + previous.0));
            previous = current;
        }
        start = last + 1;
    }
    area > 0
}

/// A point index moved by an index delta stored in `u` or `v`.
fn offset(index: usize, delta: i64) -> usize {
    (index as i64 + delta) as usize
}

impl GlyphHints {
    /// Recomputes the points from `outline` (in font units), scaled with
    /// the hints' scales (FreeType's deltas are always zero here);
    /// `units_per_em` sets the threshold
    /// below which neighbouring points count as near.
    pub(crate) fn reload(&mut self, outline: &FontOutline, units_per_em: u16) {
        for axis in &mut self.axis {
            axis.segments.clear();
            axis.edges.clear();
        }
        self.points.clear();
        self.contours.clear();
        self.contour_y_minima.clear();
        self.contour_y_maxima.clear();
        let postscript = is_postscript(outline);
        self.axis[0].major_dir = if postscript {
            Direction::DOWN
        } else {
            Direction::UP
        };
        self.axis[1].major_dir = if postscript {
            Direction::RIGHT
        } else {
            Direction::LEFT
        };
        if outline.points.is_empty() {
            return;
        }
        let near_limit = i64::from(20 * u32::from(units_per_em) / 2048);
        self.load_points(outline, near_limit);
        let mut start = 0;
        for &last in &outline.contour_ends {
            self.contours.push(start);
            start = last + 1;
        }
        let contour_count = self.contours.len();
        self.contour_y_minima = vec![0; contour_count];
        self.contour_y_maxima = vec![0; contour_count];
        for contour in 0..contour_count {
            self.compute_directions(self.contours[contour], near_limit);
        }
        self.mark_weak_points();
    }

    /// Scales the points and links each contour (with FreeType's order of
    /// updates, which loses the near flag of a contour's last point).
    fn load_points(&mut self, outline: &FontOutline, near_limit: i64) {
        let (x_scale, y_scale) = (self.x_scale, self.y_scale);
        let unset = Point {
            flags: 0,
            in_dir: Direction::NONE,
            out_dir: Direction::NONE,
            ox: 0,
            oy: 0,
            fx: 0,
            fy: 0,
            x: 0,
            y: 0,
            u: 0,
            v: 0,
            next: 0,
            prev: 0,
        };
        self.points = vec![unset; outline.points.len()];
        let points = &mut self.points;
        let mut contour = 0;
        let mut end = outline.contour_ends[0];
        let mut prev = end;
        for (index, (&(x, y), &kind)) in outline.points.iter().zip(&outline.kinds).enumerate() {
            let (ox, oy) = (mul_fix(x, x_scale), mul_fix(y, y_scale));
            let point = &mut points[index];
            point.in_dir = Direction::NONE;
            point.out_dir = Direction::NONE;
            (point.fx, point.fy) = (x as i16, y as i16);
            (point.ox, point.x, point.oy, point.y) = (ox, ox, oy, oy);
            let (end_x, end_y) = outline.points[end];
            (points[end].fx, points[end].fy) = (end_x as i16, end_y as i16);
            points[index].flags = match kind {
                PointKind::Quadratic => point_flag::CONIC,
                PointKind::Cubic => point_flag::CUBIC,
                PointKind::OnCurve => 0,
            };
            let out_x = i64::from(points[index].fx) - i64::from(points[prev].fx);
            let out_y = i64::from(points[index].fy) - i64::from(points[prev].fy);
            if out_x.abs() + out_y.abs() < near_limit {
                points[prev].flags |= point_flag::NEAR;
            }
            points[index].prev = prev;
            points[prev].next = index;
            prev = index;
            if index == end {
                contour += 1;
                if let Some(&next_end) = outline.contour_ends.get(contour) {
                    end = next_end;
                    prev = end;
                }
            }
        }
    }

    /// The in and out directions of the points of the contour starting at
    /// `first`, accumulating runs of near points (which become weak).
    fn compute_directions(&mut self, first: usize, near_limit: i64) {
        let near_limit2 = 2 * near_limit - 1;
        let points = &mut self.points;
        let delta = |points: &[Point], a: usize, b: usize| {
            (
                i64::from(points[a].fx) - i64::from(points[b].fx),
                i64::from(points[a].fy) - i64::from(points[b].fy),
            )
        };
        // The first point may be part of a run of near points: go back to
        // the first one that is not.
        let mut point = first;
        let mut prev = points[first].prev;
        while prev != first {
            let (out_x, out_y) = delta(points, point, prev);
            if out_x.abs() + out_y.abs() >= near_limit2 {
                break;
            }
            point = prev;
            prev = points[prev].prev;
        }
        let first = point;
        // `u` and `v` hold the index deltas to the next and previous
        // non-near points.
        let mut curr = first;
        points[curr].u = first as i64 - curr as i64;
        points[first].v = -points[curr].u;
        let (mut out_x, mut out_y) = (0i64, 0i64);
        let mut next = first;
        loop {
            let point = next;
            next = points[point].next;
            let (dx, dy) = delta(points, next, point);
            out_x += dx;
            out_y += dy;
            if out_x.abs() + out_y.abs() < near_limit {
                points[next].flags |= point_flag::WEAK_INTERPOLATION;
            } else {
                points[curr].u = next as i64 - curr as i64;
                points[next].v = -points[curr].u;
                let out_dir = Direction::of(out_x, out_y);
                points[curr].out_dir = out_dir;
                curr = points[curr].next;
                while curr != next {
                    points[curr].in_dir = out_dir;
                    points[curr].out_dir = out_dir;
                    curr = points[curr].next;
                }
                points[next].in_dir = out_dir;
                points[curr].u = first as i64 - curr as i64;
                points[first].v = -points[curr].u;
                out_x = 0;
                out_y = 0;
            }
            if next == first {
                break;
            }
        }
    }

    /// Tags the points FreeType interpolates instead of aligning: control
    /// points, points inside a straight run or a flat corner, spikes, and
    /// the middle of vectors pointing into the same quadrant.
    fn mark_weak_points(&mut self) {
        let points = &mut self.points;
        let in_out = |points: &[Point], index: usize| {
            let point = points[index];
            let next_u = points[offset(index, point.u)];
            let prev_v = points[offset(index, point.v)];
            (
                i64::from(point.fx) - i64::from(prev_v.fx),
                i64::from(point.fy) - i64::from(prev_v.fy),
                i64::from(next_u.fx) - i64::from(point.fx),
                i64::from(next_u.fy) - i64::from(point.fy),
            )
        };
        let relink = |points: &mut [Point], index: usize| {
            let (next_u, prev_v) = (
                offset(index, points[index].u),
                offset(index, points[index].v),
            );
            points[prev_v].u = next_u as i64 - prev_v as i64;
            points[next_u].v = -points[prev_v].u;
        };
        for index in 0..points.len() {
            let point = points[index];
            if point.flags & point_flag::WEAK_INTERPOLATION != 0 {
                continue;
            }
            if point.in_dir == Direction::NONE && point.out_dir == Direction::NONE {
                let (in_x, in_y, out_x, out_y) = in_out(points, index);
                if (in_x ^ out_x) >= 0 && (in_y ^ out_y) >= 0 {
                    points[index].flags |= point_flag::WEAK_INTERPOLATION;
                    relink(points, index);
                }
            }
        }
        for index in 0..points.len() {
            let point = points[index];
            if point.flags & point_flag::WEAK_INTERPOLATION != 0 {
                continue;
            }
            let weak = if point.flags & point_flag::CONTROL != 0 {
                true
            } else if point.out_dir == point.in_dir {
                if point.out_dir == Direction::NONE {
                    let (in_x, in_y, out_x, out_y) = in_out(points, index);
                    let flat = corner_is_flat(in_x, in_y, out_x, out_y);
                    if flat {
                        relink(points, index);
                    }
                    flat
                } else {
                    true
                }
            } else {
                point.in_dir.is_opposite(point.out_dir)
            };
            if weak {
                points[index].flags |= point_flag::WEAK_INTERPOLATION;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hypot_and_flat_corners() {
        assert_eq!(hypot(-8, 3), 9);
        assert_eq!(hypot(3, 8), 9);
        assert!(corner_is_flat(100, 0, 100, 1));
        assert!(!corner_is_flat(100, 0, 0, 100));
        assert!(corner_is_flat(1000, 0, 0, 10), "a dominant vector");
    }

    #[test]
    fn orientation_follows_the_signed_area() {
        let square = |points: Vec<(i64, i64)>| FontOutline {
            kinds: vec![PointKind::OnCurve; points.len()],
            contour_ends: vec![points.len() - 1],
            points,
        };
        let clockwise = square(vec![(0, 0), (0, 100), (100, 100), (100, 0)]);
        let counter = square(vec![(0, 0), (100, 0), (100, 100), (0, 100)]);
        assert!(!is_postscript(&clockwise));
        assert!(is_postscript(&counter));
        assert!(!is_postscript(&square(vec![(0, 0), (0, 100)])), "collapsed");
        assert!(!is_postscript(&FontOutline::default()));
    }

    #[test]
    fn directions_of_vectors() {
        assert_eq!(Direction::of(0, 10), Direction::UP);
        assert_eq!(Direction::of(-10, 0), Direction::LEFT);
        assert_eq!(Direction::of(10, 1), Direction::NONE);
        assert_eq!(Direction::of(100, -1), Direction::RIGHT);
        assert_eq!(Direction::of(0, -5), Direction::DOWN);
        assert!(Direction::UP.is_opposite(Direction::DOWN));
        assert_eq!(Direction::LEFT.abs(), Direction::RIGHT);
    }
}
