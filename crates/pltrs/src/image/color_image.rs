//! The RGBA pixels of RGB and RGBA images, as matplotlib's
//! `ScalarMappable.to_rgba(image)` prepares them for `imsave` and for
//! callers: RGB pixels are opaque, a pixel with a NaN channel becomes
//! transparent black, float channels must lie in `[0, 1]` and 8-bit
//! channels stand for `channel / 255`.

use super::{ImageData, ImageIoError};
use crate::numeric::Grid;

/// A float channel type of colour images (`f64` or `f32`).
trait Channel: Copy + PartialOrd + From<u8> + Into<f64> + std::ops::Mul<Output = Self> {}

impl Channel for f64 {}
impl Channel for f32 {}

/// The pixels of a float image as RGBA (`rgba` gives each its alpha),
/// pixels with a NaN channel zeroed, every channel checked against
/// `[0, 1]`.
fn checked_pixels<P, T: Channel>(
    grid: &Grid<P>,
    rgba: impl Fn(&P) -> [T; 4],
) -> Result<Vec<[T; 4]>, ImageIoError> {
    let zero = T::from(0);
    let is_nan = |v: &T| v.partial_cmp(v).is_none();
    let pixels: Vec<[T; 4]> = grid
        .as_slice()
        .iter()
        .map(|pixel| {
            let pixel = rgba(pixel);
            if pixel.iter().any(is_nan) {
                [zero; 4]
            } else {
                pixel
            }
        })
        .collect();
    let one = T::from(1);
    if pixels.iter().flatten().any(|&v| v < zero || v > one) {
        return Err(ImageIoError::ColorRange);
    }
    Ok(pixels)
}

/// Checked float pixels as bytes: multiplied by 255 in the image's
/// precision, then truncated as numpy's `astype(np.uint8)`.
fn float_bytes<T: Channel>(pixels: Vec<[T; 4]>) -> Vec<[u8; 4]> {
    let scale = T::from(255);
    let byte = |v: T| (v * scale).into() as u8;
    pixels.into_iter().map(|pixel| pixel.map(byte)).collect()
}

/// Checked float pixels widened to `f64`.
fn widened<T: Channel>(pixels: Vec<[T; 4]>) -> Vec<[f64; 4]> {
    pixels
        .into_iter()
        .map(|pixel| pixel.map(Into::into))
        .collect()
}

/// 8-bit pixels as fractions of 255.
fn fractions(pixels: impl Iterator<Item = [u8; 4]>) -> Vec<[f64; 4]> {
    pixels
        .map(|pixel| pixel.map(|v| f64::from(v) / 255.0))
        .collect()
}

/// The RGBA colours of an RGB or RGBA image, row-major, with channels in
/// `[0, 1]` (matplotlib's `ScalarMappable.to_rgba(image)`).
///
/// ```
/// use pltrs::image::{ImageData, color_image_rgba};
/// use pltrs::numeric::Grid;
/// let image = Grid::from_vec(1, 2, vec![[0.5, 0.5, 0.5], [f64::NAN, 0.5, 0.5]])?;
/// let pixels = color_image_rgba(&ImageData::Rgb(image))?;
/// assert_eq!(pixels, [[0.5, 0.5, 0.5, 1.0], [0.0; 4]]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// When a float channel lies outside `[0, 1]`, or the image holds scalars
/// (which a colormap maps instead).
pub fn color_image_rgba(data: &ImageData) -> Result<Vec<[f64; 4]>, ImageIoError> {
    match data {
        ImageData::Scalar(_) => Err(ImageIoError::ScalarImage),
        ImageData::Rgb(grid) => checked_pixels(grid, |&[r, g, b]| [r, g, b, 1.0]),
        ImageData::Rgba(grid) => checked_pixels(grid, |&pixel| pixel),
        ImageData::RgbF32(grid) => checked_pixels(grid, |&[r, g, b]| [r, g, b, 1.0]).map(widened),
        ImageData::RgbaF32(grid) => checked_pixels(grid, |&pixel| pixel).map(widened),
        ImageData::RgbBytes(grid) => Ok(fractions(
            grid.as_slice().iter().map(|&[r, g, b]| [r, g, b, 255]),
        )),
        ImageData::RgbaBytes(grid) => Ok(fractions(grid.as_slice().iter().copied())),
    }
}

/// The 8-bit RGBA pixels of an RGB or RGBA image, row-major (matplotlib's
/// `ScalarMappable.to_rgba(image, bytes=True)`): float channels are
/// multiplied by 255 and truncated.
///
/// # Errors
///
/// As [`color_image_rgba`].
pub fn color_image_bytes(data: &ImageData) -> Result<Vec<[u8; 4]>, ImageIoError> {
    match data {
        ImageData::Scalar(_) => Err(ImageIoError::ScalarImage),
        ImageData::Rgb(grid) => checked_pixels(grid, |&[r, g, b]| [r, g, b, 1.0]).map(float_bytes),
        ImageData::Rgba(grid) => checked_pixels(grid, |&pixel| pixel).map(float_bytes),
        ImageData::RgbF32(grid) => {
            checked_pixels(grid, |&[r, g, b]| [r, g, b, 1.0]).map(float_bytes)
        }
        ImageData::RgbaF32(grid) => checked_pixels(grid, |&pixel| pixel).map(float_bytes),
        ImageData::RgbBytes(grid) => Ok(grid
            .as_slice()
            .iter()
            .map(|&[r, g, b]| [r, g, b, 255])
            .collect()),
        ImageData::RgbaBytes(grid) => Ok(grid.as_slice().to_vec()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgba(pixels: Vec<[f64; 4]>) -> ImageData {
        let n = pixels.len();
        ImageData::Rgba(Grid::from_vec(1, n, pixels).unwrap())
    }

    #[test]
    fn nan_pixels_become_transparent_black() {
        let image = rgba(vec![[f64::NAN, 0.5, 0.5, 0.5], [0.5, 0.5, 0.5, 0.5]]);
        assert_eq!(
            color_image_rgba(&image).unwrap(),
            [[0.0; 4], [0.5, 0.5, 0.5, 0.5]]
        );
        assert_eq!(color_image_bytes(&image).unwrap(), [[0; 4], [127; 4]]);
    }

    #[test]
    fn float_channels_must_lie_in_the_unit_range() {
        let image = rgba(vec![[42.0, 0.5, 0.5, 1.0]]);
        assert!(matches!(
            color_image_rgba(&image),
            Err(ImageIoError::ColorRange)
        ));
        assert!(matches!(
            color_image_bytes(&image),
            Err(ImageIoError::ColorRange)
        ));
    }

    #[test]
    fn byte_images_are_fractions_of_255() {
        let image = ImageData::RgbBytes(Grid::from_vec(1, 1, vec![[255, 0, 51]]).unwrap());
        assert_eq!(color_image_rgba(&image).unwrap(), [[1.0, 0.0, 0.2, 1.0]]);
        assert_eq!(color_image_bytes(&image).unwrap(), [[255, 0, 51, 255]]);
    }

    #[test]
    fn scalar_images_are_not_colour_images() {
        let image = ImageData::Scalar(super::super::ScalarField::new(
            Grid::from_vec(1, 1, vec![0.5]).unwrap(),
        ));
        assert!(matches!(
            color_image_rgba(&image),
            Err(ImageIoError::ScalarImage)
        ));
    }
}
