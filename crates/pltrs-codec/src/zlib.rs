//! The zlib format (RFC 1950): a two-byte header, a DEFLATE stream and an
//! Adler-32 trailer.
//!
//! ```
//! use pltrs_codec::{zlib, CompressionLevel};
//!
//! let packed = zlib::compress(b"hello, hello, hello", CompressionLevel::DEFAULT)?;
//! assert_eq!(&packed[..2], [0x78, 0x9C]);
//! assert_eq!(zlib::decompress(&packed)?, b"hello, hello, hello");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::error::Error;
use std::fmt;

use pltrs_core::alloc::OutOfMemory;

use crate::checksum::adler32;
use crate::deflate::encode::{self, Deflater, Wrap};
use crate::deflate::{self, CompressionLevel, InflateError, Options};

/// The `FDICT` flag: a preset dictionary identifier follows the header.
const FLAG_DICTIONARY: u8 = 0x20;

/// An error found while decoding a zlib stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZlibError {
    /// The input ended inside the header or the checksum trailer.
    UnexpectedEof,
    /// The header names a compression method other than DEFLATE (8).
    UnsupportedMethod(u8),
    /// The header declares a window larger than 32 KiB.
    InvalidWindowSize(u8),
    /// The header's check bits are wrong (`CMF * 256 + FLG` is not a
    /// multiple of 31).
    InvalidHeaderCheck,
    /// The stream needs a preset dictionary, which is not supported.
    PresetDictionary,
    /// The compressed data is malformed.
    Inflate(InflateError),
    /// The Adler-32 of the decompressed data does not match the trailer.
    ChecksumMismatch {
        /// Checksum stored in the stream.
        expected: u32,
        /// Checksum of the decompressed data.
        actual: u32,
    },
    /// Bytes follow the end of the stream.
    TrailingData {
        /// Number of bytes after the stream.
        len: usize,
    },
}

impl fmt::Display for ZlibError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::UnexpectedEof => f.write_str("zlib stream ended unexpectedly"),
            Self::UnsupportedMethod(method) => {
                write!(f, "unsupported zlib compression method {method}")
            }
            Self::InvalidWindowSize(info) => {
                write!(f, "invalid zlib window size 2^{}", u32::from(info) + 8)
            }
            Self::InvalidHeaderCheck => f.write_str("invalid zlib header check bits"),
            Self::PresetDictionary => f.write_str("zlib preset dictionaries are not supported"),
            Self::Inflate(error) => write!(f, "invalid zlib data: {error}"),
            Self::ChecksumMismatch { expected, actual } => write!(
                f,
                "zlib checksum mismatch: stream says {expected:#010x}, data gives {actual:#010x}"
            ),
            Self::TrailingData { len } => write!(f, "{len} bytes of data after the zlib stream"),
        }
    }
}

impl Error for ZlibError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Inflate(error) => Some(error),
            _ => None,
        }
    }
}

impl From<InflateError> for ZlibError {
    fn from(error: InflateError) -> Self {
        Self::Inflate(error)
    }
}

/// Compresses `data` into a new zlib stream at `level`, as zlib-ng's
/// `compress2` does.
///
/// # Errors
///
/// When the memory for the compressor or the output cannot be allocated.
pub fn compress(data: &[u8], level: CompressionLevel) -> Result<Vec<u8>, OutOfMemory> {
    compress_with(data, Options::new(level))
}

/// Compresses `data` into a new zlib stream as zlib-ng does with
/// `options`.
///
/// # Errors
///
/// When the memory for the compressor or the output cannot be allocated.
pub fn compress_with(data: &[u8], options: Options) -> Result<Vec<u8>, OutOfMemory> {
    let mut out = Vec::new();
    encode::one_shot(data, options, Wrap::Zlib, &mut None, &mut out)?;
    Ok(out)
}

/// Appends the zlib compression of `data` at `level` to `out`.
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
    encode::one_shot(data, Options::new(level), Wrap::Zlib, &mut None, out)
}

/// Compresses stream after stream with the same tables, for writers of
/// many streams (PDF's): each stream is the one [`compress_with`] writes,
/// without allocating and clearing the compressor's 300 KiB of tables
/// anew for each.
///
/// ```
/// use pltrs_codec::{zlib, CompressionLevel};
/// use pltrs_codec::deflate::Options;
///
/// let mut compressor = zlib::Compressor::new();
/// let options = Options::new(CompressionLevel::DEFAULT);
/// for text in [&b"first stream"[..], b"second, second"] {
///     let mut packed = Vec::new();
///     compressor.compress_into(text, options, &mut packed)?;
///     assert_eq!(packed, zlib::compress_with(text, options)?);
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Default)]
pub struct Compressor {
    tables: Option<Deflater>,
}

