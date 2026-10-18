//! matplotlib's canonical unit shapes (rectangles, circles, arcs, stars, ...).
//!
//! The vertex layouts are identical to matplotlib's, including its
//! eight-segment cubic circle approximation, so markers and patches built on
//! top of them render the same way.

use std::f64::consts::{FRAC_PI_2, TAU};

use super::{Path, PathCode};
use crate::Point;

/// matplotlib's tangent length for a 45° cubic arc on the unit circle.
const MAGIC: f64 = 0.265_203_1;

/// The most cubic pieces of an arc. An arc within one turn needs at most
/// 16; the bound keeps spans that overflow between huge angles
/// (and absurd explicit counts) from exhausting memory.
const MAX_ARC_SEGMENTS: usize = 1 << 16;

impl Path {
    /// The closed unit square from `(0, 0)` to `(1, 1)`.
    pub fn unit_rectangle() -> Self {
        Self::closed(vec![
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(1.0, 1.0),
            Point::new(0.0, 1.0),
            Point::new(0.0, 0.0),
        ])
    }

    /// A regular polygon with `sides` vertices on the unit circle, pointing up.
    pub fn unit_regular_polygon(sides: usize) -> Self {
        let step = TAU / sides as f64;
        Self::closed(
            (0..=sides)
                .map(|i| {
                    let theta = step * i as f64 + FRAC_PI_2;
                    Point::new(theta.cos(), theta.sin())
                })
                .collect(),
        )
    }

    /// A regular star with `points` tips on the unit circle and inner
    /// vertices at radius `inner_radius`, pointing up.
    pub fn unit_regular_star(points: usize, inner_radius: f64) -> Self {
        let n = points * 2;
        let step = TAU / n as f64;
        Self::closed(
            (0..=n)
                .map(|i| {
                    let theta = step * i as f64 + FRAC_PI_2;
                    let r = if i % 2 == 1 { inner_radius } else { 1.0 };
                    Point::new(r * theta.cos(), r * theta.sin())
                })
                .collect(),
        )
    }

    /// A regular asterisk: a star whose inner radius is zero.
    pub fn unit_regular_asterisk(points: usize) -> Self {
        Self::unit_regular_star(points, 0.0)
    }

    /// The unit circle as eight cubic Bézier segments.
    pub fn unit_circle() -> Self {
        Self::circle(Point::ORIGIN, 1.0)
    }

    /// A circle of `radius` around `center`, built like
    /// [`unit_circle`](Self::unit_circle).
    pub fn circle(center: Point, radius: f64) -> Self {
        let mut vertices = circle_quadrant_vertices();
        vertices.push(Point::new(0.0, -1.0));
        let mut codes = vec![PathCode::Curve4; vertices.len()];
        codes[0] = PathCode::MoveTo;
        *codes.last_mut().expect("circle has vertices") = PathCode::ClosePoly;
        let vertices = vertices.into_iter().map(|p| p * radius + center).collect();
        Self::new(vertices, codes).expect("circle codes are well formed")
    }

    /// The right half of the unit circle, closed through its diameter.
    pub fn unit_circle_righthalf() -> Self {
        let mut vertices: Vec<Point> = circle_quadrant_vertices().into_iter().take(13).collect();
        vertices.push(Point::new(0.0, -1.0));
        let mut codes = vec![PathCode::Curve4; vertices.len()];
        codes[0] = PathCode::MoveTo;
        *codes.last_mut().expect("half circle has vertices") = PathCode::ClosePoly;
        Self::new(vertices, codes).expect("half-circle codes are well formed")
    }

    /// The unit-circle arc from `theta1` to `theta2` degrees as cubic
    /// Béziers (matplotlib's `Path.arc`). `theta2` is unwrapped to the
    /// shortest arc within one turn unless the span is a whole number of
    /// turns, which yields a full circle. `segments` defaults to a power of
    /// two keeping each piece within 90°, and is capped at 65 536.
    pub fn arc(theta1: f64, theta2: f64, segments: Option<usize>) -> Self {
        arc_path(theta1, theta2, segments, false)
    }

    /// A pie wedge of the unit circle from `theta1` to `theta2` degrees:
    /// centre, arc, back to centre (matplotlib's `Path.wedge`).
    pub fn wedge(theta1: f64, theta2: f64, segments: Option<usize>) -> Self {
        arc_path(theta1, theta2, segments, true)
    }
}

/// The 25 vertices of matplotlib's circle: a start point plus eight cubic
/// segments travelling counter-clockwise from `(0, -1)`.
fn circle_quadrant_vertices() -> Vec<Point> {
    let sqrt_half = 0.5_f64.sqrt();
    let magic45 = sqrt_half * MAGIC;
    let p = Point::new;
    vec![
        p(0.0, -1.0),
        p(MAGIC, -1.0),
        p(sqrt_half - magic45, -sqrt_half - magic45),
        p(sqrt_half, -sqrt_half),
        p(sqrt_half + magic45, -sqrt_half + magic45),
        p(1.0, -MAGIC),
        p(1.0, 0.0),
        p(1.0, MAGIC),
        p(sqrt_half + magic45, sqrt_half - magic45),
        p(sqrt_half, sqrt_half),
        p(sqrt_half - magic45, sqrt_half + magic45),
        p(MAGIC, 1.0),
        p(0.0, 1.0),
        p(-MAGIC, 1.0),
        p(-sqrt_half + magic45, sqrt_half + magic45),
        p(-sqrt_half, sqrt_half),
        p(-sqrt_half - magic45, sqrt_half - magic45),
        p(-1.0, MAGIC),
        p(-1.0, 0.0),
        p(-1.0, -MAGIC),
        p(-sqrt_half - magic45, -sqrt_half + magic45),
        p(-sqrt_half, -sqrt_half),
        p(-sqrt_half + magic45, -sqrt_half - magic45),
        p(-MAGIC, -1.0),
        p(0.0, -1.0),
    ]
}

