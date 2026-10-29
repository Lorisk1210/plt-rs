//! The scale of one 3-D axis as the projection sees it: data values are
//! mapped to transformed coordinates before they are projected, and
//! values outside the scale's domain are masked (matplotlib 3.11's
//! `axis.get_transform()` in `_apply_scale_transforms` and
//! `_scale_invalid_mask`).

use std::rc::Rc;

use crate::scale::{LinearScale, Scale, ScaleName};
use crate::transforms::AxisMap;

/// A scale and its data-to-transformed map, shared by the axes sharing an
/// axis.
#[derive(Clone, Debug)]
pub(crate) struct AxisScale {
    scale: Rc<dyn Scale>,
    /// `None` for linear-looking scales.
    map: Option<Rc<dyn AxisMap>>,
}

impl Default for AxisScale {
    /// The linear scale.
    fn default() -> Self {
        Self::new(Rc::new(LinearScale))
    }
}

impl AxisScale {
    /// The transform of `scale`.
    pub(crate) fn new(scale: Rc<dyn Scale>) -> Self {
        let map = scale.axis_map();
        Self { scale, map }
    }

    /// The scale object.
    pub(crate) fn scale(&self) -> &Rc<dyn Scale> {
        &self.scale
    }

    /// The name of the scale.
    pub(crate) fn name(&self) -> ScaleName {
        self.scale.name()
    }

    /// Maps a data value to transformed coordinates.
    pub(crate) fn forward(&self, value: f64) -> f64 {
        self.map.as_ref().map_or(value, |m| m.forward(value))
    }

    /// Maps a transformed coordinate back to data.
    pub(crate) fn inverse(&self, value: f64) -> f64 {
        self.map.as_ref().map_or(value, |m| m.inverse(value))
    }

    /// Whether `value` can be shown: finite and in the scale's domain
    /// (matplotlib's `val_in_range`).
    pub(crate) fn contains(&self, value: f64) -> bool {
        self.scale.val_in_range(value)
    }

    /// `value` in transformed coordinates, or NaN when it cannot be shown
    /// (a masked point of matplotlib's 3-D artists).
    pub(crate) fn masked_forward(&self, value: f64) -> f64 {
        if self.contains(value) {
            self.forward(value)
        } else {
            f64::NAN
        }
    }

    /// `value`, or NaN when it cannot be shown (matplotlib's
    /// `_nan_out_of_scale_range`).
    pub(crate) fn masked(&self, value: f64) -> f64 {
        if self.contains(value) {
            value
        } else {
            f64::NAN
        }
    }

    /// `[vmin, vmax]` restricted to the scale's domain; `minpos` is the
    /// smallest positive datum (matplotlib's `limit_range_for_scale`).
    pub(crate) fn limit_range(&self, vmin: f64, vmax: f64, minpos: f64) -> (f64, f64) {
        self.scale.limit_range_for_scale(vmin, vmax, minpos)
    }

    /// The interval `(v0, v1)` widened by `fraction` of its span on both
    /// sides in transformed coordinates, then mapped back; an infinite
    /// widening leaves it unchanged.
    pub(crate) fn widened(&self, (v0, v1): (f64, f64), fraction: f64) -> (f64, f64) {
        let (t0, t1) = (self.forward(v0), self.forward(v1));
        let delta = (t1 - t0) * fraction;
        if delta.is_finite() {
            (self.inverse(t0 - delta), self.inverse(t1 + delta))
        } else {
            (v0, v1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::LogScale;

    fn log() -> AxisScale {
        AxisScale::new(Rc::new(LogScale::new(10.0).expect("valid base")))
    }

    #[test]
    fn linear_scales_only_mask_non_finite_values() {
        let linear = AxisScale::default();
        assert_eq!(linear.name(), ScaleName::Linear);
        assert_eq!(linear.forward(-3.0), -3.0);
        assert!(linear.masked_forward(f64::INFINITY).is_nan());
        assert_eq!(linear.widened((0.0, 10.0), 0.1), (-1.0, 11.0));
    }

    #[test]
    fn log_scales_transform_and_mask() {
        let log = log();
        assert!((log.forward(1000.0) - 3.0).abs() < 1e-12);
        assert!(log.masked_forward(-1.0).is_nan());
        assert!(log.masked(0.0).is_nan());
        assert_eq!(log.limit_range(-1.0, 10.0, 0.5), (0.5, 10.0));
        let (lo, hi) = log.widened((1.0, 100.0), 0.5);
        assert!((lo - 0.1).abs() < 1e-12 && (hi - 1000.0).abs() < 1e-9);
    }
}
