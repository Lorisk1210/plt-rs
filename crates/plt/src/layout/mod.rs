//! Layout engines: placing the axes of a figure so that their decorations
//! and colorbars fit, each time the figure is drawn or laid out
//! (matplotlib's `layout_engine` and `_constrained_layout`).
//!
//! A figure has no engine unless one is chosen with
//! [`Figure::set_layout_engine`] (or by the `figure.constrained_layout.use`
//! rc parameter). [`LayoutEngine::Constrained`] arranges the axes placed
//! on grids: every grid cell gets margins as large as the decorations of
//! the axes in it (tick labels, axis labels, titles) plus padding and room
//! for colorbars, and the cells share the rest of the figure.
//!
//! ```
//! use plt::layout::{ConstrainedLayout, LayoutEngine};
//! use plt::prelude::*;
//! let fig = Figure::new();
//! fig.set_layout_engine(LayoutEngine::constrained());
//! let ax = fig.subplot();
//! ax.set_title("title");
//! let (x, _, w, _) = ax.position().bounds();
//! assert!(x < 0.125 && x + w > 0.9, "tighter than the default margins");
//! ```

mod align;
mod constrained;
mod grid;
mod tight;
mod tight_grid;

pub use constrained::ConstrainedLayout;
pub use tight::{TightLayout, TightLayoutError};

pub(crate) use align::{Aligned, LabelGroups};

use plt_core::Anchor;

use crate::artist::{ArtistHandle, ArtistId};
use crate::axes_grid1::{Divider, InsetLocator};
use crate::figure::{Figure, FigureState};
use crate::rc::RcParams;
use crate::spines::SpineSide;

/// How a figure arranges its axes when it is drawn (matplotlib's layout
/// engines).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LayoutEngine {
    /// Constrained layout (matplotlib's `ConstrainedLayoutEngine`).
    Constrained(ConstrainedLayout),
    /// Tight layout at every draw (matplotlib's `TightLayoutEngine`,
    /// `layout="tight"` or `figure.autolayout`).
    Tight(TightLayout),
}

impl LayoutEngine {
    /// Constrained layout with its defaults (`layout="constrained"`).
    pub fn constrained() -> Self {
        Self::Constrained(ConstrainedLayout::default())
    }

    /// Constrained layout closing the gaps of fixed-aspect axes
    /// (`layout="compressed"`).
    pub fn compressed() -> Self {
        Self::Constrained(ConstrainedLayout {
            compress: true,
            ..ConstrainedLayout::default()
        })
    }

    /// Tight layout with its defaults (`layout="tight"`).
    pub fn tight() -> Self {
        Self::Tight(TightLayout::default())
    }

    /// Whether colorbars of grid-placed axes may take their space from the
    /// axes' grid cell (matplotlib's `colorbar_gridspec`).
    pub fn colorbar_gridspec(&self) -> bool {
        match self {
            Self::Constrained(_) => false,
            Self::Tight(_) => true,
        }
    }
}

/// Where a colorbar took its space from (matplotlib's `_colorbar_info`
/// and the parents' `_colorbars`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ColorbarLayout {
    /// The colorbar axes.
    pub(crate) cax: ArtistId,
    /// The axes the space was taken from.
    pub(crate) parents: Vec<ArtistId>,
    /// The side of the parents the colorbar is on.
    pub(crate) location: SpineSide,
    /// The fraction of the parents used.
    pub(crate) fraction: f64,
    /// The factor applied to the colorbar's length.
    pub(crate) shrink: f64,
    /// The ratio of the colorbar's long to short side.
    pub(crate) aspect: f64,
    /// The gap to the parents, as a fraction of them.
    pub(crate) pad: f64,
    /// Where the colorbar sits in its slot.
    pub(crate) anchor: Anchor,
}

/// What a figure keeps for its layout engine.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct LayoutState {
    /// The engine, if any.
    pub(crate) engine: Option<LayoutEngine>,
    /// Colorbars that took space from parent axes, in creation order.
    pub(crate) colorbars: Vec<ColorbarLayout>,
    /// Whether the engine must run before positions are used.
    pub(crate) pending: bool,
    /// The axes whose labels or titles align ([`Figure::align_labels`],
    /// [`Figure::align_titles`]).
    pub(crate) label_groups: LabelGroups,
    /// The dividers placing axes (`axes_grid1`), indexed by their ids.
    pub(crate) dividers: Vec<Divider>,
    /// The locators placing inset axes (`axes_grid1.inset_locator`),
    /// indexed by their ids.
    pub(crate) insets: Vec<InsetLocator>,
}

impl LayoutState {
    /// No colorbars, and the engine the rc parameters ask for:
    /// `figure.autolayout` before `figure.constrained_layout.use`, as in
    /// matplotlib's `set_layout_engine(None)`.
    pub(crate) fn from_rc(rc: &RcParams) -> Self {
        let engine = if rc.figure_autolayout {
            Some(LayoutEngine::tight())
        } else {
            rc.figure_constrained_layout_use
                .then(|| LayoutEngine::Constrained(ConstrainedLayout::from_rc(rc)))
        };
        Self {
            engine,
            colorbars: Vec::new(),
            pending: true,
            label_groups: LabelGroups::default(),
            dividers: Vec::new(),
            insets: Vec::new(),
        }
    }
}

impl Figure {
    /// Chooses how the figure arranges its axes when drawn (matplotlib's
    /// `set_layout_engine`); `None` keeps the axes where they are.
    ///
    /// ```
    /// use plt::figure::Figure;
    /// use plt::layout::LayoutEngine;
    /// let fig = Figure::new();
    /// fig.set_layout_engine(LayoutEngine::constrained());
    /// assert!(fig.layout_engine().is_some());
    /// fig.set_layout_engine(None);
    /// ```
    pub fn set_layout_engine(&self, engine: impl Into<Option<LayoutEngine>>) -> &Self {
        let mut state = self.raw().figure_state().borrow_mut();
        state.layout.engine = engine.into();
        state.layout.pending = true;
        state.stale = true;
        self
    }

    /// The layout engine (matplotlib's `get_layout_engine`).
    pub fn layout_engine(&self) -> Option<LayoutEngine> {
        self.raw().figure_state().borrow().layout.engine
    }
}

impl FigureState {
    /// Runs the layout engine if a layout pass is due: after the figure
    /// changed, and before every draw.
    pub(crate) fn run_pending_layout(&mut self) {
        // A window of the figure being printed was measured on the last
        // layout, which the print keeps (matplotlib turns the layout
        // engine off while printing).
        if !std::mem::take(&mut self.layout.pending) || self.print_window.is_some() {
            return;
        }
        match self.layout.engine {
            Some(LayoutEngine::Constrained(params)) => constrained::execute(self, &params),
            Some(LayoutEngine::Tight(params)) => {
                let axes = self.all_axes();
                // A figure tight layout cannot fit keeps its parameters,
                // as matplotlib's engine does (after a warning).
                let _ = self.tight_layout_pass(self.root, &axes, &params);
            }
            None => {}
        }
    }

    /// The colorbars that took space from `axes`, in creation order.
    pub(crate) fn colorbars_of(&self, axes: ArtistId) -> Vec<ColorbarLayout> {
        self.layout
            .colorbars
            .iter()
            .filter(|colorbar| colorbar.parents.contains(&axes))
            .filter(|colorbar| self.store.contains(colorbar.cax))
            .cloned()
            .collect()
    }
}
