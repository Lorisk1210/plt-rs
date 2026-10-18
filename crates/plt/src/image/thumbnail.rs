//! Thumbnails of image files (matplotlib's `image.thumbnail`).

use std::fmt;
use std::path::Path;

use super::{ImageError, ImageIoError, Interpolation, imread};
use crate::axes::ImshowOptions;
use crate::figure::{Figure, SaveError, SaveOptions};
use crate::rc::Aspect;

/// The resolution thumbnails are laid out and saved at; it cancels out.
const DPI: f64 = 100.0;

/// Options of [`thumbnail`] (matplotlib's keyword arguments).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThumbnailOptions {
    /// The size of the thumbnail relative to the image.
    pub scale: f64,
    /// How the image is resampled.
    pub interpolation: Interpolation,
}

impl Default for ThumbnailOptions {
    /// matplotlib's defaults: a tenth of the size, bilinear resampling.
    fn default() -> Self {
        Self {
            scale: 0.1,
            interpolation: Interpolation::Bilinear,
        }
    }
}

/// Why a thumbnail could not be made.
#[derive(Debug)]
pub enum ThumbnailError {
    /// The image could not be read.
    Read(ImageIoError),
    /// The image could not be shown.
    Image(ImageError),
    /// The thumbnail could not be saved.
    Save(SaveError),
}

impl fmt::Display for ThumbnailError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(err) => write!(f, "cannot read the image: {err}"),
            Self::Image(err) => write!(f, "cannot show the image: {err}"),
            Self::Save(err) => write!(f, "cannot save the thumbnail: {err}"),
        }
    }
}

impl std::error::Error for ThumbnailError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read(err) => Some(err),
            Self::Image(err) => Some(err),
            Self::Save(err) => Some(err),
        }
    }
}

/// Writes the image file `infile`, scaled by `options.scale`, to
/// `thumbfile` in the format its extension names, and returns the figure
/// that drew it (matplotlib's `image.thumbnail`): the image fills an
/// axes without frame or ticks, resampled with `options.interpolation`.
///
/// # Errors
///
/// When `infile` cannot be read as an image or `thumbfile` cannot be
/// written.
pub fn thumbnail(
    infile: impl AsRef<Path>,
    thumbfile: impl AsRef<Path>,
    options: &ThumbnailOptions,
) -> Result<Figure, ThumbnailError> {
    let image = imread(infile).map_err(ThumbnailError::Read)?;
    let (rows, cols) = image.shape();
    let figure = Figure::with_size(
        cols as f64 / DPI * options.scale,
        rows as f64 / DPI * options.scale,
        DPI,
    );
    let ax = figure.add_axes([0.0, 0.0, 1.0, 1.0]);
    ax.set_frame_on(false)
        .set_xticks(Vec::<f64>::new())
        .set_yticks(Vec::<f64>::new());
    let imshow = ImshowOptions {
        aspect: Some(Aspect::Auto),
        resample: Some(true),
        interpolation: Some(options.interpolation),
        ..ImshowOptions::default()
    };
    ax.imshow(image, imshow).map_err(ThumbnailError::Image)?;
    let save = SaveOptions {
        dpi: Some(DPI),
        ..SaveOptions::default()
    };
    figure
        .savefig_with(thumbfile, &save)
        .map_err(ThumbnailError::Save)?;
    Ok(figure)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::numeric::Grid;

    #[test]
    fn thumbnails_are_scaled_copies() {
        let dir = std::env::temp_dir().join(format!("plt-thumbnail-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (image, thumb) = (dir.join("image.png"), dir.join("thumb.png"));
        let pixels = Grid::from_fn(40, 60, |r, c| [(r * 6) as u8, (c * 4) as u8, 128_u8]);
        let png =
            crate::image::imsave_png(pixels, &crate::image::ImsaveOptions::default()).unwrap();
        std::fs::write(&image, png).unwrap();
        let options = ThumbnailOptions {
            scale: 0.5,
            ..ThumbnailOptions::default()
        };
        thumbnail(&image, &thumb, &options).unwrap();
        assert_eq!(imread(&thumb).unwrap().shape(), (20, 30));
        assert!(matches!(
            thumbnail(dir.join("missing.png"), &thumb, &options),
            Err(ThumbnailError::Read(_))
        ));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
