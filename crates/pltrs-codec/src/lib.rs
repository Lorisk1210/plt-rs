//! Image and compression codecs for pltrs.
//!
//! The crate has no dependencies and no `unsafe` code. It has these
//! modules:
//!
//! * [`av1`]: an intra-only AV1 encoder for still pictures. It encodes
//!   4:2:0, 4:4:4 and monochrome, lossy or lossless.
//! * [`avif`]: AVIF files of RGBA pixels, coded with [`av1`], with the
//!   layout that Pillow writes through libavif.
//! * [`base64`]: base64 encoding and decoding, to put binary data in text.
//! * [`checksum`]: CRC-32 (PNG, gzip) and Adler-32 (zlib) checksums, as
//!   incremental hashers and as one-shot functions.
//! * [`gif`]: animated GIF encoding with a shared, quantized palette. It
//!   decodes the first frame of a file as Pillow opens it.
//! * [`jpeg`]: baseline JPEG encoding that gives the Pillow output byte for
//!   byte. Baseline, extended and progressive decoding gives the Pillow
//!   pixels, with the arithmetic of libjpeg-turbo.
//! * [`deflate`]: raw DEFLATE (RFC 1951). The compressor writes the bytes
//!   of zlib-ng 2.3 at levels 0 to 9 and all strategies. The decompressor
//!   is table-driven and validates strictly.
//! * [`tiff`]: uncompressed RGBA TIFF files with the layout that Pillow
//!   writes, and decoding of the TIFF layouts and compressions that Pillow
//!   reads.
//! * [`raster`]: decoded JPEG, TIFF, GIF and WebP images in the Pillow mode
//!   of the file. They convert to RGBA as Pillow converts them.
//! * [`sha1`]: the SHA-1 digest, for the WebSocket handshake.
//! * [`sha256`]: the SHA-256 digest (SVG ids, the `text.usetex` cache).
//! * [`webp`]: decoding of WebP files as Pillow opens them, with lossless
//!   (VP8L) and lossy (VP8, with alpha) bitstreams, still or animated. It
//!   also encodes still images and animations without loss.
//! * [`zlib`]: the RFC 1950 container (header and Adler-32 trailer) around
//!   DEFLATE, in both directions.
//! * [`png`]: a PNG encoder and decoder. The encoder writes gray, gray with
//!   alpha, RGB and RGBA at 8 or 16 bits. It filters and compresses the
//!   rows as Pillow does with zlib-ng, and writes the `pHYs` resolution and
//!   text metadata. The decoder reads all standard PNG files: all color
//!   types and bit depths, palettes, `tRNS` transparency and Adam7
//!   interlacing.
//!
//! Each decoder treats its input as untrusted. Bad data gives an error
//! value with a description, never a panic. The decoders check declared
//! image sizes against the input before they allocate memory.
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
