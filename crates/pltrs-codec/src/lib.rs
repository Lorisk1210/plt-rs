//! Self-contained image and compression codecs for pltrs.
//!
//! The crate has no dependencies and no `unsafe` code. It provides:
//!
//! * [`av1`] — an intra-only AV1 encoder for still pictures: 4:2:0,
//!   4:4:4 and monochrome, lossy or lossless.
//! * [`avif`] — AVIF files of RGBA pixels, coded with [`av1`] and laid out
//!   as Pillow writes them through libavif.
//! * [`base64`] — base64 encoding for embedding binary data in text, and
//!   decoding.
//! * [`checksum`] — CRC-32 (PNG, gzip) and Adler-32 (zlib) checksums, both as
//!   incremental hashers and as one-shot helpers.
//! * [`gif`] — animated GIF encoding with a shared, quantised palette, and
//!   decoding of a file's first frame as Pillow opens it.
//! * [`jpeg`] — baseline JPEG encoding matching Pillow's output byte for
//!   byte, and baseline, extended and progressive decoding whose pixels are
//!   Pillow's (libjpeg-turbo's arithmetic, ported).
//! * [`deflate`] — raw DEFLATE (RFC 1951): a compressor writing zlib-ng
//!   2.3's bytes at levels 0–9 and every strategy, and a strictly
//!   validating, table-driven decompressor.
//! * [`tiff`] — uncompressed RGBA TIFF files, laid out as Pillow writes them,
//!   and decoding of the TIFF layouts and compressions Pillow reads.
//! * [`raster`] — decoded JPEG, TIFF, GIF and WebP images in the Pillow mode
//!   the same file opens in, convertible to RGBA as Pillow converts them.
//! * [`sha1`] — the SHA-1 digest, as the WebSocket handshake needs it.
//! * [`sha256`] — the SHA-256 digest (SVG ids, the `text.usetex` cache).
//! * [`webp`] — decoding of WebP files as Pillow opens them, lossless
//!   (VP8L) and lossy (VP8, with alpha) bitstreams, still or animated, and
//!   lossless encoding of still images and animations.
//! * [`zlib`] — the RFC 1950 container (header and Adler-32 trailer) around
//!   DEFLATE, in both directions.
//! * [`png`] — a PNG encoder (grey, grey + alpha, RGB and RGBA at 8 or 16 bits,
//!   rows filtered and compressed as Pillow does with zlib-ng, `pHYs`
//!   resolution and text metadata) and a
//!   decoder for every standard PNG flavour: all colour types and bit depths,
//!   palettes, `tRNS` transparency and Adam7 interlacing.
//!
//! Every decoder in this crate treats its input as untrusted: malformed data
//! produces a descriptive error value, never a panic, and declared image
//! sizes are checked against the input before anything is allocated.
//!
//! # Example
//!
//! ```
//! use pltrs_codec::png::{self, ImageRef, PhysicalDimensions};
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

pub mod av1;
pub mod avif;
pub mod base64;
pub mod checksum;
pub mod deflate;
pub mod gif;
pub mod jpeg;
pub mod png;
pub mod raster;
pub mod sha1;
pub mod sha256;
pub mod tiff;
pub mod webp;
pub mod zlib;

pub use deflate::CompressionLevel;

#[cfg(test)]
mod test_util;
