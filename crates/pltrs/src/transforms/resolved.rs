//! Concrete transforms: non-affine steps followed by one affine map.

use std::borrow::Cow;
use std::fmt;
use std::rc::Rc;

use pltrs_core::{Affine2D, Bbox, Path, Point};

use super::blend::blend_rows;
use super::program_calls;

/// An invertible map of one coordinate (e.g. a log scale).
pub trait AxisMap: fmt::Debug + std::any::Any {
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
    /// Maps every value of `values` in place, as matplotlib's transforms
    /// map arrays. The default maps them one by one; a map calling the
    /// program's code overrides it to call it once for all of them.
    fn forward_all(&self, values: &mut [f64]) {
        for value in values {
            *value = self.forward(*value);
        }
    }
    /// [`forward_all`](AxisMap::forward_all) for the inverse map.
    fn inverse_all(&self, values: &mut [f64]) {
        for value in values {
            *value = self.inverse(*value);
        }
    }
    /// Maps every value of `values` in place as matplotlib's
    /// `Transform.transform` maps the values it is given
    /// (`axis.get_transform().transform(...)`, which autoscaling calls): a
    /// map calling the program's code hands them over as a column,
    /// shaped `(N, 1)`, where drawing hands a row
    /// ([`forward_all`](AxisMap::forward_all)). The default maps them as
    /// `forward_all` does.
    fn forward_column(&self, values: &mut [f64]) {
        self.forward_all(values);
    }
    /// [`forward_column`](AxisMap::forward_column) for the inverse map.
    fn inverse_column(&self, values: &mut [f64]) {
        self.inverse_all(values);
    }
    /// Maps every value of `values` in place as matplotlib maps the
    /// coordinates of a mesh it draws, a masked array: a map calling the
    /// program's code hands them over as a masked row
    /// ([`ArrayLayout::MaskedRow`](crate::scale::ArrayLayout::MaskedRow)).
    /// The default maps them as [`forward_all`](AxisMap::forward_all)
    /// does.
    fn forward_masked_row(&self, values: &mut [f64]) {
        self.forward_all(values);
    }
    /// Whether the map runs code of the program's that observes its
    /// calls (a scale of a host language's functions): the axes it scales
    /// are then ticked as matplotlib ticks them, as often (see
    /// [`TickUpdater`](crate::axes::TickUpdater)). `false` by default.
    fn calls_program(&self) -> bool {
        false
    }
}

