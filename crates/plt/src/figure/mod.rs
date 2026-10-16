//! Figures: the top-level container of everything drawn (matplotlib.figure).

mod frame;
mod state;
#[cfg(test)]
mod tests;

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use plt_core::{Bbox, Rgba};
use plt_render::Renderer;

pub(crate) use frame::FigureFrame;
pub(crate) use state::FigureState;

use crate::artist::{ArtistHandle, DrawError, RawHandle};
use crate::axes::{Axes, AxesState};
use crate::gridspec::{GridSpec, GridSpecError, SubplotOverrides, SubplotParams, SubplotSpec};
use crate::numeric::Grid;

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
/// let ax = fig.add_axes([0.1, 0.1, 0.8, 0.8]);
/// ax.set_xlim(0.0, 10.0);
/// assert_eq!(ax.xlim(), (0.0, 10.0));
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
            .field("dpi", &state.dpi)
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
    /// A 6.4 × 4.8 inch figure at 100 dpi with a white background.
    pub fn new() -> Self {
        Self::with_size(6.4, 4.8, 100.0)
    }

    /// A figure of the given size in inches and resolution.
    pub fn with_size(width: f64, height: f64, dpi: f64) -> Self {
        let state = FigureState::new((width, height), dpi, Rgba::WHITE, Rgba::WHITE);
        let root = state.root;
        let state = Rc::new(RefCell::new(state));
        let root = RawHandle::new(Rc::clone(&state), root);
        Self { state, root }
    }

    /// Canvas size in inches.
    pub fn size_inches(&self) -> (f64, f64) {
        self.state.borrow().size_inches
    }

    /// Resizes the canvas (in inches).
    pub fn set_size_inches(&self, width: f64, height: f64) -> &Self {
        let mut state = self.state.borrow_mut();
        state.size_inches = (width, height);
        state.stale = true;
        self
    }

    /// Canvas resolution in dots per inch.
    pub fn dpi(&self) -> f64 {
        self.state.borrow().dpi
    }

    /// Changes the canvas resolution.
    pub fn set_dpi(&self, dpi: f64) -> &Self {
        let mut state = self.state.borrow_mut();
        state.dpi = dpi;
        state.stale = true;
        self
    }

    /// Canvas size in pixels.
    pub fn size_pixels(&self) -> (f64, f64) {
        self.state.borrow().size_pixels()
    }

    /// Sets the background colour.
    pub fn set_facecolor(&self, color: Rgba) -> &Self {
        self.root
            .with_mut::<FigureFrame, _>(|frame| frame.facecolor = color);
        self
    }

    /// The background colour.
    pub fn facecolor(&self) -> Rgba {
        self.root.with::<FigureFrame, _>(|frame| frame.facecolor)
    }

    /// Adds axes occupying `[left, bottom, width, height]` in figure fractions.
    pub fn add_axes(&self, rect: [f64; 4]) -> Axes {
        let [left, bottom, width, height] = rect;
        self.insert_axes(Bbox::from_bounds(left, bottom, width, height), None)
    }

    /// Adds axes filling the default subplot area (`fig.add_subplot()`).
    pub fn subplot(&self) -> Axes {
        let spec = GridSpec::new(1, 1)
            .and_then(|grid| grid.cell(0, 0))
            .expect("a 1x1 grid has a cell (0, 0)");
        self.add_subplot_spec(&spec)
            .expect("the default subplot parameters are valid")
    }

    /// Adds axes in cell `index` (1-based, row-major) of an
    /// `nrows × ncols` grid (`fig.add_subplot(nrows, ncols, index)`).
    pub fn add_subplot(
        &self,
        nrows: usize,
        ncols: usize,
        index: usize,
    ) -> Result<Axes, GridSpecError> {
        let spec = GridSpec::new(nrows, ncols)?.numbered(index)?;
        self.add_subplot_spec(&spec)
    }

    /// Adds axes covering a grid cell or block (`fig.add_subplot(gs[...])`).
    pub fn add_subplot_spec(&self, spec: &SubplotSpec) -> Result<Axes, GridSpecError> {
        let position = spec.position(&self.subplot_params())?;
        Ok(self.insert_axes(position, Some(spec.clone())))
    }

    /// A grid of `nrows × ncols` axes (`fig.subplots(nrows, ncols)`),
    /// indexed by `(row, col)`.
    pub fn subplots(&self, nrows: usize, ncols: usize) -> Result<Grid<Axes>, GridSpecError> {
        let grid = GridSpec::new(nrows, ncols)?;
        let mut cells = Vec::with_capacity(nrows * ncols);
        for row in 0..nrows {
            for col in 0..ncols {
                cells.push(self.add_subplot_spec(&grid.cell(row, col)?)?);
            }
        }
        Ok(Grid::from_vec(nrows, ncols, cells).expect("one axes per grid cell"))
    }

    /// A grid specification using this figure's subplot parameters.
    pub fn add_gridspec(&self, nrows: usize, ncols: usize) -> Result<GridSpec, GridSpecError> {
        GridSpec::new(nrows, ncols)
    }

    /// The margins and spacing of subplot grids.
    pub fn subplot_params(&self) -> SubplotParams {
        self.root
            .with::<FigureFrame, _>(|frame| frame.subplot_params)
    }

    /// Changes the subplot margins and spacing and moves every grid-placed
    /// axes accordingly (`fig.subplots_adjust(...)`).
    pub fn subplots_adjust(&self, overrides: &SubplotOverrides) -> Result<&Self, GridSpecError> {
        let params = self.subplot_params().updated(overrides)?;
        let mut state = self.state.borrow_mut();
        if let Some(frame) = state.store.get_as_mut::<FigureFrame>(self.root.id()) {
            frame.subplot_params = params;
        }
        let ids: Vec<_> = state.store.ids().collect();
        for id in ids {
            if let Some(axes) = state.store.get_as_mut::<AxesState>(id)
                && let Some(spec) = &axes.subplot_spec
            {
                axes.position = spec.position(&params)?;
            }
        }
        state.stale = true;
        Ok(self)
    }

    fn insert_axes(&self, position: Bbox, spec: Option<SubplotSpec>) -> Axes {
        let mut axes = AxesState::new(position, &crate::rc::params());
        axes.subplot_spec = spec;
        let id = self
            .state
            .borrow_mut()
            .adopt(self.root.id(), Box::new(axes));
        Axes::from_raw(RawHandle::new(Rc::clone(&self.state), id))
    }

    /// Every axes of the figure, in creation order.
    pub fn axes(&self) -> Vec<Axes> {
        let state = self.state.borrow();
        let frame = state
            .store
            .get_as::<FigureFrame>(state.root)
            .expect("the root frame lives as long as the figure");
        frame
            .children
            .iter()
            .filter(|&&id| state.store.get_as::<AxesState>(id).is_some())
            .map(|&id| Axes::from_raw(RawHandle::new(Rc::clone(&self.state), id)))
            .collect()
    }

    /// Renders the figure through `renderer`.
    ///
    /// Everything drawable is drawn; artists with invalid properties (for
    /// example an unknown colour name) are skipped and reported.
    pub fn draw(&self, renderer: &mut dyn Renderer) -> Result<(), DrawErrors> {
        self.state.borrow_mut().prepare();
        let errors = self.state.borrow().draw(renderer);
        self.state.borrow_mut().stale = false;
        if errors.is_empty() {
            Ok(())
        } else {
            Err(DrawErrors(errors))
        }
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
