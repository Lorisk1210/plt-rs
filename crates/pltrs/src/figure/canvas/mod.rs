//! The raster canvas a figure keeps between draws (matplotlib's
//! `FigureCanvasAgg` and its cached renderer), and blitting: redrawing
//! only what changes.
//!
//! A full [`FigureCanvas::draw`] fills the canvas, leaving out the
//! animated artists ([`ArtistHandle::set_animated`]). A program saves the
//! background with [`FigureCanvas::copy_from_bbox`], then for every update
//! puts it back with [`FigureCanvas::restore_region`], draws the changed
//! artists with [`Axes::draw_artist`](crate::axes::Axes::draw_artist) or
//! [`Figure::draw_artist`] and shows the result with
//! [`FigureCanvas::blit`]: a window showing the figure updates only that
//! part of itself.
//!
//! ```
//! use pltrs::prelude::*;
//! let fig = Figure::with_size(2.0, 2.0, 50.0);
//! let ax = fig.subplot();
//! let line = ax.plot(vec![0.0, 1.0], vec![0.0, 1.0])?;
//! line.set_animated(true);
//! let canvas = fig.canvas();
//! canvas.draw()?;
//! let background = canvas.copy_from_bbox(&fig.bbox())?;
//! for i in 0..3 {
//!     canvas.restore_region(&background)?;
//!     line.set_data(&[0.0, 1.0], &[0.0, f64::from(i)]);
//!     ax.draw_artist(&line)?;
//!     canvas.blit(Some(&ax.bbox()))?;
//! }
//! assert_eq!(canvas.buffer_rgba()?.width(), 100);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod artists;
mod ratio;
mod region;
mod state;
#[cfg(test)]
mod tests;

use std::fmt;

use pltrs_core::{Bbox, Point};
use pltrs_raster::{Pixmap, RasterError};
use pltrs_render::{RasterBackendError, RgbaImage};
use pltrs_text::TextError;

use super::{DrawErrors, Figure};
use crate::artist::{ArtistHandle, DrawError};
use crate::events::Event;
pub use region::{BufferRegion, PixelBox};
use state::{CanvasKey, Generation, KeptCanvas};
pub(crate) use state::{CanvasState, Presentation};

/// Why a canvas operation failed.
#[derive(Debug)]
pub enum CanvasError {
    /// The canvas could not be made at the figure's size and resolution,
    /// or its pixels could not be encoded.
    Backend(RasterBackendError),
    /// The pixels of a region could not be stored.
    Pixels(RasterError),
    /// A box has a coordinate that is not finite or no pixel coordinate,
    /// or is upside down.
    InvalidBox(Bbox),
    /// The region comes from another figure, or the canvas was drawn,
    /// resized or given another resolution since it was saved.
    StaleRegion,
    /// The canvas has not been drawn yet.
    NotDrawn,
    /// The artist belongs to another figure.
    ForeignArtist,
    /// A device pixel ratio that is not finite and positive.
    InvalidPixelRatio(f64),
    /// Some artists could not be drawn; the others were.
    Draw(DrawErrors),
    /// A line of text could not be typeset.
    Text(TextError),
}

impl fmt::Display for CanvasError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(err) => write!(f, "cannot render the canvas: {err}"),
            Self::Pixels(err) => write!(f, "cannot save the region: {err}"),
            Self::InvalidBox(bbox) => write!(
                f,
                "invalid box ({}, {}, {}, {})",
                bbox.x0(),
                bbox.y0(),
                bbox.x1(),
                bbox.y1()
            ),
            Self::StaleRegion => f.write_str(
                "the region was saved from another figure or before the canvas was last drawn or resized",
            ),
            Self::NotDrawn => f.write_str("the canvas has not been drawn yet"),
            Self::ForeignArtist => f.write_str("the artist belongs to another figure"),
            Self::InvalidPixelRatio(ratio) => write!(
                f,
                "the device pixel ratio must be finite and positive, not {ratio}"
            ),
            Self::Draw(err) => err.fmt(f),
            Self::Text(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for CanvasError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Backend(err) => Some(err),
            Self::Pixels(err) => Some(err),
            Self::Draw(err) => Some(err),
            Self::Text(err) => Some(err),
            Self::InvalidBox(_)
            | Self::StaleRegion
            | Self::NotDrawn
            | Self::ForeignArtist
            | Self::InvalidPixelRatio(_) => None,
        }
    }
}

/// The raster canvas of a figure (matplotlib's `fig.canvas` with the Agg
/// renderer it keeps): what the figure was last drawn as, changed by
/// restored regions and artists drawn one by one, and shown by the
/// figure's windows. Subfigures share the canvas of their figure.
///
/// `FigureCanvas` is a cheap handle; clones refer to the same canvas.
#[derive(Clone, Debug, PartialEq)]
pub struct FigureCanvas {
    figure: Figure,
}

impl Figure {
    /// The canvas the figure keeps between draws (matplotlib's
    /// `fig.canvas`); a subfigure gives its figure's.
    pub fn canvas(&self) -> FigureCanvas {
        FigureCanvas {
            figure: Self::root_of(&self.state),
        }
    }

