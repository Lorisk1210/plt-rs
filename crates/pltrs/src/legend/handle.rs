//! What a legend can list, and how the entries of `legend()` are found
//! (matplotlib's `_get_legend_handles` and `_parse_legend_args`).

use pltrs_core::archive::{Archive, ArchiveError, Decoder, Encoder};
use std::any::Any;
use std::fmt;
use std::rc::Rc;

use super::handlers::{HandlerKey, HandlerMap, LegendHandler, TypeKey};
use crate::artist::{
    Artist, ArtistBase, ArtistHandle, ArtistId, ArtistLookup, ArtistStore, RawHandle, downcast_ref,
    is_hidden_label,
};
use crate::axes::{AxesState, BarContainer, ErrorbarContainer, StemContainer};
use crate::axes_grid1::parasites_of;
use crate::collections::{Collection, EllipseCollection, EventCollection};
use crate::container::{Container, ContainerIds};
use crate::lines::{Line2D, LineArtist, LineProps};
use crate::patches::{Patch, PatchArtist, PatchProps, PatchShape, Rectangle};

/// Something a legend can show an entry for: a line, patch or collection,
/// a bar, error bar or stem container, or any object a
/// [`LegendHandler`] draws (matplotlib's legend handles).
///
/// Every artist handle converts into one, borrowed; the handles of what
/// legends draw (lines, patches, collections and the containers) convert
/// owned as well.
///
/// ```
/// use pltrs::prelude::*;
/// use pltrs::legend::{LegendHandle, LegendOptions};
/// let fig = Figure::new();
/// let ax = fig.subplot();
/// let line = ax.plot([0.0, 1.0], [0.0, 1.0])?;
/// let legend = fig.legend(LegendOptions::default().handles([&line]).labels(["rising"]))?;
/// assert_eq!(legend.labels(), ["rising"]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct LegendHandle(pub(crate) Source);

pltrs_core::archive_struct!(LegendHandle(v0));

/// How the handles of a tuple share the handle box of their entry
/// (matplotlib's `HandlerTuple`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HandlerTuple {
    /// Into how many parts, side by side, the box is divided; handles
    /// cycle through the parts. `None` gives every handle a part; the
    /// default, 1, overlays them all.
    pub ndivide: Option<usize>,
    /// The gap between parts as a fraction of the font size (`None` = the
    /// legend's `borderpad`).
    pub pad: Option<f64>,
}

pltrs_core::archive_struct!(HandlerTuple { ndivide, pad });

impl Default for HandlerTuple {
    fn default() -> Self {
        Self {
            ndivide: Some(1),
            pad: None,
        }
    }
}

/// The artists a legend entry is made from.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Source {
    /// A single artist.
    Artist(ArtistId),
    /// The artists of a plotting call.
    Container(ContainerIds),
    /// An artist drawn nowhere but in legends (matplotlib's proxy artists,
    /// such as the handles of `legend_elements`).
    Proxy(Proxy),
    /// Several handles in one entry (matplotlib's tuple handles).
    Tuple(Vec<Source>),
    /// A handle drawn with its own number of markers in place of the
    /// legend's `numpoints` (matplotlib's `HandlerLine2D(numpoints=...)`).
    Npoints(Box<Source>, usize),
    /// A handle drawn by a handler of its own (matplotlib's handler map
    /// keyed by the handle itself).
    Handled(Box<Source>, OwnHandler),
    /// An object that is no artist, drawn by the handler registered for
    /// its type.
    Object(LegendObject),
}

pltrs_core::archive_enum!(Source {
    Artist(v0),
    Container(v0),
    Proxy(v0),
    Tuple(v0),
    Npoints(v0, v1),
    Handled(v0, v1),
    Object(v0),
});

/// The handler of one entry; entries compare equal when they share it.
#[derive(Clone)]
pub(crate) struct OwnHandler(pub(crate) Rc<dyn LegendHandler>);

pltrs_core::archive_struct!(OwnHandler(v0));

impl fmt::Debug for OwnHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl PartialEq for OwnHandler {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// An object a legend shows through the handler of its type; handles
/// compare equal when they share the object.
#[derive(Clone)]
pub(crate) struct LegendObject {
    pub(crate) value: Rc<dyn Any>,
    pub(crate) key: TypeKey,
}

/// The object is a value of the program's, which cannot be archived.
impl Archive for LegendObject {
    fn encode(&self, _encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        Err(ArchiveError::unsupported(format!(
            "a legend handle of type {}",
            self.key.name()
        )))
    }

