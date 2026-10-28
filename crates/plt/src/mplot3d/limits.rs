//! Data and view limits of a 3-D axes and their autoscaling (matplotlib's
//! `xy_dataLim`, `zz_dataLim`, `auto_scale_xyz`, `autoscale_view`,
//! `_set_bound3d`, `_set_lim3d`, `Axis._set_lim` and
//! `_set_lims_from_transformed`).

use super::axis3d::AxisSpan;
use super::coords::Axis3;
use super::error::Axes3DError;
use super::math::Vec3;
use super::plot::columns;
use super::share::LimitChange;
use super::state::Axes3DState;
use crate::rc::RcParams;
use crate::scale::ScaleName;

/// Padding added on both sides of autoscaled limits, as a fraction of
/// their span (matplotlib's `_view_margin`, matching matplotlib 3.8).
const VIEW_MARGIN: f64 = 1.0 / 48.0;

/// Initial x and y data limits, inset so that the default view matches
/// matplotlib 3.8.
const XY_MARGIN: f64 = 0.05 * 10.0 / 11.0;

/// The extent of data along two directions and their smallest positive
/// values (matplotlib's `Bbox` used as a data-limits box).
#[derive(Clone, Copy, Debug, PartialEq)]
struct DataBox {
    lo: [f64; 2],
    hi: [f64; 2],
    minpos: [f64; 2],
}

impl DataBox {
    fn new(lo: f64, hi: f64) -> Self {
        Self {
            lo: [lo; 2],
            hi: [hi; 2],
            minpos: [f64::INFINITY; 2],
        }
    }

    /// Includes the finite `points` in the directions selected by
    /// `update`, starting afresh when `ignore` is set (matplotlib's
    /// `update_from_path`); no points leave the box unchanged.
    fn update(&mut self, points: &[[f64; 2]], ignore: bool, update: [bool; 2]) {
        if points.is_empty() {
            return;
        }
        let mut next = if ignore {
            Self {
                lo: [f64::INFINITY; 2],
                hi: [f64::NEG_INFINITY; 2],
                minpos: [f64::INFINITY; 2],
            }
        } else {
            *self
        };
        for point in points.iter().filter(|p| p.iter().all(|c| c.is_finite())) {
            for (i, &value) in point.iter().enumerate() {
                next.lo[i] = next.lo[i].min(value);
                next.hi[i] = next.hi[i].max(value);
                if value > 0.0 {
                    next.minpos[i] = next.minpos[i].min(value);
                }
            }
        }
        for i in (0..2).filter(|&i| update[i]) {
            self.lo[i] = next.lo[i];
            self.hi[i] = next.hi[i];
            self.minpos[i] = next.minpos[i];
        }
    }
}

/// The limits of the three axes.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Limits3D {
    /// x and y data limits.
    xy: DataBox,
    /// z data limits (in the first direction).
    zz: DataBox,
    /// View intervals of x, y and z; decreasing when inverted.
    view: [(f64, f64); 3],
    /// Whether each axis follows the data.
    pub(crate) autoscale: [bool; 3],
    /// Autoscaling margins as fractions of the data span.
    pub(crate) margins: [f64; 3],
    /// Whether autoscaling skips the locators' view-limit rounding.
    pub(crate) tight: bool,
    /// Whether explicit limits get the view margin (`axes3d.automargin`).
    pub(crate) automargin: bool,
}

impl Limits3D {
    /// The initial limits; `zmargin` is the z autoscaling margin.
    pub(crate) fn new(zmargin: f64, rc: &RcParams) -> Self {
        Self {
            xy: DataBox::new(XY_MARGIN, 1.0 - XY_MARGIN),
            zz: DataBox::new(0.0, 1.0),
            view: [(0.0, 1.0); 3],
            autoscale: [true; 3],
            margins: [rc.axes_xmargin, rc.axes_ymargin, zmargin],
            tight: false,
            automargin: rc.axes3d_automargin,
        }
    }

    /// The view interval of one axis.
    pub(crate) fn view(&self, axis: Axis3) -> (f64, f64) {
        self.view[axis.index()]
    }

    /// The view interval of one axis in increasing order (matplotlib's
    /// `get_xbound`).
    pub(crate) fn bound(&self, axis: Axis3) -> (f64, f64) {
        let (a, b) = self.view(axis);
        if a <= b { (a, b) } else { (b, a) }
    }

    /// Whether the axis runs from high to low values.
    pub(crate) fn inverted(&self, axis: Axis3) -> bool {
        let (a, b) = self.view(axis);
        b < a
    }

    /// The smallest positive datum along one axis (matplotlib's
    /// `xy_dataLim.minposx` and friends).
    pub(crate) fn minpos(&self, axis: Axis3) -> f64 {
        match axis {
            Axis3::X => self.xy.minpos[0],
            Axis3::Y => self.xy.minpos[1],
            Axis3::Z => self.zz.minpos[0],
        }
    }