fn arc_path(theta1: f64, theta2: f64, segments: Option<usize>, is_wedge: bool) -> Path {
    let turns = (theta2 - theta1) / 360.0;
    let nearest = turns.round_ties_even();
    let full_circle = nearest != 0.0 && (turns - nearest).abs() <= 1e-12;
    let eta2 = if full_circle {
        theta1 + 360.0
    } else {
        theta2 - 360.0 * turns.floor()
    };
    let (eta1, eta2) = (theta1.to_radians(), eta2.to_radians());
    // NaN spans cast to zero pieces, infinite ones saturate: both clamp.
    let n = segments
        .unwrap_or_else(|| 2.0_f64.powf(((eta2 - eta1) / FRAC_PI_2).ceil()) as usize)
        .clamp(1, MAX_ARC_SEGMENTS);

    let deta = (eta2 - eta1) / n as f64;
    let t = (0.5 * deta).tan();
    let alpha = deta.sin() * ((4.0 + 3.0 * t * t).sqrt() - 1.0) / 3.0;
    // Same arithmetic as numpy.linspace(eta1, eta2, n + 1).
    let step = (eta2 - eta1) / n as f64;
    let angle = |i: usize| if i == n { eta2 } else { i as f64 * step + eta1 };

    let mut vertices = Vec::with_capacity(3 * n + 4);
    let mut codes = Vec::with_capacity(3 * n + 4);
    // linspace's first sample `0 * step + eta1`: NaN when the span is.
    let start = Point::new(angle(0).cos(), angle(0).sin());
    if is_wedge {
        vertices.extend([Point::ORIGIN, start]);
        codes.extend([PathCode::MoveTo, PathCode::LineTo]);
    } else {
        vertices.push(start);
        codes.push(PathCode::MoveTo);
    }
    for i in 0..n {
        let (a, b) = (angle(i), angle(i + 1));
        let pa = Point::new(a.cos(), a.sin());
        let pb = Point::new(b.cos(), b.sin());
        vertices.push(pa + Point::new(-pa.y, pa.x) * alpha);
        vertices.push(pb - Point::new(-pb.y, pb.x) * alpha);
        vertices.push(pb);
        codes.extend([PathCode::Curve4; 3]);
    }
    if is_wedge {
        vertices.extend([Point::ORIGIN, Point::ORIGIN]);
        codes.extend([PathCode::LineTo, PathCode::ClosePoly]);
    }
    Path::new(vertices, codes).expect("arc codes are well formed")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Bbox;

    #[test]
    fn rectangle_is_closed_unit_square() {
        let r = Path::unit_rectangle();
        assert_eq!(r.len(), 5);
        assert_eq!(r.code_at(4), PathCode::ClosePoly);
        assert_eq!(r.extents(), Bbox::UNIT);
    }

    #[test]
    fn circle_layout_matches_matplotlib() {
        let c = Path::unit_circle();
        assert_eq!(c.len(), 26);
        assert_eq!(c.code_at(0), PathCode::MoveTo);
        assert_eq!(c.code_at(25), PathCode::ClosePoly);
        assert_eq!(c.vertices()[1], Point::new(MAGIC, -1.0));
        let e = c.extents();
        assert!((e.xmax() - 1.0).abs() < 1e-12 && (e.ymin() + 1.0).abs() < 1e-12);
    }

    #[test]
    fn regular_polygon_points_up() {
        let tri = Path::unit_regular_polygon(3);
        assert_eq!(tri.len(), 4);
        assert!((tri.vertices()[0] - Point::new(0.0, 1.0)).length() < 1e-12);
    }

    #[test]
    fn star_alternates_radii() {
        let star = Path::unit_regular_star(5, 0.5);
        assert_eq!(star.len(), 11);
        assert!((star.vertices()[1].length() - 0.5).abs() < 1e-12);
        assert!((star.vertices()[2].length() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn arc_segment_counts() {
        // matplotlib uses 2^ceil(span / 90°) cubic pieces.
        assert_eq!(Path::arc(0.0, 90.0, None).len(), 7);
        assert_eq!(Path::arc(0.0, 180.0, None).len(), 13);
        assert_eq!(Path::arc(0.0, 360.0, None).len(), 49);
        assert_eq!(
            Path::arc(0.0, 720.0, None).len(),
            49,
            "whole turns draw one circle"
        );
        assert_eq!(Path::wedge(0.0, 90.0, None).len(), 10);
        assert_eq!(Path::arc(0.0, 90.0, Some(1)).len(), 4);
    }

    #[test]
    fn arc_endpoints_lie_on_circle() {
        let arc = Path::arc(30.0, 120.0, None);
        let last = *arc.vertices().last().expect("non-empty");
        assert!(
            (last - Point::new(120f64.to_radians().cos(), 120f64.to_radians().sin())).length()
                < 1e-12
        );
    }

    #[test]
    fn garbled_spans_stay_bounded() {
        // The span overflows to -inf turns and unwraps to an infinite angle,
        // which would ask for 2^inf pieces.
        assert_eq!(
            Path::arc(f64::MAX, -1e300, None).len(),
            3 * MAX_ARC_SEGMENTS + 1
        );
        assert_eq!(Path::arc(0.0, f64::NAN, None).len(), 4);
        assert_eq!(Path::wedge(f64::INFINITY, 0.0, Some(0)).len(), 7);
    }
}