    fn decode(_decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        Err(ArchiveError::invalid("a legend handle of a program's type"))
    }
}

impl fmt::Debug for LegendObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LegendObject")
            .field("key", &self.key)
            .finish_non_exhaustive()
    }
}

impl PartialEq for LegendObject {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.value, &other.value)
    }
}

/// An artist that exists only as a legend handle; handles compare equal
/// when they share the artist.
#[derive(Clone, Debug)]
pub(crate) enum Proxy {
    /// A line (matplotlib's `Line2D([], [], ...)` proxies).
    Line(Rc<LineArtist>),
    /// A patch (matplotlib's `Patch(...)` proxies).
    Patch(Rc<PatchArtist<Rectangle>>),
}

pltrs_core::archive_enum!(Proxy {
    Line(v0),
    Patch(v0),
});

impl Proxy {
    fn base(&self) -> &ArtistBase {
        match self {
            Self::Line(line) => line.base(),
            Self::Patch(patch) => patch.base(),
        }
    }
}

impl PartialEq for Proxy {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Line(a), Self::Line(b)) => Rc::ptr_eq(a, b),
            (Self::Patch(a), Self::Patch(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl LegendHandle {
    /// A handle showing `line`, which is drawn nowhere else.
    pub(crate) fn proxy(line: LineArtist) -> Self {
        Self(Source::Proxy(Proxy::Line(Rc::new(line))))
    }

    /// A line drawn only in legends, styled and labelled by `props`
    /// (matplotlib's proxy artist `Line2D([], [], ...)`).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::legend::{LegendHandle, LegendOptions};
    /// use pltrs::markers::Marker;
    /// let ax = Figure::new().subplot();
    /// let star: Marker = "*".parse()?;
    /// let stars = LegendHandle::line(LineProps::default().color("b").marker(star).label("stars"));
    /// let legend = ax.legend(LegendOptions::default().handles([stars]))?;
    /// assert_eq!(legend.labels(), ["stars"]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn line(props: LineProps) -> Self {
        let mut line = LineArtist::new(Vec::new(), Vec::new(), crate::rc::params());
        props.apply(&mut line);
        Self::proxy(line)
    }

    /// A patch drawn only in legends, styled and labelled by `props`
    /// (matplotlib's proxy artist `Patch(...)`).
    pub fn patch(props: PatchProps) -> Self {
        let mut patch = PatchArtist::new(Rectangle::new((0.0, 0.0), 1.0, 1.0), crate::rc::params());
        props.apply(&mut patch);
        Self(Source::Proxy(Proxy::Patch(Rc::new(patch))))
    }

    /// Several handles drawn in one entry, overlaid or side by side as
    /// `layout` says (matplotlib's tuple handles, drawn by `HandlerTuple`).
    /// A tuple has no label of its own: give the entry one.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::legend::{HandlerTuple, LegendHandle, LegendOptions};
    /// let ax = Figure::new().subplot();
    /// let line = ax.plot([0.0, 1.0], [0.0, 1.0])?;
    /// let dots = ax.scatter(vec![0.5], vec![0.5])?;
    /// let both = LegendHandle::tuple([line.into(), dots.into()], HandlerTuple::default());
    /// ax.legend(LegendOptions::default().handles([both]).labels(["line and dot"]))?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn tuple(handles: impl IntoIterator<Item = LegendHandle>, layout: HandlerTuple) -> Self {
        Self(Source::Tuple(
            handles.into_iter().map(|handle| handle.0).collect(),
        ))
        .with_handler(layout)
    }

    /// A handle standing for `object`, which is no artist: the legend
    /// draws it with the handler registered for its type,
    /// [`HandlerKey::of::<T>()`](HandlerKey::of), in the
    /// [`handler_map`](super::LegendOptions::handler_map) or the default
    /// one (matplotlib's handles of any class, with
    /// `handler_map={AnyObject: AnyObjectHandler()}`); the handler can
    /// read the object back through
    /// [`HandlerEntry::object`](super::HandlerEntry::object). The handle
    /// has no label of its own: give the entry one.
    pub fn object<T: Any>(object: T) -> Self {
        Self(Source::Object(LegendObject {
            value: Rc::new(object),
            key: TypeKey::of::<T>(),
        }))
    }

    /// The same handle drawn by `handler` whatever the handler map says
    /// (matplotlib's `handler_map={handle: handler}`, keyed by the handle
    /// itself).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::legend::{HandlerKey, HandlerLine2D, HandlerPatch, LegendHandle, LegendOptions};
    /// let ax = Figure::new().subplot();
    /// let line = ax.plot_with([0.0, 1.0], [0.0, 1.0], LineProps::default().label("data"))?;
    /// // The handle's own handler wins over the legend's handler map.
    /// let own = LegendHandle::from(&line).with_handler(HandlerLine2D);
    /// let options = LegendOptions::default()
    ///     .handles([own])
    ///     .handler(HandlerKey::Line, HandlerPatch);
    /// assert_eq!(ax.legend(options)?.labels(), ["data"]);
    /// // A handler of another kind of handle fails, as in matplotlib.
    /// let boxed = LegendHandle::from(&line).with_handler(HandlerPatch);
    /// assert!(ax.legend(LegendOptions::default().handles([boxed])).is_err());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn with_handler(self, handler: impl LegendHandler + 'static) -> Self {
        Self(Source::Handled(
            Box::new(self.0),
            OwnHandler(Rc::new(handler)),
        ))
    }

    /// The same handle with `numpoints` markers on its line, whatever the
    /// legend's `numpoints` (matplotlib's
    /// `handler_map={line: HandlerLine2D(numpoints=...)}`).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::legend::{LegendHandle, LegendOptions};
    /// let ax = Figure::new().subplot();
    /// let line = ax.plot_with([0.0, 1.0], [0.0, 1.0], LineProps::fmt("o-")?.label("four"))?;
    /// let four = LegendHandle::from(&line).with_numpoints(4);
    /// let legend = ax.legend(LegendOptions::default().handles([four]))?;
    /// assert_eq!(legend.labels(), ["four"]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn with_numpoints(self, numpoints: usize) -> Self {
        Self(Source::Npoints(Box::new(self.0), numpoints))
    }
}

