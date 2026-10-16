//! Borrowed pixel data handed to the encoder.

use super::error::EncodeError;
use super::header::valid_dimensions;
use super::types::{BitDepth, ColorType, PixelFormat};

/// A borrowed, validated view of pixels to encode.
///
/// Rows run top to bottom with no padding between them. 16-bit samples are
/// big-endian byte pairs (PNG's own order); 1-, 2- and 4-bit grey samples are
/// packed most significant bits first, each row starting on a fresh byte.
///
/// ```
/// use plt_codec::png::{BitDepth, ColorType, ImageRef};
///
/// let pixels = [0u8; 3 * 2 * 4];
/// let image = ImageRef::rgba8(3, 2, &pixels)?;
/// assert_eq!(image.row_bytes(), 12);
///
/// // Two 16-bit grey pixels: 0x0102 and 0xFFFF.
/// let image = ImageRef::new(2, 1, ColorType::Gray, BitDepth::Sixteen, &[1, 2, 0xFF, 0xFF])?;
/// assert!(ImageRef::rgba8(3, 3, &pixels).is_err());
/// # Ok::<(), plt_codec::png::EncodeError>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageRef<'a> {
    width: u32,
    height: u32,
    format: PixelFormat,
    data: &'a [u8],
}

impl<'a> ImageRef<'a> {
    /// Wraps `data` as a `width` × `height` image in the given format.
    ///
    /// The encoder supports grey at every bit depth and grey + alpha, RGB and
    /// RGBA at 8 or 16 bits; indexed images are not supported.
    pub fn new(
        width: u32,
        height: u32,
        color: ColorType,
        bit_depth: BitDepth,
        data: &'a [u8],
    ) -> Result<Self, EncodeError> {
        if !valid_dimensions(width, height) {
            return Err(EncodeError::InvalidDimensions { width, height });
        }
        let format = PixelFormat::new(color, bit_depth)
            .filter(|_| color != ColorType::Indexed)
            .ok_or(EncodeError::UnsupportedFormat { color, bit_depth })?;
        let expected = format
            .image_bytes(width, height)
            .ok_or(EncodeError::ImageTooLarge)?;
        if data.len() != expected {
            return Err(EncodeError::DataLengthMismatch {
                expected,
                actual: data.len(),
            });
        }
        Ok(Self {
            width,
            height,
            format,
            data,
        })
    }

    /// Wraps 8-bit RGBA pixels (4 bytes per pixel), the format of rendered
    /// figures.
    pub fn rgba8(width: u32, height: u32, data: &'a [u8]) -> Result<Self, EncodeError> {
        Self::new(width, height, ColorType::Rgba, BitDepth::Eight, data)
    }

    /// Wraps 8-bit RGB pixels (3 bytes per pixel).
    pub fn rgb8(width: u32, height: u32, data: &'a [u8]) -> Result<Self, EncodeError> {
        Self::new(width, height, ColorType::Rgb, BitDepth::Eight, data)
    }

    /// Width in pixels.
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Colour type of the pixels.
    pub const fn color_type(&self) -> ColorType {
        self.format.color
    }

    /// Bits per sample.
    pub const fn bit_depth(&self) -> BitDepth {
        self.format.depth
    }

    /// The pixel bytes.
    pub const fn data(&self) -> &'a [u8] {
        self.data
    }

    /// Bytes per row.
    pub fn row_bytes(&self) -> usize {
        self.data.len() / self.height as usize
    }

    pub(crate) const fn format(&self) -> PixelFormat {
        self.format
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_dimensions_format_and_length() {
        assert_eq!(
            ImageRef::rgba8(0, 1, &[]),
            Err(EncodeError::InvalidDimensions {
                width: 0,
                height: 1
            })
        );
        assert_eq!(
            ImageRef::rgba8(1, 1 << 31, &[]),
            Err(EncodeError::InvalidDimensions {
                width: 1,
                height: 1 << 31
            })
        );
        assert_eq!(
            ImageRef::new(1, 1, ColorType::Indexed, BitDepth::Eight, &[0]),
            Err(EncodeError::UnsupportedFormat {
                color: ColorType::Indexed,
                bit_depth: BitDepth::Eight
            })
        );
        assert_eq!(
            ImageRef::new(1, 1, ColorType::Rgb, BitDepth::Four, &[0, 0]),
            Err(EncodeError::UnsupportedFormat {
                color: ColorType::Rgb,
                bit_depth: BitDepth::Four
            })
        );
        assert_eq!(
            ImageRef::rgb8(2, 2, &[0; 11]),
            Err(EncodeError::DataLengthMismatch {
                expected: 12,
                actual: 11
            })
        );
    }

    #[test]
    fn packed_grey_rows_are_padded_to_bytes() {
        let image = ImageRef::new(9, 2, ColorType::Gray, BitDepth::One, &[0; 4]).unwrap();
        assert_eq!(image.row_bytes(), 2);
        assert_eq!((image.width(), image.height()), (9, 2));
        assert_eq!(
            (image.color_type(), image.bit_depth()),
            (ColorType::Gray, BitDepth::One)
        );
        assert_eq!(image.data().len(), 4);
    }
}
