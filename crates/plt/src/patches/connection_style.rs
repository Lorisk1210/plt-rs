//! Paths connecting two points (matplotlib's `ConnectionStyle`), clipped
//! to the patches at their ends and shrunk away from them.

use std::str::FromStr;

use plt_core::bezier::{Direction, get_intersection, inside_circle, split_path_inout};
use plt_core::{Path, PathCode, Point};

use super::style_spec::StyleSpec;
use super::{PatchError, StyleError};

/// The bisection tolerance of matplotlib's path splitting.
const TOLERANCE: f64 = 0.01;

/// How two points are connected. Angles are in degrees; lengths
/// (`rad`, arms) in display pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConnectionStyle {
    /// A quadratic Bézier curve whose control point lies `rad` times the
    /// distance between the points off their midpoint (`arc3`).
    Arc3 {
        /// Curvature.
        rad: f64,
    },
    /// A quadratic Bézier curve whose control point is where the lines
    /// leaving the points at `angle_a` and `angle_b` meet (`angle3`).
    Angle3 {
        /// Direction at the start.
        angle_a: f64,
        /// Direction at the end.
        angle_b: f64,
    },
    /// Two straight lines meeting where the lines leaving the points at
    /// `angle_a` and `angle_b` cross, the corner rounded by `rad`
    /// (`angle`).
    Angle {
        /// Direction at the start.
        angle_a: f64,
        /// Direction at the end.
        angle_b: f64,
        /// Corner radius.
        rad: f64,
    },
    /// Arms of length `arm_a` and `arm_b` leaving the points at the given
    /// angles, joined by a line, corners rounded by `rad` (`arc`).
    Arc {
        /// Direction of the first arm.
        angle_a: f64,
        /// Direction of the second arm.
        angle_b: f64,
        /// Length of the first arm (none when unset or zero).
        arm_a: Option<f64>,
        /// Length of the second arm (none when unset or zero).
        arm_b: Option<f64>,
        /// Corner radius.
        rad: f64,
    },
    /// A bracket: perpendicular arms of `arm + fraction × distance` joined
    /// by a line parallel to the points (or at `angle`) (`bar`).
    Bar {
        /// Minimum length of the first arm.
        arm_a: f64,
        /// Minimum length of the second arm.
        arm_b: f64,
        /// Fraction of the distance added to the arms.
        fraction: f64,
        /// Angle of the joining line; `None` is parallel to the points.
        angle: Option<f64>,
    },
}

impl Default for ConnectionStyle {
    /// A straight line (`arc3` with zero curvature).
    fn default() -> Self {
        Self::Arc3 { rad: 0.0 }
    }
}

/// A closed region in display pixels an arrow is clipped to (matplotlib
/// passes the patch itself as `patchA`/`patchB`).
#[derive(Clone, Debug, PartialEq)]
pub struct ClipRegion {
    /// The outline in display pixels.
    pub path: Path,
    /// Tolerance around the outline (matplotlib uses the edge width, in
    /// pixels, when the edge is visible).
    pub radius: f64,
}

impl ClipRegion {
    /// Whether `point` lies in any sub-path of the region
    /// (`Patch.contains`).
    pub fn contains(&self, point: Point) -> bool {
        self.path
            .subpaths()
            .iter()
            .any(|sub| sub.contains_point(point, None, self.radius))
    }
}

fn direction(degrees: f64) -> Direction {
    let (sin, cos) = degrees.to_radians().sin_cos();
    Direction { cos, sin }
}

fn coded(segments: &[(PathCode, Point)]) -> Path {
    let (codes, vertices) = segments.iter().copied().unzip();
    Path::new(vertices, codes).expect("connections start with a move")
}

/// Cuts the start of `path` inside `in_start` and its end inside `in_stop`,
/// keeping the path when it does not cross them (`_Base._clip`).
fn clip(
    path: Path,
    in_start: Option<&dyn Fn(Point) -> bool>,
    in_stop: Option<&dyn Fn(Point) -> bool>,
) -> Path {
    let path = match in_start {
        Some(inside) => {
            split_path_inout(&path, inside, TOLERANCE, false).map_or(path, |(_, out)| out)
        }
        None => path,
    };
    match in_stop {
        Some(inside) => {
            split_path_inout(&path, inside, TOLERANCE, false).map_or(path, |(inn, _)| inn)
        }
        None => path,
    }
}

