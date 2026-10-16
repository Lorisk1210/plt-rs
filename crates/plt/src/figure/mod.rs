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

use crate::artist::{ArtistHandle, RawHandle};
use crate::axes::{Axes, AxesState};

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
        self.root.with_mut::<FigureFrame, _>(|frame| frame.facecolor = color);
        self
    }

    /// The background colour.
    pub fn facecolor(&self) -> Rgba {
        self.root.with::<FigureFrame, _>(|frame| frame.facecolor)
    }

    /// Adds axes occupying `[left, bottom, width, height]` in figure fractions.
    pub fn add_axes(&self, rect: [f64; 4]) -> Axes {
        let [left, bottom, width, height] = rect;
        let position = Bbox::from_bounds(left, bottom, width, height);
        let id = self
            .state
            .borrow_mut()
            .adopt(self.root.id(), Box::new(AxesState::new(position)));
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
    pub fn draw(&self, renderer: &mut dyn Renderer) {
        let mut state = self.state.borrow_mut();
        state.draw(renderer);
        state.stale = false;
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
