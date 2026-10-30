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
//! Once a colorbar exists, the engine can only be swapped for one that
//! places colorbars the same way; removing the engine leaves a
//! [`LayoutEngine::PlaceHolder`] that remembers how.
//!
//! ```
//! use pltrs::layout::{ConstrainedLayout, LayoutEngine};
//! use pltrs::prelude::*;
//! let fig = Figure::new();
//! fig.set_layout_engine(LayoutEngine::constrained())?;
//! let ax = fig.subplot();
//! ax.set_title("title");
//! let (x, _, w, _) = ax.position().bounds();
//! assert!(x < 0.125 && x + w > 0.9, "tighter than the default margins");
//! # Ok::<(), pltrs::layout::LayoutEngineError>(())
//! ```

mod align;
mod constrained;
mod grid;
mod tight;
mod tight_grid;

pub use constrained::ConstrainedLayout;
pub use tight::{TightLayout, TightLayoutError};

pub(crate) use align::{Aligned, LabelGroups};

use std::fmt;
use std::str::FromStr;

use pltrs_core::Anchor;

use crate::artist::{ArtistHandle, ArtistId};
use crate::axes_grid1::{DividerState, InsetLocator};
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
    /// No layout, standing in for a removed engine (matplotlib's
    /// `PlaceHolderLayoutEngine`, what `set_layout_engine("none")` and
    /// `tight_layout` leave behind).
    PlaceHolder(PlaceHolderLayout),
}

pltrs_core::archive_enum!(LayoutEngine {
    Constrained(v0),
    Tight(v0),
    PlaceHolder(v0),
});

/// An engine that places nothing but keeps the colorbar placement and
/// the subplot-parameter compatibility of the engine it replaced, so that
/// an engine placing colorbars differently cannot be set once a colorbar
/// exists (matplotlib's `PlaceHolderLayoutEngine`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlaceHolderLayout {
    /// Whether the subplot parameters may still be changed by hand
    /// (matplotlib's `adjust_compatible`).
    pub adjust_compatible: bool,
    /// Whether colorbars of grid-placed axes take their space from the
    /// axes' grid cell (matplotlib's `colorbar_gridspec`).
    pub colorbar_gridspec: bool,
}

pltrs_core::archive_struct!(PlaceHolderLayout {
    adjust_compatible,
    colorbar_gridspec
});

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
            Self::PlaceHolder(placeholder) => placeholder.colorbar_gridspec,
        }
    }

    /// Whether the engine works with the subplot parameters, so that
    /// changing them by hand (`subplots_adjust`) still means something
    /// (matplotlib's `adjust_compatible`).
    pub fn adjust_compatible(&self) -> bool {
        match self {
            Self::Constrained(_) => false,
            Self::Tight(_) => true,
            Self::PlaceHolder(placeholder) => placeholder.adjust_compatible,
        }
    }

    /// The placeholder standing in for this engine once it is removed.
    fn placeholder(&self) -> Self {
        Self::PlaceHolder(PlaceHolderLayout {
            adjust_compatible: self.adjust_compatible(),
            colorbar_gridspec: self.colorbar_gridspec(),
        })
    }
}

impl FromStr for LayoutEngine {
    type Err = UnknownLayout;

    /// The engine named `name` with its defaults (matplotlib's
    /// `layout="constrained"`, `"compressed"` or `"tight"`).
    ///
    /// ```
    /// use pltrs::layout::LayoutEngine;
    /// assert_eq!("compressed".parse(), Ok(LayoutEngine::compressed()));
    /// assert!("foobar".parse::<LayoutEngine>().is_err());
    /// ```
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "constrained" => Ok(Self::constrained()),
            "compressed" => Ok(Self::compressed()),
            "tight" => Ok(Self::tight()),
            _ => Err(UnknownLayout(name.to_owned())),
        }
    }
}

/// A layout engine name other than `"constrained"`, `"compressed"` and
/// `"tight"`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownLayout(pub String);

impl fmt::Display for UnknownLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid layout {:?}: expected constrained, compressed or tight",
            self.0
        )
    }
}

impl std::error::Error for UnknownLayout {}

/// Why a figure kept its layout engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LayoutEngineError {
    /// A colorbar exists and the new engine places colorbars differently
    /// from the current one (their `colorbar_gridspec` differ), which
    /// matplotlib refuses with a `RuntimeError`.
    IncompatibleColorbars,
}

impl fmt::Display for LayoutEngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncompatibleColorbars => f.write_str(
                "Colorbar layout of new layout engine not compatible with old engine, and a \
                 colorbar has been created.  Engine not changed.",
            ),
        }
    }
}

