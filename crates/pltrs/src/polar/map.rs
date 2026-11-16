//! Polar to Cartesian coordinates of points and paths (matplotlib's
//! `PolarTransform`): segments at constant radius become arcs, other
//! segments are subdivided, and inside a polygon frame both are cut
//! into straight pieces.

use std::any::Any;
use std::f64::consts::TAU;
use std::rc::Rc;

use pltrs_core::path::MAX_INTERPOLATED_VERTICES;
use pltrs_core::{Path, PathCode, Point};

use crate::transforms::{PointMap, ResolvedTransform};

/// Polar to Cartesian coordinates around the origin, applied after the
/// radial scale and the angular shift (matplotlib's `PolarTransform`).
#[derive(Clone, Copy, Debug)]
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

pltrs_core::archive_struct!(PolarMap {
    scaled_origin,
    origin,
    sign,
    polygon
});

/// `inverse`, a frozen inverse of a polar Axes' data transform, mapping
/// radii with the radial `origin` and `sign` the Axes have now, as
/// matplotlib's `InvertedPolarTransform`, which its frozen inverses keep
/// live, reads them (`get_rorigin`, `get_rsign`) when it maps a point.
pub(crate) fn with_radial_frame(
    inverse: &ResolvedTransform,
    origin: f64,
    sign: f64,
) -> ResolvedTransform {
    inverse.with_inverted_planes(|map| {
        let map = (map as &dyn Any).downcast_ref::<PolarMap>()?;
        Some(Rc::new(PolarMap {
            origin,
            sign,
            ..*map
        }) as Rc<dyn PointMap>)
    })
}

/// The radial frame of a polar map: the radii at the centre (scaled and
/// as given) and the sign of the radial axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RadialFrame {
    scaled_origin: f64,
    origin: f64,
    sign: f64,
}

pltrs_core::archive_struct!(RadialFrame {
    scaled_origin,
    origin,
    sign
});

/// The radial frame `transform` maps polar coordinates with, if it does.
pub(crate) fn radial_frame(transform: &ResolvedTransform) -> Option<RadialFrame> {
    transform.plane_maps().find_map(|map| {
        (map as &dyn Any)
            .downcast_ref::<PolarMap>()
            .map(|map| RadialFrame {
                scaled_origin: map.scaled_origin,
                origin: map.origin,
                sign: map.sign,
            })
    })
}

/// `transform` mapping polar coordinates with the radial `frame` instead:
/// the frame a line's path was mapped with, as matplotlib's cached path
/// keeps it (see `lines::KeptPolarFrame`).
pub(crate) fn with_mapped_frame(
    transform: &ResolvedTransform,
    frame: RadialFrame,
) -> ResolvedTransform {
    transform.with_planes(|map| {
        let map = (map as &dyn Any).downcast_ref::<PolarMap>()?;
        Some(Rc::new(PolarMap {
            scaled_origin: frame.scaled_origin,
            origin: frame.origin,
            sign: frame.sign,
            ..*map
        }) as Rc<dyn PointMap>)
    })
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

/// Angles (in degrees) past which a turn added to them is lost to
/// rounding: an arc starting or ending there starts at its start's place
/// on the circle instead.
const MAX_DEGREES: f64 = 1e15;

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
    /// spans), but at most [`MAX_TURNS`] times, and from the start's place
    /// on the circle past [`MAX_DEGREES`].
    fn push_arc(&mut self, mut from: f64, mut to: f64, r: f64) {
        if from.abs().max(to.abs()) > MAX_DEGREES {
            let most = 360.0 * (MAX_TURNS + 1.0);
            let span = (to - from).clamp(-most, most);
            from = from.rem_euclid(360.0);
            to = from + span;
        }
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
    /// has vertices (the radar example's `RadarTransform`). Spirals are
    /// cut into fewer pieces where the path's steps would make more than
    /// [`MAX_INTERPOLATED_VERTICES`] vertices, as [`Path::interpolated`]
    /// cuts the segments of a polygon frame.
    fn transform_path(&self, path: &Path) -> Path {
        let steps = path
            .interpolation_steps()
            .min(MAX_INTERPOLATED_VERTICES / path.len().max(1));
        if path.is_empty() || steps <= 1 {
            return self.straight_path(path);
        }
        match self.polygon {
            Some(vertices) => self.straight_path(&path.interpolated(vertices)),
            None => self.curved_path(path, steps),
        }
    }

    /// matplotlib's `transform_path_non_affine` curves a path of more than
    /// one interpolation step through its segments without NaN or
    /// infinities (`iter_segments`), and refuses to make a path of none.
    fn refused_path(&self, path: &Path) -> Option<&'static str> {
        if path.is_empty() || path.interpolation_steps() <= 1 || self.polygon.is_some() {
            return None;
        }
        let codes = path.resolved_codes();
        let vertices = path.vertices();
        let mut index = 0;
        while index < vertices.len() {
            let code = codes[index];
            let segment = &vertices[index..(index + code.vertex_count()).min(vertices.len())];
            if code != PathCode::ClosePoly
                && segment.iter().all(|p| p.x.is_finite() && p.y.is_finite())
            {
                return None;
            }
            index += segment.len().max(1);
        }
        Some("'vertices' must be 2D with shape (N, 2), but your input has shape (0,)")
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
            // A view limit of 2⁶³ radians: adding 360° to the start
            // angle changed nothing, and the turns were retraced forever.
            ((-1.0_f64).to_degrees(), 2.0_f64.powi(63).to_degrees()),
            (5.3e20, 5.3e20 + 1e6),
        ] {
            assert!(arc(from, to) < 100_000, "{from} to {to}");
        }
    }

    #[test]
    fn spirals_stay_within_the_vertex_limit() {
        // The steps a path asks for (`set_interpolation_steps`) cut every
        // spiral segment into that many pieces, past the memory or the
        // capacity of a vector.
        let spiral: Vec<Point> = (0..1000)
            .map(|i| Point::new(f64::from(i) * 0.1, 1.0 + f64::from(i)))
            .collect();
        for steps in [1_000_000, usize::MAX] {
            let path = Path::polyline(spiral.clone()).with_interpolation_steps(steps);
            let mapped = PolarMap::PLAIN.transform_path(&path);
            assert!(mapped.len() <= MAX_INTERPOLATED_VERTICES, "{steps} steps");
        }
        let one = Path::polyline(spiral[..2].to_vec()).with_interpolation_steps(5);
        assert_eq!(PolarMap::PLAIN.transform_path(&one).len(), 6);
    }

    #[test]
    fn curves_of_no_finite_segment_are_refused() {
        let line = |points: [(f64, f64); 2], steps: usize| {
            let points = points.map(|(x, y)| Point::new(x, y)).to_vec();
            Path::polyline(points).with_interpolation_steps(steps)
        };
        let refused = |path: &Path| PolarMap::PLAIN.refused_path(path);
        assert!(refused(&line([(f64::NAN, 1.0), (1.0, f64::INFINITY)], 180)).is_some());
        // One finite point is a segment; straight paths are mapped as they are.
        assert_eq!(refused(&line([(f64::NAN, 1.0), (1.0, 2.0)], 180)), None);
        assert_eq!(refused(&line([(f64::NAN, 1.0), (f64::NAN, 2.0)], 1)), None);
        assert_eq!(
            refused(&Path::polyline(Vec::new()).with_interpolation_steps(180)),
            None
        );
    }
}
