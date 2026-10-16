//! The `IHDR` chunk: dimensions, pixel format and interlacing.

use super::chunk::ChunkType;
use super::error::DecodeError;
use super::types::{BitDepth, ColorType, PixelFormat};

/// Largest width or height PNG allows.
const MAX_DIMENSION: u32 = (1 << 31) - 1;

/// Whether PNG allows an image of `width` × `height` pixels.
pub(crate) fn valid_dimensions(width: u32, height: u32) -> bool {
    (1..=MAX_DIMENSION).contains(&width) && (1..=MAX_DIMENSION).contains(&height)
}

/// The decoded contents of `IHDR`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Header {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) format: PixelFormat,
    pub(crate) interlaced: bool,
}

impl Header {
    /// The 13-byte `IHDR` payload.
    pub(crate) fn to_bytes(self) -> [u8; 13] {
        let mut bytes = [0; 13];
        bytes[..4].copy_from_slice(&self.width.to_be_bytes());
        bytes[4..8].copy_from_slice(&self.height.to_be_bytes());
        bytes[8] = self.format.depth.bits();
        bytes[9] = self.format.color.code();
        // Bytes 10 and 11: compression and filter method 0.
        bytes[12] = u8::from(self.interlaced);
        bytes
    }

    /// Parses and validates an `IHDR` payload.
    pub(crate) fn parse(data: &[u8]) -> Result<Self, DecodeError> {
        let bytes = <&[u8; 13]>::try_from(data).map_err(|_| DecodeError::InvalidChunkLength {
            chunk: ChunkType::IHDR,
            length: data.len(),
        })?;
        let width = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let height = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let [bit_depth, color_type, compression, filter, interlace] =
            [bytes[8], bytes[9], bytes[10], bytes[11], bytes[12]];
        if !valid_dimensions(width, height) {
            return Err(DecodeError::InvalidDimensions { width, height });
        }
        let format = ColorType::from_code(color_type)
            .zip(BitDepth::from_bits(bit_depth))
            .and_then(|(color, depth)| PixelFormat::new(color, depth))
            .ok_or(DecodeError::InvalidFormat {
                color_type,
                bit_depth,
            })?;
        if compression != 0 {
            return Err(DecodeError::UnsupportedCompressionMethod(compression));
        }
        if filter != 0 {
            return Err(DecodeError::UnsupportedFilterMethod(filter));
        }
        let interlaced = match interlace {
            0 => false,
            1 => true,
            other => return Err(DecodeError::UnsupportedInterlaceMethod(other)),
        };
        Ok(Self {
            width,
            height,
            format,
            interlaced,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header() -> Header {
        Header {
            width: 640,
            height: 480,
            format: PixelFormat::new(ColorType::Rgba, BitDepth::Eight).unwrap(),
            interlaced: false,
        }
    }

    #[test]
    fn round_trips() {
        let bytes = header().to_bytes();
        assert_eq!(bytes, [0, 0, 2, 128, 0, 0, 1, 224, 8, 6, 0, 0, 0]);
        assert_eq!(Header::parse(&bytes), Ok(header()));
    }

    #[test]
    fn rejects_invalid_fields() {
        let good = header().to_bytes();
        let with = |index: usize, value: u8| {
            let mut bytes = good;
            bytes[index] = value;
            Header::parse(&bytes)
        };
        assert_eq!(
            Header::parse(&good[..12]),
            Err(DecodeError::InvalidChunkLength {
                chunk: ChunkType::IHDR,
                length: 12
            })
        );
        let mut zero_width = good;
        zero_width[2] = 0;
        zero_width[3] = 0;
        assert_eq!(
            Header::parse(&zero_width),
            Err(DecodeError::InvalidDimensions {
                width: 0,
                height: 480
            })
        );
        assert!(matches!(
            with(0, 0x80),
            Err(DecodeError::InvalidDimensions { .. })
        ));
        assert_eq!(
            with(8, 4),
            Err(DecodeError::InvalidFormat {
                color_type: 6,
                bit_depth: 4
            })
        );
        assert_eq!(
            with(9, 5),
            Err(DecodeError::InvalidFormat {
                color_type: 5,
                bit_depth: 8
            })
        );
        assert_eq!(
            with(10, 1),
            Err(DecodeError::UnsupportedCompressionMethod(1))
        );
        assert_eq!(with(11, 1), Err(DecodeError::UnsupportedFilterMethod(1)));
        assert_eq!(with(12, 2), Err(DecodeError::UnsupportedInterlaceMethod(2)));
        assert_eq!(with(12, 1).map(|h| h.interlaced), Ok(true));
    }
}
