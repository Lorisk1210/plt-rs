//! What a legend can list, and how the entries of `legend()` are found
//! (matplotlib's `_get_legend_handles` and `_parse_legend_args`).

use std::rc::Rc;

use crate::artist::{Artist, ArtistBase, ArtistHandle, ArtistId, ArtistStore, is_hidden_label};
use crate::axes::{AxesState, BarContainer, ErrorbarContainer, StemContainer};
use crate::axes_grid1::parasites_of;
use crate::collections::{CollectionArtist, CollectionKind};
use crate::container::{Container, ContainerIds};
use crate::lines::{LineArtist, LineProps};
use crate::patches::{PatchArtist, PatchProps, Rectangle};

/// Something a legend can show an entry for: a line, patch or collection,
/// or a bar, error bar or stem container (matplotlib's legend handles).
///
/// ```
/// use plt::prelude::*;
/// use plt::legend::{LegendHandle, LegendOptions};
/// let fig = Figure::new();
/// let ax = fig.subplot();
/// let line = ax.plot([0.0, 1.0], [0.0, 1.0])?;
/// let legend = fig.legend(LegendOptions::default().handles([&line]).labels(["rising"]))?;
/// assert_eq!(legend.labels(), ["rising"]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct LegendHandle(pub(crate) Source);

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
    Tuple(Vec<Source>, HandlerTuple),
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
    /// use plt::prelude::*;
    /// use plt::legend::{LegendHandle, LegendOptions};
    /// use plt::markers::Marker;
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
    /// use plt::prelude::*;
    /// use plt::legend::{HandlerTuple, LegendHandle, LegendOptions};
    /// let ax = Figure::new().subplot();
    /// let line = ax.plot([0.0, 1.0], [0.0, 1.0])?;
    /// let dots = ax.scatter(vec![0.5], vec![0.5])?;
    /// let both = LegendHandle::tuple([LegendHandle::from(&line), LegendHandle::from(&dots)], HandlerTuple::default());
    /// ax.legend(LegendOptions::default().handles([both]).labels(["line and dot"]))?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn tuple(handles: impl IntoIterator<Item = LegendHandle>, layout: HandlerTuple) -> Self {
        Self(Source::Tuple(
            handles.into_iter().map(|handle| handle.0).collect(),
            layout,
        ))
    }
}

impl<T: ArtistHandle> From<&T> for LegendHandle {
    fn from(artist: &T) -> Self {
        Self(Source::Artist(artist.id()))
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

impl From<&BarContainer> for LegendHandle {
    fn from(bars: &BarContainer) -> Self {
        Self::from(Container::Bar(bars.clone()))
    }
}

impl From<&ErrorbarContainer> for LegendHandle {
    fn from(errors: &ErrorbarContainer) -> Self {
        Self::from(Container::Errorbar(errors.clone()))
    }
}

impl From<&StemContainer> for LegendHandle {
    fn from(stems: &StemContainer) -> Self {
        Self::from(Container::Stem(stems.clone()))
    }
}

impl Source {
    /// The legend label, if the artists still exist.
    pub(crate) fn label(&self, store: &ArtistStore) -> Option<String> {
        match self {
            Self::Artist(id) => store.get(*id).map(|a| a.base().label.clone()),
            Self::Container(container) => container.label().map(str::to_owned),
            Self::Proxy(proxy) => Some(proxy.base().label.clone()),
            Self::Tuple(..) => None,
        }
    }

    /// Whether legends can draw an entry for it (matplotlib's handler map
    /// lookup).
    pub(crate) fn has_handler(&self, store: &ArtistStore) -> bool {
        match self {
            Self::Artist(id) => artist_has_handler(store, *id),
            Self::Container(_) | Self::Proxy(_) | Self::Tuple(..) => true,
        }
    }
}

/// Whether the artist is a line, a patch, or a line, path or polygon
/// collection (the classes of matplotlib's default handler map).
fn artist_has_handler(store: &ArtistStore, id: ArtistId) -> bool {
    let Some(artist) = store.get(id) else {
        return false;
    };
    if store.get_as::<LineArtist>(id).is_some() || artist.as_patch().is_some() {
        return true;
    }
    store.get_as::<CollectionArtist>(id).is_some_and(|c| {
        matches!(
            c.kind,
            CollectionKind::Line | CollectionKind::Path | CollectionKind::Poly
        )
    })
}

/// The artists and containers of `axes` a legend can list, unless
/// labelled `_nolegend_`, in plotting order with the containers last,
/// each axes followed by its parasites (matplotlib's
/// `_get_legend_handles`).
pub(crate) fn candidates(store: &ArtistStore, axes: &[ArtistId]) -> Vec<Source> {
    axes.iter()
        .flat_map(|&id| std::iter::once(id).chain(parasites_of(store, id)))
        .filter_map(|id| store.get_as::<AxesState>(id))
        .flat_map(|axes| {
            let artists = axes.children.iter().map(|&id| Source::Artist(id));
            let containers = axes.containers.iter().cloned().map(Source::Container);
            artists.chain(containers).collect::<Vec<_>>()
        })
        .filter(|source| {
            source.has_handler(store) && source.label(store).as_deref() != Some("_nolegend_")
        })
        .collect()
}

/// The entries of a legend: the given handles and labels, or the
/// candidates of `axes` (matplotlib's `_parse_legend_args`). Unequal
/// lists are cut to the shorter one.
pub(crate) fn entries(
    store: &ArtistStore,
    axes: &[ArtistId],
    handles: Option<Vec<LegendHandle>>,
    labels: Option<Vec<String>>,
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
        (None, Some(labels)) => candidates(store, axes).into_iter().zip(labels).collect(),
        (None, None) => candidates(store, axes)
            .into_iter()
            .filter_map(|source| {
                let label = source.label(store)?;
                (!is_hidden_label(&label)).then_some((source, label))
            })
            .collect(),
    }
}