impl std::error::Error for LayoutEngineError {}

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

pltrs_core::archive_struct!(ColorbarLayout {
    cax,
    parents,
    location,
    fraction,
    shrink,
    aspect,
    pad,
    anchor
});

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
    pub(crate) dividers: Vec<DividerState>,
    /// The locators placing inset axes (`axes_grid1.inset_locator`),
    /// indexed by their ids.
    pub(crate) insets: Vec<InsetLocator>,
}

pltrs_core::archive_struct!(LayoutState {
    engine,
    colorbars,
    pending,
    label_groups,
    dividers,
    insets
});

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

impl LayoutState {
    /// Replaces the engine with `engine`, or with a placeholder for the
    /// current one when `None` (matplotlib's `"none"`), unless a colorbar
    /// exists (`has_colorbars`) and the two place colorbars differently
    /// (matplotlib's `_check_layout_engines_compat`).
    pub(crate) fn swap_engine(
        &mut self,
        engine: Option<LayoutEngine>,
        has_colorbars: bool,
    ) -> Result<(), LayoutEngineError> {
        let Some(new) = engine else {
            self.remove_engine();
            return Ok(());
        };
        if let Some(old) = self.engine
            && old.colorbar_gridspec() != new.colorbar_gridspec()
            && has_colorbars
        {
            return Err(LayoutEngineError::IncompatibleColorbars);
        }
        self.engine = Some(new);
        self.pending = true;
        Ok(())
    }

    /// Replaces the engine, if any, with a placeholder for it.
    pub(crate) fn remove_engine(&mut self) {
        self.engine = self.engine.map(|old| old.placeholder());
        self.pending = true;
    }
}

impl Figure {
    /// Chooses how the figure arranges its axes when drawn (matplotlib's
    /// `set_layout_engine`). `None` removes the engine (matplotlib's
    /// `"none"`) and keeps the axes where they are; a figure that had an
    /// engine keeps a [`LayoutEngine::PlaceHolder`] for it.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::layout::LayoutEngine;
    /// let fig = Figure::new();
    /// fig.set_layout_engine(LayoutEngine::constrained())?;
    /// assert!(fig.layout_engine().is_some());
    /// fig.set_layout_engine(None)?;
    /// assert!(matches!(fig.layout_engine(), Some(LayoutEngine::PlaceHolder(_))));
    /// # Ok::<(), pltrs::layout::LayoutEngineError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`LayoutEngineError::IncompatibleColorbars`] when a colorbar exists
    /// and the new engine places colorbars differently from the current
    /// one (constrained layout beside the axes, tight layout and no engine
    /// in the axes' grid cell); the engine is then left unchanged.
    pub fn set_layout_engine(
        &self,
        engine: impl Into<Option<LayoutEngine>>,
    ) -> Result<&Self, LayoutEngineError> {
        let mut state = self.raw().figure_state().borrow_mut();
        let has_colorbars = crate::colorbar::has_colorbars(&state.store);
        state.layout.swap_engine(engine.into(), has_colorbars)?;
        state.stale = true;
        Ok(self)
    }

    /// The layout engine (matplotlib's `get_layout_engine`).
    pub fn layout_engine(&self) -> Option<LayoutEngine> {
        self.raw().figure_state().borrow().layout.engine
    }

    /// Lays the figure out once with `engine`, whatever engine the figure
    /// has, which it keeps (matplotlib's `LayoutEngine.execute(fig)`).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::layout::LayoutEngine;
    /// let fig = Figure::new();
    /// fig.set_layout_engine(None)?;
    /// fig.subplot().set_title("title");
    /// let (own, params) = (fig.layout_engine(), fig.subplot_params());
    /// fig.execute_layout(LayoutEngine::tight());
    /// assert_ne!(fig.subplot_params(), params, "the tight layout moved the grid");
    /// assert_eq!(fig.layout_engine(), own);
    /// # Ok::<(), pltrs::layout::LayoutEngineError>(())
    /// ```
    pub fn execute_layout(&self, engine: LayoutEngine) {
        let mut state = self.raw().figure_state().borrow_mut();
        let own = state.layout.engine.replace(engine);
        state.layout.pending = true;
        state.prepare();
        state.layout.engine = own;
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
                let mut axes = self.all_axes();
                axes.extend(crate::mplot3d::axes3d_ids(&self.store));
                // A figure tight layout cannot fit keeps its parameters,
                // as matplotlib's engine does (after a warning).
                let _ = self.tight_layout_pass(self.root, &axes, &params);
            }
            Some(LayoutEngine::PlaceHolder(_)) | None => {}
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::ImshowOptions;
    use crate::colorbar::ColorbarOptions;
    use crate::numeric::Grid;

