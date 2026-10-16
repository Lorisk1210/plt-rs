//! Data limits and automatic view limits (matplotlib's `dataLim`,
//! `update_from_path` and `autoscale_view`).

use plt_core::{Bbox, Point};

use super::axis::AxisState;
use crate::ticker::{AxisInfo, TickContext, TickerError};

/// The extent of the data plotted in an axes, plus the smallest positive
/// coordinate per axis (needed to autoscale logarithmic axes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DataLimits {
    /// Bounding box of all finite data points.
    pub bbox: Bbox,
    /// Smallest strictly positive x and y seen (infinite when none).
    pub minpos: Point,
}

impl Default for DataLimits {
    fn default() -> Self {
        Self {
            bbox: Bbox::NULL,
            minpos: Point::new(f64::INFINITY, f64::INFINITY),
        }
    }
}

impl DataLimits {
    /// Extends the limits with `points`, considering only points whose two
    /// coordinates are finite, and only the requested axes
    /// (matplotlib's `Bbox.update_from_path`).
    pub fn update(&mut self, points: &[Point], update_x: bool, update_y: bool) {
        let finite = points.iter().filter(|p| p.is_finite());
        for p in finite {
            if update_x {
                self.bbox.p0.x = self.bbox.p0.x.min(p.x);
                self.bbox.p1.x = self.bbox.p1.x.max(p.x);
                if p.x > 0.0 {
                    self.minpos.x = self.minpos.x.min(p.x);
                }
            }
            if update_y {
                self.bbox.p0.y = self.bbox.p0.y.min(p.y);
                self.bbox.p1.y = self.bbox.p1.y.max(p.y);
                if p.y > 0.0 {
                    self.minpos.y = self.minpos.y.min(p.y);
                }
            }
        }
    }
}

/// Everything needed to autoscale one axis of an axes.
#[derive(Debug)]
pub(crate) struct AxisAutoscale<'a> {
    /// The axis (scale and major locator).
    pub(crate) axis: &'a AxisState,
    /// What the locator may know about the axis.
    pub(crate) info: &'a AxisInfo,
    /// Margin as a fraction of the data span.
    pub(crate) margin: f64,
    /// Sorted sticky values the margins must not cross.
    pub(crate) stickies: &'a [f64],
    /// Skip the locator's `view_limits` rounding.
    pub(crate) tight: bool,
}

/// matplotlib's `handle_single_axis` inside `autoscale_view`: the new
/// `(lower, upper)` view interval for one axis.
pub(crate) fn autoscale_axis(request: &AxisAutoscale<'_>) -> Result<(f64, f64), TickerError> {
    let axis = request.axis;
    let ctx = TickContext::new(request.info, &axis.settings);
    let (data0, data1) = request.info.data_interval;
    let (mut x0, mut x1) = axis.major_locator.nonsingular(data0, data1, &ctx);

    // Prevent margins from crossing sticky values (with a relative
    // tolerance for floating-point noise).
    let tol = 1e-5 * (x1 - x0).abs();
    let x0_bound = request
        .stickies
        .iter()
        .rev()
        .find(|&&s| s < x0 + tol)
        .copied();
    let x1_bound = request.stickies.iter().find(|&&s| s > x1 - tol).copied();

    // Add the margins in scaled space, then map back.
    (x0, x1) = axis
        .scale()
        .limit_range_for_scale(x0, x1, request.info.minpos);
    let inverse = |v: f64| axis.map().map_or(v, |m| m.inverse(v));
    let (t0, t1) = (axis.forward(x0), axis.forward(x1));
    let delta = (t1 - t0) * request.margin;
    let delta = if delta.is_finite() { delta } else { 0.0 };
    x0 = inverse(t0 - delta);
    x1 = inverse(t1 + delta);

    if let Some(bound) = x0_bound {
        x0 = x0.max(bound);
    }
    if let Some(bound) = x1_bound {
        x1 = x1.min(bound);
    }
    if request.tight {
        Ok((x0, x1))
    } else {
        axis.major_locator.view_limits(x0, x1, &ctx)
    }
}

/// Orders `(lower, upper)` to match the current orientation of an axis
/// (matplotlib's `set_xbound` keeps inverted axes inverted).
pub(crate) fn oriented(bound: (f64, f64), inverted: bool) -> (f64, f64) {
    let (lo, hi) = if bound.0 <= bound.1 {
        bound
    } else {
        (bound.1, bound.0)
    };
    if inverted { (hi, lo) } else { (lo, hi) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rc::RcParams;
    use crate::ticker::AxisName;

    fn linear(data: (f64, f64), margin: f64, stickies: &[f64]) -> (f64, f64) {
        let axis = AxisState::new(AxisName::X, &RcParams::matplotlib());
        let info = AxisInfo {
            data_interval: data,
            ..AxisInfo::DUMMY
        };
        autoscale_axis(&AxisAutoscale {
            axis: &axis,
            info: &info,
            margin,
            stickies,
            tight: false,
        })
        .expect("linear locators do not fail")
    }

    #[test]
    fn data_limits_skip_partial_nans() {
        let mut limits = DataLimits::default();
        limits.update(
            &[
                Point::new(1.0, 2.0),
                Point::new(f64::NAN, 10.0),
                Point::new(-3.0, -1.0),
            ],
            true,
            true,
        );
        assert_eq!(limits.bbox, Bbox::from_extents(-3.0, -1.0, 1.0, 2.0));
        assert_eq!(limits.minpos, Point::new(1.0, 2.0));
        limits.update(&[Point::new(100.0, 100.0)], false, true);
        assert_eq!(limits.bbox.x1(), 1.0, "x untouched");
        assert_eq!(limits.bbox.y1(), 100.0);
    }

    #[test]
    fn margins_like_matplotlib() {
        // ax.plot([1, 2, 3, 4], ...) -> xlim (0.85, 4.15)
        let (lo, hi) = linear((1.0, 4.0), 0.05, &[]);
        assert!((lo - 0.85).abs() < 1e-12 && (hi - 4.15).abs() < 1e-12);
    }

    #[test]
    fn sticky_edges_stop_margins() {
        // imshow/bar-like stickies at the data edges keep the view tight.
        assert_eq!(linear((0.0, 10.0), 0.05, &[0.0, 10.0]), (0.0, 10.0));
        let (lo, hi) = linear((0.0, 10.0), 0.05, &[0.0]);
        assert_eq!(lo, 0.0);
        assert!((hi - 10.5).abs() < 1e-12);
    }

    #[test]
    fn empty_data_gives_default_interval() {
        let (lo, hi) = linear((f64::INFINITY, f64::NEG_INFINITY), 0.05, &[]);
        assert!((lo + 0.055).abs() < 1e-12 && (hi - 0.055).abs() < 1e-12);
    }

    #[test]
    fn orientation_is_preserved() {
        assert_eq!(oriented((1.0, 2.0), true), (2.0, 1.0));
        assert_eq!(oriented((2.0, 1.0), false), (1.0, 2.0));
    }
}
