//! Raw DEFLATE (RFC 1951) compression and decompression.
//!
//! # Compression
//!
//! [`compress`] works like zlib: LZ77 over a 32 KiB window with hash chains
//! (greedy parsing for levels 1–3, lazy matching for 4–9), then every block
//! of up to 16 Ki tokens is written as whichever of stored, fixed-Huffman or
//! dynamic-Huffman is smallest. Dynamic blocks use optimal length-limited
//! Huffman codes. Level 0 writes stored blocks only.
//!
//! # Decompression
//!
//! [`decompress`] and friends decode with lookup tables and validate the
//! stream as strictly as zlib: malformed input yields an [`InflateError`],
//! never a panic. A size limit guards against decompression bombs.
//!
//! ```
//! use plt_codec::{deflate, CompressionLevel};
//!
//! let text = b"to be or not to be, that is the question; to be or not to be";
//! let packed = deflate::compress(text, CompressionLevel::DEFAULT);
//! assert!(packed.len() < text.len());
//! assert_eq!(deflate::decompress(&packed)?, text);
//! # Ok::<(), deflate::InflateError>(())
//! ```

mod bit_reader;
mod bit_writer;
mod block;
mod decode_table;
mod error;
mod huffman;
mod inflate;
mod level;
mod lz77;
mod matcher;
mod tables;

pub use error::{HuffmanTable, InflateError};
pub use level::{CompressionLevel, InvalidCompressionLevel};

use bit_writer::BitWriter;
use block::BlockSink;
use level::Parsing;

/// Compresses `data` into a new raw DEFLATE stream.
pub fn compress(data: &[u8], level: CompressionLevel) -> Vec<u8> {
    let mut out = Vec::new();
    compress_into(data, level, &mut out);
    out
}

/// Appends the raw DEFLATE compression of `data` to `out`.
pub fn compress_into(data: &[u8], level: CompressionLevel, out: &mut Vec<u8>) {
    let params = level.params();
    out.reserve(match params.parsing {
        Parsing::Stored => data.len() + 5 * (data.len() / 65_535 + 1),
        Parsing::Greedy | Parsing::Lazy => data.len() / 4 + 64,
    });
    let mut writer = BitWriter::new(out);
    match params.parsing {
        Parsing::Stored => block::write_stored(&mut writer, data, true),
        Parsing::Greedy => lz77::greedy(data, &params, BlockSink::new(data, &mut writer)),
        Parsing::Lazy => lz77::lazy(data, &params, BlockSink::new(data, &mut writer)),
    }
    writer.align_to_byte();
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
