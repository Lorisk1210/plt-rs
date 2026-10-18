//! Legends: a framed list of handles and labels describing what is
//! plotted (`matplotlib.legend` and `matplotlib.legend_handler`).
//!
//! [`Axes::legend`] lists the labelled artists and containers of an axes,
//! [`Figure::legend`] those of every axes of a figure; both take
//! [`LegendOptions`], matplotlib's keyword arguments, with unset options
//! read from the `legend.*` rcParams:
//!
//! ```
//! use plt::prelude::*;
//! use plt::legend::LegendOptions;
//! let ax = Figure::new().subplot();
//! ax.plot_with([0.0, 1.0], [0.0, 1.0], LineProps::default().label("rising"))?;
//! ax.plot_with([0.0, 1.0], [1.0, 0.0], LineProps::default().label("_hidden"))?;
//! let legend = ax.legend(LegendOptions::default().loc("upper left").title("Trends"))?;
//! assert_eq!(legend.labels(), ["rising"]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Like matplotlib's `Legend`, a legend is built from an offset-box tree
//! ([`crate::offsetbox`]): a title above columns of entries, each a handle
//! box beside a text. The handle of an entry is drawn by the handler of
//! the original artist's kind (lines, patches and bars, step patches,
//! polygon, line and path collections, error bar and stem containers),
//! which copies its look into new artists. Where the legend goes —
//! including `loc="best"`, the position covering the least data — is
//! decided in the figure's layout pass.

mod artist;
mod best;
mod build;
mod error;
mod handle;
mod handlers;
mod labelcolor;
mod layout;
mod options;
mod settings;
#[cfg(test)]
mod tests;

use std::rc::Rc;

pub(crate) use artist::LegendArtist;
pub use error::LegendError;
pub use handle::LegendHandle;
pub(crate) use layout::lay_out_legends;
pub use options::{
    BboxToAnchor, LegendAlignment, LegendLabelColor, LegendLoc, LegendMode, LegendOptions,
};
pub(crate) use settings::OutsideEdge;

use artist::LegendParent;
use settings::Placement;

use plt_core::Bbox;

use crate::artist::{Artist, ArtistHandle, ArtistId, LayoutContext, RawHandle};
use crate::axes::{Axes, AxesState};
use crate::figure::{Figure, FigureFrame, FigureState};
use crate::patches::{FancyBboxPatch, Patch};
use crate::text::{Text, TextArtist};

/// A handle to a legend (matplotlib's `Legend`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Legend {
    raw: RawHandle,
}

impl ArtistHandle for Legend {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl Legend {
    fn handle(&self, id: ArtistId) -> RawHandle {
        RawHandle::new(Rc::clone(self.raw.figure_state()), id)
    }

    fn read<R>(&self, f: impl FnOnce(&LegendArtist) -> R) -> R {
        self.raw.with::<LegendArtist, _>(f)
    }

    /// The label texts in entry order (matplotlib's `get_texts`).
    pub fn texts(&self) -> Vec<Text> {
        let ids = self.read(|legend| legend.texts.clone());
        ids.into_iter()
            .map(|id| Text::from_raw(self.handle(id)))
            .collect()
    }

    /// The label strings in entry order.
    pub fn labels(&self) -> Vec<String> {
        self.texts().iter().map(Text::text).collect()
    }

    /// The title text (matplotlib's `get_title`), empty when unset.
    pub fn title(&self) -> Text {
        Text::from_raw(self.handle(self.read(|legend| legend.title)))
    }

    /// Sets the title; an empty one hides it (matplotlib's `set_title`).
    pub fn set_title(&self, title: impl Into<String>) -> &Self {
        let title = title.into();
        let shown = !title.is_empty();
        let text = self.raw.with_mut::<LegendArtist, _>(|legend| {
            if let Some(title_box) = legend.title_box_mut() {
                title_box.set_visible(shown);
            }
            legend.title
        });
        self.handle(text).with_mut::<TextArtist, _>(|text| {
            text.text = title;
            text.base_mut().visible = shown;
        });
        self
    }

    /// The frame (matplotlib's `get_frame`).
    pub fn frame(&self) -> Patch<FancyBboxPatch> {
        Patch::from_raw(self.handle(self.read(|legend| legend.frame)))
    }

    /// Whether the frame is drawn (matplotlib's `get_frame_on`).
    pub fn frame_on(&self) -> bool {
        self.frame().visible()
    }

    /// Shows or hides the frame and its shadow (matplotlib's
    /// `set_frame_on`).
    pub fn set_frame_on(&self, on: bool) -> &Self {
        let (frame, shadow) = self.read(|legend| (legend.frame, legend.shadow));
        for id in std::iter::once(frame).chain(shadow) {
            self.handle(id).with_base_mut(|base| base.visible = on);
        }
        self
    }

