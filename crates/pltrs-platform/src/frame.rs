//! Pixel buffers presented to windows.
//!
//! # Pixel convention
//!
//! Frames are **premultiplied RGBA8**, row-major, top row first, tightly
//! packed (`width * 4` bytes per row) — the layout produced by the pltrs
//! rasterizer. Windows are opaque, so presenting drops the alpha channel,
//! which composites the frame over black. Fully opaque frames (the normal
//! case, since figures paint an opaque background) therefore look the same
//! whether their data is premultiplied or straight.

use crate::error::Error;
use crate::geometry::PhysicalSize;

/// Bytes per RGBA8 pixel.
const BYTES_PER_PIXEL: usize = 4;

/// Number of bytes of a tightly packed RGBA8 buffer of `size`.
pub(crate) fn rgba_len(size: PhysicalSize) -> Result<usize, Error> {
    usize::try_from(size.area())
        .ok()
        .and_then(|pixels| pixels.checked_mul(BYTES_PER_PIXEL))
        .ok_or(Error::InvalidArgument("frame dimensions overflow"))
}

/// A borrowed premultiplied RGBA8 image to present to a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame<'a> {
    size: PhysicalSize,
    pixels: &'a [u8],
}

impl<'a> Frame<'a> {
    /// Wraps `pixels`, checking that it holds exactly `width * height * 4`
    /// bytes.
    pub fn new(width: u32, height: u32, pixels: &'a [u8]) -> Result<Self, Error> {
        let size = PhysicalSize::new(width, height);
        let expected = rgba_len(size)?;
        if pixels.len() != expected {
            return Err(Error::BufferSize {
                expected,
                actual: pixels.len(),
            });
        }
        Ok(Self { size, pixels })
    }

    /// Frame dimensions in physical pixels.
    pub const fn size(&self) -> PhysicalSize {
        self.size
    }

    /// Frame width in pixels.
    pub const fn width(&self) -> u32 {
        self.size.width
    }

    /// Frame height in pixels.
    pub const fn height(&self) -> u32 {
        self.size.height
    }

    /// All pixel bytes.
    pub const fn pixels(&self) -> &'a [u8] {
        self.pixels
    }

    /// The bytes of row `y` (empty if out of range).
    pub fn row(&self, y: u32) -> &'a [u8] {
        let stride = self.size.width as usize * BYTES_PER_PIXEL;
        let start = y as usize * stride;
        self.pixels.get(start..start + stride).unwrap_or(&[])
    }
}

/// An owned premultiplied RGBA8 image, e.g. a frame recorded by the
/// headless backend or read back from a window.
#[derive(Clone, PartialEq, Eq)]
pub struct OwnedFrame {
    size: PhysicalSize,
    pixels: Vec<u8>,
}

impl OwnedFrame {
    /// Takes ownership of `pixels`, checking that it holds exactly
    /// `width * height * 4` bytes.
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Result<Self, Error> {
        Frame::new(width, height, &pixels)?;
        Ok(Self {
            size: PhysicalSize::new(width, height),
            pixels,
        })
    }

    /// A frame of `size` filled with one RGBA colour.
    pub fn filled(size: PhysicalSize, rgba: [u8; 4]) -> Result<Self, Error> {
        let len = rgba_len(size)?;
        let pixels = rgba.iter().copied().cycle().take(len).collect();
        Ok(Self { size, pixels })
    }

    /// Borrows this image as a [`Frame`].
    pub fn as_frame(&self) -> Frame<'_> {
        Frame {
            size: self.size,
            pixels: &self.pixels,
        }
    }

    /// Image dimensions.
    pub const fn size(&self) -> PhysicalSize {
        self.size
    }

    /// All pixel bytes.
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Mutable access to the pixel bytes.
    pub fn pixels_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }

    /// The RGBA value at `(x, y)`, or `None` outside the image.
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.size.width || y >= self.size.height {
            return None;
        }
        let at = (y as usize * self.size.width as usize + x as usize) * BYTES_PER_PIXEL;
        let px = self.pixels.get(at..at + BYTES_PER_PIXEL)?;
        Some([px[0], px[1], px[2], px[3]])
    }

    /// Consumes the image, returning its pixel bytes.
    pub fn into_pixels(self) -> Vec<u8> {
        self.pixels
    }
}

impl std::fmt::Debug for OwnedFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OwnedFrame")
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_checks_length() {
        let data = [0_u8; 24];
        let frame = Frame::new(3, 2, &data).unwrap();
        assert_eq!(frame.size(), PhysicalSize::new(3, 2));
        assert_eq!(frame.row(1).len(), 12);
        assert_eq!(frame.row(2), b"");
        assert_eq!(
            Frame::new(3, 3, &data),
            Err(Error::BufferSize {
                expected: 36,
                actual: 24
            })
        );
    }

    #[test]
    fn empty_frames_are_valid() {
        let frame = Frame::new(0, 10, &[]).unwrap();
        assert!(frame.size().is_empty());
        assert_eq!(frame.row(0), b"");
    }

    #[test]
    fn huge_frames_are_rejected_without_overflow() {
        assert!(matches!(
            Frame::new(u32::MAX, u32::MAX, &[]),
            Err(Error::InvalidArgument(_) | Error::BufferSize { .. })
        ));
    }

    #[test]
    fn owned_frame_access() {
        let mut frame = OwnedFrame::filled(PhysicalSize::new(2, 2), [1, 2, 3, 4]).unwrap();
        assert_eq!(frame.pixel(1, 1), Some([1, 2, 3, 4]));
        assert_eq!(frame.pixel(2, 0), None);
        frame.pixels_mut()[0] = 9;
        assert_eq!(frame.pixel(0, 0), Some([9, 2, 3, 4]));
        assert_eq!(frame.as_frame().pixels().len(), 16);
        assert!(OwnedFrame::new(1, 1, vec![0; 3]).is_err());
        assert_eq!(frame.into_pixels().len(), 16);
    }
}
