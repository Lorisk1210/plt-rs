//! `RangeFrameAxes`: rectilinear axes whose data keeps a pad, given when
//! the axes is made, off every edge; the right button of the pan/zoom
//! tool changes the pad, and the zoom-to-rectangle tool zooms into a box
//! along x alone.

use pltrs::axes::{AxisName, ProjectionKind};
use pltrs::navigation::{DragKey, PointerButton, ZoomDirection};
use pltrs::projections::{
    self, PanStart, Projection, ProjectionError, ProjectionView, ViewLimits, ZoomBox,
};
use pltrs::transforms::{Affine2D, Point, ResolvedTransform};

/// How many display pixels a drag with the right button moves to change
/// the pad by one (`PAD_PIXELS`).
const PAD_PIXELS: f64 = 1000.0;

/// The largest pad a drag reaches (`MAX_PAD`).
const MAX_PAD: f64 = 0.4;

/// The keywords `RangeFrameAxes` is made with.
#[derive(Debug, PartialEq)]
pub(crate) struct RangeFrameOptions {
    /// The fraction of the axes the data keeps off every edge (`pad`).
    pub(crate) pad: f64,
}

impl Default for RangeFrameOptions {
    fn default() -> Self {
        Self { pad: 0.05 }
    }
}

/// The cases' `RangeFrameAxes`, registered as `range_frame`.
#[derive(Clone, Debug)]
pub(crate) struct RangeFrame {
    /// The fraction of the axes the data keeps off every edge.
    pub(crate) pad: f64,
    /// The pad when a drag with the right button started (`_pad_pan`).
    pad_pan: Option<f64>,
}

impl RangeFrame {
    /// `RangeFrameAxes.__init__`.
    fn new(options: &RangeFrameOptions) -> Self {
        Self {
            pad: options.pad,
            pad_pan: None,
        }
    }

    /// `_inset`: the unit square onto the part of the axes the data
    /// keeps to.
    fn inset(&self) -> Affine2D {
        let scale = 1.0 - 2.0 * self.pad;
        Affine2D::scaling(scale, scale).translate(self.pad, self.pad)
    }
}

/// Registers `RangeFrameAxes` (`register_projection(RangeFrameAxes)`).
pub(crate) fn register() -> Result<ProjectionKind, ProjectionError> {
    projections::register_with(RangeFrame::new)
}

impl Projection for RangeFrame {
    fn name(&self) -> &'static str {
        "range_frame"
    }

    /// `transScale + transLimits + _inset`; the gridlines, ticks and
    /// spines blend it with the axes fractions, as `_xaxis_transform` and
    /// `_yaxis_transform` do.
    fn data_to_axes(&self, view: &ProjectionView<'_>) -> ResolvedTransform {
        view.rectilinear()
            .then(ResolvedTransform::affine(self.inset()))
    }

    /// `start_pan`: the right button changes the pad, the left one pans.
    fn start_pan(&mut self, start: &PanStart) -> bool {
        let own = start.button() == PointerButton::Right;
        self.pad_pan = own.then_some(self.pad);
        own
    }

    /// `drag_pan`: the pad follows the pointer across.
    fn drag_pan(&mut self, start: &PanStart, _: DragKey, at: Point) -> ViewLimits {
        if let Some(pad) = self.pad_pan {
            self.pad = (pad + (at.x - start.at().x) / PAD_PIXELS).clamp(0.0, MAX_PAD);
        }
        ViewLimits::default()
    }

    /// `end_pan`.
    fn end_pan(&mut self) {
        self.pad_pan = None;
    }

    /// `_set_view_from_bbox`: a box dragged to zoom in sets the x limits
    /// alone; the wheel and zooming out zoom as rectilinear axes do.
    fn zoom_to_box(&mut self, zoom: &ZoomBox) -> Option<ViewLimits> {
        if zoom.wheel().is_some() || zoom.direction() == ZoomDirection::Out {
            return None;
        }
        if !zoom.changes(AxisName::X) {
            return Some(ViewLimits::default());
        }
        let corners = zoom.corners();
        let to_data = zoom.view().to_data();
        let a = to_data.transform_point(Point::new(corners.x0(), corners.y0()));
        let b = to_data.transform_point(Point::new(corners.x1(), corners.y1()));
        Some(ViewLimits {
            x: Some((a.x.min(b.x), a.x.max(b.x))),
            y: None,
        })
    }
}
