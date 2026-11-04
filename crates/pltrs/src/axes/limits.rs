//! Automatic view limits (matplotlib's `autoscale_view`).

use super::axis::AxisState;
use crate::ticker::{AxisInfo, TickContext, TickerError};

/// Everything needed to autoscale one axis of an axes.
#[derive(Debug)]
pub(crate) struct AxisAutoscale<'a> {
    /// The axis (scale and major locator).
    pub(crate) axis: &'a AxisState,
    /// What the locator may know about the axis.
    pub(crate) info: &'a AxisInfo,
    /// Smallest positive datum over the axes sharing this axis.
    pub(crate) minpos: f64,
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
    let settings = axis.settings();
    let ctx = TickContext::new(request.info, &settings);
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

    // Add the margins in scaled space, then map back, each way once for
    // both ends (matplotlib transforms them as one array).
    (x0, x1) = axis.scale().limit_range_for_scale(x0, x1, request.minpos);
    let mut ends = [x0, x1];
    axis.forward_column(&mut ends);
    let [t0, t1] = ends;
    let delta = (t1 - t0) * request.margin;
    let delta = if delta.is_finite() { delta } else { 0.0 };
    let mut ends = [t0 - delta, t1 + delta];
    axis.inverse_column(&mut ends);
    [x0, x1] = ends;

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
            minpos: info.minpos,
            margin,
            stickies,
            tight: false,
        })
        .expect("linear locators do not fail")
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
