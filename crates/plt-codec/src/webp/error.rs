//! Why WebP data could not be decoded.

use std::fmt;

/// Why a WebP file could not be decoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// The data does not start with a RIFF `WEBP` header.
    NotWebp,
    /// The data ends inside a chunk or a bitstream.
    Truncated,
    /// The file holds no `VP8 ` or `VP8L` image.
    NoImage,
    /// A chunk too short for its fields, or contradicting the others.
    MalformedChunk {
        /// The chunk's four-character code.
        fourcc: [u8; 4],
    },
    /// The image has more pixels than Pillow accepts.
    TooLarge {
        /// The image width.
        width: u32,
        /// The image height.
        height: u32,
    },
    /// A VP8L header with a wrong signature or version.
    LosslessHeader,
    /// A prefix code that is empty, incomplete or over-subscribed.
    InvalidPrefixCode,
    /// A VP8L image declaring the same transform twice.
    RepeatedTransform,
    /// A colour cache of more than 11 bits, or of none.
    InvalidColorCache,
    /// A backward reference before the image start or past its end.
    InvalidBackwardReference,
    /// A VP8 frame with a wrong frame tag, start code or dimensions.
    LossyHeader,
    /// A VP8 frame whose partitions do not fit in the data.
    InvalidPartitions,
    /// An `ALPH` chunk with an unknown compression or filter.
    InvalidAlpha,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotWebp => f.write_str("not a WebP file"),
            Self::Truncated => f.write_str("the WebP data ends early"),
            Self::NoImage => f.write_str("the WebP file holds no image"),
            Self::MalformedChunk { fourcc } => {
                write!(
                    f,
                    "malformed WebP {} chunk",
                    String::from_utf8_lossy(fourcc)
                )
            }
            Self::TooLarge { width, height } => {
                write!(f, "a {width} × {height} WebP image is too large to decode")
            }
            Self::LosslessHeader => f.write_str("invalid VP8L header"),
            Self::InvalidPrefixCode => f.write_str("invalid prefix code in VP8L data"),
            Self::RepeatedTransform => f.write_str("a VP8L transform appears twice"),
            Self::InvalidColorCache => f.write_str("invalid VP8L colour cache size"),
            Self::InvalidBackwardReference => {
                f.write_str("a VP8L backward reference leaves the image")
            }
            Self::LossyHeader => f.write_str("invalid VP8 frame header"),
            Self::InvalidPartitions => f.write_str("VP8 partitions exceed the data"),
            Self::InvalidAlpha => f.write_str("invalid WebP alpha data"),
        }
    }
}

impl std::error::Error for DecodeError {}
