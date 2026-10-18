//! Why a TIFF file could not be decoded.

use std::fmt;

/// Why a TIFF file could not be decoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// The data does not start with a TIFF header.
    NotTiff,
    /// The file has no image directory.
    NoImage,
    /// A tag the image needs is missing.
    MissingTag(u16),
    /// A tag has a value of the wrong type or out of range.
    InvalidTag(u16),
    /// The layout of samples is not one Pillow reads: the combination of
    /// photometric interpretation, sample format, fill order, bits per
    /// sample and extra samples.
    UnsupportedLayout,
    /// A compression scheme other than none, `PackBits`, LZW and Deflate.
    UnsupportedCompression(u16),
    /// A predictor other than none and horizontal differencing of 8- or
    /// 16-bit samples.
    UnsupportedPredictor(u16),
    /// Samples stored in separate planes.
    UnsupportedPlanarConfiguration,
    /// A side of the image is zero.
    EmptyImage,
    /// The image has more pixels than the input can hold, or than Pillow
    /// accepts.
    TooLarge {
        /// The declared width.
        width: u32,
        /// The declared height.
        height: u32,
    },
    /// A strip or tile lies outside the file.
    Truncated,
    /// A strip or tile does not decompress to the data it should hold.
    CorruptData,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotTiff => f.write_str("not a TIFF file"),
            Self::NoImage => f.write_str("the TIFF file has no image"),
            Self::MissingTag(tag) => write!(f, "the TIFF image lacks tag {tag}"),
            Self::InvalidTag(tag) => write!(f, "invalid value of TIFF tag {tag}"),
            Self::UnsupportedLayout => f.write_str("unknown TIFF pixel layout"),
            Self::UnsupportedCompression(scheme) => {
                write!(f, "unsupported TIFF compression {scheme}")
            }
            Self::UnsupportedPredictor(predictor) => {
                write!(f, "unsupported TIFF predictor {predictor}")
            }
            Self::UnsupportedPlanarConfiguration => {
                f.write_str("TIFF images with separate sample planes are not supported")
            }
            Self::EmptyImage => f.write_str("the TIFF image has no pixels"),
            Self::TooLarge { width, height } => {
                write!(f, "a {width} × {height} TIFF image is too large to decode")
            }
            Self::Truncated => f.write_str("the TIFF image data lies outside the file"),
            Self::CorruptData => f.write_str("corrupt TIFF image data"),
        }
    }
}

impl std::error::Error for DecodeError {}
