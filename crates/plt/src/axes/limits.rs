//! Data limits and automatic view limits (matplotlib's `dataLim`,
//! `update_from_path` and `autoscale_view`).

use std::rc::Rc;

use plt_core::{Bbox, Point};

use crate::transforms::{AxisMap, nonsingular};

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
pub(crate) struct AxisAutoscale<'a> {
    /// Data interval `(min, max)`; non-finite when there is no data.
    pub(crate) data: (f64, f64),
    /// Smallest positive data value.
    pub(crate) minpos: f64,
    /// Margin as a fraction of the data span.
    pub(crate) margin: f64,
    /// Sorted sticky values the margins must not cross.
    pub(crate) stickies: &'a [f64],
    /// The axis scale (`None` = linear).
    pub(crate) scale: Option<&'a Rc<dyn AxisMap>>,
    /// The scale's `limit_range_for_scale(vmin, vmax, minpos)`.
    pub(crate) limit_range: &'a dyn Fn(f64, f64, f64) -> (f64, f64),
    /// Skip the locator's `view_limits` rounding.
    pub(crate) tight: bool,
}

impl std::fmt::Debug for AxisAutoscale<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AxisAutoscale")
            .field("data", &self.data)
            .field("minpos", &self.minpos)
            .field("margin", &self.margin)
            .field("stickies", &self.stickies)
            .field("tight", &self.tight)
            .finish_non_exhaustive()
    }
}

/// matplotlib's `handle_single_axis` inside `autoscale_view`: the new
/// `(lower, upper)` view interval for one axis.
pub(crate) fn autoscale_axis(axis: &AxisAutoscale<'_>) -> (f64, f64) {
    let (mut x0, mut x1) = nonsingular(axis.data.0, axis.data.1, 0.05, 1e-15, true);

    // Prevent margins from crossing sticky values (with a relative
    // tolerance for floating-point noise).
    let tol = 1e-5 * (x1 - x0).abs();
    let x0_bound = axis.stickies.iter().rev().find(|&&s| s < x0 + tol).copied();
    let x1_bound = axis.stickies.iter().find(|&&s| s > x1 - tol).copied();

    // Add the margins in scaled space, then map back.
    let forward = |v: f64| axis.scale.map_or(v, |m| m.forward(v));
    let inverse = |v: f64| axis.scale.map_or(v, |m| m.inverse(v));
    (x0, x1) = (axis.limit_range)(x0, x1, axis.minpos);
    let (t0, t1) = (forward(x0), forward(x1));
    let delta = (t1 - t0) * axis.margin;
    let delta = if delta.is_finite() { delta } else { 0.0 };
    x0 = inverse(t0 - delta);
    x1 = inverse(t1 + delta);

    if let Some(bound) = x0_bound {
        x0 = x0.max(bound);
    }
    if let Some(bound) = x1_bound {
        x1 = x1.min(bound);
    }
    if axis.tight {
        (x0, x1)
    } else {
        nonsingular(x0, x1, 0.001, 1e-15, true)
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

    fn linear(data: (f64, f64), margin: f64, stickies: &[f64]) -> (f64, f64) {
        autoscale_axis(&AxisAutoscale {
            data,
            minpos: f64::INFINITY,
            margin,
            stickies,
            scale: None,
            limit_range: &|lo, hi, _| (lo, hi),
            tight: false,
        })
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
