//! Planar helpers of `matplotlib.bezier` for offsetting quadratic curves:
//! the building blocks of arrow bodies and heads.

use super::segment::BezierError;
use crate::Point;

/// A unit direction `(cos θ, sin θ)`, or `(0, 0)` for a degenerate one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Direction {
    /// `cos θ`.
    pub cos: f64,
    /// `sin θ`.
    pub sin: f64,
}

/// How two directions relate (matplotlib's `check_if_parallel` result).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parallel {
    /// Same direction (matplotlib's `1`).
    Same,
    /// Opposite directions (matplotlib's `-1`).
    Opposite,
}

/// The direction from `p0` to `p1` (matplotlib's `get_cos_sin`).
pub fn get_cos_sin(p0: Point, p1: Point) -> Direction {
    let dx = p1.x - p0.x;
    let dy = p1.y - p0.y;
    let d = (dx * dx + dy * dy).sqrt();
    if d == 0.0 {
        return Direction { cos: 0.0, sin: 0.0 };
    }
    Direction {
        cos: dx / d,
        sin: dy / d,
    }
}

/// Whether the vectors `d1` and `d2` are parallel within `tolerance`
/// radians (matplotlib's `check_if_parallel`; it compares `atan2(dx, dy)`).
pub fn check_if_parallel(d1: Point, d2: Point, tolerance: f64) -> Option<Parallel> {
    let theta1 = d1.x.atan2(d1.y);
    let theta2 = d2.x.atan2(d2.y);
    let dtheta = (theta1 - theta2).abs();
    if dtheta < tolerance {
        Some(Parallel::Same)
    } else if (dtheta - std::f64::consts::PI).abs() < tolerance {
        Some(Parallel::Opposite)
    } else {
        None
    }
}

/// The intersection of the line through `c1` along `t1` with the line
/// through `c2` along `t2` (matplotlib's `get_intersection`).
pub fn get_intersection(
    c1: Point,
    t1: Direction,
    c2: Point,
    t2: Direction,
) -> Result<Point, BezierError> {
    let line1_rhs = t1.sin * c1.x - t1.cos * c1.y;
    let line2_rhs = t2.sin * c2.x - t2.cos * c2.y;
    let (a, b) = (t1.sin, -t1.cos);
    let (c, d) = (t2.sin, -t2.cos);
    let ad_bc = a * d - b * c;
    if ad_bc.abs() < 1e-12 {
        return Err(BezierError::ParallelLines);
    }
    let (a_, b_, c_, d_) = (d / ad_bc, -b / ad_bc, -c / ad_bc, a / ad_bc);
    Ok(Point::new(
        a_ * line1_rhs + b_ * line2_rhs,
        c_ * line1_rhs + d_ * line2_rhs,
    ))
}

/// The two points at distance `length` from `c` on the perpendicular to
/// `t`: first the one rotated clockwise from `t`, then the counter-clockwise
/// one (matplotlib's `get_normal_points`; its "left" side).
pub fn get_normal_points(c: Point, t: Direction, length: f64) -> (Point, Point) {
    if length == 0.0 {
        return (c, c);
    }
    let (cos1, sin1) = (t.sin, -t.cos);
    let (cos2, sin2) = (-t.sin, t.cos);
    (
        Point::new(length * cos1 + c.x, length * sin1 + c.y),
        Point::new(length * cos2 + c.x, length * sin2 + c.y),
    )
}

/// The quadratic control points through `c1` and `c2` whose curve passes
/// through `mid` at `t = 0.5` (matplotlib's `find_control_points`).
pub fn find_control_points(c1: Point, mid: Point, c2: Point) -> [Point; 3] {
    let cm = Point::new(
        0.5 * (4.0 * mid.x - (c1.x + c2.x)),
        0.5 * (4.0 * mid.y - (c1.y + c2.y)),
    );
    [c1, cm, c2]
}