/// An invertible map of the plane (e.g. a polar projection).
pub trait PointMap: fmt::Debug + std::any::Any {
    /// Forward map.
    fn forward(&self, point: Point) -> Point;
    /// Inverse map.
    fn inverse(&self, point: Point) -> Point;
    /// Maps every point of `points` in place, as matplotlib's transforms
    /// map arrays. The default maps them one by one; a map calling the
    /// program's code overrides it to call it once for all of them.
    fn forward_all(&self, points: &mut [Point]) {
        for point in points {
            *point = self.forward(*point);
        }
    }
    /// [`forward_all`](PointMap::forward_all) for the inverse map.
    fn inverse_all(&self, points: &mut [Point]) {
        for point in points {
            *point = self.inverse(*point);
        }
    }
    /// Maps a path (matplotlib's `transform_path_non_affine`). The default
    /// maps every vertex; projections that bend straight lines override it
    /// to emit curves, typically honouring the path's
    /// [`interpolation_steps`](Path::interpolation_steps).
    fn transform_path(&self, path: &Path) -> Path {
        let mut mapped = path.clone();
        self.forward_all(mapped.vertices_mut());
        mapped
    }
    /// Maps the outline of a patch being drawn, as matplotlib's
    /// `Patch.draw` maps it (`transform_path_non_affine`, anew at every
    /// draw), where [`transform_path`](PointMap::transform_path) may keep
    /// what it mapped as a line's `TransformedPath` does. The default maps
    /// it as `transform_path` does.
    fn transform_outline(&self, path: &Path) -> Path {
        self.transform_path(path)
    }
    /// Whether the map runs code of the program's (a transform of a host
    /// language's), which a draw lets change the figure (see
    /// [`ResolvedTransform::calls_program`]). `false` by default.
    fn calls_program(&self) -> bool {
        false
    }
    /// What names the program's code the map runs, so that pltrs' own
    /// work answers from what the program answered before instead of
    /// calling it again (matplotlib calls the program where its artists
    /// map their parts, which pltrs models apart): maps giving the same
    /// key answer alike while one of them lives. `None`, the default,
    /// keeps no answers, the map being called every time.
    fn program_key(&self) -> Option<usize> {
        None
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

pltrs_core::archive_enum!(Step {
    Affine(v0),
    Separable(v0, v1),
    Plane(v0),
    Blend(v0, v1),
});

impl Step {
    fn apply(&self, p: Point) -> Point {
        match self {
            Self::Affine(affine) => affine.transform_point(p),
            Self::Separable(x, y) => Point::new(
                x.as_ref().map_or(p.x, |m| program_calls::forward(m, p.x)),
                y.as_ref().map_or(p.y, |m| program_calls::forward(m, p.y)),
            ),
            Self::Plane(map) => program_calls::forward_point(map, p),
            Self::Blend(x, y) => Point::new(
                x.transform_point_non_affine(p).x,
                y.transform_point_non_affine(p).y,
            ),
        }
    }

    /// [`apply`](Self::apply) to every point of `points`, each map of the
    /// step called once for all of them.
    fn apply_all(&self, points: &mut [Point]) {
        match self {
            Self::Affine(affine) => {
                for p in points {
                    *p = affine.transform_point(*p);
                }
            }
            Self::Separable(x, y) => {
                if let Some(map) = x {
                    map_coordinates(points, map, |p| &mut p.x);
                }
                if let Some(map) = y {
                    map_coordinates(points, map, |p| &mut p.y);
                }
            }
            Self::Plane(map) => program_calls::forward_points(map, points),
            Self::Blend(x, y) => {
                let mut xs = points.to_vec();
                x.transform_points_non_affine(&mut xs);
                y.transform_points_non_affine(points);
                for (p, from_x) in points.iter_mut().zip(xs) {
                    p.x = from_x.x;
                }
            }
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
            Self::Blend(x, y) => Point::new(
                x.transform_point_non_affine_masked(p).x,
                y.transform_point_non_affine_masked(p).y,
            ),
            Self::Affine(_) | Self::Plane(_) => self.apply(p),
        }
    }

    /// Whether the step is `other`: the same maps, the same affine map.
    fn same_as(&self, other: &Self) -> bool {
        let same_map = |a: &Option<Rc<dyn AxisMap>>, b: &Option<Rc<dyn AxisMap>>| match (a, b) {
            (Some(a), Some(b)) => std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)),
            (None, None) => true,
            _ => false,
        };
        match (self, other) {
            (Self::Affine(a), Self::Affine(b)) => a == b,
            (Self::Separable(ax, ay), Self::Separable(bx, by)) => {
                same_map(ax, bx) && same_map(ay, by)
            }
            (Self::Plane(a), Self::Plane(b)) => std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)),
            (Self::Blend(ax, ay), Self::Blend(bx, by)) => ax.same_steps(bx) && ay.same_steps(by),
            _ => false,
        }
    }

    /// Whether the step maps through a scale of the program's (a map of
    /// one coordinate calling its code), and through no map of the plane
    /// calling it.
    fn scales_call_program(&self) -> Option<bool> {
        match self {
            Self::Affine(_) => Some(false),
            Self::Separable(x, y) => {
                Some([x, y].into_iter().flatten().any(|map| map.calls_program()))
            }
            Self::Plane(map) => (!map.calls_program()).then_some(false),
            Self::Blend(x, y) => Some(x.program_scales()? | y.program_scales()?),
        }
    }

    /// Whether the step runs code of the program's.
    fn calls_program(&self) -> bool {
        match self {
            Self::Affine(_) => false,
            Self::Separable(x, y) => [x, y].into_iter().flatten().any(|map| map.calls_program()),
            Self::Plane(map) => map.calls_program(),
            Self::Blend(x, y) => x.calls_program() || y.calls_program(),
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
            return program_calls::transform_path(map, &path);
        }
        self.apply_all(path.vertices_mut());
        path
    }

    /// Maps the outline of a patch being drawn (see
    /// [`PointMap::transform_outline`]).
    fn transform_outline(&self, mut path: Path) -> Path {
        if let Self::Plane(map) = self {
            return map.transform_outline(&path);
        }
        self.apply_all(path.vertices_mut());
        path
    }
}

