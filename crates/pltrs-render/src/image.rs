//! Raster images handed to renderers.

use std::fmt;

/// An RGBA8 image with straight alpha, rows stored top to bottom.
#[derive(Clone, PartialEq, Eq)]
pub struct RgbaImage {
    width: usize,
    height: usize,
    data: Vec<u8>,
}

impl fmt::Debug for RgbaImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RgbaImage")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

/// The pixel buffer does not match the declared dimensions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageSizeError {
    /// Bytes required (`width * height * 4`).
    pub expected: usize,
    /// Bytes supplied.
    pub actual: usize,
}

impl fmt::Display for ImageSizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RGBA image needs {} bytes, got {}",
            self.expected, self.actual
        )
    }
}

impl std::error::Error for ImageSizeError {}

impl RgbaImage {
    /// Wraps `data` (`width * height * 4` bytes, row-major, top row first).
    pub fn new(width: usize, height: usize, data: Vec<u8>) -> Result<Self, ImageSizeError> {
        let expected = width * height * 4;
        if data.len() == expected {
            Ok(Self {
                width,
                height,
                data,
            })
        } else {
            Err(ImageSizeError {
                expected,
                actual: data.len(),
            })
        }
    }

    /// A fully transparent image.
    pub fn transparent(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            data: vec![0; width * height * 4],
        }
    }

    /// Width in pixels.
    pub const fn width(&self) -> usize {
        self.width
    }

    /// Height in pixels.
    pub const fn height(&self) -> usize {
        self.height
    }

    /// The raw RGBA bytes.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Mutable raw RGBA bytes.
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Consumes the image, returning its bytes.
    pub fn into_data(self) -> Vec<u8> {
        self.data
    }

    /// The smallest box `(left, top, right, bottom)` (ends exclusive)
    /// holding every pixel that is not fully transparent; `None` when all
    /// are (matplotlib's `cbook._get_nonzero_slices` of the alpha
    /// channel).
    pub fn opaque_bounds(&self) -> Option<(usize, usize, usize, usize)> {
        let alpha = |x: usize, y: usize| self.data[(y * self.width + x) * 4 + 3];
        let rows: Vec<usize> = (0..self.height)
            .filter(|&y| (0..self.width).any(|x| alpha(x, y) != 0))
            .collect();
        let columns: Vec<usize> = (0..self.width)
            .filter(|&x| (0..self.height).any(|y| alpha(x, y) != 0))
            .collect();
        Some((
            *columns.first()?,
            *rows.first()?,
            *columns.last()? + 1,
            *rows.last()? + 1,
        ))
    }

    /// The part `(left, top, right, bottom)` (ends exclusive) of the
    /// image, which must lie inside it.
    pub(crate) fn crop(&self, (left, top, right, bottom): (usize, usize, usize, usize)) -> Self {
        let data: Vec<u8> = (top..bottom)
            .flat_map(|y| {
                self.data[(y * self.width + left) * 4..(y * self.width + right) * 4].iter()
            })
            .copied()
            .collect();
        Self::new(right - left, bottom - top, data).expect("the crop holds its own pixels")
    }

    /// The pixel at `(x, y)` (top-left origin), if in bounds.
    pub fn pixel(&self, x: usize, y: usize) -> Option<[u8; 4]> {
        (x < self.width && y < self.height).then(|| {
            let i = (y * self.width + x) * 4;
            [
                self.data[i],
                self.data[i + 1],
                self.data[i + 2],
                self.data[i + 3],
            ]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crops_to_the_opaque_pixels() {
        let mut data = vec![0; 4 * 3 * 4];
        data[(4 + 2) * 4 + 3] = 255;
        let image = RgbaImage::new(4, 3, data).unwrap();
        assert_eq!(image.opaque_bounds(), Some((2, 1, 3, 2)));
        assert_eq!(image.crop((2, 1, 3, 2)).data(), &[0, 0, 0, 255]);
        assert_eq!(RgbaImage::transparent(2, 2).opaque_bounds(), None);
    }

    #[test]
    fn size_validation_and_access() {
        assert_eq!(
            RgbaImage::new(2, 2, vec![0; 15]),
            Err(ImageSizeError {
                expected: 16,
                actual: 15
            })
        );
        let mut img = RgbaImage::transparent(2, 1);
        img.data_mut()[4..8].copy_from_slice(&[1, 2, 3, 4]);
        assert_eq!(img.pixel(1, 0), Some([1, 2, 3, 4]));
        assert_eq!(img.pixel(2, 0), None);
    }
}
