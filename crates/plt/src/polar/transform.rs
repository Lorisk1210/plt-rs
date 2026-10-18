//! The polar projection: (θ, r) onto the axes circle (matplotlib's
//! `PolarTransform`, `PolarAffine` and `_WedgeBbox`), plus the geometry
//! of the background wedge and the spines derived from it.

use std::f64::consts::TAU;
use std::rc::Rc;

use plt_core::{Affine2D, Bbox, Path, PathCode, Point};

use super::PolarState;
use crate::patches::{PatchShape, Wedge};
use crate::spines::SpineSide;
use crate::transforms::{AxisMap, PointMap, ResolvedTransform, Step, bbox_to_unit, unit_to_bbox};

/// Whether `[theta_min, theta_max]` (degrees) spans the whole circle
/// (matplotlib's `_is_full_circle_deg`).
pub(crate) fn full_circle_deg(theta_min: f64, theta_max: f64) -> bool {
    ((theta_max - theta_min).abs() - 360.0).abs() < 1e-12
}

/// Whether `[theta_min, theta_max]` (radians) spans the whole circle
/// (matplotlib's `_is_full_circle_rad`).
pub(crate) fn full_circle_rad(theta_min: f64, theta_max: f64) -> bool {
    ((theta_max - theta_min).abs() - TAU).abs() < 1.74e-14
}

/// numpy's `sign`: `-1`, `0` or `1` (NaN stays NaN).
fn sign(value: f64) -> f64 {
    if value == 0.0 || value.is_nan() {
        value
    } else {
        value.signum()
    }
}

/// Polar to Cartesian coordinates around the origin, applied after the
/// radial scale and the angular shift (matplotlib's `PolarTransform`).
#[derive(Debug)]
pub(crate) struct PolarMap {
    /// The scaled radius drawn at the centre.
    scaled_origin: f64,
    /// The unscaled radius at the centre, which matplotlib's
    /// `InvertedPolarTransform` adds back without rescaling.
    origin: f64,
    /// `1`, or `-1` when the radial axis decreases outwards.
    sign: f64,
}

impl PolarMap {
    /// matplotlib's `PolarTransform()` without an axis: radii are drawn
    /// as they are, from the origin.
    pub(crate) const PLAIN: Self = Self {
        scaled_origin: 0.0,
        origin: 0.0,
        sign: 1.0,
    };

    /// The distance from the centre of a scaled radius (negative inside
    /// the origin).
    fn radius(&self, r: f64) -> f64 {
        (r - self.scaled_origin) * self.sign
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
    /// first, without its first vertex.
    fn push_arc(&mut self, mut from: f64, to: f64, r: f64) {
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
    /// interpolation step follow the circle.
    fn transform_path(&self, path: &Path) -> Path {
        let steps = path.interpolation_steps();
        if path.is_empty() || steps == 1 {
            let mut mapped = path.clone();
            for p in mapped.vertices_mut() {
                *p = self.forward(*p);
            }
            return mapped;
        }
        self.curved_path(path, steps)
    }
}

/// The polar geometry of one axes at its current view limits.
pub(crate) struct PolarView<'a> {
    settings: &'a PolarState,
    /// The view limits: θ along x, r along y.
    view: Bbox,
    /// The radial scale, if not linear.
    rmap: Option<&'a Rc<dyn AxisMap>>,
}

impl<'a> PolarView<'a> {
    /// The geometry of polar axes with `settings`, view limits `view` and
    /// radial scale `rmap`.
    pub(crate) fn new(
        settings: &'a PolarState,
        view: Bbox,
        rmap: Option<&'a Rc<dyn AxisMap>>,
    ) -> Self {
        Self {
            settings,
            view,
            rmap,
        }
    }

    /// The view limits.
    pub(crate) fn view(&self) -> Bbox {
        self.view
    }

    /// The polar settings.
    pub(crate) fn settings(&self) -> &PolarState {
        self.settings
    }

    /// `1` or `-1`: the factor applied to θ.
    pub(crate) fn direction(&self) -> f64 {
        self.settings.direction.sign()
    }

