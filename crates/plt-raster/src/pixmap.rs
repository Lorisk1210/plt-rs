//! RGBA8 pixel buffers with straight alpha.

use crate::color::{Rgba8, multiply};
use crate::error::RasterError;

/// Exclusive upper bound on each side of a buffer; matplotlib's
/// `RendererAgg` rejects images of 2²³ pixels or more per side.
pub const MAX_DIMENSION: u32 = 1 << 23;

/// Checks dimensions and returns the number of pixels.
pub(crate) fn pixel_count(width: u32, height: u32) -> Result<usize, RasterError> {
    if width >= MAX_DIMENSION || height >= MAX_DIMENSION {
        return Err(RasterError::InvalidDimensions { width, height });
    }
    (width as usize)
        .checked_mul(height as usize)
        .ok_or(RasterError::InvalidDimensions { width, height })
}

/// An RGBA image, 8 bits per channel, rows top to bottom, straight
/// (non-premultiplied) alpha — the layout of matplotlib's Agg buffer
/// (`pixfmt_rgba32_plain`) and of `buffer_rgba()`.
#[derive(Clone, PartialEq, Eq)]
pub struct Pixmap {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl std::fmt::Debug for Pixmap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pixmap")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

impl Pixmap {
    /// Creates a fully transparent black pixmap. Either side may be zero.
    pub fn new(width: u32, height: u32) -> Result<Self, RasterError> {
        Self::filled(width, height, Rgba8::default())
    }

    /// Creates a pixmap filled with `color`. matplotlib's canvas starts as
    /// [`Rgba8::TRANSPARENT_WHITE`].
    pub fn filled(width: u32, height: u32, color: Rgba8) -> Result<Self, RasterError> {
        let n = pixel_count(width, height)?;
        Ok(Self {
            width,
            height,
            data: color.to_array().repeat(n),
        })
    }

