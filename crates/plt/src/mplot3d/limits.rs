//! Data and view limits of a 3-D axes and their autoscaling (matplotlib's
//! `xy_dataLim`, `zz_dataLim`, `auto_scale_xyz`, `autoscale_view` and
//! `_set_lim3d`).

use super::axis3d::AxisSpan;
use super::coords::Axis3;
use super::error::Axes3DError;
use super::state::Axes3DState;
use crate::rc::RcParams;

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
    automargin: bool,
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

    /// The view limits `[xmin, xmax, ymin, ymax, zmin, zmax]` as set,
    /// inverted axes keeping their order (linear scales leave them
    /// unchanged; matplotlib's `_get_scaled_limits`).
    pub(crate) fn scaled(&self) -> [f64; 6] {
        let [(x0, x1), (y0, y1), (z0, z1)] = self.view;
        [x0, x1, y0, y1, z0, z1]
    }

    /// The limits tick helpers see for one axis.
    pub(crate) fn span(&self, axis: Axis3) -> AxisSpan {
        let (data, minpos) = match axis {
            Axis3::X => ((self.xy.lo[0], self.xy.hi[0]), self.xy.minpos[0]),
            Axis3::Y => ((self.xy.lo[1], self.xy.hi[1]), self.xy.minpos[1]),
            Axis3::Z => ((self.zz.lo[0], self.zz.hi[0]), self.zz.minpos[0]),
        };
        AxisSpan {
            view: self.view(axis),
            data,
            minpos,
        }
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
    /// Includes data in the data limits and autoscales (matplotlib's
    /// `auto_scale_xyz`).
    pub(crate) fn auto_scale_xyz(
        &mut self,
        x: &[f64],
        y: &[f64],
        z: Option<&[f64]>,
        had_data: bool,
    ) -> Result<(), Axes3DError> {
        self.limits.include(x, y, z, had_data);
        self.autoscale_view()
    }

    /// Fits the view of every autoscaled axis to the data (matplotlib's
    /// `autoscale_view`).
    pub(crate) fn autoscale_view(&mut self) -> Result<(), Axes3DError> {
        for axis in Axis3::ALL {
            if self.limits.autoscale[axis.index()] {
                self.autoscale_axis(axis)?;
            }
        }
        Ok(())
    }

    /// matplotlib's `_autoscale_axis` on a linear axis.
    fn autoscale_axis(&mut self, axis: Axis3) -> Result<(), Axes3DError> {
        let span = self.limits.span(axis);
        let ticker = &self.axes[axis.index()];
        let (mut v0, mut v1) = ticker.nonsingular(&span, span.data.0, span.data.1);
        let margin = self.limits.margins[axis.index()];
        if margin > 0.0 {
            let delta = (v1 - v0) * margin;
            let delta = if delta.is_finite() { delta } else { 0.0 };
            (v0, v1) = (v0 - delta, v1 + delta);
        }
        if !self.limits.tight {
            (v0, v1) = ticker
                .view_limits(&span, v0, v1)
                .map_err(Axes3DError::Ticker)?;
        }
        self.set_bound(axis, v0, v1, Some(VIEW_MARGIN))
    }

    /// Sets the bounds of one axis keeping its orientation (matplotlib's
    /// `_set_bound3d`).
    fn set_bound(
        &mut self,
        axis: Axis3,
        lower: f64,
        upper: f64,
        view_margin: Option<f64>,
    ) -> Result<(), Axes3DError> {
        let (lo, hi) = (lower.min(upper), lower.max(upper));
        let (a, b) = if self.limits.inverted(axis) {
            (hi, lo)
        } else {
            (lo, hi)
        };
        self.set_lim3d(axis, a, b, None, view_margin)
    }

    /// Sets the view limits of one axis, adding `view_margin` (by default
    /// only with `axes3d.automargin`), and turns autoscaling on or off if
    /// `auto` says so (matplotlib's `_set_lim3d`).
    pub(crate) fn set_lim3d(
        &mut self,
        axis: Axis3,
        lower: f64,
        upper: f64,
        auto: Option<bool>,
        view_margin: Option<f64>,
    ) -> Result<(), Axes3DError> {
        if !(lower.is_finite() && upper.is_finite()) {
            return Err(Axes3DError::NonFiniteLimits(lower, upper));
        }
        let margin = view_margin.unwrap_or(if self.limits.automargin {
            VIEW_MARGIN
        } else {
            0.0
        });
        let delta = (upper - lower) * margin;
        let (lower, upper) = if delta.is_finite() {
            (lower - delta, upper + delta)
        } else {
            (lower, upper)
        };
        self.set_lim(axis, lower, upper, auto);
        Ok(())
    }

    /// matplotlib's `Axis._set_lim`: singular limits are expanded and
    /// `v0 > v1` inverts the axis.
    fn set_lim(&mut self, axis: Axis3, v0: f64, v1: f64, auto: Option<bool>) {
        let span = self.limits.span(axis);
        let (a, b) = self.axes[axis.index()].nonsingular(&span, v0, v1);
        let (lo, hi) = (a.min(b), a.max(b));
        self.limits.view[axis.index()] = if v0 > v1 { (hi, lo) } else { (lo, hi) };
        if let Some(auto) = auto {
            self.limits.autoscale[axis.index()] = auto;
        }
    }

    /// Flips the direction of one axis without changing its autoscaling
    /// (matplotlib's `invert_xaxis`).
    pub(crate) fn invert(&mut self, axis: Axis3) {
        let (a, b) = self.limits.view(axis);
        self.set_lim(axis, b, a, None);
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
}
