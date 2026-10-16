//! Self-contained DEFLATE, zlib and PNG codecs for plt-rs.
//!
//! The crate has no dependencies and no `unsafe` code. It provides:
//!
//! * [`checksum`] — CRC-32 (PNG, gzip) and Adler-32 (zlib) checksums, both as
//!   incremental hashers and as one-shot helpers.
//! * [`deflate`] — raw DEFLATE (RFC 1951): an LZ77 + Huffman compressor with
//!   zlib-style levels 0–9, and a strictly validating, table-driven
//!   decompressor.
//! * [`zlib`] — the RFC 1950 container (header and Adler-32 trailer) around
//!   DEFLATE, in both directions.
//! * [`png`] — a PNG encoder (grey, grey + alpha, RGB and RGBA at 8 or 16 bits,
//!   adaptive row filtering, `pHYs` resolution and text metadata) and a
//!   decoder for every standard PNG flavour: all colour types and bit depths,
//!   palettes, `tRNS` transparency and Adam7 interlacing.
//!
//! Every decoder in this crate treats its input as untrusted: malformed data
//! produces a descriptive error value, never a panic.
//!
//! # Example
//!
//! ```
//! use plt_codec::png::{self, ImageRef, PhysicalDimensions};
//!
//! // A 2×1 RGBA image: an opaque red pixel and a half-transparent blue one.
//! let pixels = [255, 0, 0, 255, 0, 0, 255, 128];
//! let image = ImageRef::rgba8(2, 1, &pixels)?;
//! let bytes = png::Encoder::new().dpi(100.0).encode(image)?;
//!
//! let decoded = png::decode(&bytes)?;
//! assert_eq!(decoded.to_rgba8(), pixels);
//! assert_eq!(decoded.metadata().physical, PhysicalDimensions::from_dpi(100.0));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
#![forbid(unsafe_code)]

pub mod checksum;
pub mod deflate;
pub mod png;
pub mod zlib;

pub use deflate::CompressionLevel;

#[cfg(test)]
mod test_util;
