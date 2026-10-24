//! What a colorbar shows: the colour mapping of an image, a collection, a
//! triangle mesh, a contour set or a bare norm and colormap (matplotlib's
//! `mappable`), and the link through which a colorbar follows it.

use std::cell::RefCell;
use std::rc::Rc;

use plt_core::Rgba;

use super::ColorbarError;
use crate::artist::{Artist, ArtistHandle, ArtistId, ArtistStore, RawHandle};
use crate::axes::{Axes, AxesState, Tripcolor};
use crate::collections::{Collection, CollectionArtist, TriMesh, TriMeshArtist};
use crate::colorizer::{MappingChange, change_mapping};
use crate::colors::{Extend, ScalarMappable, colormaps};
use crate::contour::{ContourSet, ContourSetArtist};
use crate::figure::FigureState;
use crate::image::{AxesImage, AxesImageArtist, ScalarKind};

/// Something with a colour mapping a colorbar can show.
///
/// ```
/// use plt::colorbar::Mappable;
/// use plt::colors::{Normalize, ScalarMappable, colormaps};
/// let sm = ScalarMappable::with_norm(colormaps::get("cividis")?, Box::new(Normalize::new(0.0, 10.0)));
/// let mappable = Mappable::from(&sm);
/// assert!(matches!(mappable, Mappable::Scalar(_)));
/// # Ok::<(), plt::colors::ColormapError>(())
/// ```
#[derive(Clone, Copy, Debug)]
pub enum Mappable<'a> {
    /// An image (`imshow`, `matshow`).
    Image(&'a AxesImage),
    /// A colour-mapped collection (`scatter`, `pcolormesh`, flat
    /// `tripcolor`, ...).
    Collection(&'a Collection),
    /// A Gouraud-shaded triangle mesh (`tripcolor`).
    TriMesh(&'a TriMesh),
    /// Line or filled contours.
    Contour(&'a ContourSet),
    /// A norm and colormap not drawn anywhere; the colorbar keeps a copy.
    Scalar(&'a ScalarMappable),
}

impl<'a> From<&'a AxesImage> for Mappable<'a> {
    fn from(image: &'a AxesImage) -> Self {
        Self::Image(image)
    }
}

impl<'a> From<&'a Collection> for Mappable<'a> {
    fn from(collection: &'a Collection) -> Self {
        Self::Collection(collection)
    }
}

impl<'a> From<&'a TriMesh> for Mappable<'a> {
    fn from(mesh: &'a TriMesh) -> Self {
        Self::TriMesh(mesh)
    }
}

impl<'a> From<&'a Tripcolor> for Mappable<'a> {
    fn from(plot: &'a Tripcolor) -> Self {
        match plot {
            Tripcolor::Flat(collection) => Self::Collection(collection),
            Tripcolor::Gouraud(mesh) => Self::TriMesh(mesh),
        }
    }
}

impl<'a> From<&'a ContourSet> for Mappable<'a> {
    fn from(contours: &'a ContourSet) -> Self {
        Self::Contour(contours)
    }
}

impl<'a> From<&'a ScalarMappable> for Mappable<'a> {
    fn from(mappable: &'a ScalarMappable) -> Self {
        Self::Scalar(mappable)
    }
}

/// What a colorbar draws from its mappable.
#[derive(Clone, Debug)]
pub(crate) struct Source {
    /// A copy of the colormap and norm.
    pub(crate) mapping: ScalarMappable,
    /// The mappable's uniform opacity.
    pub(crate) alpha: Option<f64>,
    /// The levels of a contour set.
    pub(crate) contours: Option<ContourLevels>,
}

/// The levels and line styles of a contour set (matplotlib's `_levels`,
/// `cvalues`, ...).
#[derive(Clone, Debug)]
pub(crate) struct ContourLevels {
    /// The contour levels.
    pub(crate) levels: Vec<f64>,
    /// The levels with the `extend` sentinels: the colorbar boundaries.
    pub(crate) extended: Vec<f64>,
    /// The value coloured between each pair of boundaries.
    pub(crate) cvalues: Vec<f64>,
    /// Whether the values are levels or colour indices.
    pub(crate) cvalue_kind: ScalarKind,
    /// Which ends are extended.
    pub(crate) extend: Extend,
    /// Whether the contours are filled.
    pub(crate) filled: bool,
    /// The colour of every level, opacity applied.
    pub(crate) colors: Vec<Rgba>,
    /// The line width of every level.
    pub(crate) linewidths: Vec<f64>,
    /// The hatch of every band, the colorbar's blocks from the lower
    /// extension to the upper one (`None` throughout unless the bands of a
    /// filled set are hatched).
    pub(crate) hatches: Vec<Option<String>>,
}

impl ContourLevels {
    /// Whether the colorbar draws hatched patches instead of a mesh.
    pub(crate) fn hatched(&self) -> bool {
        self.hatches.iter().any(Option::is_some)
    }
}

/// The colour-mapped artist a colorbar follows, by id in the colorbar's
/// figure (a handle would keep the figure alive from inside itself).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MappableLink {
    Image(ArtistId),
    Collection(ArtistId),
    TriMesh(ArtistId),
    /// The artist keeping a contour set's state.
    Contour(ArtistId),
}

impl MappableLink {
    /// The artist followed.
    pub(crate) fn id(self) -> ArtistId {
        match self {
            Self::Image(id) | Self::Collection(id) | Self::TriMesh(id) | Self::Contour(id) => id,
        }
    }

