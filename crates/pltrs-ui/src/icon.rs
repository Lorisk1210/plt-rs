//! Vector icons: outlines on a square design grid, painted in one colour
//! at any size.

use pltrs_core::Rgba;
use pltrs_core::{Affine2D, Path};

use pltrs_raster::{Painter, Pixmap};

use crate::canvas::Canvas;
use crate::error::UiError;
use crate::geometry::Rect;

/// A monochrome vector icon drawn on a `grid × grid` design square (y
/// down, like window pixels): stroked lines and filled shapes.
///
/// ```
/// use pltrs_core::{Path, Point};
/// use pltrs_ui::Icon;
/// let minus = Icon::new(24.0).stroke(Path::polyline(vec![
///     Point::new(5.0, 12.0),
///     Point::new(19.0, 12.0),
/// ]));
/// assert_eq!(minus.grid(), 24.0);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Icon {
    grid: f64,
    stroke_width: f64,
    strokes: Vec<Path>,
    fills: Vec<Path>,
}

impl Icon {
    /// An empty icon on a `grid`-unit square with strokes 1.75 units wide.
    pub fn new(grid: f64) -> Self {
        Self {
            grid,
            stroke_width: 1.75,
            strokes: Vec::new(),
            fills: Vec::new(),
        }
    }

    /// The side of the design square.
    pub fn grid(&self) -> f64 {
        self.grid
    }

    /// Adds a line drawn with round caps and joins.
    #[must_use]
    pub fn stroke(mut self, path: Path) -> Self {
        self.strokes.push(path);
        self
    }

    /// Adds a filled shape.
    #[must_use]
    pub fn fill(mut self, path: Path) -> Self {
        self.fills.push(path);
        self
    }

    /// Sets the width of the strokes in design units.
    #[must_use]
    pub fn with_stroke_width(mut self, width: f64) -> Self {
        self.stroke_width = width;
        self
    }

    /// The icon drawn in `color` over a `size`-pixel square of `background`
    /// with rounded corners — an application icon — as straight RGBA bytes,
    /// row by row.
    ///
    /// # Errors
    ///
    /// When `size` is too large for an image.
    pub fn app_icon(&self, size: u32, background: Rgba, color: Rgba) -> Result<Vec<u8>, UiError> {
        let mut pixmap = Pixmap::new(size, size)?;
        let mut painter = Painter::new();
        let side = f64::from(size);
        let rect = Rect::new(0.0, 0.0, side, side);
        let mut canvas = Canvas::new(&mut pixmap, &mut painter, rect, 1.0);
        canvas.fill_rounded_rect(rect, side * 0.22, background)?;
        self.paint(&mut canvas, rect.inset(side * 0.14), color)?;
        Ok(pixmap.data().to_vec())
    }

    /// The icon drawn in `color` on a transparent `size`-pixel square, as
    /// straight RGBA bytes, row by row (a toolbar image).
    ///
    /// # Errors
    ///
    /// When `size` is too large for an image.
    pub fn render(&self, size: u32, color: Rgba) -> Result<Vec<u8>, UiError> {
        let mut pixmap = Pixmap::new(size, size)?;
        let mut painter = Painter::new();
        let side = f64::from(size);
        let rect = Rect::new(0.0, 0.0, side, side);
        let mut canvas = Canvas::new(&mut pixmap, &mut painter, rect, 1.0);
        self.paint(&mut canvas, rect, color)?;
        Ok(pixmap.data().to_vec())
    }

    /// Paints the icon in `color`, scaled to the largest square centred in
    /// `rect`.
    pub fn paint(&self, canvas: &mut Canvas<'_>, rect: Rect, color: Rgba) -> Result<(), UiError> {
        let side = rect.width.min(rect.height);
        let scale = side / self.grid;
        let center = rect.center();
        let place =
            Affine2D::scaling(scale, scale).translate(center.x - side / 2.0, center.y - side / 2.0);
        for shape in &self.fills {
            canvas.fill_path(&shape.transformed(&place), color)?;
        }
        for line in &self.strokes {
            canvas.stroke_path(&line.transformed(&place), self.stroke_width * scale, color)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_icons_are_opaque_squares_with_rounded_corners() {
        let icon = Icon::new(24.0).stroke(Path::polyline(vec![
            pltrs_core::Point::new(4.0, 12.0),
            pltrs_core::Point::new(20.0, 12.0),
        ]));
        let rgba = icon
            .app_icon(32, Rgba::new(0.2, 0.4, 0.9, 1.0), Rgba::WHITE)
            .unwrap();
        assert_eq!(rgba.len(), 32 * 32 * 4);
        let alpha = |x: usize, y: usize| rgba[(y * 32 + x) * 4 + 3];
        assert_eq!(alpha(0, 0), 0, "rounded corner");
        assert_eq!(alpha(16, 4), 255);
        let center = (16 * 32 + 16) * 4;
        assert!(rgba[center] > 200, "the stroke is white");
    }

    #[test]
    fn rendered_icons_are_strokes_on_transparency() {
        let icon = Icon::new(24.0).stroke(Path::polyline(vec![
            pltrs_core::Point::new(4.0, 12.0),
            pltrs_core::Point::new(20.0, 12.0),
        ]));
        let rgba = icon.render(24, Rgba::new(0.0, 0.0, 1.0, 1.0)).unwrap();
        assert_eq!(rgba.len(), 24 * 24 * 4);
        let pixel = |x: usize, y: usize| &rgba[(y * 24 + x) * 4..(y * 24 + x) * 4 + 4];
        assert_eq!(pixel(12, 2)[3], 0, "transparent off the stroke");
        let on = pixel(12, 12);
        assert_eq!(on[2], 255, "straight blue");
        assert!(on[3] > 200, "mostly covered");
    }
}
