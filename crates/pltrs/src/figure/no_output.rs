//! Drawing a figure without output (matplotlib's
//! `Figure.draw_without_rendering`): every artist goes through its draw,
//! so the figure ends up as drawn and the problems drawing meets are
//! reported, but nothing is painted.

use pltrs_core::{Affine2D, Path, Point, Rgba};
use pltrs_render::{GraphicsContext, PathCollection, QuadMesh, Renderer, RgbaImage};
use pltrs_text::TextRun;
use pltrs_text::tex::{TexError, TexText};

use super::{DrawErrors, Figure};

/// A renderer of a canvas's size and resolution that paints nothing
/// (matplotlib's renderer with `_draw_disabled`).
#[derive(Clone, Copy, Debug)]
struct NoOutput {
    size: (f64, f64),
    dpi: f64,
}

impl Renderer for NoOutput {
    fn canvas_size(&self) -> (f64, f64) {
        self.size
    }

    fn dpi(&self) -> f64 {
        self.dpi
    }

    fn draw_path(&mut self, _: &GraphicsContext, _: &Path, _: &Affine2D, _: Option<Rgba>) {}

    fn draw_markers(
        &mut self,
        _: &GraphicsContext,
        _: &Path,
        _: &Affine2D,
        _: &Path,
        _: &Affine2D,
        _: Option<Rgba>,
    ) {
    }

    fn draw_path_collection(&mut self, _: &GraphicsContext, _: &PathCollection<'_>) {}

    fn draw_quad_mesh(&mut self, _: &GraphicsContext, _: &QuadMesh<'_>) {}

    fn draw_gouraud_triangles(
        &mut self,
        _: &GraphicsContext,
        _: &[[Point; 3]],
        _: &[[Rgba; 3]],
        _: &Affine2D,
    ) {
    }

    fn draw_image(&mut self, _: &GraphicsContext, _: Point, _: &RgbaImage, _: Option<&Affine2D>) {}

    fn draw_text(&mut self, _: &GraphicsContext, _: &TextRun, _: Point, _: f64) {}

    fn draw_tex(
        &mut self,
        _: &GraphicsContext,
        _: &TextRun,
        _: &TexText,
        _: Point,
        _: f64,
    ) -> Result<(), TexError> {
        Ok(())
    }
}

impl Figure {
    /// Draws the figure without any output (matplotlib's
    /// `draw_without_rendering`): it is laid out, every artist is drawn
    /// as for a canvas and counts as drawn, and the draw event is sent,
    /// but nothing is painted. Useful to get the final size of artists
    /// that require a draw before their size is known.
    ///
    /// # Errors
    ///
    /// The problems met while drawing, as [`Figure::draw`] reports them.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// ax.plot([0.0, 1.0], [0.0, 1.0])?;
    /// fig.draw_without_rendering()?;
    /// assert!(!fig.is_stale() && !ax.stale());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn draw_without_rendering(&self) -> Result<(), DrawErrors> {
        let mut renderer = NoOutput {
            size: self.size_pixels(),
            dpi: self.dpi(),
        };
        self.draw(&mut renderer)
    }
}

#[cfg(test)]
mod tests {
    use crate::artist::DrawErrorKind;
    use crate::figure::Figure;

    #[test]
    fn drawing_without_output_reports_what_drawing_meets() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.text(0.0, 0.0, r"$\wrong{math}$");
        let errors = fig.draw_without_rendering().expect_err("malformed math");
        assert_eq!(errors.0.len(), 1);
        assert_eq!(errors.0[0].kind, DrawErrorKind::InvalidValue);
        assert!(errors.0[0].message.contains("Unknown symbol"), "{errors}");
        assert!(!fig.is_stale());
    }
}
