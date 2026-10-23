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
//! box beside a text. The handle of an entry is drawn by a
//! [`LegendHandler`]: the one the [`HandlerMap`] of the legend (on top of
//! the default map) holds for the original artist's kind — lines,
//! patches and bars, step patches, polygon, line and path collections,
//! error bar and stem containers, tuples — which copies its look into new
//! artists, or one of the user's own. Where the legend goes — including
//! `loc="best"`, the position covering the least data — is decided in the
//! figure's layout pass.

mod artist;
mod best;
mod build;
mod drag;
mod error;
mod handle;
mod handlers;
mod labelcolor;
mod layout;
mod options;
mod settings;
#[cfg(test)]
mod tests;

use std::cell::RefCell;
use std::rc::Rc;

pub(crate) use artist::LegendArtist;
pub use error::LegendError;
pub use handle::{HandlerTuple, LegendHandle};
pub use handlers::{
    HandleBox, HandlerArtist, HandlerEntry, HandlerErrorbar, HandlerKey, HandlerLine2D,
    HandlerLineCollection, HandlerMap, HandlerPatch, HandlerPathCollection, HandlerPolyCollection,
    HandlerStem, HandlerStepPatch, LegendHandler, TypeKey,
};
pub(crate) use layout::lay_out_legends;
pub use options::{
    BboxToAnchor, LegendAlignment, LegendLabelColor, LegendLoc, LegendMode, LegendOptions,
    LegendShadow, LegendShadowStyle,
};
pub(crate) use settings::OutsideEdge;

use artist::LegendParent;
use settings::{Location, Placement};

use plt_core::Bbox;

