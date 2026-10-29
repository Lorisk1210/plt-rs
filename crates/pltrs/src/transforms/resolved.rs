//! Concrete transforms: non-affine steps followed by one affine map.

use std::borrow::Cow;
use std::fmt;
use std::rc::Rc;

use pltrs_core::{Affine2D, Bbox, Path, Point};

use super::blend::blend_rows;

/// An invertible map of one coordinate (e.g. a log scale).
pub trait AxisMap: fmt::Debug {
    /// Forward map.
    fn forward(&self, value: f64) -> f64;
    /// Inverse map.
    fn inverse(&self, value: f64) -> f64;
    /// The forward map of an element of a masked array: NaN (masked)
    /// where numpy's masked operations leave the domain of the map, such
    /// as a logarithm of a non-positive value, which [`forward`] may clip
    /// instead (matplotlib maps scatter offsets, masked arrays, this way).
    ///
    /// [`forward`]: AxisMap::forward
    fn forward_masked(&self, value: f64) -> f64 {
        self.forward(value)
    }
}

/// An invertible map of the plane (e.g. a polar projection).
pub trait PointMap: fmt::Debug {
    /// Forward map.
    fn forward(&self, point: Point) -> Point;
    /// Inverse map.
    fn inverse(&self, point: Point) -> Point;
    /// Maps a path (matplotlib's `transform_path_non_affine`). The default
    /// maps every vertex; projections that bend straight lines override it
    /// to emit curves, typically honouring the path's
    /// [`interpolation_steps`](Path::interpolation_steps).
    fn transform_path(&self, path: &Path) -> Path {
        let mut mapped = path.clone();
        for p in mapped.vertices_mut() {
            *p = self.forward(*p);
        }
        mapped
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
            Self::Blend(x, y) => Point::new(
                x.transform_point_non_affine(p).x,
                y.transform_point_non_affine(p).y,
            ),
        }
    }

    /// [`apply`](Self::apply) to an element of a masked array (see
    /// [`AxisMap::forward_masked`]).
    fn apply_masked(&self, p: Point) -> Point {
        match self {
            Self::Separable(x, y) => Point::new(
                x.as_ref().map_or(p.x, |m| m.forward_masked(p.x)),
                y.as_ref().map_or(p.y, |m| m.forward_masked(p.y)),
            ),
            Self::Blend(x, y) => Point::new(x.non_affine_masked(p).x, y.non_affine_masked(p).y),
            Self::Affine(_) | Self::Plane(_) => self.apply(p),
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
            Self::Blend(x, y) => Self::Blend(
                Rc::new(x.non_affine_inverted()?),
                Rc::new(y.non_affine_inverted()?),
            ),
        })
    }

    /// Maps a path: plane maps may bend it, every other stage maps the
    /// vertices.
    fn transform_path(&self, mut path: Path) -> Path {
        if let Self::Plane(map) = self {
            return map.transform_path(&path);
        }
        for p in path.vertices_mut() {
            *p = self.apply(*p);
        }
        path
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

    /// Whether the transform leaves every point where it is.
    pub fn is_identity(&self) -> bool {
        self.is_affine() && self.affine.is_identity()
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
    ///
    /// Like matplotlib's `BlendedGenericTransform`, the non-affine parts
    /// are blended first (the blend step applies only them) and the affine
    /// parts after, as one affine; this
    /// keeps an infinite non-affine coordinate on one side (logit of an
    /// axes edge) from reaching the other side's affine as `0 · ∞`.
    pub fn blend(x: Self, y: Self) -> Self {
        let affine = blend_rows(&x.affine, &y.affine);
        if x.is_affine() && y.is_affine() {
            return Self::affine(affine);
        }
        Self::with_step(Step::Blend(Rc::new(x), Rc::new(y)), affine)
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

    /// The inverse of the non-affine steps alone.
    fn non_affine_inverted(&self) -> Option<Self> {
        let steps = self
            .steps
            .iter()
            .rev()
            .map(Step::inverted)
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            steps,
            affine: Affine2D::IDENTITY,
        })
    }

    /// Maps one point.
    pub fn transform_point(&self, p: Point) -> Point {
        let p = self.steps.iter().fold(p, |p, step| step.apply(p));
        self.affine.transform_point(p)
    }

    /// Maps one point of a masked array: NaN where a non-linear scale
    /// masks it (see [`AxisMap::forward_masked`]).
    pub fn transform_point_masked(&self, p: Point) -> Point {
        self.affine.transform_point(self.non_affine_masked(p))
    }

    /// The non-affine steps applied to one point of a masked array.
    fn non_affine_masked(&self, p: Point) -> Point {
        self.steps.iter().fold(p, |p, step| step.apply_masked(p))
    }

    /// The direction, in degrees, that `angle` (degrees) at `point` takes
    /// after the transform: the angle of the segment from `point` to a
    /// point pushed `1e-5` along `angle` (matplotlib's `transform_angles`
    /// with its default push-off).
    pub fn transform_angle(&self, angle: f64, point: Point) -> f64 {
        let radians = angle.to_radians();
        let pushed = Point::new(
            point.x + 1e-5 * radians.cos(),
            point.y + 1e-5 * radians.sin(),
        );
        let (from, to) = (self.transform_point(point), self.transform_point(pushed));
        (to.y - from.y).atan2(to.x - from.x).to_degrees()
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

    /// The box spanned by the images of the two corners of `bbox`
    /// (matplotlib's `transform_bbox`): neither ordered nor grown to
    /// enclose the image of the whole box, which a rotation or a
    /// non-affine step may bend beyond them.
    pub fn transform_bbox(&self, bbox: &Bbox) -> Bbox {
        Bbox::new(self.transform_point(bbox.p0), self.transform_point(bbox.p1))
    }

    /// Splits a path for drawing: the non-affine steps are applied to the
    /// path one after the other (plane maps may turn lines into curves),
    /// and the affine part is returned for the renderer to apply.
    pub fn split_path<'a>(&self, path: &'a Path) -> (Cow<'a, Path>, Affine2D) {
        if self.steps.is_empty() {
            return (Cow::Borrowed(path), self.affine);
        }
        let mapped = self
            .steps
            .iter()
            .fold(path.clone(), |path, step| step.transform_path(path));
        (Cow::Owned(mapped), self.affine)
    }

    /// Splits a path of positions (markers) for drawing like
    /// [`split_path`](Self::split_path), but maps the vertices one by one
    /// so that no projection subdivides it (matplotlib's
    /// `get_transformed_points_and_affine`).
    pub fn split_points<'a>(&self, points: &'a Path) -> (Cow<'a, Path>, Affine2D) {
        if self.steps.is_empty() {
            return (Cow::Borrowed(points), self.affine);
        }
        let mut mapped = points.clone();
        for p in mapped.vertices_mut() {
            *p = self.transform_point_non_affine(*p);
        }
        (Cow::Owned(mapped), self.affine)
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
    use crate::testing::close_points;

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

    fn log_x_then_scale() -> ResolvedTransform {
        ResolvedTransform::with_step(
            Step::Separable(Some(Rc::new(Log10)), None),
            Affine2D::scaling(100.0, 2.0),
        )
    }

    #[test]
    fn masked_points_outside_a_log_scale_are_lost() {
        use crate::scale::{LogTransform, NonPositive};

        let log = LogTransform::new(10.0, NonPositive::Clip).expect("a valid base");
        let log_y = ResolvedTransform::with_step(
            Step::Separable(None, Some(Rc::new(log))),
            Affine2D::scaling(1.0, 2.0),
        );
        let zero = Point::new(1.0, 0.0);
        assert_eq!(log_y.transform_point(zero), Point::new(1.0, -2000.0));
        assert!(log_y.transform_point_masked(zero).y.is_nan());
        assert_eq!(
            log_y.transform_point_masked(Point::new(1.0, 100.0)),
            Point::new(1.0, 4.0)
        );
        let blended = ResolvedTransform::blend(ResolvedTransform::identity(), log_y);
        assert!(blended.transform_point_masked(zero).y.is_nan());
        assert_eq!(
            blended.transform_point_masked(Point::new(1.0, 10.0)),
            Point::new(1.0, 2.0)
        );
    }

    #[test]
    fn angles_turn_with_the_transform() {
        let stretch = ResolvedTransform::affine(Affine2D::scaling(1.0, 2.0));
        let turned = stretch.transform_angle(45.0, Point::new(3.0, 4.0));
        assert!((turned - 2f64.atan().to_degrees()).abs() < 1e-6);
        let identity = ResolvedTransform::affine(Affine2D::IDENTITY);
        assert!((identity.transform_angle(-30.0, Point::new(1.0, 1.0)) + 30.0).abs() < 1e-6);
        let log = log_x_then_scale();
        assert!((log.transform_angle(90.0, Point::new(10.0, 1.0)) - 90.0).abs() < 1e-6);
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
        assert!(close_points(
            t.transform_point(Point::new(1000.0, 3.0)),
            Point::new(300.0, 6.0)
        ));
        let inv = t.inverted().expect("invertible");
        assert!(close_points(
            inv.transform_point(Point::new(300.0, 6.0)),
            Point::new(1000.0, 3.0)
        ));
    }

    #[test]
    fn composition_keeps_order() {
        let t = ResolvedTransform::affine(Affine2D::scaling(10.0, 1.0)).then(log_x_then_scale());
        // (10 * 10 = 100) -> log10 = 2 -> * 100 = 200
        assert!(close_points(
            t.transform_point(Point::new(10.0, 1.0)),
            Point::new(200.0, 2.0)
        ));
        let inv = t.inverted().expect("invertible");
        assert!(close_points(
            inv.transform_point(Point::new(200.0, 2.0)),
            Point::new(10.0, 1.0)
        ));
    }

    #[test]
    fn blend_takes_components() {
        let x = ResolvedTransform::affine(Affine2D::scaling(2.0, 0.0));
        let y = log_x_then_scale().then(ResolvedTransform::affine(Affine2D::translation(0.0, 1.0)));
        let blended = ResolvedTransform::blend(x, y);
        assert!(close_points(
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
    fn blend_keeps_infinite_coordinates_on_their_side() {
        // x from a transform with a log y map, y from axes fractions: at
        // y = 0 the log is -inf, which must not turn x into 0 * -inf.
        let data = ResolvedTransform::with_step(
            Step::Separable(None, Some(Rc::new(Log10))),
            Affine2D::scaling(100.0, 2.0),
        );
        let axes = ResolvedTransform::affine(Affine2D::scaling(1.0, 50.0));
        let blended = ResolvedTransform::blend(data, axes);
        assert_eq!(
            blended.transform_point(Point::new(0.5, 0.0)),
            Point::new(50.0, 0.0)
        );
        let inverse = blended.inverted().expect("invertible");
        assert!(close_points(
            inverse.transform_point(Point::new(50.0, 25.0)),
            Point::new(0.5, 0.5)
        ));
    }

    #[test]
    fn bboxes_map_their_two_corners() {
        let t = log_x_then_scale();
        let mapped = t.transform_bbox(&Bbox::from_extents(1000.0, 3.0, 10.0, -1.0));
        assert!(close_points(mapped.p0, Point::new(300.0, 6.0)));
        assert!(
            close_points(mapped.p1, Point::new(100.0, -2.0)),
            "not reordered"
        );
        let rotated = ResolvedTransform::affine(Affine2D::rotation(std::f64::consts::FRAC_PI_2));
        let quarter = rotated.transform_bbox(&Bbox::UNIT);
        assert!(
            close_points(quarter.p1, Point::new(-1.0, 1.0)),
            "corners only"
        );
        assert!(!t.transform_bbox(&Bbox::NULL).is_finite());
    }

    #[test]
    fn split_path_applies_only_non_affine_part() {
        let t = log_x_then_scale();
        let path = Path::polyline(vec![Point::new(10.0, 1.0), Point::new(100.0, 2.0)]);
        let (partial, affine) = t.split_path(&path);
        assert!(close_points(partial.vertices()[1], Point::new(2.0, 2.0)));
        assert_eq!(affine, Affine2D::scaling(100.0, 2.0));
        assert!(close_points(
            t.transform_path(&path).vertices()[0],
            Point::new(100.0, 2.0)
        ));
    }
}