    /// The axes the artist is drawn in, if it is still drawn in one.
    pub(crate) fn host_axes(self, figure: &Rc<RefCell<FigureState>>) -> Option<Axes> {
        host_axes(&RawHandle::new(Rc::clone(figure), self.id()))
    }

    /// Gives the artist the norm limits its new colorbar settled on, as
    /// matplotlib's colorbar does by changing the norm it shares with its
    /// mappable (`_process_values` fills unset limits and widens singular
    /// ones); the artist is recoloured and its other colorbars follow.
    pub(crate) fn share_limits(self, figure: &Rc<RefCell<FigureState>>, vmin: f64, vmax: f64) {
        let raw = RawHandle::new(Rc::clone(figure), self.id());
        let change = MappingChange::Clim(Some(vmin), Some(vmax));
        match self {
            Self::Image(_) => change_mapping::<AxesImageArtist>(&raw, change),
            Self::Collection(_) => change_mapping::<CollectionArtist>(&raw, change),
            Self::TriMesh(_) => change_mapping::<TriMeshArtist>(&raw, change),
            Self::Contour(_) => change_mapping::<ContourSetArtist>(&raw, change),
        }
    }

    /// What to draw from the artist as it is now; `None` once it is gone.
    pub(crate) fn source(self, store: &ArtistStore) -> Option<Result<Source, ColorbarError>> {
        let plain = |mapping: ScalarMappable, alpha| {
            Ok(Source {
                mapping,
                alpha,
                contours: None,
            })
        };
        Some(match self {
            Self::Image(id) => {
                let image = store.get_as::<AxesImageArtist>(id)?;
                plain(image.core.mappable.borrow().clone(), image.base().alpha)
            }
            Self::Collection(id) => {
                let collection = store.get_as::<CollectionArtist>(id)?;
                let alpha = collection.base().alpha;
                match &collection.mapping {
                    Some(mapped) => plain(mapped.source.mappable.clone(), alpha),
                    None => colormaps::get(&crate::rc::params().image_cmap)
                        .map_err(ColorbarError::from)
                        .and_then(|cmap| plain(ScalarMappable::new(cmap), alpha)),
                }
            }
            Self::TriMesh(id) => {
                let mesh = store.get_as::<TriMeshArtist>(id)?;
                plain(mesh.mappable().clone(), mesh.base().alpha)
            }
            Self::Contour(id) => {
                let state = &store.get_as::<ContourSetArtist>(id)?.state;
                state
                    .mapping
                    .clone()
                    .ok_or(ColorbarError::NoColors)
                    .map(|mapping| Source {
                        mapping,
                        alpha: state.alpha,
                        contours: Some(ContourLevels {
                            levels: state.levels.clone(),
                            extended: state.extended.clone(),
                            cvalues: state.cvalues.clone(),
                            cvalue_kind: state.cvalue_kind,
                            extend: state.extend,
                            filled: state.filled,
                            colors: state.colors.iter().map(|&c| state.with_alpha(c)).collect(),
                            linewidths: state.linewidths.clone(),
                            hatches: (0..state.extended.len().saturating_sub(1))
                                .map(|band| state.hatched().then(|| state.hatch(band)).flatten())
                                .collect(),
                        }),
                    })
            }
        })
    }
}

impl Mappable<'_> {
    /// The figure and artist of a drawn mappable, or the norm and colormap
    /// of a bare one.
    fn target(&self) -> Result<(&Rc<RefCell<FigureState>>, MappableLink), &ScalarMappable> {
        let (raw, link) = match self {
            Self::Image(image) => (image.raw(), MappableLink::Image(image.id())),
            Self::Collection(collection) => {
                (collection.raw(), MappableLink::Collection(collection.id()))
            }
            Self::TriMesh(mesh) => (mesh.raw(), MappableLink::TriMesh(mesh.id())),
            Self::Contour(contours) => {
                (contours.axes().raw(), MappableLink::Contour(contours.id()))
            }
            Self::Scalar(mapping) => return Err(mapping),
        };
        Ok((raw.figure_state(), link))
    }

    /// The figure and artist a colorbar follows; `None` for a bare norm
    /// and colormap.
    pub(crate) fn link(&self) -> Option<(&Rc<RefCell<FigureState>>, MappableLink)> {
        self.target().ok()
    }

    /// The colormap, norm, opacity and contour levels to draw from.
    pub(crate) fn source(&self) -> Result<Source, ColorbarError> {
        match self.target() {
            Ok((figure, link)) => link
                .source(&figure.borrow().store)
                .unwrap_or(Err(ColorbarError::RemovedMappable)),
            Err(mapping) => Ok(Source {
                mapping: mapping.clone(),
                alpha: None,
                contours: None,
            }),
        }
    }

    /// The axes the mappable is drawn in.
    pub(crate) fn axes(&self) -> Option<Axes> {
        match self {
            Self::Image(image) => host_axes(image.raw()),
            Self::Collection(collection) => host_axes(collection.raw()),
            Self::TriMesh(mesh) => host_axes(mesh.raw()),
            Self::Contour(contours) => Some(contours.axes().clone()),
            Self::Scalar(_) => None,
        }
    }
}

/// The axes an artist is drawn in, if its parent is one.
fn host_axes(raw: &RawHandle) -> Option<Axes> {
    let figure = raw.figure_state();
    let parent = {
        let state = figure.borrow();
        let parent = state.store.get(raw.id())?.base().parent?;
        state.store.get_as::<AxesState>(parent).map(|_| parent)
    }?;
    Some(Axes::from_raw(RawHandle::new(Rc::clone(figure), parent)))
}
