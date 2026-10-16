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