use crate::artist::{Artist, ArtistHandle, ArtistId, LayoutContext, RawHandle};
use crate::axes::{Axes, AxesState};
use crate::figure::{Figure, FigureFrame, FigureState};
use crate::patches::{FancyBboxPatch, Patch};
use crate::text::{Text, TextArtist};
use crate::transforms::Transform;

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

    /// The handle of the new legend `id`, draggable if asked.
    fn made(figure: &Rc<RefCell<FigureState>>, id: ArtistId, draggable: bool) -> Self {
        let legend = Self {
            raw: RawHandle::new(Rc::clone(figure), id),
        };
        legend.set_draggable(draggable);
        legend
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
        let (frame, shadow) = self.read(|legend| (legend.frame, legend.shadow_id()));
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

    /// Where the legend goes (matplotlib's `_loc`): `"best"`, the code of
    /// an anchored position (an `"outside ..."` name gives the position
    /// inside its edge), or the lower-left corner in fractions of the
    /// anchor box.
    pub fn loc(&self) -> LegendLoc {
        match self.read(|legend| legend.location) {
            Location::Best => LegendLoc::Name("best".to_owned()),
            Location::Anchored(anchor) => LegendLoc::Code(i64::from(anchor.code())),
            Location::Point(x, y) => LegendLoc::Point(x, y),
        }
    }

    /// The box `loc` refers to, with its coordinates, when it is not the
    /// parent axes or figure (matplotlib's `get_bbox_to_anchor`, which
    /// gives the box in display coordinates).
    pub fn bbox_to_anchor(&self) -> Option<(Bbox, Transform)> {
        self.read(|legend| legend.anchor.clone())
    }

    /// Places the legend relative to `anchor` in the coordinates of
    /// `transform` (by default fractions of the parent), or back in its
    /// parent's box with `None` (matplotlib's `set_bbox_to_anchor`).
    pub fn set_bbox_to_anchor(
        &self,
        anchor: Option<BboxToAnchor>,
        transform: Option<Transform>,
    ) -> &Self {
        self.raw.with_mut::<LegendArtist, _>(|legend| {
            let fractions = legend.parent.fractions();
            legend.anchor = anchor.map(|anchor| (anchor.bbox(), transform.unwrap_or(fractions)));
        });
        self
    }

    /// The handlers the legend draws with: the default handler map with
    /// those the legend was given added (matplotlib's
    /// `get_legend_handler_map`).
    pub fn handler_map(&self) -> HandlerMap {
        handlers::with_defaults(&self.read(|legend| legend.handler_map.clone()))
    }

    /// The handler the legend draws `handle` with, if any: the handle's
    /// own, else the one its kind has in [`handler_map`](Self::handler_map)
    /// (matplotlib's `get_legend_handler`).
    pub fn legend_handler(&self, handle: &LegendHandle) -> Option<Rc<dyn LegendHandler>> {
        let map = self.handler_map();
        let state = self.raw.figure_state().borrow();
        map.handler_for(&handle.0, &state.store)
    }

    /// The handler map every new legend starts from (matplotlib's
    /// `Legend.get_default_handler_map`), per thread like the rc
    /// parameters.
    pub fn default_handler_map() -> HandlerMap {
        handlers::default_handlers()
    }

    /// Replaces the handler map every new legend starts from (matplotlib's
    /// `Legend.set_default_handler_map`).
    pub fn set_default_handler_map(map: HandlerMap) {
        handlers::set_default_handlers(map);
    }

    /// Adds the handlers of `map` to the handler map every new legend
    /// starts from (matplotlib's `Legend.update_default_handler_map`).
    pub fn update_default_handler_map(map: &HandlerMap) {
        handlers::update_default_handlers(map);
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

/// The legends among `children` that are the axes legend, not kept as
/// ordinary artists (at most one).
fn axes_legends(state: &FigureState, children: &[ArtistId]) -> Vec<ArtistId> {
    legends_in(state, children)
        .into_iter()
        .filter(|&id| {
            state
                .store
                .get_as::<LegendArtist>(id)
                .is_some_and(|legend| !legend.kept)
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
        let handlers = handlers::with_defaults(&options.handler_map);
        let figure = self.raw().figure_state();
        let mut state = figure.borrow_mut();
        let entries = handle::entries(
            &state.store,
            &[self.id()],
            options.handles.take(),
            options.labels.take(),
            &handlers,
        );
        let previous = state
            .store
            .get_as::<AxesState>(self.id())
            .map(|axes| axes_legends(&state, &axes.children))
            .unwrap_or_default();
        let id = build::build(
            &mut state,
            LegendParent::Axes(self.id()),
            entries,
            &options,
            &handlers,
        )?;
        for old in previous {
            state.remove_artist(old);
        }
        drop(state);
        Ok(Legend::made(figure, id, options.draggable))
    }

    /// The axes legend, if it has one (matplotlib's `get_legend`).
    pub fn get_legend(&self) -> Option<Legend> {
        let figure = self.raw().figure_state();
        let state = figure.borrow();
        let axes = state.store.get_as::<AxesState>(self.id())?;
        let id = axes_legends(&state, &axes.children).into_iter().next()?;
        Some(Legend {
            raw: RawHandle::new(Rc::clone(figure), id),
        })
    }

    /// Keeps `legend` on the axes as an ordinary artist, so that the next
    /// [`Axes::legend`] adds a legend beside it instead of replacing it
    /// (matplotlib's `ax.add_artist(legend)` for showing several legends).
    /// The kept legend is no longer the one [`Axes::get_legend`] returns.
    pub fn keep_legend(&self, legend: &Legend) {
        legend
            .raw
            .with_mut::<LegendArtist, _>(|legend| legend.kept = true);
    }

    /// The handles and labels `legend()` would list (matplotlib's
    /// `get_legend_handles_labels`).
    pub fn legend_handles_labels(&self) -> (Vec<LegendHandle>, Vec<String>) {
        let handlers = handlers::default_handlers();
        let state = self.raw().figure_state().borrow();
        handle::entries(&state.store, &[self.id()], None, None, &handlers)
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
        let handlers = handlers::with_defaults(&options.handler_map);
        let axes: Vec<ArtistId> = self.axes().iter().map(ArtistHandle::id).collect();
        let figure = self.raw().figure_state();
        let mut state = figure.borrow_mut();
        let entries = handle::entries(
            &state.store,
            &axes,
            options.handles.take(),
            options.labels.take(),
            &handlers,
        );
        let id = build::build(
            &mut state,
            LegendParent::Figure(self.id()),
            entries,
            &options,
            &handlers,
        )?;
        drop(state);
        Ok(Legend::made(figure, id, options.draggable))
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
