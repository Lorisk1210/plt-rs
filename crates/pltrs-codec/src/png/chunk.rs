//! PNG chunk framing: length, type, data and CRC.

use std::fmt;

use super::error::DecodeError;
use crate::checksum::Crc32;

/// The eight bytes every PNG file starts with.
pub const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1A, b'\n'];

/// Largest chunk length PNG allows (2^31 − 1).
pub(crate) const MAX_CHUNK_LEN: u32 = (1 << 31) - 1;

/// A four-letter chunk type such as `IHDR`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkType(pub [u8; 4]);

impl ChunkType {
    /// Image header.
    pub const IHDR: Self = Self(*b"IHDR");
    /// Palette.
    pub const PLTE: Self = Self(*b"PLTE");
    /// Image data.
    pub const IDAT: Self = Self(*b"IDAT");
    /// Image end.
    pub const IEND: Self = Self(*b"IEND");
    /// Simple transparency.
    pub const TRNS: Self = Self(*b"tRNS");
    /// Physical pixel dimensions.
    pub const PHYS: Self = Self(*b"pHYs");
    /// Latin-1 text.
    pub const TEXT: Self = Self(*b"tEXt");
    /// Compressed Latin-1 text.
    pub const ZTXT: Self = Self(*b"zTXt");
    /// International (UTF-8) text.
    pub const ITXT: Self = Self(*b"iTXt");

    /// Whether decoders must understand this chunk (upper-case first letter).
    pub const fn is_critical(self) -> bool {
        self.0[0].is_ascii_uppercase()
    }

    /// Whether every byte is an ASCII letter, as PNG requires.
    const fn is_valid(self) -> bool {
        let [a, b, c, d] = self.0;
        a.is_ascii_alphabetic()
            && b.is_ascii_alphabetic()
            && c.is_ascii_alphabetic()
            && d.is_ascii_alphabetic()
    }
}

impl fmt::Display for ChunkType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for &byte in &self.0 {
            write!(f, "{}", std::ascii::escape_default(byte))?;
        }
        Ok(())
    }
}

impl fmt::Debug for ChunkType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ChunkType({self})")
    }
}

/// Appends a complete chunk to `out`.
pub(crate) fn write_chunk(out: &mut Vec<u8>, kind: ChunkType, data: &[u8]) {
    let length = u32::try_from(data.len())
        .ok()
        .filter(|&len| len <= MAX_CHUNK_LEN)
        .expect("encoder chunks are split well below the PNG size limit");
    out.extend_from_slice(&length.to_be_bytes());
    out.extend_from_slice(&kind.0);
    out.extend_from_slice(data);
    let mut crc = Crc32::new();
    crc.update(&kind.0);
    crc.update(data);
    out.extend_from_slice(&crc.finish().to_be_bytes());
}

/// A chunk read from a PNG stream, with its CRC already verified.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Chunk<'a> {
    pub(crate) kind: ChunkType,
    pub(crate) data: &'a [u8],
}

/// Splits the bytes after the signature into chunks.
#[derive(Debug)]
pub(crate) struct ChunkReader<'a> {
    rest: &'a [u8],
}

impl<'a> ChunkReader<'a> {
    /// Checks the signature and positions the reader on the first chunk.
    pub(crate) fn new(png: &'a [u8]) -> Result<Self, DecodeError> {
        let rest = png
            .strip_prefix(&SIGNATURE)
            .ok_or(DecodeError::InvalidSignature)?;
        Ok(Self { rest })
    }

