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

use pltrs_core::alloc::{self, OutOfMemory};

use crate::checksum::adler32;
use crate::deflate::input::{ByteSource, Window};
use crate::deflate::{self, CompressionLevel, InflateError};

/// Compression method 8 (DEFLATE) with a 32 KiB window (`CINFO` = 7).
const CMF_DEFLATE_32K: u8 = 0x78;
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

/// Compresses `data` into a new zlib stream.
///
/// # Errors
///
/// When the memory for the output cannot be allocated.
pub fn compress(data: &[u8], level: CompressionLevel) -> Result<Vec<u8>, OutOfMemory> {
    let mut out = Vec::new();
    compress_into(data, level, &mut out)?;
    Ok(out)
}

/// Appends the zlib compression of `data` to `out`.
///
/// # Errors
///
/// When the memory for the output cannot be allocated.
pub fn compress_into(
    data: &[u8],
    level: CompressionLevel,
    out: &mut Vec<u8>,
) -> Result<(), OutOfMemory> {
    alloc::try_reserve(out, 2)?;
    out.extend_from_slice(&header(level));
    deflate::compress_into(data, level, out)?;
    alloc::try_reserve(out, 4)?;
    out.extend_from_slice(&adler32(data).to_be_bytes());
    Ok(())
}

/// The zlib stream [`compress`] makes of the bytes `source` produces,
/// which are read a window at a time rather than held all at once.
///
/// # Errors
///
/// When the memory for the window or the output cannot be allocated.
pub(crate) fn compress_source(
    source: impl ByteSource,
    level: CompressionLevel,
) -> Result<Vec<u8>, OutOfMemory> {
    let mut window = Window::new(source)?;
    let mut out = alloc::try_with_capacity(2)?;
    out.extend_from_slice(&header(level));
    deflate::compress_input(&mut window, level, &mut out)?;
    alloc::try_reserve(&mut out, 4)?;
    out.extend_from_slice(&window.checksum().to_be_bytes());
    Ok(out)
}

/// The two header bytes zlib writes for `level`: the `FLEVEL` field records
/// the speed/size trade-off, the low bits make the header a multiple of 31.
fn header(level: CompressionLevel) -> [u8; 2] {
    let level_flags: u8 = match level.get() {
        0 | 1 => 0,
        2..=5 => 1,
        6 => 2,
        _ => 3,
    };
    let flags = level_flags << 6;
    let remainder = (u16::from(CMF_DEFLATE_32K) << 8 | u16::from(flags)) % 31;
    let check = ((31 - remainder) % 31) as u8;
    [CMF_DEFLATE_32K, flags | check]
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
            assert_eq!(
                header(CompressionLevel::new(level).unwrap()),
                bytes,
                "level {level}"
            );
        }
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

    /// The bytes of a slice, as a source that makes them on demand.
    struct Slice<'a>(&'a [u8]);

    impl ByteSource for Slice<'_> {
        fn len(&self) -> usize {
            self.0.len()
        }

        fn append(&mut self, start: usize, count: usize, out: &mut Vec<u8>) {
            out.extend_from_slice(&self.0[start..(start + count).min(self.0.len())]);
        }
    }

    #[test]
    fn sources_compress_to_the_stream_of_their_whole_bytes() {
        let mut rng = Rng::new(12);
        let mut inputs = varied_inputs(&mut rng);
        // Longer than a window's refill, compressible and not.
        inputs.push(rng.bytes(400_000));
        inputs.push((0..600_000).map(|i| (i / 7 % 13) as u8).collect());
        for input in &inputs {
            for level in [0, 1, 3, 6, 9] {
                let level = CompressionLevel::new(level).unwrap();
                assert_eq!(
                    compress_source(Slice(input), level).unwrap(),
                    compress(input, level).unwrap(),
                    "{} bytes at level {}",
                    input.len(),
                    level.get()
                );
            }
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
