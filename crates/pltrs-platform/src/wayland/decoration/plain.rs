//! The built-in title bar painter, used until a toolkit installs its own:
//! a flat bar with a close and a maximize button, and no title text (the
//! platform has no fonts).

use crate::canvas::Canvas;
use crate::decoration::{DecorationPainter, TitleBar, TitleBarButton};
use crate::geometry::PhysicalRect;

/// The bar of a focused window.
const BAR: [u8; 4] = [235, 235, 235, 255];
/// The bar of a window without focus.
const BAR_INACTIVE: [u8; 4] = [246, 246, 246, 255];
/// The line under the bar.
const SEPARATOR: [u8; 4] = [208, 208, 208, 255];
/// A hovered button's backdrop.
const HOVER: [u8; 4] = [216, 216, 216, 255];
/// A pressed button's backdrop.
const PRESSED: [u8; 4] = [196, 196, 196, 255];
/// Button symbols of a focused window.
const SYMBOL: [u8; 4] = [46, 46, 46, 255];
/// Button symbols of a window without focus.
const SYMBOL_INACTIVE: [u8; 4] = [138, 138, 138, 255];

/// Draws a plain title bar.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct PlainPainter;

impl DecorationPainter for PlainPainter {
    fn paint(&self, bar: &TitleBar<'_>, pixels: &mut [u8]) {
        let mut canvas = Canvas::new(pixels, bar.size.width);
        let (width, height) = (f64::from(bar.size.width), f64::from(bar.size.height));
        let scale = bar.scale_factor;
        let background = if bar.focused { BAR } else { BAR_INACTIVE };
        canvas.fill_rect((0.0, 0.0), (width, height), background);
        canvas.fill_rect((0.0, height - scale), (width, height), SEPARATOR);
        let symbol = if bar.focused { SYMBOL } else { SYMBOL_INACTIVE };
        for (button, rect) in bar.buttons() {
            let backdrop = if bar.pressed == Some(button) {
                Some(PRESSED)
            } else if bar.hovered == Some(button) {
                Some(HOVER)
            } else {
                None
            };
            let glyph = Glyph::new(rect, scale);
            if let Some(color) = backdrop {
                glyph.backdrop(&mut canvas, color);
            }
            match button {
                TitleBarButton::Close => glyph.close(&mut canvas, symbol),
                TitleBarButton::Maximize if bar.maximized => glyph.restore(&mut canvas, symbol),
                TitleBarButton::Maximize => glyph.maximize(&mut canvas, symbol),
            }
        }
    }
}

/// Where a button's symbol goes: centred in its square, 10 logical pixels
/// wide.
struct Glyph {
    centre: (f64, f64),
    half: f64,
    stroke: f64,
    scale: f64,
}

impl Glyph {
    fn new(rect: PhysicalRect, scale: f64) -> Self {
        Self {
            centre: (
                f64::from(rect.x) + f64::from(rect.width) / 2.0,
                f64::from(rect.y) + f64::from(rect.height) / 2.0,
            ),
            half: 5.0 * scale,
            stroke: (1.5 * scale).max(1.0),
            scale,
        }
    }

    /// A rounded-off square behind the symbol.
    fn backdrop(&self, canvas: &mut Canvas<'_>, color: [u8; 4]) {
        let r = self.half + 6.0 * self.scale;
        let (x, y) = self.centre;
        canvas.fill_rect((x - r, y - r), (x + r, y + r), color);
    }

    fn close(&self, canvas: &mut Canvas<'_>, color: [u8; 4]) {
        let ((x, y), h) = (self.centre, self.half * 0.85);
        canvas.line((x - h, y - h), (x + h, y + h), self.stroke, color);
        canvas.line((x - h, y + h), (x + h, y - h), self.stroke, color);
    }

    fn maximize(&self, canvas: &mut Canvas<'_>, color: [u8; 4]) {
        let ((x, y), h) = (self.centre, self.half);
        self.square(canvas, (x - h, y - h), (x + h, y + h), color);
    }

    /// Two overlapping squares: the window is maximized.
    fn restore(&self, canvas: &mut Canvas<'_>, color: [u8; 4]) {
        let ((x, y), h) = (self.centre, self.half);
        let offset = 2.5 * self.scale;
        self.square(
            canvas,
            (x - h + offset, y - h),
            (x + h, y + h - offset),
            color,
        );
        self.square(
            canvas,
            (x - h, y - h + offset),
            (x + h - offset, y + h),
            color,
        );
    }

    fn square(
        &self,
        canvas: &mut Canvas<'_>,
        (x0, y0): (f64, f64),
        (x1, y1): (f64, f64),
        color: [u8; 4],
    ) {
        let s = self.stroke / 2.0;
        canvas.line((x0 - s, y0), (x1 + s, y0), self.stroke, color);
        canvas.line((x0 - s, y1), (x1 + s, y1), self.stroke, color);
        canvas.line((x0, y0), (x0, y1), self.stroke, color);
        canvas.line((x1, y0), (x1, y1), self.stroke, color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::PhysicalSize;

    fn bar(focused: bool, hovered: Option<TitleBarButton>) -> TitleBar<'static> {
        TitleBar {
            title: "plain",
            size: PhysicalSize::new(120, 34),
            scale_factor: 1.0,
            focused,
            maximized: false,
            hovered,
            pressed: None,
        }
    }

    fn paint(bar: &TitleBar<'_>) -> Vec<u8> {
        let mut pixels = vec![0; bar.size.width as usize * bar.size.height as usize * 4];
        PlainPainter.paint(bar, &mut pixels);
        pixels
    }

    #[test]
    fn paints_an_opaque_bar_with_symbols() {
        let focused = paint(&bar(true, None));
        assert!(focused.chunks(4).all(|px| px[3] == 255), "opaque");
        assert_eq!(&focused[..4], BAR);
        let close_centre = (17 * 120 + 103) * 4;
        assert_ne!(
            &focused[close_centre..close_centre + 4],
            BAR,
            "the × crosses there"
        );
        let unfocused = paint(&bar(false, None));
        assert_eq!(&unfocused[..4], BAR_INACTIVE);
        let hovered = paint(&bar(true, Some(TitleBarButton::Close)));
        let corner = (8 * 120 + 95) * 4;
        assert_eq!(&hovered[corner..corner + 4], HOVER);
    }
}
