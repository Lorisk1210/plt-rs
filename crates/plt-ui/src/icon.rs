//! Vector icons: outlines on a square design grid, painted in one colour
//! at any size.

use plt_core::Rgba;
use plt_core::{Affine2D, Path};

use crate::canvas::Canvas;
use crate::error::UiError;
use crate::geometry::Rect;

/// A monochrome vector icon drawn on a `grid × grid` design square (y
/// down, like window pixels): stroked lines and filled shapes.
///
/// ```
/// use plt_core::{Path, Point};
/// use plt_ui::Icon;
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