    /// Reads the next chunk; running out of data is an error because a valid
    /// stream ends with `IEND`, after which nothing more is read.
    pub(crate) fn next_chunk(&mut self) -> Result<Chunk<'a>, DecodeError> {
        let (&header, rest) = self
            .rest
            .split_first_chunk::<8>()
            .ok_or(DecodeError::UnexpectedEof)?;
        let [l0, l1, l2, l3, t0, t1, t2, t3] = header;
        let length = u32::from_be_bytes([l0, l1, l2, l3]);
        let kind = ChunkType([t0, t1, t2, t3]);
        if length > MAX_CHUNK_LEN {
            return Err(DecodeError::ChunkTooLong { length });
        }
        if !kind.is_valid() {
            return Err(DecodeError::InvalidChunkType(kind));
        }
        let length = length as usize;
        if rest.len() < length + 4 {
            return Err(DecodeError::UnexpectedEof);
        }
        let (data, rest) = rest.split_at(length);
        let (crc_bytes, rest) = rest.split_at(4);
        let stored = u32::from_be_bytes([crc_bytes[0], crc_bytes[1], crc_bytes[2], crc_bytes[3]]);
        let mut crc = Crc32::new();
        crc.update(&kind.0);
        crc.update(data);
        let computed = crc.finish();
        if stored != computed {
            return Err(DecodeError::CrcMismatch {
                chunk: kind,
                stored,
                computed,
            });
        }
        self.rest = rest;
        Ok(Chunk { kind, data })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_iend_like_every_png() {
        let mut out = Vec::new();
        write_chunk(&mut out, ChunkType::IEND, &[]);
        assert_eq!(
            out,
            [0, 0, 0, 0, b'I', b'E', b'N', b'D', 0xAE, 0x42, 0x60, 0x82]
        );
    }

    #[test]
    fn reads_back_written_chunks() {
        let mut png = SIGNATURE.to_vec();
        write_chunk(&mut png, ChunkType::TEXT, b"Title\0plot");
        write_chunk(&mut png, ChunkType::IEND, &[]);
        let mut reader = ChunkReader::new(&png).unwrap();
        let text = reader.next_chunk().unwrap();
        assert_eq!(
            (text.kind, text.data),
            (ChunkType::TEXT, &b"Title\0plot"[..])
        );
        assert!(!text.kind.is_critical());
        assert_eq!(reader.next_chunk().unwrap().kind, ChunkType::IEND);
        assert_eq!(reader.next_chunk().unwrap_err(), DecodeError::UnexpectedEof);
    }

    #[test]
    fn rejects_damaged_framing() {
        assert_eq!(
            ChunkReader::new(b"\x89PNG").unwrap_err(),
            DecodeError::InvalidSignature
        );
        let mut png = SIGNATURE.to_vec();
        write_chunk(&mut png, ChunkType::IEND, &[]);

        let mut bad_crc = png.clone();
        *bad_crc.last_mut().unwrap() ^= 1;
        assert!(matches!(
            ChunkReader::new(&bad_crc).unwrap().next_chunk(),
            Err(DecodeError::CrcMismatch {
                chunk: ChunkType::IEND,
                ..
            })
        ));

        let mut bad_type = png.clone();
        bad_type[12] = b'1';
        assert_eq!(
            ChunkReader::new(&bad_type)
                .unwrap()
                .next_chunk()
                .unwrap_err(),
            DecodeError::InvalidChunkType(ChunkType(*b"1END"))
        );

        let mut too_long = png.clone();
        too_long[8] = 0x80;
        assert_eq!(
            ChunkReader::new(&too_long)
                .unwrap()
                .next_chunk()
                .unwrap_err(),
            DecodeError::ChunkTooLong {
                length: 0x8000_0000
            }
        );

        let truncated = &png[..png.len() - 1];
        assert_eq!(
            ChunkReader::new(truncated)
                .unwrap()
                .next_chunk()
                .unwrap_err(),
            DecodeError::UnexpectedEof
        );
    }

    #[test]
    fn displays_chunk_types_safely() {
        assert_eq!(ChunkType::PHYS.to_string(), "pHYs");
        assert_eq!(ChunkType([b'a', 0, 0xFF, b'Z']).to_string(), "a\\x00\\xffZ");
        assert_eq!(format!("{:?}", ChunkType::IDAT), "ChunkType(IDAT)");
    }
}
