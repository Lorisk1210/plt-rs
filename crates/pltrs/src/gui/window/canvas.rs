//! The figure in its window: fitting it to the canvas area, drawing it
//! and presenting what changed on its canvas.

use pltrs_platform::{Context, PhysicalRect};
use pltrs_raster::Pixmap;
use pltrs_ui::{Surface, UiWindow, WidgetId};

use super::FigureWindow;
use crate::events::Event;
use crate::figure::{CanvasError, Figure, PixelBox, Presentation};
use crate::gui::GuiError;

impl FigureWindow {
    /// Gives the figure the size of the canvas area at the window's scale
    /// factor, its device pixel ratio (which scales the resolution the
    /// figure was made at); returns whether its size in pixels or its
    /// resolution changed.
    pub(super) fn fit_figure(&mut self) -> bool {
        let size = self.canvas_size;
        if size.width == 0 || size.height == 0 {
            return false;
        }
        let before = (self.figure.width_height(), self.figure.dpi());
        // A degenerate scale factor leaves the resolution alone, and at a
        // degenerate resolution the figure keeps its size.
        let _ = self
            .figure
            .canvas()
            .set_device_pixel_ratio(self.window.ui().scale_factor());
        let dpi = self.figure.dpi();
        let _ = self
            .figure
            .set_size_inches(f64::from(size.width) / dpi, f64::from(size.height) / dpi);
        before.0 != self.figure.width_height() || before.1.total_cmp(&dpi).is_ne()
    }

    /// Tells the figure's callbacks the size of the canvas area
    /// (matplotlib's `resize_event`).
    pub(super) fn announce_size(&self) {
        self.figure.process_event(&Event::Resize {
            width: f64::from(self.canvas_size.width),
            height: f64::from(self.canvas_size.height),
        });
    }

    /// Draws the figure onto its canvas at the window's size and
    /// resolution and shows it; artists that cannot be drawn are skipped
    /// and the first problem is shown in the status line.
    pub(super) fn render(&mut self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        let size = self.canvas_size;
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        if self.fit_figure() {
            // The scale factor changed.
            self.announce_size();
        }
        let problem = match self.figure.canvas().draw() {
            Ok(()) => None,
            Err(CanvasError::Draw(problem)) => Some(problem),
            Err(err) => return Err(err.into()),
        };
        self.present(cx)?;
        if let Some(problem) = problem {
            self.set_status(cx, problem.to_string())?;
        }
        Ok(())
    }

    /// Shows what changed on the figure's canvas since it was last shown:
    /// all of it after a draw, or only the blitted regions, updating only
    /// those parts of the window (matplotlib's `blit`).
    pub(super) fn present(&mut self, cx: &mut Context<'_>) -> Result<(), GuiError> {
        present_figure(&self.figure, &mut self.window, self.canvas, cx)
    }
}

/// Shows in the surface `canvas` of `window` what changed on the canvas
/// of `figure` since it was last shown: all of it after a draw, or only
/// the blitted regions, updating only those parts of the window
/// (matplotlib's `blit`).
pub(in crate::gui) fn present_figure<M: 'static>(
    figure: &Figure,
    window: &mut UiWindow<M>,
    canvas: WidgetId,
    cx: &mut Context<'_>,
) -> Result<(), GuiError> {
    let figure_canvas = figure.canvas();
    match figure_canvas.take_presentation() {
        Presentation::Nothing => Ok(()),
        Presentation::Whole => {
            if let Some(image) = figure_canvas.with_pixels(Pixmap::clone) {
                window.update(cx, canvas, |surface: &mut Surface<M>| {
                    surface.set_image(image);
                })?;
            }
            Ok(())
        }
        Presentation::Regions(regions) => figure_canvas
            .with_pixels(|pixels| {
                regions.into_iter().try_for_each(|region| {
                    let region = window_rect(region);
                    window.update_region(cx, canvas, region, |surface: &mut Surface<M>| {
                        surface.blit(pixels, region);
                    })?;
                    Ok(())
                })
            })
            .unwrap_or(Ok(())),
    }
}

/// A blitted part of the canvas as a rectangle of the canvas area.
fn window_rect(region: PixelBox) -> PhysicalRect {
    let corner = |at: i64| u32::try_from(at).unwrap_or(0);
    PhysicalRect::new(
        corner(region.x0),
        corner(region.y0),
        region.width(),
        region.height(),
    )
}