impl ConnectionStyle {
    /// The raw connection from `a` to `b` (each style's `connect`).
    pub fn connect(&self, a: Point, b: Point) -> Result<Path, PatchError> {
        use PathCode::{Curve3, LineTo, MoveTo};
        Ok(match *self {
            Self::Arc3 { rad } => {
                let mid = Point::new(f64::midpoint(a.x, b.x), f64::midpoint(a.y, b.y));
                let (dx, dy) = (b.x - a.x, b.y - a.y);
                let control = Point::new(mid.x + rad * dy, mid.y - rad * dx);
                coded(&[(MoveTo, a), (Curve3, control), (Curve3, b)])
            }
            Self::Angle3 { angle_a, angle_b } => {
                let corner = get_intersection(a, direction(angle_a), b, direction(angle_b))
                    .map_err(|_| PatchError::ParallelAngles)?;
                coded(&[(MoveTo, a), (Curve3, corner), (Curve3, b)])
            }
            Self::Angle {
                angle_a,
                angle_b,
                rad,
            } => {
                let corner = get_intersection(a, direction(angle_a), b, direction(angle_b))
                    .map_err(|_| PatchError::ParallelAngles)?;
                let mut segments = vec![(MoveTo, a)];
                if rad == 0.0 {
                    segments.push((LineTo, corner));
                } else {
                    let toward = |p: Point| {
                        let (dx, dy) = (p.x - corner.x, p.y - corner.y);
                        let f = rad / dx.hypot(dy);
                        Point::new(corner.x + dx * f, corner.y + dy * f)
                    };
                    segments.extend([(LineTo, toward(a)), (Curve3, corner), (Curve3, toward(b))]);
                }
                segments.push((LineTo, b));
                coded(&segments)
            }
            Self::Arc {
                angle_a,
                angle_b,
                arm_a,
                arm_b,
                rad,
            } => arc_connection(a, b, (angle_a, arm_a), (angle_b, arm_b), rad),
            Self::Bar {
                arm_a,
                arm_b,
                fraction,
                angle,
            } => bar_connection(a, b, arm_a, arm_b, fraction, angle),
        })
    }

    /// The connection from `a` to `b` clipped to `patch_a` and `patch_b`,
    /// then shrunk by `shrink_a` and `shrink_b` pixels at its ends
    /// (`_Base.__call__`).
    pub fn route(
        &self,
        (a, b): (Point, Point),
        (shrink_a, shrink_b): (f64, f64),
        patch_a: Option<&ClipRegion>,
        patch_b: Option<&ClipRegion>,
    ) -> Result<Path, PatchError> {
        let path = self.connect(a, b)?;
        let in_a = patch_a.map(|region| move |p: Point| region.contains(p));
        let in_b = patch_b.map(|region| move |p: Point| region.contains(p));
        let path = clip(
            path,
            in_a.as_ref().map(|f| f as &dyn Fn(Point) -> bool),
            in_b.as_ref().map(|f| f as &dyn Fn(Point) -> bool),
        );
        let vertices = path.vertices();
        let (Some(&first), Some(&last)) = (vertices.first(), vertices.last()) else {
            return Ok(path);
        };
        let around_a = (shrink_a != 0.0).then(|| inside_circle(first, shrink_a));
        let around_b = (shrink_b != 0.0).then(|| inside_circle(last, shrink_b));
        Ok(clip(
            path,
            around_a.as_ref().map(|f| f as &dyn Fn(Point) -> bool),
            around_b.as_ref().map(|f| f as &dyn Fn(Point) -> bool),
        ))
    }
}