impl Compressor {
    /// A compressor that allocates its tables at its first stream.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends the zlib stream of `data` with `options` to `out`.
    ///
    /// # Errors
    ///
    /// When the memory for the compressor or the output cannot be
    /// allocated (what was written of the stream stays in `out`).
    pub fn compress_into(
        &mut self,
        data: &[u8],
        options: Options,
        out: &mut Vec<u8>,
    ) -> Result<(), OutOfMemory> {
        encode::one_shot(data, options, Wrap::Zlib, &mut self.tables, out)
    }
}

/// Decompresses a complete zlib stream; trailing bytes are an error.
pub fn decompress(input: &[u8]) -> Result<Vec<u8>, ZlibError> {
    decompress_with_limit(input, usize::MAX)
}

/// Like [`decompress`], but fails instead of producing more than `limit`
/// bytes.
pub fn decompress_with_limit(input: &[u8], limit: usize) -> Result<Vec<u8>, ZlibError> {
    let mut out = Vec::with_capacity(limit.min(input.len().saturating_mul(4)));
    let used = decompress_into(input, &mut out, limit)?;
    if used < input.len() {
        return Err(ZlibError::TrailingData {
            len: input.len() - used,
        });
    }
    Ok(out)
}

/// Decompresses the zlib stream at the start of `input`, appending at most
/// `limit` bytes to `out`, and returns the number of input bytes the stream
/// occupied (bytes after it are left alone).
///
/// On error, `out` may hold part of the decompressed data.
pub fn decompress_into(input: &[u8], out: &mut Vec<u8>, limit: usize) -> Result<usize, ZlibError> {
    let &[cmf, flags] = input.first_chunk::<2>().ok_or(ZlibError::UnexpectedEof)?;
    if cmf & 0x0F != 8 {
        return Err(ZlibError::UnsupportedMethod(cmf & 0x0F));
    }
    if cmf >> 4 > 7 {
        return Err(ZlibError::InvalidWindowSize(cmf >> 4));
    }
    if (u16::from(cmf) << 8 | u16::from(flags)) % 31 != 0 {
        return Err(ZlibError::InvalidHeaderCheck);
    }
    if flags & FLAG_DICTIONARY != 0 {
        return Err(ZlibError::PresetDictionary);
    }
    let start = out.len();
    let stream_end = 2 + deflate::decompress_into(&input[2..], out, limit)?;
    let trailer = input
        .get(stream_end..)
        .and_then(<[u8]>::first_chunk::<4>)
        .ok_or(ZlibError::UnexpectedEof)?;
    let expected = u32::from_be_bytes(*trailer);
    let actual = adler32(&out[start..]);
    if expected != actual {
        return Err(ZlibError::ChecksumMismatch { expected, actual });
    }
    Ok(stream_end + 4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deflate::encode::RowSource;
    use crate::deflate::{MemoryLevel, Strategy};
    use crate::test_util::{Rng, varied_inputs};

    #[test]
    fn headers_match_zlib_for_every_level() {
        let expected = [
            [0x78, 0x01],
            [0x78, 0x01],
            [0x78, 0x5E],
            [0x78, 0x5E],
            [0x78, 0x5E],
            [0x78, 0x5E],
            [0x78, 0x9C],
            [0x78, 0xDA],
            [0x78, 0xDA],
            [0x78, 0xDA],
        ];
        for (level, bytes) in (0..=9).zip(expected) {
            let packed = compress(b"", CompressionLevel::new(level).unwrap()).unwrap();
            assert_eq!(packed[..2], bytes, "level {level}");
        }
        for strategy in [Strategy::HuffmanOnly, Strategy::Rle, Strategy::Fixed] {
            let options = Options::new(CompressionLevel::BEST).strategy(strategy);
            assert_eq!(compress_with(b"", options).unwrap()[..2], [0x78, 0x01]);
        }
        let filtered = Options::new(CompressionLevel::BEST).strategy(Strategy::Filtered);
        assert_eq!(compress_with(b"", filtered).unwrap()[..2], [0x78, 0xDA]);
    }

    #[test]
    fn matches_reference_stream() {
        let packed = compress(b"hello", CompressionLevel::DEFAULT).unwrap();
        let reference = [
            0x78, 0x9C, 0xCB, 0x48, 0xCD, 0xC9, 0xC9, 0x07, 0x00, 0x06, 0x2C, 0x02, 0x15,
        ];
        assert_eq!(packed, reference);
        assert_eq!(decompress(&reference).unwrap(), b"hello");
    }

    #[test]
    fn round_trips_at_every_level() {
        let mut rng = Rng::new(11);
        for input in varied_inputs(&mut rng).iter().step_by(3) {
            for level in 0..=9 {
                let packed = compress(input, CompressionLevel::new(level).unwrap()).unwrap();
                assert_eq!(decompress(&packed).unwrap(), *input, "level {level}");
            }
        }
    }

    /// Rows of `len` bytes cut from `data`.
    struct Rows<'a> {
        data: &'a [u8],
        len: usize,
    }

    impl encode::RowSource for Rows<'_> {
        fn rows(&self) -> usize {
            self.data.len() / self.len
        }

        fn row(&mut self, index: usize) -> &[u8] {
            &self.data[index * self.len..(index + 1) * self.len]
        }
    }

    #[test]
    fn rows_arrive_in_buffers_of_the_given_size() {
        let mut rng = Rng::new(12);
        let noise = rng.bytes(300_000);
        let pattern: Vec<u8> = (0..300_000).map(|i| (i / 7 % 13) as u8).collect();
        for data in [&noise, &pattern] {
            for level in [0, 1, 3, 6, 9] {
                let options = Options::new(CompressionLevel::new(level).unwrap());
                let mut rows = Rows { data, len: 1201 };
                let packed = encode::rows(&mut rows, options, 65_536).unwrap();
                let whole = rows.rows() * rows.len;
                assert_eq!(decompress(&packed.stream).unwrap(), data[..whole]);
                assert_eq!(packed.ends.last(), Some(&packed.stream.len()));
                let full = &packed.ends[..packed.ends.len() - 1];
                assert!(
                    full.iter()
                        .zip(1..)
                        .all(|(&end, index)| end == index * 65_536),
                    "every buffer full but the last, level {level}"
                );
            }
        }
    }

    #[test]
    fn a_reused_compressor_writes_what_a_new_one_writes() {
        let mut rng = Rng::new(13);
        let mut inputs = varied_inputs(&mut rng);
        // Long inputs first, so that short ones follow tables full of
        // their hashes and window bytes.
        inputs.sort_by_key(|input| std::cmp::Reverse(input.len()));
        let mut compressor = Compressor::new();
        for (index, input) in inputs.iter().enumerate() {
            let level = CompressionLevel::new((index % 10) as u8).unwrap();
            let memory = MemoryLevel::new([8, 9, 1][index % 3]).unwrap();
            let options = Options::new(level).memory_level(memory);
            let mut packed = vec![0xAA];
            compressor
                .compress_into(input, options, &mut packed)
                .unwrap();
            assert_eq!(
                packed[1..],
                compress_with(input, options).unwrap(),
                "input {index}"
            );
        }
    }

    #[test]
    fn rejects_bad_headers_and_trailers() {
        let good = compress(b"payload payload", CompressionLevel::DEFAULT).unwrap();
        assert_eq!(decompress(&[]), Err(ZlibError::UnexpectedEof));
        assert_eq!(decompress(&[0x78]), Err(ZlibError::UnexpectedEof));
        assert_eq!(
            decompress(&[0x79, 0x9C]),
            Err(ZlibError::UnsupportedMethod(9))
        );
        assert_eq!(
            decompress(&[0x88, 0x98]),
            Err(ZlibError::InvalidWindowSize(8))
        );
        assert_eq!(
            decompress(&[0x78, 0x9D]),
            Err(ZlibError::InvalidHeaderCheck)
        );
        assert_eq!(decompress(&[0x78, 0xBB]), Err(ZlibError::PresetDictionary));

        let mut corrupted = good.clone();
        let last = corrupted.len() - 1;
        corrupted[last] ^= 0x01;
        assert!(matches!(
            decompress(&corrupted),
            Err(ZlibError::ChecksumMismatch { .. })
        ));
        assert_eq!(
            decompress(&good[..good.len() - 2]),
            Err(ZlibError::UnexpectedEof)
        );

        let mut trailing = good.clone();
        trailing.extend_from_slice(&[1, 2, 3]);
        assert_eq!(
            decompress(&trailing),
            Err(ZlibError::TrailingData { len: 3 })
        );
        let mut out = Vec::new();
        assert_eq!(
            decompress_into(&trailing, &mut out, usize::MAX),
            Ok(good.len())
        );
        assert_eq!(out, b"payload payload");

        let broken_deflate = [0x78, 0x9C, 0x07, 0x00];
        assert_eq!(
            decompress(&broken_deflate),
            Err(ZlibError::Inflate(InflateError::InvalidBlockType))
        );
        assert!(
            ZlibError::Inflate(InflateError::InvalidBlockType)
                .source()
                .is_some()
        );
    }

    #[test]
    fn limit_applies_to_decompressed_size() {
        let packed = compress(&[7; 1000], CompressionLevel::BEST).unwrap();
        assert_eq!(decompress_with_limit(&packed, 1000).unwrap().len(), 1000);
        assert_eq!(
            decompress_with_limit(&packed, 999),
            Err(ZlibError::Inflate(InflateError::OutputLimitExceeded {
                limit: 999
            }))
        );
    }
}