    /// Moves the legend (matplotlib's `set_loc`).
    ///
    /// # Errors
    ///
    /// When `loc` is not a location, is `"best"` for a figure legend or
    /// `"outside ..."` for an axes legend.
    pub fn set_loc(&self, loc: impl Into<LegendLoc>) -> Result<&Self, LegendError> {
        let parent = self.read(|legend| legend.parent);
        let placement = Placement::parse(&loc.into(), matches!(parent, LegendParent::Figure(_)))?;
        self.raw.with_mut::<LegendArtist, _>(|legend| {
            legend.location = placement.location;
            legend.outside = placement.outside;
        });
        Ok(self)
    }
}

/// The edges and extents (display pixels) of the `"outside ..."` legends
/// of figure frame `frame` without a `bbox_to_anchor`, which constrained
/// layout makes room for.
pub(crate) fn outside_legends(state: &FigureState, frame: ArtistId) -> Vec<(OutsideEdge, Bbox)> {
    let Some(node) = state.store.get_as::<FigureFrame>(frame) else {
        return Vec::new();
    };
    let layout = LayoutContext::new(state);
    node.children
        .iter()
        .filter_map(|&id| {
            let legend = state.store.get_as::<LegendArtist>(id)?;
            let edge = legend.outside.filter(|_| legend.anchor.is_none())?;
            Some((edge, legend.window_extent(&layout)?))
        })
        .collect()
}

/// The legends among `children`.
fn legends_in(state: &FigureState, children: &[ArtistId]) -> Vec<ArtistId> {
    children
        .iter()
        .copied()
        .filter(|&id| state.store.get_as::<LegendArtist>(id).is_some())
        .collect()
}

impl Axes {
    /// Places a legend on the axes, replacing its previous one
    /// (matplotlib's `Axes.legend`). Without explicit handles it lists
    /// the artists and containers whose labels do not start with `_`.
    ///
    /// # Errors
    ///
    /// When an option is invalid.
    pub fn legend(&self, mut options: LegendOptions) -> Result<Legend, LegendError> {
        let figure = self.raw().figure_state();
        let mut state = figure.borrow_mut();
        let entries = handle::entries(
            &state.store,
            &[self.id()],
            options.handles.take(),
            options.labels.take(),
        );
        let previous = state
            .store
            .get_as::<AxesState>(self.id())
            .map(|axes| legends_in(&state, &axes.children))
            .unwrap_or_default();
        let id = build::build(&mut state, LegendParent::Axes(self.id()), entries, &options)?;
        for old in previous {
            state.remove_artist(old);
        }
        Ok(Legend {
            raw: RawHandle::new(Rc::clone(figure), id),
        })
    }

    /// The axes legend, if it has one (matplotlib's `get_legend`).
    pub fn get_legend(&self) -> Option<Legend> {
        let figure = self.raw().figure_state();
        let state = figure.borrow();
        let axes = state.store.get_as::<AxesState>(self.id())?;
        let id = legends_in(&state, &axes.children).into_iter().next()?;
        Some(Legend {
            raw: RawHandle::new(Rc::clone(figure), id),
        })
    }

    /// The handles and labels `legend()` would list (matplotlib's
    /// `get_legend_handles_labels`).
    pub fn legend_handles_labels(&self) -> (Vec<LegendHandle>, Vec<String>) {
        let state = self.raw().figure_state().borrow();
        handle::entries(&state.store, &[self.id()], None, None)
            .into_iter()
            .map(|(source, label)| (LegendHandle(source), label))
            .unzip()
    }
}

impl Figure {
    /// Places a legend on the figure (matplotlib's `Figure.legend`).
    /// Without explicit handles it lists the labelled artists and
    /// containers of every axes of the figure.
    ///
    /// # Errors
    ///
    /// When an option is invalid, including `loc="best"`.
    pub fn legend(&self, mut options: LegendOptions) -> Result<Legend, LegendError> {
        let axes: Vec<ArtistId> = self.axes().iter().map(ArtistHandle::id).collect();
        let figure = self.raw().figure_state();
        let mut state = figure.borrow_mut();
        let entries = handle::entries(
            &state.store,
            &axes,
            options.handles.take(),
            options.labels.take(),
        );
        let id = build::build(
            &mut state,
            LegendParent::Figure(self.id()),
            entries,
            &options,
        )?;
        Ok(Legend {
            raw: RawHandle::new(Rc::clone(figure), id),
        })
    }

    /// The figure legends in the order they were made (matplotlib's
    /// `fig.legends`).
    pub fn legends(&self) -> Vec<Legend> {
        let figure = self.raw().figure_state();
        let state = figure.borrow();
        let ids = state
            .store
            .get_as::<FigureFrame>(self.id())
            .map(|frame| legends_in(&state, &frame.children))
            .unwrap_or_default();
        ids.into_iter()
            .map(|id| Legend {
                raw: RawHandle::new(Rc::clone(figure), id),
            })
            .collect()
    }
}