/// `ConnectionStyle.Arc.connect`.
fn arc_connection(
    a: Point,
    b: Point,
    (angle_a, arm_a): (f64, Option<f64>),
    (angle_b, arm_b): (f64, Option<f64>),
    rad: f64,
) -> Path {
    use PathCode::{Curve3, LineTo, MoveTo};
    let arm = |arm: Option<f64>| arm.filter(|&length| length != 0.0);
    let mut segments = vec![(MoveTo, a)];
    let mut rounded: Vec<Point> = Vec::new();
    let toward = |from: Point, to: Point, length: f64| {
        let (dx, dy) = (to.x - from.x, to.y - from.y);
        let dd = (dx * dx + dy * dy).sqrt();
        Point::new(from.x + length * dx / dd, from.y + length * dy / dd)
    };
    let distance = |from: Point, to: Point| {
        let (dx, dy) = (to.x - from.x, to.y - from.y);
        (dx * dx + dy * dy).sqrt()
    };
    if let Some(arm_a) = arm(arm_a) {
        let d = direction(angle_a);
        rounded.push(Point::new(
            a.x + (arm_a - rad) * d.cos,
            a.y + (arm_a - rad) * d.sin,
        ));
        rounded.push(Point::new(a.x + arm_a * d.cos, a.y + arm_a * d.sin));
    }
    if let Some(arm_b) = arm(arm_b) {
        let d = direction(angle_b);
        let arm_end = Point::new(b.x + arm_b * d.cos, b.y + arm_b * d.sin);
        let from = match rounded.last() {
            Some(&last) => {
                rounded.push(toward(last, arm_end, rad));
                segments.push((LineTo, rounded[0]));
                segments.push((Curve3, rounded[1]));
                segments.push((Curve3, rounded[2]));
                last
            }
            None => a,
        };
        let length = distance(from, arm_end) - rad;
        rounded = vec![toward(from, arm_end, length), arm_end];
    }
    if let Some(&last) = rounded.last() {
        rounded.push(toward(last, b, rad));
        segments.push((LineTo, rounded[0]));
        segments.push((Curve3, rounded[1]));
        segments.push((Curve3, rounded[2]));
    }
    segments.push((LineTo, b));
    coded(&segments)
}

/// `ConnectionStyle.Bar.connect`.
fn bar_connection(
    a: Point,
    b0: Point,
    arm_a: f64,
    mut arm_b: f64,
    fraction: f64,
    angle: Option<f64>,
) -> Path {
    use PathCode::{LineTo, MoveTo};
    let theta1 = (b0.y - a.y).atan2(b0.x - a.x);
    let (dx, dy) = (b0.x - a.x, b0.y - a.y);
    let dd = (dx * dx + dy * dy).sqrt();
    let (mut ddx, mut ddy) = (dx / dd, dy / dd);
    let mut b = b0;
    if let Some(angle) = angle {
        let theta0 = angle.to_radians();
        let dtheta = theta1 - theta0;
        let dl = dd * dtheta.sin();
        let d_long = dd * dtheta.cos();
        b = Point::new(a.x + d_long * theta0.cos(), a.y + d_long * theta0.sin());
        arm_b -= dl;
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let dd2 = (dx * dx + dy * dy).sqrt();
        (ddx, ddy) = (dx / dd2, dy / dd2);
    }
    let f = fraction * dd + arm_a.max(arm_b);
    coded(&[
        (MoveTo, a),
        (LineTo, Point::new(a.x + f * ddy, a.y - f * ddx)),
        (LineTo, Point::new(b.x + f * ddy, b.y - f * ddx)),
        (LineTo, b0),
    ])
}

impl FromStr for ConnectionStyle {
    type Err = StyleError;

    /// Parses matplotlib's description, e.g. `"arc3,rad=0.3"` or
    /// `"angle,angleA=-90,angleB=180,rad=5"`.
    fn from_str(text: &str) -> Result<Self, StyleError> {
        let mut spec = StyleSpec::parse(text)?;
        let name = spec.name.clone();
        let mut value = |key: &str, default: f64| spec.take(key).unwrap_or(default);
        let style = match name.as_str() {
            "arc3" => Self::Arc3 {
                rad: value("rad", 0.0),
            },
            "angle3" => Self::Angle3 {
                angle_a: value("angleA", 90.0),
                angle_b: value("angleB", 0.0),
            },
            "angle" => Self::Angle {
                angle_a: value("angleA", 90.0),
                angle_b: value("angleB", 0.0),
                rad: value("rad", 0.0),
            },
            "arc" => Self::Arc {
                angle_a: value("angleA", 0.0),
                angle_b: value("angleB", 0.0),
                arm_a: spec.take("armA"),
                arm_b: spec.take("armB"),
                rad: spec.take("rad").unwrap_or(0.0),
            },
            "bar" => Self::Bar {
                arm_a: value("armA", 0.0),
                arm_b: value("armB", 0.0),
                fraction: value("fraction", 0.3),
                angle: spec.take("angle"),
            },
            _ => return Err(spec.unknown()),
        };
        spec.finish()?;
        Ok(style)
    }
}

#[cfg(test)]
mod tests;