    /// Wraps existing RGBA bytes (row-major, top row first).
    pub fn from_vec(width: u32, height: u32, data: Vec<u8>) -> Result<Self, RasterError> {
        let expected = pixel_count(width, height)? * 4;
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

    /// Builds a pixmap from premultiplied RGBA bytes, converting to straight
    /// alpha (Agg's `demultiply`).
    pub fn from_premultiplied(
        width: u32,
        height: u32,
        mut data: Vec<u8>,
    ) -> Result<Self, RasterError> {
        for px in data.as_chunks_mut::<4>().0 {
            let a = u32::from(px[3]);
            for c in &mut px[..3] {
                let v = u32::from(*c);
                *c = if v * a == 0 {
                    0
                } else if v >= a {
                    255
                } else {
                    ((v * 255 + (a >> 1)) / a) as u8
                };
            }
        }
        Self::from_vec(width, height, data)
    }

    /// Width in pixels.
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// The raw RGBA bytes, row-major, top row first.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Mutable access to the raw RGBA bytes.
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Consumes the pixmap, returning its bytes.
    pub fn into_vec(self) -> Vec<u8> {
        self.data
    }

    /// One row of pixels (`width × 4` bytes), or `None` when out of range.
    pub fn row(&self, y: u32) -> Option<&[u8]> {
        let stride = self.width as usize * 4;
        (y < self.height).then(|| &self.data[y as usize * stride..][..stride])
    }

    /// Mutable access to one row of pixels.
    pub fn row_mut(&mut self, y: u32) -> Option<&mut [u8]> {
        let stride = self.width as usize * 4;
        (y < self.height).then(|| &mut self.data[y as usize * stride..][..stride])
    }

    /// The pixel at `(x, y)`, or `None` when out of range.
    pub fn pixel(&self, x: u32, y: u32) -> Option<Rgba8> {
        self.offset(x, y).map(|i| {
            Rgba8::new(
                self.data[i],
                self.data[i + 1],
                self.data[i + 2],
                self.data[i + 3],
            )
        })
    }

    /// Overwrites the pixel at `(x, y)`; returns `false` when out of range.
    pub fn set_pixel(&mut self, x: u32, y: u32, color: Rgba8) -> bool {
        match self.offset(x, y) {
            Some(i) => {
                self.data[i..i + 4].copy_from_slice(&color.to_array());
                true
            }
            None => false,
        }
    }

    /// Overwrites every pixel with `color` (matplotlib's `clear()`).
    pub fn fill(&mut self, color: Rgba8) {
        self.data.as_chunks_mut::<4>().0.fill(color.to_array());
    }

    /// Copies the `width × height` region whose top-left corner is
    /// `(x, y)` into a new pixmap (matplotlib's `copy_from_bbox`). Parts
    /// outside this pixmap come out transparent black.
    pub fn copy_region(
        &self,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> Result<Self, RasterError> {
        let mut out = Self::new(width, height)?;
        out.copy_from(self, -i64::from(x), -i64::from(y));
        Ok(out)
    }

    /// Copies `src` into this pixmap with its top-left corner at `(x, y)`,
    /// replacing (not blending) the covered pixels — matplotlib's
    /// `restore_region`. Pixels falling outside are dropped.
    pub fn paste(&mut self, src: &Self, x: i32, y: i32) {
        self.copy_from(src, i64::from(x), i64::from(y));
    }

    fn copy_from(&mut self, src: &Self, dx: i64, dy: i64) {
        let x0 = dx.max(0);
        let x1 = (dx + i64::from(src.width)).min(i64::from(self.width));
        if x0 >= x1 {
            return;
        }
        let len = (x1 - x0) as usize * 4;
        let src_x = (x0 - dx) as usize * 4;
        for y in dy.max(0)..(dy + i64::from(src.height)).min(i64::from(self.height)) {
            let src_row = &src.data[(y - dy) as usize * src.width as usize * 4..];
            let dst_row = y as usize * self.width as usize * 4;
            self.data[dst_row + x0 as usize * 4..][..len].copy_from_slice(&src_row[src_x..][..len]);
        }
    }

    /// The pixels in premultiplied-alpha form (each colour channel
    /// multiplied by alpha with Agg's rounding).
    pub fn to_premultiplied(&self) -> Vec<u8> {
        let mut out = self.data.clone();
        for px in out.as_chunks_mut::<4>().0 {
            let a = px[3];
            for c in &mut px[..3] {
                *c = multiply(*c, a);
            }
        }
        out
    }

    /// The pixels as packed RGB, discarding alpha.
    pub fn to_rgb(&self) -> Vec<u8> {
        self.data
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|&[r, g, b, _]| [r, g, b])
            .collect()
    }

    fn offset(&self, x: u32, y: u32) -> Option<usize> {
        (x < self.width && y < self.height)
            .then(|| (y as usize * self.width as usize + x as usize) * 4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction_and_access() {
        let mut p = Pixmap::filled(3, 2, Rgba8::TRANSPARENT_WHITE).unwrap();
        assert_eq!(p.data().len(), 24);
        assert_eq!(p.pixel(2, 1), Some(Rgba8::TRANSPARENT_WHITE));
        assert!(p.set_pixel(1, 1, Rgba8::new(1, 2, 3, 4)));
        assert!(!p.set_pixel(3, 0, Rgba8::default()));
        assert_eq!(p.pixel(1, 1), Some(Rgba8::new(1, 2, 3, 4)));
        assert_eq!(p.pixel(0, 2), None);
        assert_eq!(&p.row(1).unwrap()[4..8], &[1, 2, 3, 4]);
    }

    #[test]
    fn rejects_bad_dimensions_and_lengths() {
        assert!(Pixmap::new(MAX_DIMENSION, 1).is_err());
        assert_eq!(
            Pixmap::from_vec(2, 2, vec![0; 15]),
            Err(RasterError::DataLength {
                expected: 16,
                actual: 15
            })
        );
        assert_eq!(Pixmap::new(0, 5).unwrap().data(), []);
    }

    #[test]
    fn region_copy_and_paste_round_trip() {
        let mut p = Pixmap::new(4, 4).unwrap();
        p.set_pixel(1, 1, Rgba8::new(9, 9, 9, 9));
        let region = p.copy_region(1, 1, 2, 2).unwrap();
        assert_eq!(region.pixel(0, 0), Some(Rgba8::new(9, 9, 9, 9)));
        let outside = p.copy_region(-1, -1, 2, 2).unwrap();
        assert_eq!(outside.pixel(0, 0), Some(Rgba8::default()));
        let mut q = Pixmap::filled(4, 4, Rgba8::new(1, 1, 1, 1)).unwrap();
        q.paste(&region, 3, 3);
        assert_eq!(q.pixel(3, 3), Some(Rgba8::new(9, 9, 9, 9)));
        q.paste(&region, -10, 0);
        assert_eq!(q.pixel(0, 0), Some(Rgba8::new(1, 1, 1, 1)));
    }

    #[test]
    fn premultiplication_round_trip() {
        let p = Pixmap::from_vec(2, 1, vec![200, 100, 0, 128, 10, 20, 30, 0]).unwrap();
        let pre = p.to_premultiplied();
        assert_eq!(&pre[..4], &[100, 50, 0, 128]);
        let back = Pixmap::from_premultiplied(2, 1, pre).unwrap();
        assert_eq!(back.pixel(0, 0), Some(Rgba8::new(199, 100, 0, 128)));
        assert_eq!(back.pixel(1, 0), Some(Rgba8::new(0, 0, 0, 0)));
        assert_eq!(p.to_rgb(), vec![200, 100, 0, 10, 20, 30]);
    }
}