    /// θ direction and offset (matplotlib's `transShift`).
    pub(crate) fn shift(&self) -> Affine2D {
        Affine2D::scaling(self.direction(), 1.0).translate(self.settings.theta_offset, 0.0)
    }

    /// The view limits after the θ shift (matplotlib's `_realViewLim`).
    pub(crate) fn real_view(&self) -> Bbox {
        let shift = self.shift();
        let (x0, x1) = self.view.interval_x();
        let (a, b) = (
            shift.transform_point(Point::new(x0, 0.0)).x,
            shift.transform_point(Point::new(x1, 0.0)).x,
        );
        let (low, high) = (a.min(b), a.max(b));
        let (start, end) = if x0 > x1 { (high, low) } else { (low, high) };
        Bbox::from_extents(start, self.view.y0(), end, self.view.y1())
    }

    /// The sorted θ limits after the shift, in degrees.
    pub(crate) fn real_theta_degrees(&self) -> (f64, f64) {
        let (a, b) = self.real_view().interval_x();
        let (a, b) = (a.to_degrees(), b.to_degrees());
        (a.min(b), a.max(b))
    }

    /// Whether the θ view spans the whole circle (on the shifted limits,
    /// as matplotlib's `draw` decides it).
    pub(crate) fn is_full_circle(&self) -> bool {
        let (low, high) = self.real_theta_degrees();
        full_circle_deg(low, high)
    }

    /// Whether the θ view spans the whole circle (on the unshifted
    /// limits in radians, as matplotlib's `RadialLocator` decides it).
    pub(crate) fn is_full_circle_rad(&self) -> bool {
        let (x0, x1) = self.view.interval_x();
        full_circle_rad(x0, x1)
    }

    /// The radius at the centre (matplotlib's `get_rorigin`).
    pub(crate) fn rorigin(&self) -> f64 {
        self.settings.rorigin.unwrap_or(self.view.y0())
    }

    /// `1`, or `-1` when the radial axis decreases (`get_rsign`).
    pub(crate) fn rsign(&self) -> f64 {
        sign(self.view.y1() - self.rorigin())
    }

    /// A radius through the radial scale.
    pub(crate) fn scaled(&self, r: f64) -> f64 {
        self.rmap.map_or(r, |map| map.forward(r))
    }

    fn map(&self) -> PolarMap {
        PolarMap {
            scaled_origin: self.scaled(self.rorigin()),
            origin: self.rorigin(),
            sign: self.rsign(),
        }
    }

    /// Unit circle of the scaled radial range around `(0.5, 0.5)`
    /// (matplotlib's `PolarAffine`).
    fn projection_affine(&self) -> Affine2D {
        let (low, high) = (self.scaled(self.rorigin()), self.scaled(self.view.y1()));
        let span = high.max(low) - high.min(low);
        Affine2D::scaling(0.5 / span, 0.5 / span).translate(0.5, 0.5)
    }

    /// The part of the axes square the wedge occupies, widened to a
    /// square (matplotlib's `_WedgeBbox`; built, like matplotlib, from the
    /// unscaled radii and the wedge's control points).
    pub(crate) fn wedge_bbox(&self) -> Bbox {
        let real = self.real_view();
        let (t0, t1) = (real.x0().to_degrees(), real.x1().to_degrees());
        let (t0, t1) = if t0 > t1 { (t1, t0) } else { (t0, t1) };
        let origin = self.rorigin();
        let (r0, r1) = (real.y0() - origin, real.y1() - origin);
        let rscale = 0.5 / r1;
        let (r0, r1) = (r0 * rscale, r1 * rscale);
        let wedge = Wedge {
            width: Some((r1 - r0).min(0.5)),
            ..Wedge::new((0.5, 0.5), r1, t0, t1)
        };
        let corners = Bbox::from_points(
            wedge
                .path()
                .vertices()
                .iter()
                .copied()
                .filter(|p| p.is_finite()),
        );
        let (w, h) = (corners.width(), corners.height());
        let dh = (w - h).max(0.0) / 2.0;
        let dw = (h - w).max(0.0) / 2.0;
        Bbox::from_extents(
            corners.x0() - dw,
            corners.y0() - dh,
            corners.x1() + dw,
            corners.y1() + dh,
        )
    }

