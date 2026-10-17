//! What a colorbar shows: the colour mapping of an image, a collection, a
//! contour set or a bare norm and colormap (matplotlib's `mappable`).

use std::rc::Rc;

use plt_core::Rgba;

use super::ColorbarError;
use crate::artist::{Artist, ArtistHandle, ArtistId, RawHandle};
use crate::axes::{Axes, AxesState};
use crate::collections::{Collection, CollectionArtist};
use crate::colors::{Extend, ScalarMappable, colormaps};
use crate::contour::ContourSet;
use crate::image::{AxesImage, AxesImageArtist};

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
    /// A colour-mapped collection (`scatter`, `pcolormesh`, ...).
    Collection(&'a Collection),
    /// Line or filled contours.
    Contour(&'a ContourSet),
    /// A norm and colormap not drawn anywhere.
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

/// What a colorbar takes from its mappable.
#[derive(Clone, Debug)]
pub(crate) struct Source {
    /// A copy of the colormap and norm.
    pub(crate) mapping: ScalarMappable,
    /// The mappable's uniform opacity.
    pub(crate) alpha: Option<f64>,
    /// The axes the mappable is drawn in.
    pub(crate) axes: Option<Axes>,
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
    /// Which ends are extended.
    pub(crate) extend: Extend,
    /// Whether the contours are filled.
    pub(crate) filled: bool,
    /// The colour of every level, opacity applied.
    pub(crate) colors: Vec<Rgba>,
    /// The line width of every level.
    pub(crate) linewidths: Vec<f64>,
}

impl Mappable<'_> {
    /// The colormap, norm, opacity, axes and contour levels to draw from.
    pub(crate) fn source(&self) -> Result<Source, ColorbarError> {
        match self {
            Self::Image(image) => {
                let (mapping, alpha) = image.raw().with::<AxesImageArtist, _>(|artist| {
                    (artist.core.mappable.borrow().clone(), artist.base().alpha)
                });
                Ok(Source {
                    mapping,
                    alpha,
                    axes: host_axes(image.raw()),
                    contours: None,
                })
            }
            Self::Collection(collection) => {
                let (mapping, alpha) = collection.raw().with::<CollectionArtist, _>(|artist| {
                    let mapping = artist
                        .mapping
                        .as_ref()
                        .map(|mapped| mapped.source.mappable.clone());
                    (mapping, artist.base().alpha)
                });
                let mapping = match mapping {
                    Some(mapping) => mapping,
                    None => ScalarMappable::new(colormaps::get(&crate::rc::params().image_cmap)?),
                };
                Ok(Source {
                    mapping,
                    alpha,
                    axes: host_axes(collection.raw()),
                    contours: None,
                })
            }
            Self::Contour(contours) => {
                let state = contours.state();
                let mapping = state.mapping.clone().ok_or(ColorbarError::NoColors)?;
                Ok(Source {
                    mapping,
                    alpha: state.alpha,
                    axes: Some(contours.axes().clone()),
                    contours: Some(ContourLevels {
                        levels: state.levels.clone(),
                        extended: state.extended.clone(),
                        cvalues: state.cvalues.clone(),
                        extend: state.extend,
                        filled: state.filled,
                        colors: contours.colors(),
                        linewidths: state.linewidths.clone(),
                    }),
                })
            }
            Self::Scalar(mapping) => Ok(Source {
                mapping: (*mapping).clone(),
                alpha: None,
                axes: None,
                contours: None,
            }),
        }
    }
}

/// The axes an artist is drawn in, if its parent is one.
fn host_axes(raw: &RawHandle) -> Option<Axes> {
    let parent: ArtistId = raw.with_base(|base| base.parent)?;
    let figure = raw.figure_state();
    let is_axes = figure.borrow().store.get_as::<AxesState>(parent).is_some();
    is_axes.then(|| Axes::from_raw(RawHandle::new(Rc::clone(figure), parent)))
}
