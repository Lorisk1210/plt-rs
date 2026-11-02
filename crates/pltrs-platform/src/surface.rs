//! The retained contents of a window.
//!
//! Backends whose windows the platform may ask to repaint at any time
//! (macOS's `drawRect:`, Windows' `WM_PAINT`, a live resize) keep what was
//! last presented, so that they can repaint without waiting for the
//! application; [`Context::read_pixels`](crate::Context::read_pixels)
//! reads it back. The headless backend keeps one per window too.
//!
//! A surface is opaque: presenting drops the alpha channel of the
//! premultiplied frame, which composites it over black (see
//! [`Frame`]).

use crate::error::Error;
use crate::frame::{Frame, OwnedFrame, rgba_len};
use crate::geometry::{PhysicalRect, PhysicalSize};

/// Bytes per pixel of a surface.
const BYTES_PER_PIXEL: usize = 4;

/// The byte order of a surface's pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChannelOrder {
    /// Red, green, blue, alpha: the order of [`Frame`]s.
    Rgba,
    /// Blue, green, red, alpha: the native order of 32-bit Windows DIBs and
    /// of little-endian Core Graphics images.
    #[cfg(any(target_os = "macos", windows, test))]
    Bgra,
}

impl ChannelOrder {
    /// The pixel in this order of the premultiplied RGBA pixel `rgba`, made
    /// opaque.
    const fn opaque(self, rgba: [u8; 4]) -> [u8; 4] {
        let [r, g, b, _] = rgba;
        match self {
            Self::Rgba => [r, g, b, u8::MAX],
            #[cfg(any(target_os = "macos", windows, test))]
            Self::Bgra => [b, g, r, u8::MAX],
        }
    }
}

/// An opaque image the size of a window's content area.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Surface {
    order: ChannelOrder,
    size: PhysicalSize,
    pixels: Vec<u8>,
}

impl Surface {
    /// An opaque black surface of `size`.
    pub(crate) fn new(size: PhysicalSize, order: ChannelOrder) -> Result<Self, Error> {
        let black = order.opaque([0, 0, 0, 0]);
        let pixels = black
            .iter()
            .copied()
            .cycle()
            .take(rgba_len(size)?)
            .collect();
        Ok(Self {
            order,
            size,
            pixels,
        })
    }

    /// The surface's size.
    pub(crate) const fn size(&self) -> PhysicalSize {
        self.size
    }

    /// Changes the size, keeping the pixels the old and new sizes share
    /// (anchored at the top-left corner) and filling the rest with black.
    pub(crate) fn resize(&mut self, size: PhysicalSize) -> Result<(), Error> {
        if size == self.size {
            return Ok(());
        }
        let mut resized = Self::new(size, self.order)?;
        let overlap = PhysicalRect::from_size(self.size).clip_to(size);
        let (x0, x1) = byte_span(overlap);
        for y in overlap.y..overlap.y + overlap.height {
            let source = &self.row(y)[x0..x1];
            resized.row_mut(y)[x0..x1].copy_from_slice(source);
        }
        *self = resized;
        Ok(())
    }

    /// Copies `region` of `frame` to the same place of the surface,
    /// clipped to both, dropping alpha; returns the region copied.
    pub(crate) fn blit(&mut self, frame: &Frame<'_>, region: PhysicalRect) -> PhysicalRect {
        let region = region.clip_to(frame.size()).clip_to(self.size);
        let (x0, x1) = byte_span(region);
        let order = self.order;
        for y in region.y..region.y + region.height {
            let source = frame.row(y)[x0..x1].as_chunks::<4>().0;
            let target = self.row_mut(y)[x0..x1].as_chunks_mut::<4>().0;
            for (to, from) in target.iter_mut().zip(source) {
                *to = order.opaque(*from);
            }
        }
        region
    }

    /// The pixels of `region` (clipped to the surface), row after row,
    /// tightly packed, in the surface's channel order; returns them with
    /// the region they cover.
    #[cfg(any(target_os = "macos", windows, test))]
    pub(crate) fn crop(&self, region: PhysicalRect) -> (PhysicalRect, Vec<u8>) {
        let region = region.clip_to(self.size);
        let (x0, x1) = byte_span(region);
        let mut pixels = Vec::with_capacity((x1 - x0) * region.height as usize);
        for y in region.y..region.y + region.height {
            pixels.extend_from_slice(&self.row(y)[x0..x1]);
        }
        (region, pixels)
    }

    /// The surface as an opaque RGBA image.
    pub(crate) fn to_rgba(&self) -> OwnedFrame {
        let mut pixels = self.pixels.clone();
        if self.order != ChannelOrder::Rgba {
            for pixel in pixels.as_chunks_mut::<4>().0 {
                pixel.swap(0, 2);
            }
        }
        OwnedFrame::new(self.size.width, self.size.height, pixels)
            .expect("a surface holds exactly its size's pixels")
    }

