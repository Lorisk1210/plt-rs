//! Polar to Cartesian coordinates of points and paths (matplotlib's
//! `PolarTransform`): segments at constant radius become arcs, other
//! segments are subdivided, and inside a polygon frame both are cut
//! into straight pieces.

use std::f64::consts::TAU;

use plt_core::{Path, PathCode, Point};

use crate::transforms::PointMap;

/// Polar to Cartesian coordinates around the origin, applied after the
/// radial scale and the angular shift (matplotlib's `PolarTransform`).
#[derive(Debug)]
pub(crate) struct PolarMap {
    /// The scaled radius drawn at the centre.
    pub(super) scaled_origin: f64,
    /// The unscaled radius at the centre, which matplotlib's
    /// `InvertedPolarTransform` adds back without rescaling.
    pub(super) origin: f64,
    /// `1`, or `-1` when the radial axis decreases outwards.
    pub(super) sign: f64,
    /// The vertices of a polygon frame, which paths that follow the
    /// circle follow instead.
    pub(super) polygon: Option<usize>,
}

impl PolarMap {
    /// matplotlib's `PolarTransform()` without an axis: radii are drawn
    /// as they are, from the origin.
    pub(crate) const PLAIN: Self = Self {
        scaled_origin: 0.0,
        origin: 0.0,
        sign: 1.0,
        polygon: None,
    };

    /// The distance from the centre of a scaled radius (negative inside
    /// the origin).
    fn radius(&self, r: f64) -> f64 {
        (r - self.scaled_origin) * self.sign
    }

    /// The mapped vertices of `path`, joined by straight segments.
    fn straight_path(&self, path: &Path) -> Path {
        let mut mapped = path.clone().with_interpolation_steps(1);
        for p in mapped.vertices_mut() {
            *p = self.forward(*p);
        }
        mapped
    }

    /// The mapped `path` drawing constant-radius segments as arcs and
    /// other segments subdivided into `steps` pieces.
    fn curved_path(&self, path: &Path, steps: usize) -> Path {
        let codes = path.resolved_codes();
        let vertices = path.vertices();
        let mut out = CurvedPath::default();
        let mut last: Option<Point> = None;
        let mut index = 0;
        while index < vertices.len() {
            let code = codes[index];
            let Some(segment) = vertices.get(index..index + code.vertex_count()) else {
                break;
            };
            match (code, last) {
                (PathCode::LineTo, Some(from)) => self.push_line(from, segment[0], steps, &mut out),
                _ => {
                    for &p in segment {
                        out.push(self.forward(p), code);
                    }
                }
            }
            last = segment.last().copied();
            index += segment.len();
        }
        Path::new(out.vertices, out.codes).expect("mapped segments keep their codes")
    }

    /// One straight `(θ, r)` segment: a radial line, an arc at constant
    /// radius, or a subdivided spiral.
    #[expect(
        clippy::float_cmp,
        reason = "matplotlib tests for exactly constant angle or radius"
    )]
    fn push_line(&self, from: Point, to: Point, steps: usize, out: &mut CurvedPath) {
        if to.x == from.x {
            out.push(self.forward(to), PathCode::LineTo);
        } else if to.y == from.y {
            out.push_arc(from.x.to_degrees(), to.x.to_degrees(), self.radius(to.y));
        } else {
            let pieces = Path::polyline(vec![from, to]).interpolated(steps);
            for &p in &pieces.vertices()[1..] {
                out.push(self.forward(p), PathCode::LineTo);
            }
        }
    }
}

/// The most whole turns an arc retraces. matplotlib retraces every one
/// (each darkens the antialiased edges of the circle a little more), but
/// loops without end on astronomical spans.
const MAX_TURNS: f64 = 1000.0;

/// Vertices and codes of a path under construction.
#[derive(Default)]
struct CurvedPath {
    vertices: Vec<Point>,
    codes: Vec<PathCode>,
}

impl CurvedPath {
    fn push(&mut self, vertex: Point, code: PathCode) {
        self.vertices.push(vertex);
        self.codes.push(code);
    }

    /// The arc of radius `r` from `from` to `to` degrees, whole turns
    /// first, without its first vertex. Like matplotlib, which retraces
    /// the circle once per whole turn (without end for astronomical
    /// spans), but at most [`MAX_TURNS`] times.
    fn push_arc(&mut self, mut from: f64, to: f64, r: f64) {
        let span = (to - from).abs();
        if span > 360.0 * (MAX_TURNS + 1.0) {
            let kept = 360.0 * MAX_TURNS + span.rem_euclid(360.0);
            from = if from <= to { to - kept } else { to + kept };
        }
        let mut push = |theta1: f64, theta2: f64, reversed: bool| {
            let arc = Path::arc(theta1, theta2, None);
            let mut points = arc.vertices().to_vec();
            if reversed {
                points.reverse();
            }
            for p in points.into_iter().skip(1) {
                self.push(p * r, PathCode::Curve4);
            }
        };
        if from <= to {
            while to - from > 360.0 {
                push(from, from + 360.0, false);
                from += 360.0;
            }
            push(from, to, false);
        } else {
            while from - to > 360.0 {
                push(from - 360.0, from, true);
                from -= 360.0;
            }
            push(to, from, true);
        }
    }
}

impl PointMap for PolarMap {
    /// `(r cos θ, r sin θ)` of the radius from the origin; points inside
    /// the origin become NaN.
    fn forward(&self, p: Point) -> Point {
        let r = self.radius(p.y);
        let r = if r >= 0.0 { r } else { f64::NAN };
        Point::new(r * p.x.cos(), r * p.x.sin())
    }

    fn inverse(&self, p: Point) -> Point {
        let theta = p.y.atan2(p.x).rem_euclid(TAU);
        Point::new(theta, (p.x.hypot(p.y) + self.origin) * self.sign)
    }

    /// matplotlib's `transform_path_non_affine`: paths with more than one
    /// interpolation step follow the circle, or, inside a polygon frame,
    /// are cut into as many straight pieces per segment as the polygon
    /// has vertices (the radar example's `RadarTransform`).
    fn transform_path(&self, path: &Path) -> Path {
        let steps = path.interpolation_steps();
        if path.is_empty() || steps == 1 {
            return self.straight_path(path);
        }
        match self.polygon {
            Some(vertices) => self.straight_path(&path.interpolated(vertices)),
            None => self.curved_path(path, steps),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arcs_retrace_at_most_a_few_whole_turns() {
        let arc = |from: f64, to: f64| {
            let mut out = CurvedPath::default();
            out.push_arc(from, to, 1.0);
            out.vertices.len()
        };
        // Every turn is drawn, as matplotlib draws them (the
        // `mixed_errorbar_polar_caps` case has a 1146° error bar).
        assert_eq!(arc(0.0, 1000.0), 2 * arc(0.0, 360.0) + arc(0.0, 280.0));
        assert_eq!(arc(0.0, 1146.0), 3 * arc(0.0, 360.0) + arc(0.0, 66.0));
        // Found by fuzzing: astronomical spans looped without end.
        for (from, to) in [
            (0.0, 3.6e8),
            (0.0, 1e300),
            (-1e300, 1e300),
            (0.0, f64::INFINITY),
        ] {
            assert!(arc(from, to) < 100_000, "{from} to {to}");
        }
    }
}