    /// The wedge square onto the axes box at `display` (matplotlib's
    /// `transWedge + transAxes`).
    pub(crate) fn wedge_to_display(&self, display: &Bbox) -> Affine2D {
        bbox_to_unit(&self.wedge_bbox()).then(unit_to_bbox(display))
    }

    /// The complete data → display transform: radial scale, θ shift,
    /// projection, then the projection affine, the wedge and the axes box
    /// (matplotlib's `transData`).
    pub(crate) fn data_transform(&self, display: &Bbox) -> ResolvedTransform {
        let scale = match self.rmap {
            Some(map) => ResolvedTransform::with_step(
                Step::Separable(None, Some(Rc::clone(map))),
                self.shift(),
            ),
            None => ResolvedTransform::affine(self.shift()),
        };
        let affine = self
            .projection_affine()
            .then(self.wedge_to_display(display));
        scale.then(ResolvedTransform::with_step(
            Step::Plane(Rc::new(self.map())),
            affine,
        ))
    }

    /// The scaled `(rmin, rmax)` measured from the origin (matplotlib's
    /// `draw`).
    fn radial_extent(&self) -> (f64, f64) {
        let real = self.real_view();
        let origin = self.scaled(self.rorigin());
        let sign = self.rsign();
        (
            (self.scaled(real.y0()) - origin) * sign,
            (self.scaled(real.y1()) - origin) * sign,
        )
    }

    /// The background wedge in axes coordinates (matplotlib's
    /// `PolarAxes.draw` updating the `Wedge` patch).
    pub(crate) fn patch(&self) -> Wedge {
        let (theta_min, theta_max) = self.real_theta_degrees();
        let wedge = bbox_to_unit(&self.wedge_bbox());
        let center = wedge.transform_point(Point::new(0.5, 0.5));
        let edge = wedge.transform_point(Point::new(1.0, 0.0)).x;
        let radius = edge - center.x;
        let (rmin, rmax) = self.radial_extent();
        Wedge {
            width: Some((radius * (rmax - rmin) / rmax).min(radius)),
            ..Wedge::new(center, radius, theta_min, theta_max)
        }
    }

    /// Whether matplotlib's `draw` shows the spine on `side`: the outer
    /// spine always, the start and end spines of sectors and the inner
    /// spine around a hole.
    pub(crate) fn spine_shown(&self, side: SpineSide) -> bool {
        match side {
            SpineSide::Left | SpineSide::Right => !self.is_full_circle(),
            SpineSide::Bottom => {
                let patch = self.patch();
                patch.r - patch.width.unwrap_or(patch.r) != 0.0
            }
            SpineSide::Top => true,
        }
    }

    /// The outer (`Top`) or inner (`Bottom`) arc spine spanning θ
    /// `bounds`, as a path in wedge coordinates and its display affine
    /// (matplotlib's arc `Spine._adjust_location`).
    pub(crate) fn arc_spine(
        &self,
        side: SpineSide,
        bounds: (f64, f64),
        display: &Bbox,
    ) -> (Path, Affine2D) {
        let shift = |theta: f64| theta * self.direction() + self.settings.theta_offset;
        let (low, high) = (shift(bounds.0), shift(bounds.1));
        let (low, high) = if low > high { (high, low) } else { (low, high) };
        let path = Path::arc(low.to_degrees(), high.to_degrees(), None);
        let diameter = if side == SpineSide::Bottom {
            let (rmin, rmax) = (self.scaled(self.view.y0()), self.scaled(self.view.y1()));
            let origin = self.scaled(self.rorigin());
            (rmin - origin) / (rmax - origin)
        } else {
            1.0
        };
        let placement = Affine2D::scaling(diameter * 0.5, diameter * 0.5).translate(0.5, 0.5);
        (path, placement.then(self.wedge_to_display(display)))
    }