    /// The figure's box in display pixels from the bottom-left corner of
    /// the canvas (matplotlib's `Figure.bbox`); a subfigure's own part.
    pub fn bbox(&self) -> Bbox {
        let state = self.state();
        crate::artist::LayoutContext::new(&state)
            .frame_extent(self.root.id())
            .unwrap_or_else(|| {
                let (width, height) = state.size_pixels();
                Bbox::from_bounds(0.0, 0.0, width, height)
            })
    }
}

impl FigureCanvas {
    /// The figure drawn on the canvas.
    pub fn figure(&self) -> &Figure {
        &self.figure
    }

    /// Draws the whole figure onto the canvas, leaving out its animated
    /// artists, then sends [`Event::Draw`] to its callbacks (matplotlib's
    /// `canvas.draw()`). The canvas takes the figure's size and resolution
    /// first, and the regions saved before cannot be restored any more.
    /// Windows showing the figure present the whole canvas.
    ///
    /// # Errors
    ///
    /// When the canvas cannot be made, or some artists cannot be drawn
    /// (the others are).
    pub fn draw(&self) -> Result<(), CanvasError> {
        let mut kept = self.take()?;
        kept.renderer.clear();
        let drawn = self.figure.draw_without_event(&mut kept.renderer);
        kept.generation = Generation::next();
        self.keep(kept);
        self.figure.state_mut().canvas.drawn();
        self.figure.process_event(&Event::Draw);
        drawn.map_err(CanvasError::Draw)
    }

    /// Asks for a full draw (matplotlib's `draw_idle`): a figure shown in
    /// a window or a browser is drawn when they next refresh; any other is
    /// drawn at once, as matplotlib's Agg canvas does, unless such a draw
    /// is already running.
    ///
    /// # Errors
    ///
    /// As [`FigureCanvas::draw`], when the figure is drawn at once.
    pub fn draw_idle(&self) -> Result<(), CanvasError> {
        {
            let mut state = self.figure.state_mut();
            if state.canvas.presenters > 0 {
                state.stale = true;
                return Ok(());
            }
            if state.canvas.idle_drawing {
                return Ok(());
            }
            state.canvas.idle_drawing = true;
        }
        let drawn = self.draw();
        self.figure.state_mut().canvas.idle_drawing = false;
        drawn
    }

    /// Saves the pixels of the canvas inside `bbox`, a box in display
    /// pixels from the bottom-left corner such as [`Figure::bbox`] or
    /// [`Axes::bbox`](crate::axes::Axes::bbox) (matplotlib's
    /// `copy_from_bbox`): its corners are truncated to whole pixels. Parts
    /// of the box outside the canvas save nothing.
    ///
    /// # Errors
    ///
    /// When `bbox` is upside down or not finite, or the canvas cannot be
    /// made.
    pub fn copy_from_bbox(&self, bbox: &Bbox) -> Result<BufferRegion, CanvasError> {
        let kept = self.take()?;
        let saved = BufferRegion::save(&kept, bbox);
        self.keep(kept);
        saved
    }

    /// Puts a saved region back where it came from (matplotlib's
    /// `restore_region`).
    ///
    /// # Errors
    ///
    /// [`CanvasError::StaleRegion`] when the canvas was drawn, resized or
    /// given another resolution since the region was saved, or the region
    /// comes from another figure.
    pub fn restore_region(&self, region: &BufferRegion) -> Result<(), CanvasError> {
        self.restore_region_with(region, None, None)
    }

    /// Puts back the part of a saved region inside `bbox`, with the
    /// region's top-left corner moved to `xy` (matplotlib's
    /// `restore_region(region, bbox, xy)`). As in matplotlib, both are in
    /// canvas pixels from the top-left corner, like
    /// [`BufferRegion::extents`], and truncated to whole pixels; `bbox`
    /// includes its right and bottom edges and defaults to the whole
    /// region, `xy` to the corner of `bbox`.
    ///
    /// ```
    /// use pltrs::figure::{Figure, PixelBox};
    /// use pltrs::transforms::{Bbox, Point};
    /// let fig = Figure::with_size(1.0, 1.0, 100.0);
    /// let canvas = fig.canvas();
    /// canvas.draw()?;
    /// let region = canvas.copy_from_bbox(&Bbox::from_extents(10.0, 10.0, 50.0, 50.0))?;
    /// assert_eq!(region.extents(), PixelBox::new(10, 50, 50, 90));
    /// // The right half of the region, from x = 30, moved 30 pixels to the
    /// // left: the region's corner goes from x = 10 to x = -20.
    /// let half = Bbox::from_extents(30.0, 50.0, 50.0, 90.0);
    /// canvas.restore_region_with(&region, Some(half), Some(Point::new(-20.0, 50.0)))?;
    /// # Ok::<(), pltrs::figure::CanvasError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// As [`FigureCanvas::restore_region`], and when a coordinate is not
    /// finite or no pixel coordinate.
    pub fn restore_region_with(
        &self,
        region: &BufferRegion,
        bbox: Option<Bbox>,
        xy: Option<Point>,
    ) -> Result<(), CanvasError> {
        let mut kept = self.take()?;
        let restored = region.restore(&mut kept, bbox, xy);
        self.keep(kept);
        restored
    }

