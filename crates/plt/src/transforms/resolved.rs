//! Concrete transforms: non-affine steps followed by one affine map.

use std::borrow::Cow;
use std::fmt;
use std::rc::Rc;

use plt_core::{Affine2D, Path, Point};

/// An invertible map of one coordinate (e.g. a log scale).
pub trait AxisMap: fmt::Debug {
    /// Forward map.
    fn forward(&self, value: f64) -> f64;
    /// Inverse map.
    fn inverse(&self, value: f64) -> f64;
}

/// An invertible map of the plane (e.g. a polar projection).
pub trait PointMap: fmt::Debug {
    /// Forward map.
    fn forward(&self, point: Point) -> Point;
    /// Inverse map.
    fn inverse(&self, point: Point) -> Point;
    /// Whether straight segments must be subdivided before mapping because
    /// the map bends lines (polar and geographic projections).
    fn curves_lines(&self) -> bool {
        true
    }
}

/// One stage of a [`ResolvedTransform`] applied before its trailing affine.
#[derive(Clone, Debug)]
pub enum Step {
    /// An affine map.
    Affine(Affine2D),
    /// Independent maps of x and y (`None` = identity).
    Separable(Option<Rc<dyn AxisMap>>, Option<Rc<dyn AxisMap>>),
    /// A general map of the plane.
    Plane(Rc<dyn PointMap>),
    /// x taken from one transform and y from another.
    Blend(Rc<ResolvedTransform>, Rc<ResolvedTransform>),
}

impl Step {
    fn apply(&self, p: Point) -> Point {
        match self {
            Self::Affine(affine) => affine.transform_point(p),
            Self::Separable(x, y) => Point::new(
                x.as_ref().map_or(p.x, |m| m.forward(p.x)),
                y.as_ref().map_or(p.y, |m| m.forward(p.y)),
            ),
            Self::Plane(map) => map.forward(p),
            Self::Blend(x, y) => Point::new(x.transform_point(p).x, y.transform_point(p).y),
        }
    }

    fn inverted(&self) -> Option<Self> {
        Some(match self {
            Self::Affine(affine) => Self::Affine(affine.inverted()?),
            Self::Separable(x, y) => Self::Separable(
                x.as_ref()
                    .map(|m| Rc::new(InverseAxis(Rc::clone(m))) as Rc<dyn AxisMap>),
                y.as_ref()
                    .map(|m| Rc::new(InverseAxis(Rc::clone(m))) as Rc<dyn AxisMap>),
            ),
            Self::Plane(map) => Self::Plane(Rc::new(InversePlane(Rc::clone(map)))),
            // A blend is only invertible when each component is separable;
            // matplotlib makes the same assumption.
            Self::Blend(x, y) => Self::Blend(Rc::new(x.inverted()?), Rc::new(y.inverted()?)),
        })
    }

    fn curves_lines(&self) -> bool {
        match self {
            Self::Plane(map) => map.curves_lines(),
            Self::Blend(x, y) => x.curves_lines() || y.curves_lines(),
            Self::Affine(_) | Self::Separable(..) => false,
        }
    }
}

#[derive(Debug)]
struct InverseAxis(Rc<dyn AxisMap>);

impl AxisMap for InverseAxis {
    fn forward(&self, value: f64) -> f64 {
        self.0.inverse(value)
    }
    fn inverse(&self, value: f64) -> f64 {
        self.0.forward(value)
    }
}

#[derive(Debug)]
struct InversePlane(Rc<dyn PointMap>);

impl PointMap for InversePlane {
    fn forward(&self, point: Point) -> Point {
        self.0.inverse(point)
    }
    fn inverse(&self, point: Point) -> Point {
        self.0.forward(point)
    }
    fn curves_lines(&self) -> bool {
        self.0.curves_lines()
    }
}

/// A transform resolved against concrete figure geometry.
///
/// Points pass through `steps` in order and then through `affine`.
/// Renderers receive only the affine part (see [`split_path`](Self::split_path)).
#[derive(Clone, Debug)]
pub struct ResolvedTransform {
    steps: Vec<Step>,
    affine: Affine2D,
}

impl ResolvedTransform {
    /// The identity.
    pub fn identity() -> Self {
        Self::affine(Affine2D::IDENTITY)
    }

    /// A purely affine transform.
    pub fn affine(affine: Affine2D) -> Self {
        Self {
            steps: Vec::new(),
            affine,
        }
    }

    /// A non-affine step followed by an affine map.
    pub fn with_step(step: Step, affine: Affine2D) -> Self {
        Self {
            steps: vec![step],
            affine,
        }
    }

    /// Whether the transform is a single affine map.
    pub fn is_affine(&self) -> bool {
        self.steps.is_empty()
    }

    /// The trailing affine map.
    pub fn affine_part(&self) -> Affine2D {
        self.affine
    }

    /// Applies `self`, then `next`.
    #[must_use]
    pub fn then(mut self, next: Self) -> Self {
        if next.steps.is_empty() {
            self.affine = self.affine.then(next.affine);
            return self;
        }
        if !self.affine.is_identity() {
            self.steps.push(Step::Affine(self.affine));
        }
        self.steps.extend(next.steps);
        self.affine = next.affine;
        self
    }

    /// x from `x` and y from `y` (matplotlib's blended transform).
    pub fn blend(x: Self, y: Self) -> Self {
        if x.is_affine() && y.is_affine() {
            let [xa, _, xc, _, xe, _] = x.affine.values();
            let [_, yb, _, yd, _, yf] = y.affine.values();
            return Self::affine(Affine2D::from_values(xa, yb, xc, yd, xe, yf));
        }
        Self::with_step(Step::Blend(Rc::new(x), Rc::new(y)), Affine2D::IDENTITY)
    }