/// Two quadratic curves roughly parallel to `bezier2`, offset by `width`
/// on either side (matplotlib's `get_parallels`). When the control
/// polygon folds back on itself, the chord direction is used for both
/// ends (matplotlib also warns then); when the offset tangents do not
/// intersect, the middle control point is the midpoint of the ends.
pub fn get_parallels(bezier2: [Point; 3], width: f64) -> ([Point; 3], [Point; 3]) {
    let [c1, cm, c2] = bezier2;
    let (t1, t2) = if check_if_parallel(c1 - cm, cm - c2, 1e-5) == Some(Parallel::Opposite) {
        let t = get_cos_sin(c1, c2);
        (t, t)
    } else {
        (get_cos_sin(c1, cm), get_cos_sin(cm, c2))
    };
    let (c1_left, c1_right) = get_normal_points(c1, t1, width);
    let (c2_left, c2_right) = get_normal_points(c2, t2, width);
    let middle =
        |a: Point, b: Point| get_intersection(a, t1, b, t2).unwrap_or_else(|_| a.midpoint(b));
    (
        [c1_left, middle(c1_left, c2_left), c2_left],
        [c1_right, middle(c1_right, c2_right), c2_right],
    )
}

/// Two quadratic curves on either side of `bezier2` whose distance from it
/// tapers from `width * w1` at the start through `width * wm` in the middle
/// to `width * w2` at the end (matplotlib's `make_wedged_bezier2`).
pub fn make_wedged_bezier2(
    bezier2: [Point; 3],
    width: f64,
    w1: f64,
    wm: f64,
    w2: f64,
) -> ([Point; 3], [Point; 3]) {
    let [c1, cm, c3] = bezier2;
    let t1 = get_cos_sin(c1, cm);
    let t2 = get_cos_sin(cm, c3);
    let (c1_left, c1_right) = get_normal_points(c1, t1, width * w1);
    let (c3_left, c3_right) = get_normal_points(c3, t2, width * w2);
    let c12 = c1.midpoint(cm);
    let c23 = cm.midpoint(c3);
    let c123 = c12.midpoint(c23);
    let t123 = get_cos_sin(c12, c23);
    let (c123_left, c123_right) = get_normal_points(c123, t123, width * wm);
    (
        find_control_points(c1_left, c123_left, c3_left),
        find_control_points(c1_right, c123_right, c3_right),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn directions_and_normals() {
        assert_eq!(
            get_cos_sin(p(0.0, 0.0), p(3.0, 4.0)),
            Direction { cos: 0.6, sin: 0.8 }
        );
        assert_eq!(
            get_cos_sin(p(1.0, 1.0), p(1.0, 1.0)),
            Direction { cos: 0.0, sin: 0.0 }
        );
        let east = Direction { cos: 1.0, sin: 0.0 };
        assert_eq!(
            get_normal_points(p(0.0, 0.0), east, 1.0),
            (p(0.0, -1.0), p(0.0, 1.0))
        );
        assert_eq!(
            check_if_parallel(p(1.0, 0.0), p(2.0, 0.0), 1e-5),
            Some(Parallel::Same)
        );
        assert_eq!(
            check_if_parallel(p(1.0, 0.0), p(-2.0, 0.0), 1e-5),
            Some(Parallel::Opposite)
        );
        assert_eq!(check_if_parallel(p(1.0, 0.0), p(0.0, 1.0), 1e-5), None);
    }

    #[test]
    fn intersections() {
        let east = Direction { cos: 1.0, sin: 0.0 };
        let north = Direction { cos: 0.0, sin: 1.0 };
        assert_eq!(
            get_intersection(p(0.0, 0.0), east, p(1.0, -1.0), north),
            Ok(p(1.0, 0.0))
        );
        assert_eq!(
            get_intersection(p(0.0, 0.0), east, p(0.0, 1.0), east),
            Err(BezierError::ParallelLines)
        );
        assert_eq!(
            find_control_points(p(0.0, 0.0), p(1.0, 1.0), p(2.0, 0.0))[1],
            p(1.0, 2.0)
        );
    }

    #[test]
    fn parallels_of_a_straight_curve() {
        let (left, right) = get_parallels([p(0.0, 0.0), p(1.0, 0.0), p(2.0, 0.0)], 0.5);
        assert_eq!(left, [p(0.0, -0.5), p(1.0, -0.5), p(2.0, -0.5)]);
        assert_eq!(right, [p(0.0, 0.5), p(1.0, 0.5), p(2.0, 0.5)]);
        let (left, right) =
            make_wedged_bezier2([p(0.0, 0.0), p(1.0, 0.0), p(2.0, 0.0)], 0.5, 1.0, 0.5, 0.0);
        assert_eq!(left[0], p(0.0, -0.5));
        assert_eq!(right[2], p(2.0, 0.0));
    }
}
