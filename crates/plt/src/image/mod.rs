//! Images: arrays of pixels drawn in data or figure coordinates
//! (matplotlib.image).
//!
//! * [`ImageData`] — colour-mapped scalars ([`ScalarField`], with masks)
//!   or explicit RGB(A) colours, as floats (double or single precision) or
//!   bytes.
//! * [`MultiField`] — data of several variates for bivariate and
//!   multivariate colormaps ([`MultivarColorizerOptions`]), which
//!   `pcolormesh` and quad-mesh `pcolorfast` draw; as in matplotlib 3.11,
//!   images take no multivariate data.
//! * [`AxesImage`] — an image on a regular grid in data coordinates, as
//!   made by `imshow`, `matshow`, `spy` and `pcolorfast`.
//! * [`NonUniformImage`] and [`PcolorImage`] — images whose pixel centres
//!   or cell edges lie on non-uniform rectilinear grids.
//! * [`FigureImage`] — an image placed in figure pixels by
//!   [`Figure::figimage`](crate::figure::Figure::figimage).
//! * [`BboxImage`] and [`BboxImageOptions`] — an image stretched over a
//!   box, as an [`OffsetImage`](crate::offsetbox::OffsetImage) draws it or
//!   [`Axes::add_bbox_image`](crate::axes::Axes::add_bbox_image) adds it.
//! * [`Interpolation`], [`InterpolationStage`], [`Origin`], [`Extent`] and
//!   [`ImageAlpha`] — how images are oriented, placed and resampled.
//! * [`imread`] and [`imsave`] — PNG, JPEG, TIFF, GIF and WebP files to
//!   image arrays, and arrays to PNG, JPEG, TIFF, GIF or raw files
//!   ([`BitmapFormat`]); [`thumbnail`] writes a scaled copy of an image
//!   file.
//!
//! Drawing follows matplotlib's `_make_image` exactly: the image is
//! clipped to its clip box, rounded to whole pixels, resampled with Agg's
//! kernels (in data or colour space) and colour-mapped to 8-bit RGBA.

mod artist;
mod bbox_image;
mod bitmap;
mod colorize;
mod composite;
mod data;
mod error;
mod figure_image;
mod imread;
mod io;
mod make;
mod nonuniform_image;
mod normalize;
mod options;
mod pcolor_image;
mod resample;
mod stages;
mod thumbnail;
mod unsampled;
mod variates;

pub use artist::AxesImage;
pub(crate) use artist::AxesImageArtist;
pub(crate) use bbox_image::BboxImageArtist;
pub use bbox_image::{BboxImage, BboxImageOptions};
pub use bitmap::{BitmapError, BitmapFormat, BitmapSettings};
pub use colorize::{ColorizerOptions, MultivarColorizerOptions};
pub(crate) use composite::Compositor;
pub(crate) use data::Precision;
pub use data::{ImageData, ScalarField, ScalarKind};
pub use error::ImageError;
pub(crate) use figure_image::FigureImageArtist;
pub use figure_image::{FigimageOptions, FigureImage};
pub use imread::{ImreadFormat, imread, imread_bytes, imread_png};
pub use io::{ImageIoError, ImsaveOptions, imsave, imsave_bytes, imsave_format, imsave_png};
pub(crate) use make::{CoreSettings, ImageCore};
pub(crate) use nonuniform_image::NonUniformImageArtist;
pub use nonuniform_image::{NonUniformImage, NonUniformInterpolation};
pub(crate) use normalize::normalize as normalize_scalars;
pub use options::{Extent, ImageAlpha, Interpolation, InterpolationStage, Origin, UnknownName};
pub use pcolor_image::PcolorImage;
pub(crate) use pcolor_image::PcolorImageArtist;
pub(crate) use resample::Resampling;
pub use thumbnail::{ThumbnailError, ThumbnailOptions, thumbnail};
pub use variates::MultiField;

use crate::rc::RcParams;

/// The rc defaults of new images: `image.interpolation`,
/// `image.interpolation_stage` and `image.origin` (validated by rc, so
/// unknown names fall back to matplotlib's defaults).
pub(crate) fn rc_defaults(rc: &RcParams) -> (Interpolation, InterpolationStage, Origin) {
    (
        rc.image_interpolation.parse().unwrap_or_default(),
        rc.image_interpolation_stage.parse().unwrap_or_default(),
        rc.image_origin.parse().unwrap_or_default(),
    )
}