    /// A figure laid out by `engine` holding an image and its colorbar.
    fn with_colorbar(engine: LayoutEngine) -> Figure {
        let fig = Figure::new();
        fig.set_layout_engine(engine).unwrap();
        let data = Grid::from_vec(2, 2, vec![1.0, 2.0, 3.0, 4.0]).unwrap();
        let image = fig
            .subplot()
            .imshow(data, ImshowOptions::default())
            .unwrap();
        fig.colorbar(&image, ColorbarOptions::default()).unwrap();
        fig
    }

    #[test]
    fn engines_swap_freely_without_colorbars() {
        let fig = Figure::new();
        fig.set_layout_engine(LayoutEngine::constrained()).unwrap();
        fig.set_layout_engine(LayoutEngine::tight()).unwrap();
        assert_eq!(fig.layout_engine(), Some(LayoutEngine::tight()));
        fig.set_layout_engine(None).unwrap();
        fig.set_layout_engine(LayoutEngine::constrained()).unwrap();
        assert_eq!(fig.layout_engine(), Some(LayoutEngine::constrained()));
    }

    #[test]
    fn removing_an_engine_leaves_a_placeholder_for_it() {
        let fig = Figure::new();
        fig.set_layout_engine(None).unwrap();
        assert_eq!(fig.layout_engine(), None, "nothing to stand in for");
        fig.set_layout_engine(LayoutEngine::constrained()).unwrap();
        fig.set_layout_engine(None).unwrap();
        let placeholder = PlaceHolderLayout {
            adjust_compatible: false,
            colorbar_gridspec: false,
        };
        assert_eq!(
            fig.layout_engine(),
            Some(LayoutEngine::PlaceHolder(placeholder))
        );
    }

    #[test]
    fn colorbars_pin_how_colorbars_are_placed() {
        let fig = with_colorbar(LayoutEngine::constrained());
        let refused = fig.set_layout_engine(LayoutEngine::tight());
        assert_eq!(
            refused.err(),
            Some(LayoutEngineError::IncompatibleColorbars)
        );
        assert_eq!(fig.layout_engine(), Some(LayoutEngine::constrained()));
        fig.set_layout_engine(LayoutEngine::compressed()).unwrap();
        fig.set_layout_engine(None).unwrap();
        assert!(fig.set_layout_engine(LayoutEngine::tight()).is_err());
        fig.set_layout_engine(LayoutEngine::constrained()).unwrap();

        let fig = with_colorbar(LayoutEngine::tight());
        assert!(fig.set_layout_engine(LayoutEngine::constrained()).is_err());
        fig.set_layout_engine(None).unwrap();
        fig.set_layout_engine(LayoutEngine::tight()).unwrap();
    }

    #[test]
    fn tight_layout_leaves_a_placeholder_and_respects_colorbars() {
        let fig = Figure::new();
        fig.subplot();
        fig.tight_layout(TightLayout::default()).unwrap();
        assert!(matches!(
            fig.layout_engine(),
            Some(LayoutEngine::PlaceHolder(PlaceHolderLayout {
                colorbar_gridspec: true,
                ..
            }))
        ));

        let fig = with_colorbar(LayoutEngine::constrained());
        assert_eq!(
            fig.tight_layout(TightLayout::default()),
            Err(TightLayoutError::Engine(
                LayoutEngineError::IncompatibleColorbars
            ))
        );
        assert!(
            matches!(
                fig.layout_engine(),
                Some(LayoutEngine::PlaceHolder(placeholder)) if !placeholder.colorbar_gridspec
            ),
            "the constrained engine is removed all the same"
        );
    }

    #[test]
    fn constrained_figures_can_keep_their_engine_through_tight_layout() {
        crate::rc::context(|| {
            crate::rc::set("pltrs.tight_layout.keep_constrained", "True").unwrap();
            let fig = with_colorbar(LayoutEngine::constrained());
            fig.tight_layout(TightLayout::default()).unwrap();
            assert!(matches!(
                fig.layout_engine(),
                Some(LayoutEngine::Constrained(_))
            ));
            // Other figures still take one tight layout pass.
            let fig = Figure::new();
            fig.subplot();
            fig.tight_layout(TightLayout::default()).unwrap();
            assert!(matches!(
                fig.layout_engine(),
                Some(LayoutEngine::PlaceHolder(_))
            ));
        });
    }
}
