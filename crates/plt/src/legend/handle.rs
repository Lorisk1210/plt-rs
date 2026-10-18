//! What a legend can list, and how the entries of `legend()` are found
//! (matplotlib's `_get_legend_handles` and `_parse_legend_args`).

use std::rc::Rc;

use crate::artist::{Artist, ArtistHandle, ArtistId, ArtistStore, is_hidden_label};
use crate::axes::{AxesState, BarContainer, ErrorbarContainer, StemContainer};
use crate::collections::{CollectionArtist, CollectionKind};
use crate::container::{Container, ContainerIds};
use crate::lines::LineArtist;

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

/// The artists a legend entry is made from.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Source {
    /// A single artist.
    Artist(ArtistId),
    /// The artists of a plotting call.
    Container(ContainerIds),
    /// A line drawn nowhere but in legends (matplotlib's proxy artists,
    /// such as the handles of `legend_elements`).
    Proxy(ProxyLine),
}

/// A line that exists only as a legend handle; handles compare equal when
/// they share the line.
#[derive(Clone, Debug)]
pub(crate) struct ProxyLine(pub(crate) Rc<LineArtist>);

impl PartialEq for ProxyLine {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl LegendHandle {
    /// A handle showing `line`, which is drawn nowhere else.
    pub(crate) fn proxy(line: LineArtist) -> Self {
        Self(Source::Proxy(ProxyLine(Rc::new(line))))
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
            Self::Proxy(line) => Some(line.0.base().label.clone()),
        }
    }

    /// Whether legends can draw an entry for it (matplotlib's handler map
    /// lookup).
    pub(crate) fn has_handler(&self, store: &ArtistStore) -> bool {
        match self {
            Self::Artist(id) => artist_has_handler(store, *id),
            Self::Container(_) | Self::Proxy(_) => true,
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
/// labelled `_nolegend_`, in plotting order with the containers last
/// (matplotlib's `_get_legend_handles`).
pub(crate) fn candidates(store: &ArtistStore, axes: &[ArtistId]) -> Vec<Source> {
    axes.iter()
        .filter_map(|&id| store.get_as::<AxesState>(id))
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
