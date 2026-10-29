//! The shapes region selectors draw (matplotlib's `_init_shape`,
//! `_draw_shape` and `_rect_bbox` of `RectangleSelector` and
//! `EllipseSelector`).

use std::fmt;

use pltrs_core::Point;

use crate::patches::{Ellipse, PatchShape, Rectangle, RotationPoint};
use crate::widgets::ordered;

/// `(xmin, xmax, ymin, ymax)` in data coordinates.
pub(super) type Extents = (f64, f64, f64, f64);

/// A patch shape a region selector draws (matplotlib's `_init_shape`,
/// `_draw_shape` and `_rect_bbox`).
pub(super) trait RegionShape: PatchShape + Clone + fmt::Debug {
    /// The hidden shape a new selector starts with.
    fn initial() -> Self;
    /// Spans `extents`, rotated by `angle` degrees, within the view
    /// `(xlim, ylim)`.
    fn draw(&mut self, extents: Extents, angle: f64, view: ((f64, f64), (f64, f64)));
    /// `(x0, y0, width, height)` of the unrotated shape.
    fn rect_bbox(&self) -> (f64, f64, f64, f64);
    /// Sets the rotation correction for unequal axis scales.
    fn set_aspect_ratio_correction(&mut self, correction: f64);
}

impl RegionShape for Rectangle {
    fn initial() -> Self {
        Rectangle::new((0.0, 0.0), 0.0, 1.0).rotated(0.0, RotationPoint::Center)
    }

    /// The rectangle is clipped to the view.
    fn draw(
        &mut self,
        (x0, x1, y0, y1): Extents,
        angle: f64,
        (xlim, ylim): ((f64, f64), (f64, f64)),
    ) {
        let ((xmin, xmax), (ymin, ymax)) = (ordered((x0, x1)), ordered((y0, y1)));
        let (xlim, ylim) = (ordered(xlim), ordered(ylim));
        let (xmin, ymin) = (xlim.0.max(xmin), ylim.0.max(ymin));
        let (xmax, ymax) = (xmax.min(xlim.1), ymax.min(ylim.1));
        self.xy = Point::new(xmin, ymin);
        self.width = xmax - xmin;
        self.height = ymax - ymin;
        self.angle = angle;
    }

    fn rect_bbox(&self) -> (f64, f64, f64, f64) {
        let (x0, y0) = (self.xy.x, self.xy.y);
        // Like matplotlib's `get_bbox().bounds`, through the far corner.
        (x0, y0, (x0 + self.width) - x0, (y0 + self.height) - y0)
    }

    fn set_aspect_ratio_correction(&mut self, correction: f64) {
        self.aspect_ratio_correction = correction;
    }
}

impl RegionShape for Ellipse {
    fn initial() -> Self {
        Ellipse::new((0.0, 0.0), 0.0, 1.0)
    }

    fn draw(&mut self, (x0, x1, y0, y1): Extents, angle: f64, _view: ((f64, f64), (f64, f64))) {
        let ((xmin, xmax), (ymin, ymax)) = (ordered((x0, x1)), ordered((y0, y1)));
        self.center = Point::new(x0 + (x1 - x0) / 2.0, y0 + (y1 - y0) / 2.0);
        self.width = 2.0 * ((xmax - xmin) / 2.0);
        self.height = 2.0 * ((ymax - ymin) / 2.0);
        self.angle = angle;
    }

    fn rect_bbox(&self) -> (f64, f64, f64, f64) {
        let Point { x, y } = self.center;
        (
            x - self.width / 2.0,
            y - self.height / 2.0,
            self.width,
            self.height,
        )
    }

    fn set_aspect_ratio_correction(&mut self, correction: f64) {
        self.aspect_ratio_correction = correction;
    }
}
