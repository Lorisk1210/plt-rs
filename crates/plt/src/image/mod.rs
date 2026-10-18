//! Images: arrays of pixels drawn in data or figure coordinates
//! (matplotlib.image).
//!
//! * [`ImageData`] — colour-mapped scalars ([`ScalarField`], with masks)
//!   or explicit RGB(A) colours, as floats or bytes.
//! * [`AxesImage`] — an image on a regular grid in data coordinates, as
//!   made by `imshow`, `matshow`, `spy` and `pcolorfast`.
//! * [`FigureImage`] — an image placed in figure pixels by
//!   [`Figure::figimage`](crate::figure::Figure::figimage).
//! * [`BboxImageOptions`] — an image stretched over a box of the canvas,
//!   as an [`OffsetImage`](crate::offsetbox::OffsetImage) draws it.
//! * [`Interpolation`], [`InterpolationStage`], [`Origin`], [`Extent`] and
//!   [`ImageAlpha`] — how images are oriented, placed and resampled.
//! * [`imread`] and [`imsave`] — PNG files to image arrays and arrays to
//!   PNG, JPEG, TIFF, GIF or raw files ([`BitmapFormat`]).
//!
//! Drawing follows matplotlib's `_make_image` exactly: the image is
//! clipped to its clip box, rounded to whole pixels, resampled with Agg's
//! kernels (in data or colour space) and colour-mapped to 8-bit RGBA.

mod artist;
mod bbox_image;
mod bitmap;
mod colorize;
mod data;
mod error;
mod figure_image;
mod io;
mod make;
mod options;
mod pcolor_image;
mod resample;
mod stages;

pub use artist::AxesImage;
pub(crate) use artist::AxesImageArtist;
pub(crate) use bbox_image::BboxImageArtist;
pub use bbox_image::BboxImageOptions;
pub use bitmap::{BitmapError, BitmapFormat, BitmapSettings};
pub use colorize::ColorizerOptions;
pub use data::{ImageData, ScalarField, ScalarKind};
pub use error::ImageError;
pub use figure_image::{FigimageOptions, FigureImage};
pub use io::{
    ImageIoError, ImsaveOptions, imread, imread_png, imsave, imsave_bytes, imsave_format,
    imsave_png,
};
pub(crate) use make::{CoreSettings, ImageCore};
pub use options::{Extent, ImageAlpha, Interpolation, InterpolationStage, Origin, UnknownName};
pub use pcolor_image::PcolorImage;
pub(crate) use pcolor_image::PcolorImageArtist;
pub(crate) use resample::Resampling;
pub(crate) use stages::normalize as normalize_scalars;

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
