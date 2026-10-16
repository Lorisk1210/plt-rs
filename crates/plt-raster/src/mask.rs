//! Single-channel 8-bit coverage maps: clip masks and glyph bitmaps.

use crate::error::RasterError;
use crate::pixmap::pixel_count;

/// A gray8 image whose values are coverage/opacity in `0..=255`.
///
/// Used for clip-path masks (Agg's `alpha_mask_gray8`, whose value scales
/// the anti-aliasing cover of everything drawn through it) and for
/// pre-rendered coverage such as glyph bitmaps. Rows run top to bottom.
#[derive(Clone, PartialEq, Eq)]
pub struct Mask {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl std::fmt::Debug for Mask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mask")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

impl Mask {
    /// Creates an all-zero (fully clipped / empty) mask.
    pub fn new(width: u32, height: u32) -> Result<Self, RasterError> {
        let n = pixel_count(width, height)?;
        Ok(Self {
            width,
            height,
            data: vec![0; n],
        })
    }

    /// Wraps existing gray8 bytes (row-major, top row first).
    pub fn from_vec(width: u32, height: u32, data: Vec<u8>) -> Result<Self, RasterError> {
        let expected = pixel_count(width, height)?;
        if data.len() != expected {
            return Err(RasterError::DataLength {
                expected,
                actual: data.len(),
            });
        }
        Ok(Self {
            width,
            height,
            data,
        })
    }

    /// Width in pixels.
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// The raw bytes, row-major, top row first.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Mutable access to the raw bytes.
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Consumes the mask, returning its bytes.
    pub fn into_vec(self) -> Vec<u8> {
        self.data
    }

    /// The value at `(x, y)`, or `None` when out of range.
    pub fn get(&self, x: u32, y: u32) -> Option<u8> {
        (x < self.width && y < self.height)
            .then(|| self.data[y as usize * self.width as usize + x as usize])
    }

    /// One row, or `None` when out of range.
    pub fn row(&self, y: u32) -> Option<&[u8]> {
        let w = self.width as usize;
        (y < self.height).then(|| &self.data[y as usize * w..][..w])
    }

    /// Mutable access to one row.
    pub(crate) fn row_mut(&mut self, y: u32) -> Option<&mut [u8]> {
        let w = self.width as usize;
        (y < self.height).then(|| &mut self.data[y as usize * w..][..w])
    }

    /// Sets every value.
    pub fn fill(&mut self, value: u8) {
        self.data.fill(value);
    }

    /// Fails unless the mask has exactly the given dimensions.
    pub(crate) fn check_covers(&self, width: u32, height: u32) -> Result<(), RasterError> {
        if (self.width, self.height) == (width, height) {
            Ok(())
        } else {
            Err(RasterError::MaskSize {
                mask: (self.width, self.height),
                target: (width, height),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_and_validation() {
        let mut m = Mask::new(3, 2).unwrap();
        m.data_mut()[4] = 7;
        assert_eq!(m.get(1, 1), Some(7));
        assert_eq!(m.get(3, 0), None);
        assert_eq!(m.row(1), Some(&[0, 7, 0][..]));
        assert!(Mask::from_vec(2, 2, vec![0; 3]).is_err());
        assert!(m.check_covers(3, 2).is_ok());
        assert_eq!(
            m.check_covers(2, 3),
            Err(RasterError::MaskSize {
                mask: (3, 2),
                target: (2, 3)
            })
        );
        m.fill(9);
        assert!(m.data().iter().all(|&v| v == 9));
    }
}