    fn stride(&self) -> usize {
        self.size.width as usize * BYTES_PER_PIXEL
    }

    fn row(&self, y: u32) -> &[u8] {
        let start = y as usize * self.stride();
        &self.pixels[start..start + self.stride()]
    }

    fn row_mut(&mut self, y: u32) -> &mut [u8] {
        let (start, stride) = (y as usize * self.stride(), self.stride());
        &mut self.pixels[start..start + stride]
    }
}

impl std::fmt::Debug for Surface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Surface")
            .field("order", &self.order)
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}

/// The byte range of `region`'s columns within a row.
const fn byte_span(region: PhysicalRect) -> (usize, usize) {
    (
        region.x as usize * BYTES_PER_PIXEL,
        (region.x + region.width) as usize * BYTES_PER_PIXEL,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(size: PhysicalSize, rgba: [u8; 4]) -> OwnedFrame {
        OwnedFrame::filled(size, rgba).unwrap()
    }

    #[test]
    fn new_surfaces_are_opaque_black() {
        let size = PhysicalSize::new(2, 1);
        let surface = Surface::new(size, ChannelOrder::Bgra).unwrap();
        assert_eq!(
            surface.crop(PhysicalRect::from_size(size)).1,
            [0, 0, 0, 255, 0, 0, 0, 255]
        );
        assert!(Surface::new(PhysicalSize::new(u32::MAX, u32::MAX), ChannelOrder::Rgba).is_err());
    }

    #[test]
    fn blit_respects_the_region_and_forces_alpha() {
        let source = frame(PhysicalSize::new(4, 4), [10, 20, 30, 0]);
        let mut surface = Surface::new(PhysicalSize::new(3, 3), ChannelOrder::Rgba).unwrap();
        let copied = surface.blit(&source.as_frame(), PhysicalRect::new(1, 1, 10, 10));
        assert_eq!(copied, PhysicalRect::new(1, 1, 2, 2));
        let image = surface.to_rgba();
        assert_eq!(image.pixel(0, 0), Some([0, 0, 0, 255]));
        assert_eq!(image.pixel(1, 0), Some([0, 0, 0, 255]));
        assert_eq!(image.pixel(1, 1), Some([10, 20, 30, 255]));
        assert_eq!(image.pixel(2, 2), Some([10, 20, 30, 255]));
    }

    #[test]
    fn bgra_surfaces_swap_red_and_blue_and_drop_premultiplied_alpha() {
        // A half-transparent premultiplied red over black is half red.
        let source = frame(PhysicalSize::new(2, 2), [128, 0, 64, 128]);
        let mut surface = Surface::new(PhysicalSize::new(2, 2), ChannelOrder::Bgra).unwrap();
        surface.blit(&source.as_frame(), PhysicalRect::new(1, 0, 1, 2));
        let (region, pixels) = surface.crop(PhysicalRect::new(1, 0, 1, 2));
        assert_eq!(region, PhysicalRect::new(1, 0, 1, 2));
        assert_eq!(pixels, [64, 0, 128, 255, 64, 0, 128, 255]);
        assert_eq!(surface.to_rgba().pixel(1, 1), Some([128, 0, 64, 255]));
        assert_eq!(surface.to_rgba().pixel(0, 1), Some([0, 0, 0, 255]));
    }

    #[test]
    fn resizing_keeps_the_overlap() {
        let mut surface = Surface::new(PhysicalSize::new(2, 2), ChannelOrder::Bgra).unwrap();
        let white = frame(PhysicalSize::new(2, 2), [255; 4]);
        surface.blit(&white.as_frame(), PhysicalRect::new(0, 0, 2, 2));
        surface.resize(PhysicalSize::new(3, 1)).unwrap();
        let image = surface.to_rgba();
        assert_eq!(image.size(), PhysicalSize::new(3, 1));
        assert_eq!(image.pixel(1, 0), Some([255; 4]));
        assert_eq!(image.pixel(2, 0), Some([0, 0, 0, 255]));
        surface.resize(PhysicalSize::new(0, 0)).unwrap();
        assert_eq!(surface.to_rgba().pixels(), []);
    }

    #[test]
    fn crops_are_clipped_and_tightly_packed() {
        let mut surface = Surface::new(PhysicalSize::new(3, 2), ChannelOrder::Rgba).unwrap();
        let grey = frame(PhysicalSize::new(3, 2), [7, 8, 9, 255]);
        surface.blit(&grey.as_frame(), PhysicalRect::new(2, 1, 1, 1));
        let (region, pixels) = surface.crop(PhysicalRect::new(1, 1, 9, 9));
        assert_eq!(region, PhysicalRect::new(1, 1, 2, 1));
        assert_eq!(pixels, [0, 0, 0, 255, 7, 8, 9, 255]);
        let (empty, none) = surface.crop(PhysicalRect::new(5, 5, 1, 1));
        assert!(empty.is_empty() && none.is_empty());
    }
}