impl LegendHandle {
    /// The handle of `artist`: the artist itself for an artist of a
    /// figure; for a line or a rectangle made outside any figure
    /// ([`RawHandle::is_loose`]), a proxy drawn as it is now (matplotlib's
    /// proxy artists, `Line2D([], [], ...)` and `Patch(...)`). `None` for
    /// another artist made outside any figure, which no legend can show.
    pub fn of_artist(artist: &RawHandle) -> Option<Self> {
        if !artist.is_loose() {
            return Some(Self(Source::Artist(artist.id())));
        }
        let state = artist.figure_state().borrow();
        let loose = state.artist(artist.id())?;
        if let Some(line) = downcast_ref::<LineArtist>(loose) {
            return Some(Self::proxy(line.clone()));
        }
        downcast_ref::<PatchArtist<Rectangle>>(loose)
            .map(|patch| Self(Source::Proxy(Proxy::Patch(Rc::new(patch.clone())))))
    }

    /// The artist the handle shows, when it shows one artist (not a
    /// container, a proxy, a tuple or another object), as a handle in the
    /// figure of `owner`, the axes or figure the handle was found in.
    ///
    /// ```
    /// use pltrs::legend::LegendHandle;
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let line = ax.plot_with([0.0, 1.0], [0.0, 1.0], LineProps::default().label("a"))?;
    /// let (handles, _) = ax.legend_handles_labels();
    /// assert_eq!(handles[0].artist(&ax).as_ref(), Some(line.raw()));
    /// assert_eq!(LegendHandle::line(LineProps::default()).artist(&ax), None);
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn artist(&self, owner: &impl ArtistHandle) -> Option<RawHandle> {
        match self.0 {
            Source::Artist(id) => Some(RawHandle::new(Rc::clone(owner.raw().figure_state()), id)),
            _ => None,
        }
    }
}

impl<T: ArtistHandle> From<&T> for LegendHandle {
    fn from(artist: &T) -> Self {
        Self(Source::Artist(artist.id()))
    }
}

/// Implements `From` for owned artist handles of these types, as for
/// borrowed ones.
macro_rules! owned_artist_handles {
    ($($handle:ty),+) => {$(
        impl From<$handle> for LegendHandle {
            fn from(artist: $handle) -> Self {
                Self::from(&artist)
            }
        }
    )+};
}

owned_artist_handles!(Line2D, Collection, EventCollection, EllipseCollection);

impl<S: PatchShape> From<Patch<S>> for LegendHandle {
    fn from(patch: Patch<S>) -> Self {
        Self::from(&patch)
    }
}

impl From<&Container> for LegendHandle {
    fn from(container: &Container) -> Self {
        Self(Source::Container(container.into()))
    }
}