    /// The inverse transform, if every stage is invertible.
    pub fn inverted(&self) -> Option<Self> {
        let mut steps = vec![Step::Affine(self.affine.inverted()?)];
        for step in self.steps.iter().rev() {
            steps.push(step.inverted()?);
        }
        let affine = match steps.pop() {
            Some(Step::Affine(last)) => last,
            Some(other) => {
                steps.push(other);
                Affine2D::IDENTITY
            }
            None => Affine2D::IDENTITY,
        };
        Some(Self { steps, affine })
    }

    /// Maps one point.
    pub fn transform_point(&self, p: Point) -> Point {
        let p = self.steps.iter().fold(p, |p, step| step.apply(p));
        self.affine.transform_point(p)
    }

    /// Maps a slice of points in place.
    pub fn transform_points(&self, points: &mut [Point]) {
        for p in points {
            *p = self.transform_point(*p);
        }
    }

    /// Applies only the non-affine steps to a point.
    pub fn transform_point_non_affine(&self, p: Point) -> Point {
        self.steps.iter().fold(p, |p, step| step.apply(p))
    }

    /// Whether straight segments bend under this transform.
    pub fn curves_lines(&self) -> bool {
        self.steps.iter().any(Step::curves_lines)
    }

    /// Splits a path for drawing: the non-affine steps are applied to the
    /// vertices (subdividing first if lines bend), and the affine part is
    /// returned for the renderer to apply.
    pub fn split_path<'a>(&self, path: &'a Path) -> (Cow<'a, Path>, Affine2D) {
        if self.steps.is_empty() {
            return (Cow::Borrowed(path), self.affine);
        }
        let mut owned = if self.curves_lines() && path.interpolation_steps() > 1 {
            path.interpolated(path.interpolation_steps())
        } else {
            path.clone()
        };
        for p in owned.vertices_mut() {
            *p = self.transform_point_non_affine(*p);
        }
        (Cow::Owned(owned), self.affine)
    }

    /// Fully transforms a path.
    pub fn transform_path(&self, path: &Path) -> Path {
        let (partial, affine) = self.split_path(path);
        partial.transformed(&affine)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct Log10;

    impl AxisMap for Log10 {
        fn forward(&self, value: f64) -> f64 {
            value.log10()
        }
        fn inverse(&self, value: f64) -> f64 {
            10f64.powf(value)
        }
    }

    fn close(a: Point, b: Point) -> bool {
        (a - b).length() < 1e-9
    }

    fn log_x_then_scale() -> ResolvedTransform {
        ResolvedTransform::with_step(
            Step::Separable(Some(Rc::new(Log10)), None),
            Affine2D::scaling(100.0, 2.0),
        )
    }

    #[test]
    fn affine_chains_fuse() {
        let t = ResolvedTransform::affine(Affine2D::scaling(2.0, 2.0))
            .then(ResolvedTransform::affine(Affine2D::translation(1.0, 0.0)));
        assert!(t.is_affine());
        assert_eq!(
            t.transform_point(Point::new(1.0, 1.0)),
            Point::new(3.0, 2.0)
        );
    }

    #[test]
    fn non_affine_steps_then_affine() {
        let t = log_x_then_scale();
        assert!(close(
            t.transform_point(Point::new(1000.0, 3.0)),
            Point::new(300.0, 6.0)
        ));
        let inv = t.inverted().expect("invertible");
        assert!(close(
            inv.transform_point(Point::new(300.0, 6.0)),
            Point::new(1000.0, 3.0)
        ));
    }

    #[test]
    fn composition_keeps_order() {
        let t = ResolvedTransform::affine(Affine2D::scaling(10.0, 1.0)).then(log_x_then_scale());
        // (10 * 10 = 100) -> log10 = 2 -> * 100 = 200
        assert!(close(
            t.transform_point(Point::new(10.0, 1.0)),
            Point::new(200.0, 2.0)
        ));
        let inv = t.inverted().expect("invertible");
        assert!(close(
            inv.transform_point(Point::new(200.0, 2.0)),
            Point::new(10.0, 1.0)
        ));
    }

    #[test]
    fn blend_takes_components() {
        let x = ResolvedTransform::affine(Affine2D::scaling(2.0, 0.0));
        let y = log_x_then_scale().then(ResolvedTransform::affine(Affine2D::translation(0.0, 1.0)));
        let blended = ResolvedTransform::blend(x, y);
        assert!(close(
            blended.transform_point(Point::new(5.0, 4.0)),
            Point::new(10.0, 9.0)
        ));
        let affine_blend = ResolvedTransform::blend(
            ResolvedTransform::affine(Affine2D::scaling(2.0, 7.0)),
            ResolvedTransform::affine(Affine2D::scaling(5.0, 3.0)),
        );
        assert!(affine_blend.is_affine());
        assert_eq!(
            affine_blend.transform_point(Point::new(1.0, 1.0)),
            Point::new(2.0, 3.0)
        );
    }

    #[test]
    fn split_path_applies_only_non_affine_part() {
        let t = log_x_then_scale();
        let path = Path::polyline(vec![Point::new(10.0, 1.0), Point::new(100.0, 2.0)]);
        let (partial, affine) = t.split_path(&path);
        assert!(close(partial.vertices()[1], Point::new(2.0, 2.0)));
        assert_eq!(affine, Affine2D::scaling(100.0, 2.0));
        assert!(close(
            t.transform_path(&path).vertices()[0],
            Point::new(100.0, 2.0)
        ));
    }
}
