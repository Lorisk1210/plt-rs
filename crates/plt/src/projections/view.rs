//! What the hooks of a projection see of their axes: the view limits and
//! the scales.

use plt_core::Bbox;

use crate::axes::axis::AxisState;
use crate::ticker::AxisName;
use crate::transforms::{ResolvedTransform, Step, bbox_to_unit};

/// The view limits and scales of an axes, which the transforms of a
/// [`Projection`](super::Projection) are built from.
#[derive(Clone, Copy, Debug)]
pub struct ProjectionView<'a> {
    limits: Bbox,
    xaxis: &'a AxisState,
    yaxis: &'a AxisState,
}

impl<'a> ProjectionView<'a> {
    /// The view of axes with view limits `limits` and axes `xaxis` and
    /// `yaxis`.
    pub(crate) fn new(limits: Bbox, xaxis: &'a AxisState, yaxis: &'a AxisState) -> Self {
        Self {
            limits,
            xaxis,
            yaxis,
        }
    }

    /// The view limits in data coordinates (matplotlib's `viewLim`); an
    /// inverted axis has its limits the other way round.
    pub fn limits(&self) -> Bbox {
        self.limits
    }

    /// The view interval `(v0, v1)` of `axis`.
    pub fn interval(&self, axis: AxisName) -> (f64, f64) {
        match axis {
            AxisName::X => self.limits.interval_x(),
            AxisName::Y => self.limits.interval_y(),
        }
    }

    /// The data → axes-fraction transform of rectilinear axes with these
    /// limits and scales (matplotlib's `transScale + transLimits`).
    pub fn rectilinear(&self) -> ResolvedTransform {
        let (x0, x1) = scaled_interval(self.xaxis, self.limits.interval_x());
        let (y0, y1) = scaled_interval(self.yaxis, self.limits.interval_y());
        let to_unit = bbox_to_unit(&Bbox::from_extents(x0, y0, x1, y1));
        let (x_map, y_map) = (self.xaxis.map().cloned(), self.yaxis.map().cloned());
        if x_map.is_none() && y_map.is_none() {
            ResolvedTransform::affine(to_unit)
        } else {
            ResolvedTransform::with_step(Step::Separable(x_map, y_map), to_unit)
        }
    }
}

/// The view interval `(v0, v1)` of `axis` in scaled coordinates. Like
/// matplotlib's `TransformedBbox` of the view limits, it keeps the
/// orientation of the data interval: a decreasing scale (a function scale
/// such as `1 / x`) leaves the axis the way round its limits say.
fn scaled_interval(axis: &AxisState, (v0, v1): (f64, f64)) -> (f64, f64) {
    let (a, b) = (axis.forward(v0), axis.forward(v1));
    let (low, high) = (a.min(b), a.max(b));
    if v0 > v1 { (high, low) } else { (low, high) }
}