impl From<Container> for LegendHandle {
    fn from(container: Container) -> Self {
        Self::from(&container)
    }
}

/// Implements `From` for these containers, owned and borrowed, through
/// the [`Container`] variant holding each.
macro_rules! containers {
    ($($container:ty => $variant:ident),+) => {$(
        impl From<$container> for LegendHandle {
            fn from(container: $container) -> Self {
                Self::from(Container::$variant(container))
            }
        }

        impl From<&$container> for LegendHandle {
            fn from(container: &$container) -> Self {
                Self::from(container.clone())
            }
        }
    )+};
}

containers!(
    BarContainer => Bar,
    ErrorbarContainer => Errorbar,
    StemContainer => Stem
);

impl Source {
    /// The legend label, if the artists still exist.
    pub(crate) fn label(&self, store: &ArtistStore) -> Option<String> {
        match self {
            Self::Artist(id) => store.get(*id).map(|a| a.base().label.clone()),
            Self::Container(container) => container.label().map(str::to_owned),
            Self::Proxy(proxy) => Some(proxy.base().label.clone()),
            Self::Tuple(..) | Self::Object(_) => None,
            Self::Npoints(source, _) | Self::Handled(source, _) => source.label(store),
        }
    }

    /// The keys of the handler map that may hold its handler, the most
    /// specific first (matplotlib's lookup through the class's method
    /// resolution order).
    pub(crate) fn handler_keys(&self, store: &ArtistStore) -> Vec<HandlerKey> {
        match self {
            Self::Artist(id) => HandlerKey::of_artist(store, *id),
            Self::Container(ContainerIds::Bar { .. }) => vec![HandlerKey::BarContainer],
            Self::Container(ContainerIds::Errorbar(_)) => vec![HandlerKey::ErrorbarContainer],
            Self::Container(ContainerIds::Stem { .. }) => vec![HandlerKey::StemContainer],
            Self::Proxy(Proxy::Line(_)) => vec![HandlerKey::Line],
            Self::Proxy(Proxy::Patch(_)) => {
                vec![
                    HandlerKey::Type(TypeKey::of::<Rectangle>()),
                    HandlerKey::Patch,
                ]
            }
            Self::Tuple(_) => vec![HandlerKey::Tuple],
            Self::Object(object) => vec![HandlerKey::Type(object.key)],
            Self::Npoints(source, _) | Self::Handled(source, _) => source.handler_keys(store),
        }
    }
}

/// The artists and containers of `axes` a legend can list, unless
/// labelled `_nolegend_` or without a handler in `handlers`, in plotting
/// order with the containers last, each axes followed by its parasites
/// (matplotlib's `_get_legend_handles`).
pub(crate) fn candidates(
    store: &ArtistStore,
    axes: &[ArtistId],
    handlers: &HandlerMap,
) -> Vec<Source> {
    axes.iter()
        .flat_map(|&id| std::iter::once(id).chain(parasites_of(store, id)))
        .filter_map(|id| store.get_as::<AxesState>(id))
        .flat_map(|axes| {
            let artists = axes.children.iter().map(|&id| Source::Artist(id));
            let containers = axes.containers.iter().cloned().map(Source::Container);
            artists.chain(containers).collect::<Vec<_>>()
        })
        .filter(|source| {
            handlers.handler_for(source, store).is_some()
                && source.label(store).as_deref() != Some("_nolegend_")
        })
        .collect()
}

/// The entries of a legend: the given handles and labels, or the
/// candidates of `axes` that `handlers` can draw (matplotlib's
/// `_parse_legend_args`). Unequal lists are cut to the shorter one.
pub(crate) fn entries(
    store: &ArtistStore,
    axes: &[ArtistId],
    handles: Option<Vec<LegendHandle>>,
    labels: Option<Vec<String>>,
    handlers: &HandlerMap,
) -> Vec<(Source, String)> {
    match (handles, labels) {
        (Some(handles), Some(labels)) => handles.into_iter().map(|h| h.0).zip(labels).collect(),
        (Some(handles), None) => handles
            .into_iter()
            .map(|handle| {
                let label = handle.0.label(store).unwrap_or_default();
                (handle.0, label)
            })
            .collect(),
        (None, Some(labels)) => candidates(store, axes, handlers)
            .into_iter()
            .zip(labels)
            .collect(),
        (None, None) => candidates(store, axes, handlers)
            .into_iter()
            .filter_map(|source| {
                let label = source.label(store)?;
                (!is_hidden_label(&label)).then_some((source, label))
            })
            .collect(),
    }
}
