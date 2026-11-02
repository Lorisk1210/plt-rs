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
//!   made by `imshow`, `matshow`, `spy` and `pcolorfast`; its
//!   [`make_image`](AxesImage::make_image) gives the pixels drawn
//!   ([`MadeImage`]).
//! * [`NonUniformImage`] and [`PcolorImage`] — images whose pixel centres
//!   or cell edges lie on non-uniform rectilinear grids.
//! * [`FigureImage`] — an image placed in figure pixels by
//!   [`Figure::figimage`](crate::figure::Figure::figimage).
//! * [`BboxImage`] and [`BboxImageOptions`] — an image stretched over a
//!   box, as an [`OffsetImage`](crate::offsetbox::OffsetImage) draws it or
//!   [`Axes::add_bbox_image`](crate::axes::Axes::add_bbox_image) adds it.
//! * [`Interpolation`], [`InterpolationStage`], [`Origin`], [`Extent`] and
//!   [`ImageAlpha`] — how images are oriented, placed and resampled.
//! * [`resample`] and [`resample_pixels`] — Agg's resampler as
//!   matplotlib's `image.resample` exposes it, for scalar or RGBA
//!   [`Pixel`]s through an affine map or a lookup mesh ([`PixelMapping`]).
//! * [`imread`] and [`imsave`] — PNG, JPEG, TIFF, GIF and WebP files to
//!   image arrays, and arrays to PNG, JPEG, TIFF, GIF, WebP or raw files
//!   ([`BitmapFormat`]), and [`imsave_vector`] arrays of colours to PDF,
//!   PostScript or SVG; [`thumbnail`] writes a scaled copy of an image
//!   file.
//!
//! Drawing follows matplotlib's `_make_image` exactly: the image is
//! clipped to its clip box, rounded to whole pixels, resampled with Agg's
//! kernels (in data or colour space) and colour-mapped to 8-bit RGBA.

mod artist;
mod bbox_image;
mod bitmap;
mod color_image;
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
mod shared;
mod stages;
mod thumbnail;
mod unsampled;
mod variates;
mod vector_save;

pub use artist::AxesImage;
pub(crate) use artist::AxesImageArtist;
pub use bbox_image::{BboxFn, BboxImage, BboxImageOptions, Canvas};
pub(crate) use bbox_image::{BboxImageArtist, Region};
pub use bitmap::{BitmapError, BitmapFormat, BitmapSettings};
pub use color_image::{color_image_bytes, color_image_rgba};
pub use colorize::MultivarColorizerOptions;
pub(crate) use composite::Compositor;
pub use composite::{CompositePart, composite_images};
pub(crate) use data::Precision;
pub use data::{ImageData, ScalarField, ScalarKind};
pub use error::ImageError;
pub(crate) use figure_image::FigureImageArtist;
pub use figure_image::{FigimageOptions, FigureImage};
pub use imread::{ImreadFormat, imread, imread_bytes, imread_png};
pub use io::{ImageIoError, ImsaveOptions, imsave, imsave_bytes, imsave_format, imsave_png};
pub use make::MadeImage;
pub(crate) use make::{CoreSettings, ImageCore};
pub(crate) use nonuniform_image::NonUniformImageArtist;
pub use nonuniform_image::{NonUniformImage, NonUniformInterpolation};
pub(crate) use normalize::normalize as normalize_scalars;
pub use options::{Extent, ImageAlpha, Interpolation, InterpolationStage, Origin};
pub use pcolor_image::PcolorImage;
pub(crate) use pcolor_image::PcolorImageArtist;
pub use pltrs_raster::resample::Pixel;
pub use pltrs_render::RgbaImage;
pub(crate) use resample::Resampling;
pub use resample::{PixelMapping, ResampleOptions, resample, resample_pixels};
pub use thumbnail::{ThumbnailError, ThumbnailOptions, thumbnail};
pub use variates::MultiField;
pub use vector_save::{VectorImsaveError, imsave_vector};

use crate::rc::RcParams;

/// Gives an image handle type the changes of its colour mapping
/// (matplotlib's `set_cmap`, `set_norm` and `set_clim`, and the norm
/// changed in place), through the [`ColorMapped`](crate::colorizer::ColorMapped)
/// artist state behind it: the image is recoloured and its colorbars
/// follow.
macro_rules! impl_mapping_changes {
    ($handle:ty, $artist:ty) => {
        impl $handle {
            /// Replaces the colormap; the image's colorbars follow.
            pub fn set_cmap(&self, cmap: $crate::colors::Colormap) -> &Self {
                $crate::colorizer::change_mapping::<$artist>(
                    &self.raw,
                    $crate::colorizer::MappingChange::Cmap(Box::new(cmap)),
                );
                self
            }

            /// Replaces the norm, whose unset limits come from the data;
            /// the image's colorbars follow with the norm's scale and
            /// ticks.
            pub fn set_norm(&self, norm: Box<dyn $crate::colors::Norm>) -> &Self {
                $crate::colorizer::change_mapping::<$artist>(
                    &self.raw,
                    $crate::colorizer::MappingChange::Norm(norm),
                );
                self
            }

            /// Gives the norm new parameters, `norm` standing for the
            /// same norm changed in place (matplotlib setting `vmin`,
            /// `vmax`, ... of the norm object an image keeps): unlike
            /// `set_norm`, the image's colorbars keep their tickers and
            /// scale.
            pub fn retune_norm(&self, norm: Box<dyn $crate::colors::Norm>) -> &Self {
                $crate::colorizer::change_mapping::<$artist>(
                    &self.raw,
                    $crate::colorizer::MappingChange::Retune(norm),
                );
                self
            }

            /// Sets the data values mapped to the ends of the colormap
            /// (matplotlib's `set_clim`); `None` leaves a limit unchanged.
            /// The image's colorbars follow.
            pub fn set_clim(&self, vmin: Option<f64>, vmax: Option<f64>) -> &Self {
                $crate::colorizer::change_mapping::<$artist>(
                    &self.raw,
                    $crate::colorizer::MappingChange::Clim(vmin, vmax),
                );
                self
            }
        }
    };
}

use impl_mapping_changes;

/// Implements [`ColorMapped`](crate::colorizer::ColorMapped) for an image
/// artist state, given the paths of its data and of its mapping fields:
/// images map their data when drawn, so a new norm only takes its unset
/// limits from the whole array, as matplotlib's `_make_image` does.
macro_rules! impl_color_mapped {
    ($artist:ty, $($data:ident).+, $($mappable:ident).+) => {
        impl $crate::colorizer::ColorMapped for $artist {
            fn mapping_mut(&mut self) -> Option<&mut $crate::colors::ScalarMappable> {
                Some(self.$($mappable).+.get_mut())
            }

            fn remap(
                &mut self,
                _others: &mut $crate::artist::ArtistStore,
            ) -> Result<(), $crate::colors::NormError> {
                if let Some(field) = self.$($data).+.scalars() {
                    field.autoscale(self.$($mappable).+.get_mut().norm_mut());
                }
                Ok(())
            }
        }
    };
}

use impl_color_mapped;

/// The rc defaults of new images: `image.interpolation` (which rc takes
/// as any text, like matplotlib, so unknown names fall back to
/// matplotlib's default), `image.interpolation_stage` and `image.origin`.
pub(crate) fn rc_defaults(rc: &RcParams) -> (Interpolation, InterpolationStage, Origin) {
    (
        rc.image_interpolation.parse().unwrap_or_default(),
        rc.image_interpolation_stage,
        rc.image_origin,
    )
}