/// One coordinate (`coordinate` picks it) of every point of `points`
/// through `map`, called once for all of them (see
/// [`program_calls::forward_all`]).
fn map_coordinates(
    points: &mut [Point],
    map: &Rc<dyn AxisMap>,
    coordinate: impl Fn(&mut Point) -> &mut f64,
) {
    let mut values: Vec<f64> = points.iter_mut().map(|p| *coordinate(p)).collect();
    program_calls::forward_all(map, &mut values);
    for (p, value) in points.iter_mut().zip(values) {
        *coordinate(p) = value;
    }
}

#[derive(Debug)]
struct InverseAxis(Rc<dyn AxisMap>);

pltrs_core::archive_struct!(InverseAxis(v0));

impl AxisMap for InverseAxis {
    fn forward(&self, value: f64) -> f64 {
        self.0.inverse(value)
    }
    fn inverse(&self, value: f64) -> f64 {
        self.0.forward(value)
    }
    fn forward_all(&self, values: &mut [f64]) {
        self.0.inverse_all(values);
    }
    fn inverse_all(&self, values: &mut [f64]) {
        self.0.forward_all(values);
    }
    fn calls_program(&self) -> bool {
        self.0.calls_program()
    }
}

#[derive(Debug)]
struct InversePlane(Rc<dyn PointMap>);

pltrs_core::archive_struct!(InversePlane(v0));

