//! Saving an array of colours as a vector document (matplotlib's `imsave`
//! to PDF, PostScript or SVG, which bitmap encoders cannot write).

use std::fmt;

use super::{FigimageOptions, ImageData, ImageError, Origin};
use crate::figure::{Figure, Metadata, SaveError, SaveFormat, SaveOptions};
use crate::numeric::Grid;

/// Why an image cannot be saved as a vector document.
#[derive(Debug)]
pub enum VectorImsaveError {
    /// The format is not a vector format (PDF, PS, EPS or SVG).
    NotVector(SaveFormat),
    /// The image could not be placed on the page.
    Image(ImageError),
    /// The document could not be written.
    Save(SaveError),
}

impl fmt::Display for VectorImsaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotVector(format) => write!(
                f,
                "{:?} is not a vector format (pdf, ps, eps or svg)",
                format.extension()
            ),
            Self::Image(err) => err.fmt(f),
            Self::Save(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for VectorImsaveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NotVector(_) => None,
            Self::Image(err) => Some(err),
            Self::Save(err) => Some(err),
        }
    }
}

impl From<ImageError> for VectorImsaveError {
    fn from(err: ImageError) -> Self {
        Self::Image(err)
    }
}

impl From<SaveError> for VectorImsaveError {
    fn from(err: SaveError) -> Self {
        Self::Save(err)
    }
}

/// The document of `format` (PDF, PS, EPS or SVG) showing `pixels`, top
/// row first, one pixel per pixel at `dpi`, on a transparent page of their
/// size (matplotlib's `imsave` to a vector format, which draws them with
/// `figimage` on a frameless figure); `metadata` goes into the document.
///
/// ```
/// use pltrs::figure::SaveFormat;
/// use pltrs::image::imsave_vector;
/// use pltrs::numeric::Grid;
/// let red = Grid::filled(2, 3, [255, 0, 0, 255]);
/// let svg = imsave_vector(red, SaveFormat::Svg, 100.0, None)?;
/// assert!(String::from_utf8(svg)?.contains("<image"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// When the format is not a vector format, or the document cannot be
/// written.
pub fn imsave_vector(
    pixels: Grid<[u8; 4]>,
    format: SaveFormat,
    dpi: f64,
    metadata: Option<Metadata>,
) -> Result<Vec<u8>, VectorImsaveError> {
    if !matches!(
        format,
        SaveFormat::Pdf | SaveFormat::Ps | SaveFormat::Eps | SaveFormat::Svg
    ) {
        return Err(VectorImsaveError::NotVector(format));
    }
    let (rows, cols) = pixels.shape();
    let figure = Figure::with_size(cols as f64 / dpi, rows as f64 / dpi, dpi);
    figure.set_frameon(false);
    figure.figimage(
        ImageData::RgbaBytes(pixels),
        FigimageOptions {
            origin: Some(Origin::Upper),
            resize: true,
            ..FigimageOptions::default()
        },
    )?;
    let options = SaveOptions {
        dpi: Some(dpi),
        transparent: Some(true),
        metadata,
        ..SaveOptions::default()
    };
    Ok(figure.to_format(format, options)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_vector_format_holds_the_image_and_bitmaps_are_refused() {
        let pixels = Grid::from_vec(1, 2, vec![[0, 0, 255, 255], [255, 255, 0, 255]]).unwrap();
        let pdf = imsave_vector(pixels.clone(), SaveFormat::Pdf, 72.0, None).unwrap();
        assert!(pdf.starts_with(b"%PDF"));
        let eps = imsave_vector(pixels.clone(), SaveFormat::Eps, 72.0, None).unwrap();
        assert!(eps.starts_with(b"%!PS-Adobe-3.0 EPSF"));
        let ps = imsave_vector(pixels.clone(), SaveFormat::Ps, 72.0, None).unwrap();
        assert!(ps.starts_with(b"%!PS-Adobe-3.0"));
        assert!(matches!(
            imsave_vector(pixels, SaveFormat::Png, 72.0, None),
            Err(VectorImsaveError::NotVector(SaveFormat::Png))
        ));
    }
}
