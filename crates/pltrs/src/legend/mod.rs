//! Legends: a framed list of handles and labels describing what is
//! plotted (`matplotlib.legend` and `matplotlib.legend_handler`).
//!
//! [`Axes::legend`] lists the labelled artists and containers of an axes,
//! [`Figure::legend`] those of every axes of a figure; both take
//! [`LegendOptions`], matplotlib's keyword arguments, with unset options
//! read from the `legend.*` rcParams:
//!
//! ```
//! use pltrs::prelude::*;
//! use pltrs::legend::LegendOptions;
//! use pltrs::offsetbox::AnchorLoc;
//! let ax = Figure::new().subplot();
//! ax.plot_with([0.0, 1.0], [0.0, 1.0], LineProps::default().label("rising"))?;
//! ax.plot_with([0.0, 1.0], [1.0, 0.0], LineProps::default().label("_hidden"))?;
//! let legend = ax.legend(LegendOptions::default().loc(AnchorLoc::UpperLeft).title("Trends"))?;
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
mod box_offset;
mod build;
mod drag;
pub(crate) use drag::reconnect_drag;
mod error;
mod handle;
mod handlers;
mod labelcolor;
mod layout;
mod loc;
mod made;
mod options;
mod proxies;
mod settings;
#[cfg(test)]
mod tests;

use std::rc::Rc;

pub(crate) use artist::LegendArtist;
pub use drag::LegendDragUpdate;
pub use error::LegendError;
pub use handle::{HandlerTuple, LegendHandle};
pub use handlers::{
    Attach, HandleBox, HandlerArtist, HandlerEntry, HandlerErrorbar, HandlerKey, HandlerLine2D,
    HandlerLineCollection, HandlerMap, HandlerPatch, HandlerPathCollection, HandlerPolyCollection,
    HandlerStem, HandlerStepPatch, LegendHandler, TypeKey,
};
pub(crate) use layout::{lay_out_legend, lay_out_legends};
pub use loc::{LegendLoc, OutsideEdge};
pub use options::{
    BboxToAnchor, LegendAlignment, LegendLabelColor, LegendMode, LegendOptions, LegendShadow,
    LegendShadowStyle,
};
pub use settings::MAX_HANDLE_POINTS;

use artist::LegendParent;
use settings::Placement;

use pltrs_core::Bbox;

use crate::artist::{
    Artist, ArtistHandle, ArtistId, ArtistLookup, LayoutContext, RawHandle, TypedHandle,
};
use crate::axes::{Axes, AxesState};
use crate::figure::{Figure, FigureCell, FigureFrame, FigureState};
use crate::offsetbox::{OffsetBox, PackAlign};
use crate::patches::{FancyBboxPatch, Patch};
use crate::text::{Text, TextArtist};
use crate::transforms::Transform;

/// A handle to a legend (matplotlib's `Legend`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Legend {
    raw: RawHandle,
}

