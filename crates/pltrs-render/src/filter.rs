//! Image filters applied to what an artist draws (matplotlib's agg
//! filters: `Artist.set_agg_filter` with `RendererAgg.start_filter` and
//! `stop_filter`).

use crate::image::{ImageSizeError, RgbaImage};

/// The pixels an image filter works on: straight-alpha RGBA, nominally in
/// `[0, 1]`, row-major with the top row first (matplotlib's
/// `(height, width, 4)` float array).
#[derive(Clone, Debug, PartialEq)]
pub struct FilterImage {
    width: usize,
    height: usize,
    pixels: Vec<[f64; 4]>,
}

impl FilterImage {
    /// Wraps `pixels` (`width * height` of them, top row first).
    pub fn new(width: usize, height: usize, pixels: Vec<[f64; 4]>) -> Result<Self, ImageSizeError> {
        if pixels.len() == width * height {
            Ok(Self {
                width,
                height,
                pixels,
            })
        } else {
            Err(ImageSizeError {
                expected: width * height * 4,
                actual: pixels.len() * 4,
            })
        }
    }

    /// An image of one colour.
    pub fn filled(width: usize, height: usize, pixel: [f64; 4]) -> Self {
        Self {
            width,
            height,
            pixels: vec![pixel; width * height],
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

    /// The pixels, top row first.
    pub fn pixels(&self) -> &[[f64; 4]] {
        &self.pixels
    }

    /// The pixels, mutably.
    pub fn pixels_mut(&mut self) -> &mut [[f64; 4]] {
        &mut self.pixels
    }

    /// The pixel in column `x` of row `y` (from the top), if in bounds.
    pub fn pixel(&self, x: usize, y: usize) -> Option<[f64; 4]> {
        (x < self.width && y < self.height).then(|| self.pixels[y * self.width + x])
    }

    /// The 8-bit image as fractions of 255.
    pub(crate) fn from_rgba(image: &RgbaImage) -> Self {
        let pixels = image
            .data()
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| p.map(|c| f64::from(c) / 255.0))
            .collect();
        Self {
            width: image.width(),
            height: image.height(),
            pixels,
        }
    }

    /// The image in 8 bits, truncating `value * 255` as numpy's
    /// `astype(uint8)` does; values outside `[0, 1]` are clamped.
    pub(crate) fn to_rgba(&self) -> RgbaImage {
        let data = self
            .pixels
            .iter()
            .flat_map(|pixel| pixel.map(|c| (c * 255.0).clamp(0.0, 255.0) as u8))
            .collect();
        RgbaImage::new(self.width, self.height, data).expect("one RGBA quadruple per pixel")
    }
}

/// What an image filter returns: the new image, and where its bottom-left
/// corner goes in pixels from the bottom-left corner of the image it was
/// given.
#[derive(Clone, Debug, PartialEq)]
pub struct Filtered {
    /// The filtered image (of any size).
    pub image: FilterImage,
    /// The offset of its bottom-left corner.
    pub offset: (f64, f64),
}

/// An image filter (matplotlib's agg filter function): it receives what an
/// artist drew, cropped to its visible pixels, and the resolution in dots
/// per inch.
pub type FilterFn<'a> = dyn Fn(&FilterImage, f64) -> Filtered + 'a;

/// An image filter the renderer may keep (see
/// [`Renderer::stop_shared_filter`](crate::Renderer::stop_shared_filter)).
pub type SharedFilter = std::rc::Rc<FilterFn<'static>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eight_bit_round_trips_truncate_like_numpy() {
        let image = RgbaImage::new(1, 1, vec![0, 128, 255, 7]).unwrap();
        let filter = FilterImage::from_rgba(&image);
        assert_eq!(filter.to_rgba(), image);
        let odd = FilterImage::new(2, 1, vec![[0.999, -0.5, 1.5, f64::NAN], [0.5; 4]]).unwrap();
        assert_eq!(odd.to_rgba().data(), &[254, 0, 255, 0, 127, 127, 127, 127]);
        assert!(FilterImage::new(2, 2, vec![[0.0; 4]]).is_err());
        assert_eq!(
            FilterImage::filled(2, 1, [1.0; 4]).pixel(1, 0),
            Some([1.0; 4])
        );
    }
}
