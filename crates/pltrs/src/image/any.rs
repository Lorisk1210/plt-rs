//! Images of any kind (matplotlib's `_ImageBase`): the resampling
//! settings and the opacity every image keeps, whether the drawing of its
//! kind reads them or not, as matplotlib's images keep them.

use super::resample::Resampling;
use super::shared::split_alpha;
use super::{
    AxesImageArtist, BboxImageArtist, FigureImageArtist, ImageAlpha, ImageError, Interpolation,
    InterpolationStage, NonUniformImageArtist, Origin, PcolorImageArtist, rc_defaults,
};
use crate::artist::{ArtistHandle, RawHandle, TypedHandle};
use crate::rc::RcParams;

/// The settings an image artist keeps (see [`AnyImage`]).
pub(crate) trait ImageSettings {
    /// The resampling settings.
    fn resampling(&self) -> &Resampling;

    /// The resampling settings, to change.
    fn resampling_mut(&mut self) -> &mut Resampling;

    /// Whether data or colours are resampled.
    fn stage(&self) -> InterpolationStage;

    /// Changes whether data or colours are resampled.
    fn set_stage(&mut self, stage: InterpolationStage);

    /// Changes the opacity: uniform, or one value per pixel.
    fn set_alpha(&mut self, alpha: Option<ImageAlpha>);
}

/// The resampling settings and stage of an image made without settings
/// of its own: those of the `image.*` rc parameters, matplotlib's
/// `_ImageBase` defaults for the others.
pub(crate) fn default_settings(rc: &RcParams) -> (Resampling, InterpolationStage) {
    let (interpolation, stage, origin) = rc_defaults(rc);
    let resampling = Resampling {
        interpolation,
        resample: false,
        filternorm: true,
        filterrad: 4.0,
        origin,
    };
    (resampling, stage)
}

/// A checked radius of the sinc-family kernels (matplotlib's
/// `set_filterrad`).
///
/// # Errors
///
/// When the radius is not positive.
pub(crate) fn checked_filterrad(radius: f64) -> Result<f64, ImageError> {
    if radius.is_nan() || radius <= 0.0 {
        return Err(ImageError::FilterRadius(radius));
    }
    Ok(radius)
}

/// Implements [`ImageSettings`] for the image artists keeping an
/// [`ImageCore`](super::ImageCore).
macro_rules! core_settings {
    ($($artist:ty),*) => {$(
        impl ImageSettings for $artist {
            fn resampling(&self) -> &Resampling {
                &self.core.resampling
            }

            fn resampling_mut(&mut self) -> &mut Resampling {
                &mut self.core.resampling
            }

            fn stage(&self) -> InterpolationStage {
                self.core.stage
            }

            fn set_stage(&mut self, stage: InterpolationStage) {
                self.core.stage = stage;
            }

            fn set_alpha(&mut self, alpha: Option<ImageAlpha>) {
                let (uniform, grid) = split_alpha(alpha);
                crate::artist::Artist::base_mut(self).alpha = uniform;
                self.core.alpha_grid = grid;
            }
        }
    )*};
}

core_settings!(AxesImageArtist, BboxImageArtist, FigureImageArtist);

/// Runs `$body` with `$artist` bound to the image artist `$raw` reaches,
/// read (`with`) or changed (`with_mut`), whatever its kind.
macro_rules! with_any_kind {
    ($raw:expr, $access:ident, |$artist:ident| $body:expr) => {{
        let raw = $raw;
        if raw.is::<AxesImageArtist>() {
            raw.$access::<AxesImageArtist, _>(|$artist| $body)
        } else if raw.is::<BboxImageArtist>() {
            raw.$access::<BboxImageArtist, _>(|$artist| $body)
        } else if raw.is::<FigureImageArtist>() {
            raw.$access::<FigureImageArtist, _>(|$artist| $body)
        } else if raw.is::<PcolorImageArtist>() {
            raw.$access::<PcolorImageArtist, _>(|$artist| $body)
        } else {
            raw.$access::<NonUniformImageArtist, _>(|$artist| $body)
        }
    }};
}

/// A handle to an image of any kind, an [`AxesImage`](super::AxesImage),
/// [`BboxImage`](super::BboxImage), [`FigureImage`](super::FigureImage),
/// [`PcolorImage`](super::PcolorImage) or
/// [`NonUniformImage`](super::NonUniformImage): the settings every image
/// keeps (matplotlib's `_ImageBase`), which kinds whose drawing does not
/// resample (pcolor and non-uniform images) keep without drawing with
/// them, as matplotlib's do.
///
/// ```
/// use pltrs::artist::{ArtistHandle, TypedHandle};
/// use pltrs::colors::ColorizerOptions;
/// use pltrs::image::{AnyImage, Interpolation, PcolorImage};
/// let pcolor = PcolorImage::loose(ColorizerOptions::default())?;
/// let image = AnyImage::from_raw_handle(pcolor.raw()).expect("an image");
/// image.set_interpolation(Interpolation::Bicubic).set_resample(true);
/// assert_eq!(image.interpolation(), Interpolation::Bicubic);
/// assert!(image.resample());
/// assert!(image.set_filterrad(0.0).is_err());
/// # Ok::<(), pltrs::image::ImageError>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnyImage {
    raw: RawHandle,
}

impl ArtistHandle for AnyImage {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl TypedHandle for AnyImage {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        (raw.is::<AxesImageArtist>()
            || raw.is::<BboxImageArtist>()
            || raw.is::<FigureImageArtist>()
            || raw.is::<PcolorImageArtist>()
            || raw.is::<NonUniformImageArtist>())
        .then(|| Self { raw: raw.clone() })
    }
}