    /// The limits tick helpers see for one axis.
    pub(crate) fn span(&self, axis: Axis3) -> AxisSpan {
        let data = match axis {
            Axis3::X => (self.xy.lo[0], self.xy.hi[0]),
            Axis3::Y => (self.xy.lo[1], self.xy.hi[1]),
            Axis3::Z => (self.zz.lo[0], self.zz.hi[0]),
        };
        AxisSpan {
            view: self.view(axis),
            data,
            minpos: self.minpos(axis),
        }
    }

    /// Whether the z data limits are still the initial unit interval
    /// (matplotlib's `np.array_equal(zz_dataLim.get_points(), [[0, 0],
    /// [1, 1]])`).
    #[expect(
        clippy::float_cmp,
        reason = "matplotlib compares against the exact initial limits"
    )]
    pub(crate) fn z_data_is_initial(&self) -> bool {
        self.zz.lo == [0.0; 2] && self.zz.hi == [1.0; 2]
    }

    /// Gives z the initial data limits and margin of x, which a
    /// non-linear z scale needs to autoscale to positive limits
    /// (matplotlib's `_set_axis_scale` on the z axis).
    pub(crate) fn use_xy_initial_z(&mut self) {
        self.zz = DataBox::new(XY_MARGIN, 1.0 - XY_MARGIN);
        self.margins[2] = self.margins[0];
    }

    /// Includes data in the data limits, replacing them unless
    /// `had_data` (the data part of matplotlib's `auto_scale_xyz`): `x`
    /// and `y` of the same length are paired into points.
    fn include(&mut self, x: &[f64], y: &[f64], z: Option<&[f64]>, had_data: bool) {
        let ignore = !had_data;
        if x.len() == y.len() {
            let points: Vec<[f64; 2]> = x.iter().zip(y).map(|(&x, &y)| [x, y]).collect();
            self.xy.update(&points, ignore, [true, true]);
        } else {
            let alone =
                |values: &[f64]| -> Vec<[f64; 2]> { values.iter().map(|&v| [v, 1.0]).collect() };
            self.xy.update(&alone(x), ignore, [true, false]);
            let ys: Vec<[f64; 2]> = y.iter().map(|&v| [1.0, v]).collect();
            self.xy.update(&ys, ignore, [false, true]);
        }
        if let Some(z) = z {
            let points: Vec<[f64; 2]> = z.iter().map(|&v| [v, 1.0]).collect();
            self.zz.update(&points, ignore, [true, false]);
        }
    }
}

impl Axes3DState {
    /// The view limits `[xmin, xmax, ymin, ymax, zmin, zmax]` in
    /// transformed coordinates, inverted axes keeping their order
    /// (matplotlib's `_get_scaled_limits`).
    pub(crate) fn scaled_limits(&self) -> [f64; 6] {
        let [(x0, x1), (y0, y1), (z0, z1)] = Axis3::ALL.map(|axis| {
            let (lo, hi) = self.limits.view(axis);
            let scale = &self.axis(axis).scale;
            (scale.forward(lo), scale.forward(hi))
        });
        [x0, x1, y0, y1, z0, z1]
    }

    /// Includes data in the data limits and autoscales (matplotlib's
    /// `auto_scale_xyz`); values outside an axis' scale are ignored.
    pub(crate) fn auto_scale_xyz(
        &mut self,
        x: &[f64],
        y: &[f64],
        z: Option<&[f64]>,
        had_data: bool,
    ) -> Result<(), Axes3DError> {
        self.include_data(x, y, z, had_data);
        self.autoscale_view()
    }

    /// Includes data in the data limits, replacing them unless
    /// `had_data`; values outside an axis' scale are ignored.
    fn include_data(&mut self, x: &[f64], y: &[f64], z: Option<&[f64]>, had_data: bool) {
        let masked = |axis: Axis3, values: &[f64]| -> Vec<f64> {
            let scale = &self.axis(axis).scale;
            values.iter().map(|&v| scale.masked(v)).collect()
        };
        let (x, y) = (masked(Axis3::X, x), masked(Axis3::Y, y));
        let z = z.map(|z| masked(Axis3::Z, z));
        self.limits.include(&x, &y, z.as_deref(), had_data);
    }

    /// Recomputes the data limits from the geometry of the data artists
    /// (matplotlib's `relim`, which a change of unit calls); without data
    /// artists they stay as they are.
    pub(crate) fn relim(&mut self) {
        let points: Vec<Vec3> = (self.artists.iter())
            .filter(|artist| artist.art.is_data())
            .flat_map(|artist| artist.art.points())
            .collect();
        if points.is_empty() {
            return;
        }
        let [x, y, z] = columns(&points);
        self.include_data(&x, &y, Some(&z), false);
    }

