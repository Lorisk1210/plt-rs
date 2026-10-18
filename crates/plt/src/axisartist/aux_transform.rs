//! The transform of a curvilinear grid: from curved coordinates (an angle
//! and a radius, say) to the rectilinear data coordinates of the axes
//! (matplotlib's `aux_trans`: a `Transform` or a pair of functions).

use std::f64::consts::TAU;
use std::fmt;
use std::rc::Rc;

use plt_core::{Affine2D, Path, Point};

use crate::polar::PolarMap;
use crate::ticker::pymath::py_mod;
use crate::transforms::{PointMap, ResolvedTransform, Step};

/// A transform from curvilinear to rectilinear coordinates, with its
/// inverse (matplotlib's `aux_trans`).
///
/// It is built from affine maps, the polar projection and pairs of
/// functions, chained with [`AuxTransform::then`] like matplotlib's
/// `a + b`.
///
/// ```
/// use plt::axisartist::AuxTransform;
/// use plt::transforms::Affine2D;
/// use plt_core::Point;
/// // Degrees and radius onto the plane.
/// let polar = AuxTransform::affine(Affine2D::scaling(std::f64::consts::PI / 180.0, 1.0))
///     .then(AuxTransform::polar());
/// let p = polar.transform_point(Point::new(90.0, 2.0));
/// assert!(p.x.abs() < 1e-12 && (p.y - 2.0).abs() < 1e-12);
/// let back = polar.inverted().expect("invertible").transform_point(p);
/// assert!((back.x - 90.0).abs() < 1e-9 && (back.y - 2.0).abs() < 1e-12);
/// ```
#[derive(Clone, Debug)]
pub struct AuxTransform {
    /// The stages applied in order; consecutive affine maps are fused
    /// into one, as matplotlib composes them.
    stages: Vec<Stage>,
}

/// One stage of an [`AuxTransform`].
#[derive(Clone, Debug)]
enum Stage {
    Affine(Affine2D),
    Map(Rc<dyn PointMap>),
}

impl Stage {
    fn apply(&self, point: Point) -> Point {
        match self {
            Self::Affine(affine) => affine.transform_point(point),
            Self::Map(map) => map.forward(point),
        }
    }

    fn inverted(&self) -> Option<Self> {
        Some(match self {
            Self::Affine(affine) => Self::Affine(affine.inverted()?),
            Self::Map(map) => Self::Map(Rc::new(Inverse(Rc::clone(map)))),
        })
    }
}

impl AuxTransform {
    /// The identity.
    pub fn identity() -> Self {
        Self { stages: Vec::new() }
    }

    /// An affine map (matplotlib's `Affine2D` as `aux_trans`).
    pub fn affine(affine: Affine2D) -> Self {
        Self {
            stages: vec![Stage::Affine(affine)],
        }
    }

    /// The polar projection `(θ, r) ↦ (r cos θ, r sin θ)` with θ in
    /// radians; negative radii become NaN, and paths with interpolation
    /// steps follow circles at constant radius (matplotlib's
    /// `PolarAxes.PolarTransform()`).
    pub fn polar() -> Self {
        Self::map(Rc::new(Polar))
    }

    /// The transform given by `forward` and its inverse `inverse`, each
    /// taking and returning `(x, y)` (matplotlib's `(trans, inv_trans)`
    /// pair).
    pub fn from_fns(
        forward: impl Fn(f64, f64) -> (f64, f64) + 'static,
        inverse: impl Fn(f64, f64) -> (f64, f64) + 'static,
    ) -> Self {
        Self::map(Rc::new(Functions {
            forward: Rc::new(forward),
            inverse: Rc::new(inverse),
        }))
    }

    fn map(map: Rc<dyn PointMap>) -> Self {
        Self {
            stages: vec![Stage::Map(map)],
        }
    }

    /// Applies `self`, then `next` (matplotlib's `self + next`).
    #[must_use]
    pub fn then(mut self, next: Self) -> Self {
        for stage in next.stages {
            match (self.stages.last_mut(), stage) {
                (Some(Stage::Affine(first)), Stage::Affine(second)) => *first = first.then(second),
                (_, stage) => self.stages.push(stage),
            }
        }
        self
    }