    /// From `(fraction of the θ view, r)` to data (matplotlib's
    /// `blended_transform_factory(BboxTransformTo(viewLim), Identity)`).
    pub(crate) fn theta_fraction(&self) -> Affine2D {
        Affine2D::from_values(self.view.width(), 0.0, 0.0, 1.0, self.view.x0(), 0.0)
    }

    /// From `(θ, fraction of the r view)` to data (matplotlib's
    /// `blended_transform_factory(Identity, BboxTransformTo(viewLim))`).
    pub(crate) fn radius_fraction(&self) -> Affine2D {
        Affine2D::from_values(1.0, 0.0, 0.0, self.view.height(), 0.0, self.view.y0())
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::{FRAC_PI_2, PI};

    use super::*;

    fn full_view() -> Bbox {
        Bbox::from_extents(0.0, 0.0, TAU, 2.0)
    }

    #[test]
    fn full_circle_maps_onto_axes_square() {
        let settings = PolarState::default();
        let view = PolarView::new(&settings, full_view(), None);
        let wedge = view.wedge_bbox();
        assert!((wedge.x0()).abs() < 1e-12 && (wedge.x1() - 1.0).abs() < 1e-12);
        let display = Bbox::from_extents(0.0, 0.0, 100.0, 100.0);
        let data = view.data_transform(&display);
        let east = data.transform_point(Point::new(0.0, 2.0));
        assert!((east.x - 100.0).abs() < 1e-9 && (east.y - 50.0).abs() < 1e-9);
        let north = data.transform_point(Point::new(FRAC_PI_2, 1.0));
        assert!((north.x - 50.0).abs() < 1e-9 && (north.y - 75.0).abs() < 1e-9);
    }

    #[test]
    fn negative_radii_vanish_and_inverse_round_trips() {
        let settings = PolarState {
            rorigin: Some(0.5),
            ..PolarState::default()
        };
        let map = PolarView::new(&settings, full_view(), None).map();
        assert!(map.forward(Point::new(1.0, 0.25)).x.is_nan());
        let p = map.forward(Point::new(1.0, 1.5));
        let back = map.inverse(p);
        assert!((back.x - 1.0).abs() < 1e-12 && (back.y - 1.5).abs() < 1e-12);
    }

    #[test]
    fn constant_radius_segments_become_arcs() {
        let settings = PolarState::default();
        let map = PolarView::new(&settings, full_view(), None).map();
        let ring = Path::polyline(vec![Point::new(0.0, 1.0), Point::new(PI, 1.0)])
            .with_interpolation_steps(180);
        let arc = map.transform_path(&ring);
        assert_eq!(arc.code_at(1), PathCode::Curve4);
        let end = arc.vertices()[arc.len() - 1];
        assert!((end.x + 1.0).abs() < 1e-12 && end.y.abs() < 1e-12);
        let spoke = Path::polyline(vec![Point::new(1.0, 0.0), Point::new(1.0, 2.0)])
            .with_interpolation_steps(180);
        assert_eq!(map.transform_path(&spoke).len(), 2);
        let spiral = Path::polyline(vec![Point::new(0.0, 0.0), Point::new(1.0, 2.0)])
            .with_interpolation_steps(4);
        assert_eq!(map.transform_path(&spiral).len(), 5);
    }

    #[test]
    fn sector_wedge_is_squared_and_clockwise_shifts() {
        let settings = PolarState {
            direction: super::super::ThetaDirection::Clockwise,
            theta_offset: FRAC_PI_2,
            ..PolarState::default()
        };
        let view = PolarView::new(
            &settings,
            Bbox::from_extents(0.0, 0.0, FRAC_PI_2, 1.0),
            None,
        );
        let real = view.real_view();
        assert!((real.x0()).abs() < 1e-12 && (real.x1() - FRAC_PI_2).abs() < 1e-12);
        let wedge = view.wedge_bbox();
        assert!((wedge.width() - wedge.height()).abs() < 1e-12);
        assert!(!view.is_full_circle());
        assert!(view.spine_shown(SpineSide::Left));
    }
}
