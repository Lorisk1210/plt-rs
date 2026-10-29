//! The PNG decoder.

use super::decoded::DecodedImage;
use super::error::DecodeError;
use super::interlace::StreamLayout;
use super::parse;
use crate::deflate::InflateError;
use crate::zlib::{self, ZlibError};

/// DEFLATE cannot expand data by more than about 1032:1, so a decoder never
/// needs to reserve more than this multiple of the compressed size up front.
const MAX_DEFLATE_RATIO: usize = 1032;

/// Decodes PNG files, with a configurable size limit.
///
/// ```
/// use pltrs_codec::png::{self, ColorType, Decoder, ImageRef};
///
/// let gray = [0u8, 64, 128, 255];
/// let file = png::encode(ImageRef::new(2, 2, ColorType::Gray, png::BitDepth::Eight, &gray)?)?;
///
/// let image = Decoder::new().max_image_bytes(1 << 20).decode(&file)?;
/// assert_eq!((image.width(), image.height()), (2, 2));
/// assert_eq!(image.data(), gray);
/// assert_eq!(&image.to_rgba8()[4..8], [64, 64, 64, 255]);
///
/// let tiny_limit = Decoder::new().max_image_bytes(3);
/// assert!(tiny_limit.decode(&file).is_err());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decoder {
    max_image_bytes: usize,
}

impl Decoder {
    /// The default limit on decoded pixel data: 1 GiB.
    pub const DEFAULT_MAX_IMAGE_BYTES: usize = 1 << 30;

    /// A decoder with the default limit.
    pub const fn new() -> Self {
        Self {
            max_image_bytes: Self::DEFAULT_MAX_IMAGE_BYTES,
        }
    }

    /// Sets the largest pixel buffer (in PNG layout, see
    /// [`DecodedImage::data`]) the decoder will produce; larger images fail
    /// with [`DecodeError::ImageTooLarge`] before any pixel memory is
    /// allocated.
    #[must_use]
    pub const fn max_image_bytes(self, limit: usize) -> Self {
        Self {
            max_image_bytes: limit,
        }
    }

    /// Decodes a complete PNG file.
    pub fn decode(&self, png: &[u8]) -> Result<DecodedImage, DecodeError> {
        let parts = parse::read_parts(png)?;
        let header = parts.header;
        let too_large = || DecodeError::ImageTooLarge {
            limit: self.max_image_bytes,
        };
        let row_bytes = header
            .format
            .row_bytes(header.width)
            .ok_or_else(too_large)?;
        let image_bytes = header
            .format
            .image_bytes(header.width, header.height)
            .filter(|&bytes| bytes <= self.max_image_bytes)
            .ok_or_else(too_large)?;
        let layout = StreamLayout::new(&header).ok_or_else(too_large)?;
        let stream = inflate_image_data(&parts.image_data, layout.len())?;
        drop(parts.image_data);
        let data = layout.reconstruct(&stream, header.format, row_bytes, image_bytes)?;
        Ok(DecodedImage::new(
            header,
            data,
            parts.palette,
            parts.color_key,
            parts.metadata,
        ))
    }
}

impl Default for Decoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Decompresses the concatenated `IDAT` data, which must yield at least
/// `expected` bytes. Like libpng, surplus data after the image is ignored.
fn inflate_image_data(compressed: &[u8], expected: usize) -> Result<Vec<u8>, DecodeError> {
    let capacity = expected.min(compressed.len().saturating_mul(MAX_DEFLATE_RATIO));
    let mut stream = Vec::with_capacity(capacity);
    match zlib::decompress_into(compressed, &mut stream, expected) {
        Ok(_) | Err(ZlibError::Inflate(InflateError::OutputLimitExceeded { .. })) => {}
        Err(error) => return Err(error.into()),
    }
    if stream.len() < expected {
        return Err(DecodeError::ImageDataTooShort {
            expected,
            actual: stream.len(),
        });
    }
    Ok(stream)
}
