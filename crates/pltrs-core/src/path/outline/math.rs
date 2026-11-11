//! The helpers of Agg's `agg_math.h` that its stroke and contour
//! generators (and Gouraud shading) build on, with the same floating-point
//! expressions in the same order as the C++ code, and its products fused
//! where matplotlib's build of it fuses them (`numeric::fused`).

use crate::Point;
use crate::numeric::{difference_of_products, fused, sum_of_products};

/// Below this magnitude the denominator of [`calc_intersection`] counts as
/// zero, and three points as collinear (Agg's `intersection_epsilon`).
pub const INTERSECTION_EPSILON: f64 = 1.0e-30;

/// Which side of the line `a → b` the point `p` lies on (Agg's
/// `cross_product`): positive on one side, negative on the other, zero on
/// the line.
pub fn cross_product(a: Point, b: Point, p: Point) -> f64 {
    difference_of_products(p.x - b.x, b.y - a.y, p.y - b.y, b.x - a.x)
}

/// The intersection of the lines `a → b` and `c → d`, or `None` when they
/// are parallel (Agg's `calc_intersection`).
pub fn calc_intersection(a: Point, b: Point, c: Point, d: Point) -> Option<Point> {
    let num = difference_of_products(a.y - c.y, d.x - c.x, a.x - c.x, d.y - c.y);
    let den = difference_of_products(b.x - a.x, d.y - c.y, b.y - a.y, d.x - c.x);
    if den.abs() < INTERSECTION_EPSILON {
        return None;
    }
    let r = num / den;
    Some(Point::new(
        fused(r, b.x - a.x, a.x),
        fused(r, b.y - a.y, a.y),
    ))
}

/// The distance between `a` and `b` (Agg's `calc_distance`).
pub fn calc_distance(a: Point, b: Point) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    sum_of_products(dx, dx, dy, dy).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intersection() {
        let p = Point::new;
        assert_eq!(
            calc_intersection(p(0.0, 0.0), p(2.0, 2.0), p(0.0, 2.0), p(2.0, 0.0)),
            Some(p(1.0, 1.0))
        );
        assert_eq!(
            calc_intersection(p(0.0, 0.0), p(1.0, 0.0), p(0.0, 1.0), p(1.0, 1.0)),
            None
        );
    }

    #[test]
    fn sides_and_distances() {
        let p = Point::new;
        assert!(cross_product(p(0.0, 0.0), p(1.0, 0.0), p(0.5, 1.0)) < 0.0);
        assert!(cross_product(p(0.0, 0.0), p(1.0, 0.0), p(0.5, -1.0)) > 0.0);
        assert_eq!(cross_product(p(0.0, 0.0), p(1.0, 0.0), p(3.0, 0.0)), 0.0);
        assert_eq!(calc_distance(p(1.0, 1.0), p(4.0, 5.0)), 5.0);
    }
}
