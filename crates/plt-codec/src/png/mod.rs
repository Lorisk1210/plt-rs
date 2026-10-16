//! PNG encoding and decoding.
//!
//! # Encoding
//!
//! Wrap pixels in an [`ImageRef`] (which validates the buffer size) and pass
//! it to [`encode`] or to a configured [`Encoder`]. The encoder writes grey
//! (1–16 bits), grey + alpha, RGB and RGBA (8 or 16 bits), choosing a scanline
//! filter per row adaptively, and can add a `pHYs` resolution and text
//! entries. Output is never interlaced.
//!
//! # Decoding
//!
//! [`decode`] (or [`Decoder`], to change the size limit) reads every
//! standard PNG: all colour types and bit depths, palettes, `tRNS`
//! transparency, Adam7 interlacing, any number of `IDAT` chunks. CRCs are
//! verified, unknown ancillary chunks skipped. The result is a
//! [`DecodedImage`], which keeps the file's native pixel layout and converts
//! to 8- or 16-bit RGBA on demand.
//!
//! ```
//! use plt_codec::png::{self, ColorType, ImageRef};
//!
//! let rgba = [10, 20, 30, 255, 40, 50, 60, 128];
//! let file = png::Encoder::new()
//!     .text("Software", "plt-rs")
//!     .encode(ImageRef::rgba8(2, 1, &rgba)?)?;
//!
//! let image = png::decode(&file)?;
//! assert_eq!(image.color_type(), ColorType::Rgba);
//! assert_eq!(image.to_rgba8(), rgba);
//! assert_eq!(image.metadata().text[0].text, "plt-rs");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod chunk;
mod convert;
mod decoded;
mod decoder;
mod encoder;
mod error;
mod filter;
mod header;
mod image;
mod interlace;
mod metadata;
mod parse;
mod text;
mod types;

pub use chunk::{ChunkType, SIGNATURE};
pub use decoded::{ColorKey, DecodedImage};
pub use decoder::Decoder;
pub use encoder::Encoder;
pub use error::{DecodeError, EncodeError};
pub use filter::{FilterStrategy, FilterType};
pub use image::ImageRef;
pub use metadata::{Metadata, PhysicalDimensions, PhysicalUnit, TextEntry};
pub use types::{BitDepth, ColorType};

/// Encodes `image` with the default [`Encoder`] settings.
pub fn encode(image: ImageRef<'_>) -> Result<Vec<u8>, EncodeError> {
    Encoder::new().encode(image)
}

/// Decodes a PNG file with the default [`Decoder`] limits.
pub fn decode(png: &[u8]) -> Result<DecodedImage, DecodeError> {
    Decoder::new().decode(png)
}

#[cfg(test)]
mod tests;