impl AnyImage {
    fn read<R>(&self, f: impl FnOnce(&dyn ImageSettings) -> R) -> R {
        with_any_kind!(&self.raw, with, |image| f(image))
    }

    fn update(&self, f: impl FnOnce(&mut dyn ImageSettings)) -> &Self {
        with_any_kind!(&self.raw, with_mut, |image| f(image));
        self
    }

    /// The interpolation used when resampling (matplotlib's
    /// `get_interpolation`); a non-uniform image draws `Nearest` as the
    /// nearest centre and any other bilinearly.
    pub fn interpolation(&self) -> Interpolation {
        self.read(|image| image.resampling().interpolation)
    }

    /// Changes the interpolation (matplotlib's `set_interpolation`).
    pub fn set_interpolation(&self, interpolation: Interpolation) -> &Self {
        self.update(|image| image.resampling_mut().interpolation = interpolation)
    }

    /// Whether data or colours are resampled, as set (matplotlib's
    /// `get_interpolation_stage`).
    pub fn interpolation_stage(&self) -> InterpolationStage {
        self.read(|image| image.stage())
    }

    /// Changes whether data or colours are resampled.
    pub fn set_interpolation_stage(&self, stage: InterpolationStage) -> &Self {
        self.update(|image| image.set_stage(stage))
    }

    /// Where row 0 of the array is drawn.
    pub fn origin(&self) -> Origin {
        self.read(|image| image.resampling().origin)
    }

    /// Whether the weights of the resampling kernel are normalised
    /// (matplotlib's `get_filternorm`).
    pub fn filternorm(&self) -> bool {
        self.read(|image| image.resampling().filternorm)
    }

    /// Sets whether the weights of the resampling kernel are normalised
    /// (matplotlib's `set_filternorm`).
    pub fn set_filternorm(&self, filternorm: bool) -> &Self {
        self.update(|image| image.resampling_mut().filternorm = filternorm)
    }

    /// The radius of the sinc-family kernels (matplotlib's
    /// `get_filterrad`).
    pub fn filterrad(&self) -> f64 {
        self.read(|image| image.resampling().filterrad)
    }

    /// Sets the radius of the sinc-family kernels (matplotlib's
    /// `set_filterrad`).
    ///
    /// # Errors
    ///
    /// When the radius is not positive.
    pub fn set_filterrad(&self, radius: f64) -> Result<&Self, ImageError> {
        let radius = checked_filterrad(radius)?;
        Ok(self.update(|image| image.resampling_mut().filterrad = radius))
    }

    /// Whether downsampling resamples fully (matplotlib's `get_resample`).
    pub fn resample(&self) -> bool {
        self.read(|image| image.resampling().resample)
    }

    /// Sets whether downsampling resamples fully (matplotlib's
    /// `set_resample`).
    pub fn set_resample(&self, resample: bool) -> &Self {
        self.update(|image| image.resampling_mut().resample = resample)
    }

    /// Changes the opacity: uniform, or one value per pixel (matplotlib's
    /// `set_alpha`); images that are not resampled draw only a uniform
    /// one, as matplotlib's renderers apply it.
    pub fn set_image_alpha(&self, alpha: Option<ImageAlpha>) -> &Self {
        self.update(|image| image.set_alpha(alpha))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::axes::NonUniformOptions;
    use crate::colors::ColorizerOptions;
    use crate::image::{AxesImage, BboxImageOptions, ImageData, NonUniformImage, PcolorImage};
    use crate::numeric::Grid;

    #[test]
    fn every_kind_keeps_the_settings() {
        let axes = AxesImage::loose(ImageData::empty(), BboxImageOptions::default(), None)
            .expect("an image");
        let pcolor = PcolorImage::loose(ColorizerOptions::default()).expect("an image");
        let nonuniform = NonUniformImage::loose(NonUniformOptions::default()).expect("an image");
        for raw in [axes.raw(), pcolor.raw(), nonuniform.raw()] {
            let image = AnyImage::from_raw_handle(raw).expect("an image");
            image
                .set_interpolation(Interpolation::Hanning)
                .set_interpolation_stage(InterpolationStage::Data)
                .set_filternorm(false)
                .set_resample(true);
            image.set_filterrad(2.0).expect("a positive radius");
            assert_eq!(image.interpolation(), Interpolation::Hanning);
            assert_eq!(image.interpolation_stage(), InterpolationStage::Data);
            assert!(!image.filternorm());
            assert_eq!(image.filterrad(), 2.0);
            assert!(image.resample());
            assert!(matches!(
                image.set_filterrad(f64::NAN),
                Err(ImageError::FilterRadius(_))
            ));
            assert_eq!(image.filterrad(), 2.0, "unchanged by a refused radius");
        }
        assert_eq!(axes.interpolation(), Interpolation::Hanning);
    }

    #[test]
    fn opacity_is_uniform_or_per_pixel() {
        let pcolor = PcolorImage::loose(ColorizerOptions::default()).expect("an image");
        let image = AnyImage::from_raw_handle(pcolor.raw()).expect("an image");
        image.set_image_alpha(Some(ImageAlpha::Uniform(0.5)));
        assert_eq!(pcolor.raw().with_base(|base| base.alpha), Some(0.5));
        image.set_image_alpha(Some(ImageAlpha::Each(Grid::filled(1, 1, 0.2))));
        assert_eq!(pcolor.raw().with_base(|base| base.alpha), None);
    }

    #[test]
    fn other_artists_are_no_images() {
        let line =
            crate::lines::Line2D::loose(vec![0.0], vec![0.0], crate::lines::LineProps::default());
        assert!(AnyImage::from_raw_handle(line.raw()).is_none());
    }
}
