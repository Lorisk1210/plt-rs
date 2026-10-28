//! Figures: the top-level container of everything drawn (matplotlib.figure).

mod artists;
mod autofmt;
mod canvas;
mod clear;
mod detach;
mod draw_order;
mod frame;
mod mosaic;
mod mosaic_axes;
mod pdf_pages;
mod pgf;
mod postscript;
mod print;
mod rect;
mod save;
mod size;
mod state;
mod subfigure;
mod subplots;
mod svg;
#[cfg(test)]
mod tests;
mod texts;

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use plt_core::Rgba;
use plt_render::Renderer;

pub use canvas::{BufferRegion, CanvasError, FigureCanvas, PixelBox};
pub(crate) use canvas::{CanvasState, Presentation, Presenting};
pub(crate) use detach::attach;
pub(crate) use draw_order::draws_before;
pub(crate) use frame::{FigureFrame, SuperText};
pub use mosaic::{Mosaic, MosaicCell, MosaicError, MosaicOptions};
pub use mosaic_axes::MosaicAxes;
pub use pdf_pages::PdfPages;
pub use plt_render::{
    Metadata, PageOrientation, PageSetupError, Paper, PngMetadata, SvgFontType, SvgMetadata,
    UnknownSvgFontType,
};
pub use print::{BboxInches, PadInches, SaveOptions};
pub use rect::AxesRectError;
pub(crate) use rect::axes_bounds;
use save::whole_pixels;
pub use save::{SaveError, SaveFormat};
pub use size::{FigSize, SizeError, SizeUnit};
pub(crate) use state::FigureState;
pub use subfigure::SubfigureOptions;
pub use subplots::Share;
pub(crate) use subplots::{TITLE_PLACES, axes_title, insert_axes, make_axes};

use crate::artist::{Artist, ArtistHandle, ArtistId, ArtistLookup, DrawError, RawHandle};
use crate::axes::{Axes, AxesState, PlotError};
use crate::colors::Color;
use crate::events::Event;
use crate::gridspec::{GridSpecError, SubplotOverrides, SubplotParams};
use crate::lines::{Line2D, LineArtist, LineProps};
use crate::patches::{Patch, PatchArtist, PatchShape};
use crate::transforms::Transform;

/// The problems found while drawing a figure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrawErrors(pub Vec<DrawError>);

impl fmt::Display for DrawErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let messages: Vec<String> = self.0.iter().map(ToString::to_string).collect();
        write!(
            f,
            "could not draw {} artist(s): {}",
            self.0.len(),
            messages.join("; ")
        )
    }
}

impl std::error::Error for DrawErrors {}

/// A figure: the canvas, its background and every axes and artist on it.
///
/// `Figure` is a cheap handle; clones refer to the same figure.
///
/// ```
/// use plt::figure::Figure;
/// let fig = Figure::new();
/// let ax = fig.add_axes([0.1, 0.1, 0.8, 0.8])?;
/// ax.set_xlim(0.0, 10.0)?;
/// assert_eq!(ax.xlim(), (0.0, 10.0));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone)]
pub struct Figure {
    state: Rc<RefCell<FigureState>>,
    root: RawHandle,
}

impl fmt::Debug for Figure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = self.state.borrow();
        f.debug_struct("Figure")
            .field("size_inches", &state.size_inches)
            .field("dpi", &state.dpi())
            .finish_non_exhaustive()
    }
}

impl PartialEq for Figure {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.state, &other.state)
    }
}

impl Default for Figure {
    fn default() -> Self {
        Self::new()
    }
}

impl Figure {
    /// A figure of size `figure.figsize` at `figure.dpi`, coloured by
    /// `figure.facecolor` and `figure.edgecolor`.
    pub fn new() -> Self {
        let rc = crate::rc::params();
        let [width, height] = rc.figure_figsize;
        Self::with_size(width, height, rc.figure_dpi)
    }

    /// A figure of the given size in inches and resolution.
    pub fn with_size(width: f64, height: f64, dpi: f64) -> Self {
        let rc = crate::rc::params();
        // The rc colours are resolved now, as matplotlib's figure patch
        // resolves its colours when made.
        let color = |c: &Color| Color::from(c.resolve().unwrap_or(Rgba::WHITE));
        let state = FigureState::new(
            (width, height),
            dpi,
            color(&rc.figure_facecolor),
            color(&rc.figure_edgecolor),
        );
        let root = state.root;
        let state = Rc::new(RefCell::new(state));
        let root = RawHandle::new(Rc::clone(&state), root);
        Self { state, root }
    }