    /// Fits the view of every autoscaled axis to the data (matplotlib's
    /// `autoscale_view`).
    pub(crate) fn autoscale_view(&mut self) -> Result<(), Axes3DError> {
        self.autoscale_axes([true; 3])
    }

    /// Fits the view of the selected autoscaled axes to the data.
    pub(crate) fn autoscale_axes(&mut self, selected: [bool; 3]) -> Result<(), Axes3DError> {
        for axis in Axis3::ALL {
            if selected[axis.index()] && self.limits.autoscale[axis.index()] {
                self.autoscale_axis(axis)?;
            }
        }
        Ok(())
    }

    /// matplotlib's `_autoscale_axis`: the data limits made non-singular
    /// and valid for the scale, widened by the margin in transformed
    /// coordinates and rounded by the locator unless tight.
    fn autoscale_axis(&mut self, axis: Axis3) -> Result<(), Axes3DError> {
        let span = self.limits.span(axis);
        let state = self.axis(axis);
        let (v0, v1) = state.nonsingular(&span, span.data.0, span.data.1);
        let (mut v0, mut v1) = state.scale.limit_range(v0, v1, span.minpos);
        let margin = self.limits.margins[axis.index()];
        if margin > 0.0 {
            (v0, v1) = state.scale.widened((v0, v1), margin);
        }
        if !self.limits.tight {
            (v0, v1) = state
                .view_limits(&span, v0, v1)
                .map_err(Axes3DError::Ticker)?;
        }
        self.set_bound(axis, v0, v1, Some(VIEW_MARGIN))
    }

    /// Sets the bounds of one axis keeping its orientation and its
    /// autoscaling (matplotlib's `_set_bound3d`).
    pub(crate) fn set_bound(
        &mut self,
        axis: Axis3,
        lower: f64,
        upper: f64,
        view_margin: Option<f64>,
    ) -> Result<(), Axes3DError> {
        if !(lower.is_finite() && upper.is_finite()) {
            return Err(Axes3DError::NonFiniteLimits(lower, upper));
        }
        let (lo, hi) = (lower.min(upper), lower.max(upper));
        let (a, b) = if self.limits.inverted(axis) {
            (hi, lo)
        } else {
            (lo, hi)
        };
        self.set_lim3d(axis, a, b, None, view_margin)
    }

    /// Sets the view limits of one axis, widened by `view_margin` (by
    /// default only with `axes3d.automargin`) in transformed coordinates,
    /// turns autoscaling on or off if `auto` says so, and lets the axes
    /// sharing the axis follow (matplotlib's `_set_lim3d`).
    pub(crate) fn set_lim3d(
        &mut self,
        axis: Axis3,
        lower: f64,
        upper: f64,
        auto: Option<bool>,
        view_margin: Option<f64>,
    ) -> Result<(), Axes3DError> {
        self.set_lim3d_emitting(axis, (lower, upper), auto, view_margin, true)
    }

    /// [`Axes3DState::set_lim3d`], letting the sharing axes follow only
    /// with `emit`.
    pub(crate) fn set_lim3d_emitting(
        &mut self,
        axis: Axis3,
        (lower, upper): (f64, f64),
        auto: Option<bool>,
        view_margin: Option<f64>,
        emit: bool,
    ) -> Result<(), Axes3DError> {
        if !(lower.is_finite() && upper.is_finite()) {
            return Err(Axes3DError::NonFiniteLimits(lower, upper));
        }
        let margin = view_margin.unwrap_or(if self.limits.automargin {
            VIEW_MARGIN
        } else {
            0.0
        });
        let (lower, upper) = if margin > 0.0 {
            let scale = &self.axis(axis).scale;
            let limited = scale.limit_range(lower, upper, self.limits.minpos(axis));
            scale.widened(limited, margin)
        } else {
            let delta = (upper - lower) * margin;
            if delta.is_finite() {
                (lower - delta, upper + delta)
            } else {
                (lower, upper)
            }
        };
        if auto != Some(true) {
            self.aspect.forget_restore_limits(axis);
        }
        self.set_lim(axis, lower, upper, auto, emit);
        Ok(())
    }

