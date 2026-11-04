//! Decompressing a strip or tile as libtiff does for Pillow: `PackBits`
//! (`PackBitsDecode`), LZW (`lzw`) and Deflate (`ZIPDecode`), each
//! stopping once the strip's bytes are out and failing when they cannot
//! all be produced, then undoing horizontal differencing
//! (`horAcc8`, `horAcc16`).

use super::error::DecodeError;
use super::ifd::ByteOrder;
use super::lzw;
use crate::zlib::{self, ZlibError};

/// A compression scheme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Compression {
    /// No compression: Pillow reads the bytes itself.
    None,
    /// LZW (5).
    Lzw,
    /// Deflate, zlib-wrapped (8, or the older 32946).
    Deflate,
    /// `PackBits` run-length coding (32773).
    PackBits,
}

impl Compression {
    /// The scheme of a `Compression` tag value.
    pub(super) fn from_tag(value: u64) -> Result<Self, DecodeError> {
        match value {
            1 => Ok(Self::None),
            5 => Ok(Self::Lzw),
            8 | 32_946 => Ok(Self::Deflate),
            32_773 => Ok(Self::PackBits),
            other => Err(DecodeError::UnsupportedCompression(
                u16::try_from(other).unwrap_or(u16::MAX),
            )),
        }
    }

    /// The most bytes one byte of compressed data can stand for, which
    /// bounds the image a file of a given size can hold: a `PackBits` run
    /// of 128 bytes takes two, Deflate expands at most 1032 times and an
    /// LZW code of at least 9 bits stands for at most 5119 bytes.
    pub(super) fn max_expansion(self) -> u64 {
        match self {
            Self::None => 1,
            Self::PackBits => 64,
            Self::Deflate => 1032,
            Self::Lzw => 5119,
        }
    }

    /// Whether horizontal differencing applies: libtiff runs its
    /// predictor for LZW and Deflate only.
    pub(super) fn uses_predictor(self) -> bool {
        matches!(self, Self::Lzw | Self::Deflate)
    }

    /// The first `expected` bytes `data` decompresses to.
    pub(super) fn decompress(self, data: &[u8], expected: usize) -> Result<Vec<u8>, DecodeError> {
        let decoded = match self {
            Self::None => data.get(..expected).map(<[u8]>::to_vec),
            Self::PackBits => unpack_bits(data, expected),
            Self::Lzw => lzw::decompress(data, expected),
            Self::Deflate => inflate(data, expected),
        };
        decoded.ok_or(DecodeError::CorruptData)
    }
}

/// `PackBits`: a count `n` below 128 copies `n + 1` bytes, above it repeats
/// the next byte `257 − n` times, and 128 does nothing. Runs past the end
/// are cut short.
fn unpack_bits(data: &[u8], expected: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(expected);
    let mut rest = data;
    while out.len() < expected {
        let (&count, tail) = rest.split_first()?;
        let room = expected - out.len();
        rest = match count {
            0..=127 => {
                let len = usize::from(count) + 1;
                let literal = tail.get(..len.min(room))?;
                out.extend_from_slice(literal);
                tail.get(len.min(room)..)?
            }
            128 => tail,
            _ => {
                let (&byte, tail) = tail.split_first()?;
                let len = 257 - usize::from(count);
                out.resize(out.len() + len.min(room), byte);
                tail
            }
        };
    }
    Some(out)
}

/// Deflate: the zlib stream's first `expected` bytes. Like zlib under
/// libtiff, decompression stops once they are out (later data, or its
/// absence, does not matter), but a stream that ends early, or whose
/// checksum is reached and wrong, is corrupt.
fn inflate(data: &[u8], expected: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(expected);
    let result = zlib::decompress_into(data, &mut out, expected);
    let complete = out.len() >= expected;
    match result {
        Err(ZlibError::ChecksumMismatch { .. }) => None,
        _ if complete => {
            out.truncate(expected);
            Some(out)
        }
        _ => None,
    }
}

/// Undoes horizontal differencing in place: within each row of
/// `row_bytes`, every sample is the sum of itself and the same sample of
/// the pixel before. Samples are `sample_bytes` long (1 or 2) in the
/// file's byte order, `samples` to a pixel.
pub(super) fn undo_differencing(
    data: &mut [u8],
    row_bytes: usize,
    samples: usize,
    sample_bytes: usize,
    order: ByteOrder,
) {
    let read = |pair: [u8; 2]| match order {
        ByteOrder::Little => u16::from_le_bytes(pair),
        ByteOrder::Big => u16::from_be_bytes(pair),
    };
    let write = |value: u16| match order {
        ByteOrder::Little => value.to_le_bytes(),
        ByteOrder::Big => value.to_be_bytes(),
    };
    for row in data.chunks_mut(row_bytes) {
        if sample_bytes == 1 {
            for i in samples..row.len() {
                row[i] = row[i].wrapping_add(row[i - samples]);
            }
        } else {
            let values = row.as_chunks_mut::<2>().0;
            for i in samples..values.len() {
                values[i] = write(read(values[i]).wrapping_add(read(values[i - samples])));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CompressionLevel;

    #[test]
    fn packbits_runs_and_literals() {
        let data = [2, b'a', b'b', b'c', 0xFE, b'z', 128, 0, b'q'];
        assert_eq!(unpack_bits(&data, 7).as_deref(), Some(&b"abczzzq"[..]));
        assert_eq!(
            unpack_bits(&data, 4).as_deref(),
            Some(&b"abcz"[..]),
            "cut short"
        );
        assert_eq!(unpack_bits(&data, 8), None, "too short");
        assert_eq!(unpack_bits(&[5, 1, 2], 6), None, "a literal past the end");
    }

    #[test]
    fn deflate_stops_at_the_expected_size() {
        let packed = zlib::compress(&[7; 100], CompressionLevel::default()).unwrap();
        assert_eq!(inflate(&packed, 100), Some(vec![7; 100]));
        assert_eq!(inflate(&packed, 40), Some(vec![7; 40]));
        assert_eq!(inflate(&packed, 101), None);
        let mut bad_checksum = packed.clone();
        *bad_checksum.last_mut().unwrap() ^= 1;
        assert_eq!(inflate(&bad_checksum, 100), None);
        assert_eq!(inflate(&bad_checksum, 99), Some(vec![7; 99]));
        let no_checksum = &packed[..packed.len() - 4];
        assert_eq!(inflate(no_checksum, 100), Some(vec![7; 100]));
    }

    #[test]
    fn differencing_is_undone_per_row_and_sample() {
        let mut bytes = [10, 20, 1, 2, 1, 2, 5, 5, 1, 1, 1, 1];
        undo_differencing(&mut bytes, 6, 2, 1, ByteOrder::Little);
        assert_eq!(bytes, [10, 20, 11, 22, 12, 24, 5, 5, 6, 6, 7, 7]);
        let mut wide = [0x01, 0x00, 0xFF, 0xFF, 0x00, 0x02];
        undo_differencing(&mut wide, 6, 1, 2, ByteOrder::Big);
        assert_eq!(wide, [0x01, 0x00, 0x00, 0xFF, 0x01, 0x01]);
    }

    #[test]
    fn unknown_schemes_are_unsupported() {
        assert_eq!(
            Compression::from_tag(7),
            Err(DecodeError::UnsupportedCompression(7))
        );
        assert_eq!(Compression::from_tag(32_946), Ok(Compression::Deflate));
    }
}