    /// Canvas size in inches.
    pub fn size_inches(&self) -> (f64, f64) {
        self.state.borrow().size_inches
    }

    /// Canvas resolution in dots per inch.
    pub fn dpi(&self) -> f64 {
        self.state.borrow().dpi()
    }

    /// Changes the canvas resolution. As in matplotlib, saving with
    /// `savefig.dpi: figure` keeps the resolution the figure was made at:
    /// give [`SaveOptions::dpi`] to save at another.
    ///
    /// ```
    /// use plt::figure::{Figure, SaveOptions};
    /// let fig = Figure::with_size(2.0, 1.0, 100.0);
    /// fig.set_dpi(50.0);
    /// assert_eq!(fig.width_height(), (100, 50));
    /// let saved = fig.render_raster_with(SaveOptions::default())?.to_image();
    /// assert_eq!((saved.width(), saved.height()), (200, 100));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_dpi(&self, dpi: f64) -> &Self {
        let mut state = self.state.borrow_mut();
        state.set_dpi(dpi);
        state.stale = true;
        self
    }

    /// Canvas size in pixels.
    pub fn size_pixels(&self) -> (f64, f64) {
        self.state.borrow().size_pixels()
    }

    /// The whole pixels of a raster canvas for this figure (matplotlib's
    /// `canvas.get_width_height()`): the size in pixels truncated, but
    /// rounded up when within 1e-8 of the next pixel, so that a 9.2 in
    /// figure at 100 dpi spans 920 pixels despite floating-point error.
    ///
    /// ```
    /// use plt::figure::Figure;
    /// assert_eq!(Figure::with_size(9.2, 5.0, 100.0).width_height(), (920, 500));
    /// assert_eq!(Figure::with_size(2.555, 1.0, 100.0).width_height(), (255, 100));
    /// ```
    pub fn width_height(&self) -> (u32, u32) {
        let (width, height) = self.size_pixels();
        (whole_pixels(width), whole_pixels(height))
    }

    /// Sets the background colour (matplotlib's `set_facecolor`); a
    /// colour that does not resolve is reported when the figure is drawn.
    pub fn set_facecolor(&self, color: impl Into<Color>) -> &Self {
        let color = color.into();
        self.root
            .with_mut::<FigureFrame, _>(|frame| frame.facecolor = color);
        self
    }

    /// Sets the colour of the frame around the canvas (matplotlib's
    /// `set_edgecolor`); it shows once the frame has a width.
    pub fn set_edgecolor(&self, color: impl Into<Color>) -> &Self {
        let color = color.into();
        self.root
            .with_mut::<FigureFrame, _>(|frame| frame.edgecolor = color);
        self
    }

    /// Sets the width of the frame around the canvas in points
    /// (matplotlib's `set_linewidth`).
    pub fn set_linewidth(&self, width: f64) -> &Self {
        self.root
            .with_mut::<FigureFrame, _>(|frame| frame.linewidth = width);
        self
    }

    /// The width of the frame around the canvas in points (matplotlib's
    /// `get_linewidth`).
    pub fn linewidth(&self) -> f64 {
        self.root.with::<FigureFrame, _>(|frame| frame.linewidth)
    }

    /// Sets whether the background and the frame around the canvas are
    /// drawn (matplotlib's `set_frameon`); without them the canvas stays
    /// transparent where nothing else is drawn.
    pub fn set_frameon(&self, frameon: bool) -> &Self {
        self.root
            .with_mut::<FigureFrame, _>(|frame| frame.frameon = frameon);
        self
    }

    /// Whether the background and the frame around the canvas are drawn;
    /// initially they are, whatever `figure.frameon` says, as in figures
    /// made by pyplot's `figure`.
    pub fn frameon(&self) -> bool {
        self.root.with::<FigureFrame, _>(|frame| frame.frameon)
    }

    /// The background colour.
    pub fn facecolor(&self) -> Color {
        self.root
            .with::<FigureFrame, _>(|frame| frame.facecolor.clone())
    }

    /// The colour of the frame around the canvas.
    pub fn edgecolor(&self) -> Color {
        self.root
            .with::<FigureFrame, _>(|frame| frame.edgecolor.clone())
    }

    /// The margins and spacing of subplot grids.
    pub fn subplot_params(&self) -> SubplotParams {
        self.root
            .with::<FigureFrame, _>(|frame| frame.subplot_params)
    }

    /// Changes the subplot margins and spacing of the figure and its
    /// subfigures, which share them as in matplotlib, and moves every
    /// grid-placed axes accordingly (`fig.subplots_adjust(...)`).
    pub fn subplots_adjust(&self, overrides: &SubplotOverrides) -> Result<&Self, GridSpecError> {
        let params = self.subplot_params().updated(overrides)?;
        self.state.borrow_mut().set_subplot_params(params)?;
        Ok(self)
    }

    /// Adds a line through `(x, y)` in figure fractions, styled from the
    /// rc parameters and `props` without the property cycle (matplotlib's
    /// `fig.add_artist(Line2D(...))`).
    ///
    /// # Errors
    ///
    /// When `x` and `y` differ in length.
    pub fn add_line(&self, x: &[f64], y: &[f64], props: LineProps) -> Result<Line2D, PlotError> {
        if x.len() != y.len() {
            return Err(PlotError::LengthMismatch {
                x: x.len(),
                y: y.len(),
            });
        }
        let mut line = LineArtist::new(x.to_vec(), y.to_vec(), crate::rc::params());
        props.apply(&mut line);
        if line.base().transform.is_none() {
            line.base_mut().transform = Some(Transform::Figure(self.root.id()));
        }
        let id = self
            .state
            .borrow_mut()
            .adopt(self.root.id(), Box::new(line));
        Ok(Line2D::from_raw(RawHandle::new(Rc::clone(&self.state), id)))
    }

    /// Adds a patch in figure fractions (matplotlib's
    /// `fig.add_artist(patch)`); give it another coordinate system with
    /// [`ArtistHandle::set_transform`].
    pub fn add_patch<S: PatchShape>(&self, shape: S) -> Patch<S> {
        let mut patch = PatchArtist::new(shape, crate::rc::params());
        patch.base_mut().transform = Some(Transform::Figure(self.root.id()));
        let id = self
            .state
            .borrow_mut()
            .adopt(self.root.id(), Box::new(patch));
        Patch::from_raw(RawHandle::new(Rc::clone(&self.state), id))
    }

    /// Every axes of the figure, in creation order.
    /// The figure lists the axes of its subfigures too (matplotlib's
    /// shared axes stack); a subfigure lists only its own.
    pub fn axes(&self) -> Vec<Axes> {
        self.scoped::<AxesState>()
            .into_iter()
            .map(|id| Axes::from_raw(RawHandle::new(Rc::clone(&self.state), id)))
            .collect()
    }

    /// The artists of type `T` placed directly in this figure's frame or,
    /// for the root figure, in any frame (the scope of [`Figure::axes`]).
    pub(crate) fn scoped<T: Artist>(&self) -> Vec<ArtistId> {
        let state = self.state.borrow();
        let frame_id = self.root.id();
        if !state.is_attached(frame_id) {
            // A removed subfigure still holds its artists.
            let children = self
                .root
                .with::<FigureFrame, _>(|frame| frame.children.clone());
            return children
                .into_iter()
                .filter(|&id| state.artist_as::<T>(id).is_some())
                .collect();
        }
        let in_scope = |parent: Option<ArtistId>| match parent {
            Some(parent) if frame_id == state.root => {
                state.store.get_as::<FigureFrame>(parent).is_some()
            }
            Some(parent) => parent == frame_id,
            None => false,
        };
        state
            .store
            .ids()
            .filter(|&id| {
                state
                    .store
                    .get_as::<T>(id)
                    .is_some_and(|artist| in_scope(artist.base().parent))
            })
            .collect()
    }

    /// Renders the figure through `renderer`, then sends [`Event::Draw`]
    /// to the figure's callbacks (matplotlib's `draw_event`).
    ///
    /// Everything drawable is drawn; artists with invalid properties (for
    /// example an unknown colour name) are skipped and reported.
    pub fn draw(&self, renderer: &mut dyn Renderer) -> Result<(), DrawErrors> {
        let drawn = self.draw_without_event(renderer);
        self.process_event(&Event::Draw);
        drawn
    }

    /// [`Figure::draw`] without the draw event, for callers that send it
    /// themselves once they are done.
    pub(crate) fn draw_without_event(&self, renderer: &mut dyn Renderer) -> Result<(), DrawErrors> {
        self.prepare_layout();
        let errors = self.state.borrow().draw(renderer);
        let mut state = self.state.borrow_mut();
        state.stale = false;
        state.last_draw_dpi = Some(state.dpi());
        state.pending_errors.clear();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(DrawErrors(errors))
        }
    }

    /// Runs the layout pass (the layout engine, autoscaling and tick
    /// placement) without drawing, so positions, limits and ticks can be
    /// queried; from then on the figure counts as drawn (matplotlib's
    /// `draw_without_rendering`, or `canvas.draw()`).
    pub fn layout(&self) {
        self.prepare_layout();
        let mut state = self.state.borrow_mut();
        state.last_draw_dpi = Some(state.dpi());
    }

    /// The layout pass of [`Figure::layout`], before a draw, then the
    /// limit callbacks its autoscaling queued.
    pub(crate) fn prepare_layout(&self) {
        {
            let mut state = self.state.borrow_mut();
            state.layout.pending = true;
            state.prepare();
        }
        crate::axes::run_limit_callbacks(&self.state);
    }

    /// The root figure owning `state`.
    pub(crate) fn root_of(state: &Rc<RefCell<FigureState>>) -> Self {
        let root = state.borrow().root;
        Self {
            state: Rc::clone(state),
            root: RawHandle::new(Rc::clone(state), root),
        }
    }

    /// The figure or subfigure holding `artist` most closely: the first
    /// figure frame among its ancestors, the figure itself for the root.
    pub(crate) fn parent_of(artist: &RawHandle) -> Self {
        let state = artist.figure_state();
        let frame = {
            let figure = state.borrow();
            let parent_of = |id| figure.store.get(id).and_then(|a| a.base().parent);
            let mut ancestor = parent_of(artist.id());
            loop {
                match ancestor {
                    Some(id) if figure.store.get_as::<FigureFrame>(id).is_some() => break id,
                    Some(id) => ancestor = parent_of(id),
                    None => break figure.root,
                }
            }
        };
        Self {
            state: Rc::clone(state),
            root: RawHandle::new(Rc::clone(state), frame),
        }
    }

    /// Sends every mouse event to `ax` until [`Figure::release_mouse`]:
    /// their axes and data coordinates are those of `ax` wherever the
    /// pointer is (matplotlib's `canvas.grab_mouse`).
    pub fn grab_mouse(&self, ax: &Axes) {
        self.state.borrow_mut().mouse_grabber = Some(ax.id());
    }

    /// Ends a [`Figure::grab_mouse`] by `ax` (matplotlib's
    /// `canvas.release_mouse`).
    pub fn release_mouse(&self, ax: &Axes) {
        let mut state = self.state.borrow_mut();
        if state.mouse_grabber == Some(ax.id()) {
            state.mouse_grabber = None;
        }
    }

    /// The axes grabbing the mouse, if any.
    pub fn mouse_grabber(&self) -> Option<Axes> {
        let id = self.state.borrow().mouse_grabber?;
        self.axes().into_iter().find(|ax| ax.id() == id)
    }

    /// Makes a widget receive key presses without the window acting on its
    /// keyboard shortcuts, or gives them back (matplotlib's text boxes
    /// clear the `keymap.*` rcParams while typing).
    pub fn set_capture_keys(&self, capture: bool) {
        self.state.borrow_mut().capture_keys = capture;
    }

    /// Whether key presses are captured by a widget.
    pub fn captures_keys(&self) -> bool {
        self.state.borrow().capture_keys
    }

    /// Whether `artist` belongs to this figure (or one of its
    /// subfigures).
    pub fn contains_artist(&self, artist: &impl ArtistHandle) -> bool {
        Rc::ptr_eq(&self.state, artist.raw().figure_state())
    }

    /// Read access to the figure's state.
    pub(crate) fn state(&self) -> std::cell::Ref<'_, FigureState> {
        self.state.borrow()
    }

    /// Write access to the figure's state.
    pub(crate) fn state_mut(&self) -> std::cell::RefMut<'_, FigureState> {
        self.state.borrow_mut()
    }

    /// Whether something changed since the last draw.
    pub fn is_stale(&self) -> bool {
        self.state.borrow().stale
    }
}

impl ArtistHandle for Figure {
    fn raw(&self) -> &RawHandle {
        &self.root
    }
}