    /// matplotlib's `Axis._set_lim`: non-positive limits of a log axis are
    /// ignored, singular limits are expanded, limits are kept in the
    /// scale's domain, and `v0 > v1` inverts the axis. With `emit` the
    /// axes sharing the axis follow.
    pub(crate) fn set_lim(
        &mut self,
        axis: Axis3,
        v0: f64,
        v1: f64,
        auto: Option<bool>,
        emit: bool,
    ) {
        let (old0, old1) = self.limits.view(axis);
        let state = self.axis(axis);
        let (v0, v1) = if state.scale.name() == ScaleName::Log {
            (
                if v0 <= 0.0 { old0 } else { v0 },
                if v1 <= 0.0 { old1 } else { v1 },
            )
        } else {
            (v0, v1)
        };
        let span = self.limits.span(axis);
        let (a, b) = state.nonsingular(&span, v0, v1);
        let (a, b) = state.scale.limit_range(a, b, f64::INFINITY);
        let (lo, hi) = (a.min(b), a.max(b));
        let view = if v0 > v1 { (hi, lo) } else { (lo, hi) };
        self.limits.view[axis.index()] = view;
        if let Some(auto) = auto {
            self.limits.autoscale[axis.index()] = auto;
        }
        if emit {
            self.emitted.push(LimitChange { axis, view, auto });
        }
    }

    /// Makes one axis run from high to low values or back, keeping its
    /// range and autoscaling (matplotlib's `Axis.set_inverted`).
    pub(crate) fn set_inverted(&mut self, axis: Axis3, inverted: bool) {
        let (lo, hi) = self.limits.bound(axis);
        let (a, b) = if inverted { (hi, lo) } else { (lo, hi) };
        self.set_lim(axis, a, b, None, true);
    }

    /// Widens the view of one axis to include `[low, high]` keeping its
    /// orientation, without margins or autoscaling changes (matplotlib's
    /// `set_view_interval` without `ignore`).
    pub(crate) fn include_in_view(&mut self, axis: Axis3, low: f64, high: f64) {
        let (old0, old1) = self.limits.view(axis);
        self.limits.view[axis.index()] = if old0 < old1 {
            (low.min(high).min(old0), low.max(high).max(old1))
        } else {
            (low.max(high).max(old0), low.min(high).min(old1))
        };
    }

    /// Sets the limits of the three axes from limits in transformed
    /// coordinates, kept in each scale's domain, leaving their autoscaling
    /// alone (matplotlib's `_set_lims_from_transformed`).
    pub(crate) fn set_lims_from_transformed(
        &mut self,
        limits: [(f64, f64); 3],
    ) -> Result<(), Axes3DError> {
        let data = Axis3::ALL.map(|axis| {
            let (t0, t1) = limits[axis.index()];
            let scale = &self.axis(axis).scale;
            let (lo, hi) = (scale.inverse(t0), scale.inverse(t1));
            scale.limit_range(lo, hi, self.limits.minpos(axis))
        });
        // Checked first so that a failure leaves every axis unchanged.
        if let Some(&(lo, hi)) = data
            .iter()
            .find(|(lo, hi)| !(lo.is_finite() && hi.is_finite()))
        {
            return Err(Axes3DError::NonFiniteLimits(lo, hi));
        }
        Axis3::ALL
            .into_iter()
            .zip(data)
            .try_for_each(|(axis, (lo, hi))| self.set_lim3d(axis, lo, hi, None, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_box_ignores_or_extends() {
        let mut data = DataBox::new(0.0, 1.0);
        data.update(&[[2.0, -3.0], [f64::NAN, 9.0]], false, [true, true]);
        assert_eq!((data.lo, data.hi), ([0.0, -3.0], [2.0, 1.0]));
        assert_eq!(data.minpos, [2.0, f64::INFINITY]);
        data.update(&[[5.0, 6.0]], true, [true, false]);
        assert_eq!((data.lo, data.hi), ([5.0, -3.0], [5.0, 1.0]));
        data.update(&[], true, [true, true]);
        assert_eq!(data.lo, [5.0, -3.0], "no points change nothing");
    }

    #[test]
    fn data_is_paired_or_taken_separately() {
        let rc = RcParams::matplotlib();
        let mut limits = Limits3D::new(0.0, &rc);
        limits.include(&[1.0, 2.0], &[3.0, 4.0], Some(&[5.0, 6.0]), false);
        assert_eq!(limits.span(Axis3::X).data, (1.0, 2.0));
        assert_eq!(limits.span(Axis3::Y).data, (3.0, 4.0));
        assert_eq!(limits.span(Axis3::Z).data, (5.0, 6.0));
        limits.include(&[0.0], &[3.0, 10.0], None, true);
        assert_eq!(limits.span(Axis3::X).data, (0.0, 2.0));
        assert_eq!(limits.span(Axis3::Y).data, (3.0, 10.0));
    }

    #[test]
    fn non_linear_z_scales_start_from_the_xy_limits() {
        let rc = RcParams::matplotlib();
        let mut limits = Limits3D::new(0.0, &rc);
        assert!(limits.z_data_is_initial());
        limits.use_xy_initial_z();
        assert!(!limits.z_data_is_initial());
        assert_eq!(limits.span(Axis3::Z).data, limits.span(Axis3::X).data);
        assert_eq!(limits.margins[2], rc.axes_xmargin);
    }
}
