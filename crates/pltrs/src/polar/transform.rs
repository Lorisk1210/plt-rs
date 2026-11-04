//! The polar projection: (θ, r) onto the axes circle through the
//! [`PolarMap`] and the scaling of the wedge (matplotlib's `PolarAffine`
//! and `_WedgeBbox`), plus the geometry of the background wedge and the
//! spines derived from it.

use std::f64::consts::TAU;
use std::rc::Rc;

use pltrs_core::numeric::np_sign;
use pltrs_core::{Affine2D, Bbox, Path, Point};

use super::map::PolarMap;
use super::{PolarFrame, PolarState};
use crate::patches::{PatchShape, Wedge};
use crate::projections::{Projection, ProjectionView};
use crate::spines::SpineSide;
use crate::transforms::{AxisMap, ResolvedTransform, Step, bbox_to_unit, unit_to_bbox};

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

/// The polar geometry of one axes at its current view limits.
pub(crate) struct PolarView<'a> {
    settings: &'a PolarState,
    /// The view limits: θ along x, r along y.
    view: Bbox,
    /// The radial scale, if not linear.
    rmap: Option<&'a Rc<dyn AxisMap>>,
    /// The projection of the program the axes map data through, with what
    /// its hooks see of the axes.
    program: Option<(&'a dyn Projection, ProjectionView<'a>)>,
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
            program: None,
        }
    }

    /// The geometry of polar axes mapping data through `program`, a
    /// projection of the program whose hooks see `projection_view`.
    pub(crate) fn with_program(
        self,
        program: &'a dyn Projection,
        projection_view: ProjectionView<'a>,
    ) -> Self {
        Self {
            program: Some((program, projection_view)),
            ..self
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
        np_sign(self.view.y1() - self.rorigin())
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
            polygon: self.polygon(),
        }
    }

    /// The vertices of a polygon frame.
    fn polygon(&self) -> Option<usize> {
        match self.settings.frame {
            PolarFrame::Polygon(vertices) => Some(vertices),
            PolarFrame::Circle | PolarFrame::Path(_) => None,
        }
    }

    /// The outline of the background in axes coordinates: the wedge, the
    /// polygon of a polygon frame inscribed in the axes square, or the
    /// path of a frame of the program's.
    pub(crate) fn patch_path(&self) -> Path {
        match &self.settings.frame {
            PolarFrame::Circle => self.patch().path(),
            PolarFrame::Polygon(vertices) => inscribed_polygon(*vertices),
            PolarFrame::Path(path) => path.clone(),
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
    /// (matplotlib's `transData`); through the axes box, the transform of
    /// the program's projection the axes may map data through.
    pub(crate) fn data_transform(&self, display: &Bbox) -> ResolvedTransform {
        if let Some((program, view)) = &self.program {
            return program
                .data_to_axes(view)
                .then(ResolvedTransform::affine(unit_to_bbox(display)));
        }
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
        match (&self.settings.frame, side) {
            (_, SpineSide::Top) => return true,
            // matplotlib's `draw` only shows an inner spine around the hole
            // of a wedge.
            (PolarFrame::Polygon(_), _) | (PolarFrame::Path(_), SpineSide::Bottom) => {
                return false;
            }
            _ => {}
        }
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
    /// (matplotlib's arc `Spine._adjust_location`); the polygon of a
    /// polygon frame in axes coordinates.
    pub(crate) fn arc_spine(
        &self,
        side: SpineSide,
        bounds: (f64, f64),
        display: &Bbox,
    ) -> (Path, Affine2D) {
        if let Some(vertices) = self.polygon() {
            return (inscribed_polygon(vertices), unit_to_bbox(display));
        }
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

/// The regular polygon with `vertices` vertices inscribed in the axes
/// square, the first at the top (the radar example's
/// `RegularPolygon((0.5, 0.5), n, radius=0.5)`).
fn inscribed_polygon(vertices: usize) -> Path {
    Path::unit_regular_polygon(vertices)
        .transformed(&Affine2D::scaling(0.5, 0.5).translate(0.5, 0.5))
}

#[cfg(test)]
mod tests {
    use std::f64::consts::{FRAC_PI_2, PI};

    use pltrs_core::PathCode;

    use super::*;
    use crate::transforms::PointMap;

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
    fn polygon_frames_straighten_what_follows_the_circle() {
        let settings = PolarState {
            frame: PolarFrame::Polygon(4),
            ..PolarState::default()
        };
        let view = PolarView::new(&settings, full_view(), None);
        let ring = Path::polyline(vec![Point::new(0.0, 1.0), Point::new(TAU, 1.0)])
            .with_interpolation_steps(180);
        let square = view.map().transform_path(&ring);
        assert_eq!(square.len(), 5, "one piece between vertex directions");
        let corner = square.vertices()[1];
        assert!(corner.x.abs() < 1e-12 && (corner.y - 1.0).abs() < 1e-12);
        let line = Path::polyline(vec![Point::new(0.0, 1.0), Point::new(1.0, 2.0)]);
        assert_eq!(
            view.map().transform_path(&line).len(),
            2,
            "plain lines stay straight"
        );
        let outline = view.patch_path();
        let top = outline.vertices()[0];
        assert!((top.x - 0.5).abs() < 1e-12 && (top.y - 1.0).abs() < 1e-12);
        assert_eq!(outline.len(), 5);
        assert!(view.spine_shown(SpineSide::Top));
        assert!(!view.spine_shown(SpineSide::Bottom));
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