impl PointMap for InversePlane {
    fn forward(&self, point: Point) -> Point {
        self.0.inverse(point)
    }
    fn inverse(&self, point: Point) -> Point {
        self.0.forward(point)
    }
    fn forward_all(&self, points: &mut [Point]) {
        self.0.inverse_all(points);
    }
    fn inverse_all(&self, points: &mut [Point]) {
        self.0.forward_all(points);
    }
    fn calls_program(&self) -> bool {
        self.0.calls_program()
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

pltrs_core::archive_struct!(ResolvedTransform { steps, affine });

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

    /// Whether x and y map independently of each other (matplotlib's
    /// `is_separable`): no step is a general map of the plane, and no
    /// affine map rotates or shears.
    ///
    /// ```
    /// use pltrs::transforms::{Affine2D, ResolvedTransform};
    /// assert!(ResolvedTransform::affine(Affine2D::scaling(2.0, 3.0)).is_separable());
    /// assert!(!ResolvedTransform::affine(Affine2D::rotation(0.5)).is_separable());
    /// ```
    pub fn is_separable(&self) -> bool {
        self.affine.is_separable()
            && self.steps.iter().all(|step| match step {
                Step::Affine(affine) => affine.is_separable(),
                Step::Separable(..) | Step::Blend(..) => true,
                Step::Plane(_) => false,
            })
    }

    /// Whether applying the transform runs code of the program's: a map
    /// of a host language's (see [`AxisMap::calls_program`] and
    /// [`PointMap::calls_program`]), which matplotlib lets read and change
    /// the figure while it is drawn, so a draw lets the figure go first
    /// ([`DrawContext::free_for`](crate::artist::DrawContext::free_for)).
    pub fn calls_program(&self) -> bool {
        self.steps.iter().any(Step::calls_program)
    }

    /// Whether the non-affine steps of the transform are those of `other`
    /// (the same maps, the same affine maps between them), so that they
    /// map every point alike, whatever affine map follows them: what
    /// matplotlib's `TransformedPath` keeps stands until they change.
    pub(crate) fn same_steps(&self, other: &Self) -> bool {
        self.steps.len() == other.steps.len()
            && (self.steps.iter().zip(&other.steps)).all(|(a, b)| a.same_as(b))
    }

    /// Whether applying the transform runs code of the program's only in
    /// scales of the program's (maps of one coordinate), not in maps of
    /// the plane: a transform of the program's maps whole paths as
    /// matplotlib's `TransformedPath` maps them itself, where scales are
    /// called by the paths drawn through them.
    pub(crate) fn only_scales_call_program(&self) -> bool {
        self.program_scales().unwrap_or(false)
    }

    /// Whether applying the transform runs code of the program's in
    /// scales of the program's (`Some(true)`), in none (`Some(false)`), or
    /// in a map of the plane (`None`).
    fn program_scales(&self) -> Option<bool> {
        (self.steps.iter())
            .map(Step::scales_call_program)
            .try_fold(false, |any, step| Some(any | step?))
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
        self.affine
            .transform_point(self.transform_point_non_affine_masked(p))
    }

    /// [`transform_point_non_affine`](Self::transform_point_non_affine)
    /// for one point of a masked array.
    pub fn transform_point_non_affine_masked(&self, p: Point) -> Point {
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

    /// Maps a slice of points in place, each map of the program's called
    /// once for all of them (as matplotlib maps arrays).
    pub fn transform_points(&self, points: &mut [Point]) {
        self.transform_points_non_affine(points);
        for p in points {
            *p = self.affine.transform_point(*p);
        }
    }

    /// Applies only the non-affine steps to a point.
    pub fn transform_point_non_affine(&self, p: Point) -> Point {
        self.steps.iter().fold(p, |p, step| step.apply(p))
    }

    /// Applies only the non-affine steps to a slice of points in place,
    /// each map called once for all of them.
    pub fn transform_points_non_affine(&self, points: &mut [Point]) {
        for step in &self.steps {
            step.apply_all(points);
        }
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

    /// Splits the outline of a patch being drawn like
    /// [`split_path`](Self::split_path), its plane maps mapping it as
    /// matplotlib's `Patch.draw` does (see
    /// [`PointMap::transform_outline`]).
    pub fn split_outline<'a>(&self, path: &'a Path) -> (Cow<'a, Path>, Affine2D) {
        if self.steps.is_empty() {
            return (Cow::Borrowed(path), self.affine);
        }
        let mapped =
            (self.steps.iter()).fold(path.clone(), |path, step| step.transform_outline(path));
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
        self.transform_points_non_affine(mapped.vertices_mut());
        (Cow::Owned(mapped), self.affine)
    }

    /// Fully transforms a path.
    pub fn transform_path(&self, path: &Path) -> Path {
        let (partial, affine) = self.split_path(path);
        partial.transformed(&affine)
    }
}

pltrs_core::archive_kinds!(dyn AxisMap {
    "inverse" => InverseAxis,
    "log" => crate::scale::LogTransform,
    "symlog" => crate::scale::SymmetricalLogTransform,
    "logit" => crate::scale::LogitTransform,
    "asinh" => crate::scale::AsinhTransform,
    "func" => crate::scale::FuncTransform,
    "func-log" => crate::scale::FuncLogTransform,
    "linear" => crate::colors::LinearTransform,
});

pltrs_core::archive_kinds!(dyn PointMap {
    "inverse-plane" => InversePlane,
    "polar-to-cartesian" => crate::coords::PolarToCartesian,
    "geo" => crate::geo::GeoMap,
    "polar" => crate::polar::PolarMap,
    "aux-inverse" => crate::axisartist::AuxInverse,
    "aux-polar" => crate::axisartist::AuxPolar,
    "aux-functions" => crate::axisartist::AuxFunctions,
});

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