    /// The inverse transform, unless an affine part is singular.
    pub fn inverted(&self) -> Option<Self> {
        let stages = self
            .stages
            .iter()
            .rev()
            .map(Stage::inverted)
            .collect::<Option<_>>()?;
        Some(Self { stages })
    }

    /// Maps one point.
    pub fn transform_point(&self, point: Point) -> Point {
        self.stages
            .iter()
            .fold(point, |point, stage| stage.apply(point))
    }

    /// Maps a path; the polar projection turns its steps at constant
    /// radius into arcs when the path has interpolation steps
    /// (matplotlib's `transform_path`).
    pub fn transform_path(&self, path: &Path) -> Path {
        self.resolved().transform_path(path)
    }

    /// The transform as a chain of drawing steps.
    pub(crate) fn resolved(&self) -> ResolvedTransform {
        self.stages
            .iter()
            .fold(ResolvedTransform::identity(), |resolved, stage| {
                resolved.then(match stage {
                    Stage::Affine(affine) => ResolvedTransform::affine(*affine),
                    Stage::Map(map) => ResolvedTransform::with_step(
                        Step::Plane(Rc::clone(map)),
                        Affine2D::IDENTITY,
                    ),
                })
            })
    }
}

/// The inverse of a map of the plane.
#[derive(Debug)]
struct Inverse(Rc<dyn PointMap>);

impl PointMap for Inverse {
    fn forward(&self, point: Point) -> Point {
        self.0.inverse(point)
    }

    fn inverse(&self, point: Point) -> Point {
        self.0.forward(point)
    }
}

/// matplotlib's `PolarTransform()` and its inverse without an axis.
#[derive(Debug)]
struct Polar;

impl PointMap for Polar {
    fn forward(&self, point: Point) -> Point {
        PolarMap::PLAIN.forward(point)
    }

    /// matplotlib's `InvertedPolarTransform`: θ in `[0, 2π)` computed as
    /// `(atan2(y, x) + 2π) % 2π`.
    fn inverse(&self, point: Point) -> Point {
        let theta = py_mod(point.y.atan2(point.x) + TAU, TAU);
        Point::new(theta, point.x.hypot(point.y))
    }

    fn transform_path(&self, path: &Path) -> Path {
        PolarMap::PLAIN.transform_path(path)
    }
}

/// A user-supplied function of `(x, y)`.
type PointFn = Rc<dyn Fn(f64, f64) -> (f64, f64)>;

/// A transform given by two functions (matplotlib's `_User2DTransform`).
struct Functions {
    forward: PointFn,
    inverse: PointFn,
}

impl fmt::Debug for Functions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Functions")
    }
}

impl PointMap for Functions {
    fn forward(&self, point: Point) -> Point {
        let (x, y) = (self.forward)(point.x, point.y);
        Point::new(x, y)
    }

    fn inverse(&self, point: Point) -> Point {
        let (x, y) = (self.inverse)(point.x, point.y);
        Point::new(x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plt_core::PathCode;

    #[test]
    fn functions_invert_by_swapping() {
        let shear = AuxTransform::from_fns(|x, y| (x, y - x), |x, y| (x, y + x));
        assert_eq!(
            shear.transform_point(Point::new(3.0, 5.0)),
            Point::new(3.0, 2.0)
        );
        let inverse = shear.inverted().unwrap();
        assert_eq!(
            inverse.transform_point(Point::new(3.0, 2.0)),
            Point::new(3.0, 5.0)
        );
    }

    #[test]
    fn polar_maps_negative_radii_to_nan_and_angles_into_one_turn() {
        let polar = AuxTransform::polar();
        assert!(polar.transform_point(Point::new(0.5, -1.0)).x.is_nan());
        let inverse = polar.inverted().unwrap();
        let below = inverse.transform_point(Point::new(0.0, -1.0));
        assert!((below.x - 1.5 * std::f64::consts::PI).abs() < 1e-12);
        assert_eq!(below.y, 1.0);
    }

    #[test]
    fn polar_paths_follow_circles() {
        let quarter = Path::polyline(vec![Point::new(0.0, 1.0), Point::new(1.0, 1.0)])
            .with_interpolation_steps(100);
        let arc = AuxTransform::polar().transform_path(&quarter);
        assert!(arc.codes().unwrap().contains(&PathCode::Curve4));
        let singular = AuxTransform::affine(Affine2D::scaling(0.0, 1.0));
        assert!(singular.inverted().is_none());
    }
}