impl TypedHandle for Legend {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        raw.is::<LegendArtist>().then(|| Self { raw: raw.clone() })
    }
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
    fn made(figure: &Rc<FigureCell>, id: ArtistId, draggable: bool) -> Self {
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

    /// The title and the columns of entries as the legend lays them out,
    /// its artists as their handles (matplotlib's `_legend_box`, the first
    /// of the legend's children).
    ///
    /// ```
    /// use pltrs::offsetbox::OffsetBox;
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.plot_with([0.0, 1.0], [0.0, 1.0], LineProps::default().label("line"))?;
    /// let legend = ax.legend(Default::default())?;
    /// let OffsetBox::VPacker(packer) = legend.legend_box() else {
    ///     panic!("the title above the entries");
    /// };
    /// assert_eq!(packer.children.len(), 2);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn legend_box(&self) -> OffsetBox<RawHandle> {
        self.read(|legend| legend.legend_box.clone())
            .map(&mut |part| self.handle(part.id))
    }

    /// The artists drawing the handle of every listed handle, in order,
    /// none for handles no handler draws (the artists of matplotlib's
    /// handle boxes, which a handler's `legend_artist` adds).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let line = ax.plot_with([0.0, 1.0], [0.0, 1.0], LineProps::default().label("rising"))?;
    /// ax.plot_with([0.0, 1.0], [1.0, 0.0], LineProps::default().label("falling"))?;
    /// let legend = ax.legend(Default::default())?;
    /// let artists = legend.entry_artists();
    /// assert_eq!(artists.len(), 2);
    /// assert_eq!(artists[0].len(), 1, "the line of the first entry");
    /// assert_ne!(artists[0][0], line.raw().clone(), "a copy of the plotted line");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn entry_artists(&self) -> Vec<Vec<RawHandle>> {
        self.read(|legend| legend.handles.clone())
            .into_iter()
            .map(|ids| ids.into_iter().map(|id| self.handle(id)).collect())
            .collect()
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
    /// When `loc` is `"best"` for a figure legend or `"outside ..."` for
    /// an axes legend.
    pub fn set_loc(&self, loc: impl Into<LegendLoc>) -> Result<&Self, LegendError> {
        self.place(loc.into(), false)
    }

    /// Moves the legend back to the default location, `legend.loc` (or
    /// `"upper right"` for a figure legend when it asks for `"best"`),
    /// which counts as no choice of the program's: a slow search for the
    /// best location then warns (matplotlib's `set_loc(None)`).
    ///
    /// ```
    /// use pltrs::legend::LegendOptions;
    /// use pltrs::lines::LineProps;
    /// use pltrs::offsetbox::AnchorLoc;
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.plot_with([0.0, 1.0], [0.0, 1.0], LineProps::default().label("line"))?;
    /// let legend = ax.legend(LegendOptions::default().loc(AnchorLoc::LowerLeft))?;
    /// legend.reset_loc()?;
    /// assert_eq!(legend.loc(), pltrs::rc::params().legend_loc.loc);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// As [`Legend::set_loc`].
    pub fn reset_loc(&self) -> Result<&Self, LegendError> {
        let parent = self.read(|legend| legend.parent);
        self.place(build::default_loc(parent, &crate::rc::params()), true)
    }

    /// Places the legend at `loc`, the default location or not.
    fn place(&self, loc: LegendLoc, used_default: bool) -> Result<&Self, LegendError> {
        let parent = self.read(|legend| legend.parent);
        let placement = Placement::of(loc, matches!(parent, LegendParent::Figure(_)))?;
        self.raw.with_mut::<LegendArtist, _>(|legend| {
            legend.location = placement.location;
            legend.loc_used_default = used_default;
            legend.outside = placement.outside;
            legend.box_offset = None;
        });
        Ok(self)
    }

    /// How the title and the block of entries line up (matplotlib's
    /// `get_alignment`).
    pub fn alignment(&self) -> LegendAlignment {
        self.read(|legend| legend.alignment)
    }

    /// Packs the title and the block of entries anew, as a program's
    /// replacement for matplotlib's `_legend_box` packs them (legendkit
    /// puts the title left of, right of or below the entries): in a
    /// vertical packer, else a horizontal one, the title first or last,
    /// with `pad` around and `sep` between them in points, aligned by
    /// `align`.
    ///
    /// ```
    /// use pltrs::offsetbox::{OffsetBox, PackAlign};
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.plot_with([0.0, 1.0], [0.0, 1.0], LineProps::default().label("line"))?;
    /// let legend = ax.legend(Default::default())?;
    /// legend.set_title("T");
    /// let tall = legend.window_extent().expect("laid out");
    /// legend.repack(false, (4.0, 2.0), PackAlign::Center, true);
    /// assert!(matches!(legend.legend_box(), OffsetBox::HPacker(_)));
    /// let wide = legend.window_extent().expect("laid out");
    /// assert!(wide.width() > tall.width() && wide.height() < tall.height());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn repack(
        &self,
        vertical: bool,
        spacing: (f64, f64),
        align: PackAlign,
        title_first: bool,
    ) -> &Self {
        self.raw.with_mut::<LegendArtist, _>(|legend| {
            legend.repack(vertical, spacing, align, title_first);
        });
        self
    }

    /// Lines up the title and the block of entries as `alignment` says;
    /// the entries stay one block, so their handles line up (matplotlib's
    /// `set_alignment`).
    pub fn set_alignment(&self, alignment: LegendAlignment) -> &Self {
        self.raw
            .with_mut::<LegendArtist, _>(|legend| legend.set_alignment(alignment));
        self
    }

    /// Where the legend goes, as it was given or taken from `legend.loc`
    /// (matplotlib keeps its [`code`](LegendLoc::code) as `_loc`).
    pub fn loc(&self) -> LegendLoc {
        self.read(|legend| {
            Placement {
                location: legend.location,
                outside: legend.outside,
            }
            .loc()
        })
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

/// The legends among `children` that are the axes legend (at most one).
fn axes_legends(state: &FigureState, children: &[ArtistId]) -> Vec<ArtistId> {
    legends_in(state, children)
        .into_iter()
        .filter(|&id| {
            state
                .artist_as::<LegendArtist>(id)
                .is_some_and(|legend| legend.current)
        })
        .collect()
}

/// Makes `old`, the axes legend until now, no longer the axes legend: an
/// ordinary artist of the axes if it was kept as one, else removed.
fn retire_axes_legend(state: &mut FigureState, old: ArtistId) {
    let kept = state
        .artist_as_mut::<LegendArtist>(old)
        .is_some_and(|legend| {
            legend.current = false;
            legend.kept
        });
    if !kept {
        state.remove_artist(old);
    }
}

/// The legends among `children`.
fn legends_in(state: &FigureState, children: &[ArtistId]) -> Vec<ArtistId> {
    children
        .iter()
        .copied()
        .filter(|&id| state.artist_as::<LegendArtist>(id).is_some())
        .collect()
}

impl Axes {
    /// Places a legend on the axes, replacing its previous one
    /// (matplotlib's `Axes.legend`). Without explicit handles it lists
    /// the artists and containers whose labels do not start with `_`.
    ///
    /// # Errors
    ///
    /// When an option is invalid, such as more than [`MAX_HANDLE_POINTS`]
    /// markers per entry.
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
            .artist_as::<AxesState>(self.id())
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
            retire_axes_legend(&mut state, old);
        }
        drop(state);
        Ok(Legend::made(figure, id, options.draggable))
    }

    /// The axes legend, if it has one (matplotlib's `get_legend`;
    /// [`Axes::legend`] makes one).
    pub fn current_legend(&self) -> Option<Legend> {
        let figure = self.raw().figure_state();
        let state = figure.borrow();
        let axes = state.artist_as::<AxesState>(self.id())?;
        let id = axes_legends(&state, &axes.children).into_iter().next()?;
        Some(Legend {
            raw: RawHandle::new(Rc::clone(figure), id),
        })
    }

    /// Keeps `legend` on the axes as an ordinary artist, so that the next
    /// [`Axes::legend`] adds a legend beside it instead of replacing it
    /// (matplotlib's `ax.add_artist(legend)` for showing several legends).
    /// The axes legend kept so stays the one [`Axes::current_legend`]
    /// returns (and is drawn twice, as matplotlib draws it) until the next
    /// [`Axes::legend`]. A legend of the axes out of them (one a newer one
    /// replaced, one removed, or one made for them without becoming their
    /// legend, as matplotlib's `Legend(ax, ...)`) comes back on them as an
    /// ordinary artist only, drawn once.
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.plot_with([0.0, 1.0], [0.0, 1.0], LineProps::default().label("line"))?;
    /// let first = ax.legend(Default::default())?;
    /// let second = ax.legend(Default::default())?;
    /// assert!(!first.raw().is_attached(), "replaced");
    /// ax.keep_legend(&first);
    /// assert!(first.raw().is_attached() && second.raw().is_attached());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn keep_legend(&self, legend: &Legend) {
        let own = legend.raw.with_mut::<LegendArtist, _>(|kept| {
            kept.kept = true;
            // `add_artist` clips what it adds to the axes' patch unless it
            // has a clip path, which leaves the legend out of the axes'
            // tight box (its draw does not clip).
            if kept.base.clip_path.is_none() {
                kept.base.clip_box = Some(self.id());
            }
            kept.parent == LegendParent::Axes(self.id())
        });
        let id = legend.raw.id();
        let mut state = self.raw().figure_state().borrow_mut();
        if own && state.is_removed(id) {
            if let Some(kept) = state.artist_as_mut::<LegendArtist>(id) {
                kept.current = false;
            }
            state
                .attach_artist(self.id(), id)
                .expect("an axes takes back its own removed legend");
        }
    }

    /// Makes `legend`, a legend made for the axes, its axes legend again
    /// (matplotlib's `ax.legend_ = legend`), in place of the one it has:
    /// a legend that a newer one replaced, or one kept as an ordinary
    /// artist. Returns whether `legend` is a legend of the axes; a legend
    /// of another parent changes nothing.
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.plot_with([0.0, 1.0], [0.0, 1.0], LineProps::default().label("line"))?;
    /// let first = ax.legend(Default::default())?;
    /// let second = ax.legend(Default::default())?;
    /// assert!(ax.set_current_legend(&first));
    /// assert_eq!(ax.current_legend().map(|l| l.raw().id()), Some(first.raw().id()));
    /// assert!(!second.raw().is_attached(), "replaced");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_current_legend(&self, legend: &Legend) -> bool {
        let own = legend.raw.with_mut::<LegendArtist, _>(|made| {
            let own = made.parent == LegendParent::Axes(self.id());
            if own {
                made.current = true;
            }
            own
        });
        if !own {
            return false;
        }
        let id = legend.raw.id();
        let mut state = self.raw().figure_state().borrow_mut();
        let previous = state
            .artist_as::<AxesState>(self.id())
            .map(|axes| axes_legends(&state, &axes.children))
            .unwrap_or_default();
        for old in previous.into_iter().filter(|&old| old != id) {
            retire_axes_legend(&mut state, old);
        }
        if state.is_removed(id) {
            state
                .attach_artist(self.id(), id)
                .expect("an axes takes back its own removed legend");
        }
        true
    }

    /// The artists and containers a legend of the axes can show, with
    /// their labels, hidden ones included (matplotlib's
    /// `_get_legend_handles`, from which `legend(labels)` takes as many
    /// handles as there are labels); those labelled `_nolegend_` are left
    /// out.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.plot([0.0, 1.0], [0.0, 1.0])?;
    /// assert_eq!(ax.legend_candidates().len(), 1);
    /// assert!(ax.legend_handles_labels().0.is_empty(), "the line has no label");
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn legend_candidates(&self) -> Vec<(LegendHandle, String)> {
        let handlers = handlers::default_handlers();
        let state = self.raw().figure_state().borrow();
        handle::candidates(&state.store, &[self.id()], &handlers)
            .into_iter()
            .map(|source| {
                let label = source.label(&state.store).unwrap_or_default();
                (LegendHandle(source), label)
            })
            .collect()
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
    /// When an option is invalid, as for [`Axes::legend`], including
    /// `loc="best"`.
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
            .artist_as::<FigureFrame>(self.id())
            .map(|frame| legends_in(&state, &frame.children))
            .unwrap_or_default();
        ids.into_iter()
            .map(|id| Legend {
                raw: RawHandle::new(Rc::clone(figure), id),
            })
            .collect()
    }
}