    /// Shows the part of the canvas inside `bbox` (display pixels from the
    /// bottom-left corner; all of it with `None`) in the windows and
    /// browser pages showing the figure, which update only that part of
    /// themselves (matplotlib's `blit`). For a figure shown nowhere this
    /// does nothing, as for matplotlib's Agg canvas.
    ///
    /// # Errors
    ///
    /// When a coordinate of `bbox` is not finite.
    pub fn blit(&self, bbox: Option<&Bbox>) -> Result<(), CanvasError> {
        let (width, height) = self.figure.width_height();
        let region = match bbox {
            Some(bbox) => PixelBox::covering(bbox, width, height)?,
            None => PixelBox::canvas(width, height),
        };
        self.figure.state_mut().canvas.blitted(region);
        Ok(())
    }

    /// The pixels of the canvas: straight-alpha RGBA rows from the top
    /// (matplotlib's `buffer_rgba()`).
    ///
    /// # Errors
    ///
    /// When the canvas has not been drawn yet.
    pub fn buffer_rgba(&self) -> Result<RgbaImage, CanvasError> {
        self.with_drawn(|kept| Ok(kept.renderer.to_image()))
    }

    /// The canvas as a PNG image.
    pub(crate) fn to_png(&self) -> Result<Vec<u8>, CanvasError> {
        self.with_drawn(|kept| kept.renderer.to_png().map_err(CanvasError::Backend))
    }

    /// Runs `f` on the canvas, failing if it was never drawn.
    fn with_drawn<R>(
        &self,
        f: impl FnOnce(&KeptCanvas) -> Result<R, CanvasError>,
    ) -> Result<R, CanvasError> {
        let state = self.figure.state();
        state
            .canvas
            .kept
            .as_ref()
            .map_or(Err(CanvasError::NotDrawn), f)
    }

    /// Runs `f` on the pixels of the canvas, if it was drawn.
    pub(crate) fn with_pixels<R>(&self, f: impl FnOnce(&Pixmap) -> R) -> Option<R> {
        let state = self.figure.state();
        state
            .canvas
            .kept
            .as_ref()
            .map(|kept| f(kept.renderer.pixmap()))
    }

    /// What the figure's windows have not presented yet, which they now
    /// take on.
    pub(crate) fn take_presentation(&self) -> Presentation {
        std::mem::take(&mut self.figure.state_mut().canvas.presentation)
    }

    /// Takes the canvas out of the figure until [`FigureCanvas::keep`]:
    /// the one kept if it fits the figure's size, resolution and rcParams,
    /// else a new blank one (matplotlib's `get_renderer`).
    fn take(&self) -> Result<KeptCanvas, CanvasError> {
        let key = CanvasKey::of(&self.figure);
        let kept = self.figure.state_mut().canvas.kept.take();
        match kept {
            Some(kept) if kept.key == key => Ok(kept),
            _ => key.make().map_err(CanvasError::Backend),
        }
    }

    /// Keeps a problem met by a canvas operation that no caller can take,
    /// such as a blit from an event callback or an animation timer, to be
    /// reported with the figure's next draw.
    pub(crate) fn report(&self, outcome: Result<(), CanvasError>) {
        let problems = match outcome {
            Ok(()) => return,
            Err(CanvasError::Draw(DrawErrors(errors))) => errors,
            Err(err) => vec![DrawError::new(self.figure.id(), err.to_string())],
        };
        self.figure.state_mut().pending_errors.extend(problems);
    }

    /// Puts back the canvas [`FigureCanvas::take`] took out.
    fn keep(&self, mut kept: KeptCanvas) {
        // Figure draws do not report what the rasterizer refused (paths
        // past its limits); dropped so that it does not pile up.
        drop(kept.renderer.take_errors());
        self.figure.state_mut().canvas.kept = Some(kept);
    }
}

/// Registers a window or browser page showing a figure, while it lives:
/// the figure's blits and draws are recorded for it to present, and
/// [`FigureCanvas::draw_idle`] leaves the drawing to it.
#[derive(Debug)]
pub(crate) struct Presenting {
    figure: Figure,
}

impl Presenting {
    /// Registers a presenter of `figure`.
    pub(crate) fn new(figure: &Figure) -> Self {
        let figure = figure.canvas().figure;
        figure.state_mut().canvas.presenters += 1;
        Self { figure }
    }
}

impl Drop for Presenting {
    /// Unregisters the presenter. Once no window or page shows the figure,
    /// it leaves the display's scaling and returns to the resolution it
    /// was made at (matplotlib's `_set_base_canvas` when a manager is
    /// destroyed).
    fn drop(&mut self) {
        let mut state = self.figure.state_mut();
        let canvas = &mut state.canvas;
        canvas.presenters = canvas.presenters.saturating_sub(1);
        if canvas.presenters == 0 {
            canvas.presentation = Presentation::Nothing;
            canvas.device_pixel_ratio = 1.0;
            let dpi = state.original_dpi;
            state.set_dpi(dpi);
        }
    }
}
