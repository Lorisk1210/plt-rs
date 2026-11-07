//! Raw DEFLATE (RFC 1951) compression and decompression.
//!
//! # Compression
//!
//! [`compress`] writes the bytes zlib-ng 2.3 writes: its parsers (one
//! fixed-code block at level 1, greedy parsing at level 2, its "medium"
//! parsing with a look-ahead match at levels 3 to 6, lazy parsing at 7 to
//! 9), its match finder and hashes, and zlib's block splitting and
//! Huffman codes, ported step for step. [`compress_with`] also takes
//! zlib's strategy and memory level ([`Options`]). Level 0 writes stored
//! blocks only.
//!
//! # Decompression
//!
//! [`decompress`] and friends decode with lookup tables and validate the
//! stream as strictly as zlib: malformed input yields an [`InflateError`],
//! never a panic. A size limit guards against decompression bombs.
//!
//! ```
//! use pltrs_codec::{deflate, CompressionLevel};
//!
//! let text = b"to be or not to be, that is the question; to be or not to be";
//! let packed = deflate::compress(text, CompressionLevel::DEFAULT)?;
//! assert!(packed.len() < text.len());
//! assert_eq!(deflate::decompress(&packed)?, text);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod bit_reader;
pub(crate) mod bit_writer;
pub(crate) mod code_lengths;
mod decode_table;
pub(crate) mod encode;
mod error;
pub(crate) mod huffman;
mod inflate;
mod level;
mod tables;

pub use encode::{InvalidMemoryLevel, MemoryLevel, Options, Strategy};
pub use error::{HuffmanTable, InflateError};
pub use level::{CompressionLevel, InvalidCompressionLevel};

use pltrs_core::alloc::OutOfMemory;

use encode::Wrap;

/// Compresses `data` into a new raw DEFLATE stream at `level`.
///
/// # Errors
///
/// When the memory for the output cannot be allocated.
pub fn compress(data: &[u8], level: CompressionLevel) -> Result<Vec<u8>, OutOfMemory> {
    compress_with(data, Options::new(level))
}

/// Compresses `data` into a new raw DEFLATE stream as zlib-ng does with
/// `options`.
///
/// # Errors
///
/// When the memory for the compressor or the output cannot be allocated.
pub fn compress_with(data: &[u8], options: Options) -> Result<Vec<u8>, OutOfMemory> {
    let mut out = Vec::new();
    encode::one_shot(data, options, Wrap::Raw, &mut None, &mut out)?;
    Ok(out)
}

/// Appends the raw DEFLATE compression of `data` at `level` to `out`.
///
/// # Errors
///
/// When the memory for the compressor or the output cannot be allocated
/// (what was written of it stays in `out`).
pub fn compress_into(
    data: &[u8],
    level: CompressionLevel,
    out: &mut Vec<u8>,
) -> Result<(), OutOfMemory> {
    encode::one_shot(data, Options::new(level), Wrap::Raw, &mut None, out)
}

/// Decompresses the raw DEFLATE stream at the start of `input`.
///
/// Bytes after the end of the final block are ignored; use
/// [`decompress_into`] to learn how many input bytes the stream occupied.
pub fn decompress(input: &[u8]) -> Result<Vec<u8>, InflateError> {
    decompress_with_limit(input, usize::MAX)
}

/// Like [`decompress`], but fails with
/// [`InflateError::OutputLimitExceeded`] instead of producing more than
/// `limit` bytes.
pub fn decompress_with_limit(input: &[u8], limit: usize) -> Result<Vec<u8>, InflateError> {
    let mut out = Vec::with_capacity(limit.min(input.len().saturating_mul(4)));
    decompress_into(input, &mut out, limit)?;
    Ok(out)
}

/// Decompresses the raw DEFLATE stream at the start of `input`, appending at
/// most `limit` bytes to `out`, and returns the number of input bytes the
/// stream occupied.
///
/// Back-references cannot reach into data that was already in `out`. On
/// error, `out` may hold part of the decompressed data; in particular, on
/// [`InflateError::OutputLimitExceeded`] exactly the first `limit` bytes
/// have been appended.
pub fn decompress_into(
    input: &[u8],
    out: &mut Vec<u8>,
    limit: usize,
) -> Result<usize, InflateError> {
    inflate::inflate(input, out, limit)
}

#[cfg(test)]
mod tests;
