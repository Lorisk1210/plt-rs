//! PNG encoding and decoding errors.

use std::error::Error;
use std::fmt;

use pltrs_core::alloc::OutOfMemory;

use super::chunk::ChunkType;
use super::types::{BitDepth, ColorType};
use crate::zlib::ZlibError;

/// An error preventing an image from being encoded.
#[derive(Clone, Debug, PartialEq)]
pub enum EncodeError {
    /// Width or height is zero or above 2^31 − 1.
    InvalidDimensions {
        /// The requested width.
        width: u32,
        /// The requested height.
        height: u32,
    },
    /// The encoder does not write this colour type / bit depth combination.
    UnsupportedFormat {
        /// The requested colour type.
        color: ColorType,
        /// The requested bit depth.
        bit_depth: BitDepth,
    },
    /// The pixel data does not have the size the dimensions imply.
    DataLengthMismatch {
        /// Bytes the dimensions and format require.
        expected: usize,
        /// Bytes supplied.
        actual: usize,
    },
    /// The image is too large to address in memory.
    ImageTooLarge,
    /// A text keyword is empty, longer than 79 bytes, not printable Latin-1,
    /// or has leading, trailing or consecutive spaces.
    InvalidTextKeyword(String),
    /// A text value contains a NUL character or does not fit in a chunk.
    InvalidTextValue {
        /// The keyword of the offending entry.
        keyword: String,
    },
    /// A resolution is not a finite positive number of dots per inch that
    /// fits the `pHYs` chunk.
    InvalidDpi(f64),
    /// An animated PNG needs at least one frame.
    NoFrames,
    /// An animation frame differs in size or format from the first one.
    FrameMismatch {
        /// The offending frame.
        index: usize,
    },
    /// The memory for the encoded image cannot be allocated.
    OutOfMemory(OutOfMemory),
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimensions { width, height } => {
                write!(f, "invalid PNG dimensions {width}×{height}")
            }
            Self::UnsupportedFormat { color, bit_depth } => {
                write!(f, "cannot encode {bit_depth} {color} PNG images")
            }
            Self::DataLengthMismatch { expected, actual } => write!(
                f,
                "pixel data has {actual} bytes but the image needs {expected}"
            ),
            Self::ImageTooLarge => f.write_str("image is too large to encode"),
            Self::InvalidTextKeyword(keyword) => {
                write!(f, "invalid PNG text keyword {keyword:?}")
            }
            Self::InvalidTextValue { keyword } => {
                write!(
                    f,
                    "invalid PNG text for keyword {keyword:?}: NUL character or too long"
                )
            }
            Self::InvalidDpi(dpi) => write!(f, "invalid resolution of {dpi} dpi"),
            Self::NoFrames => f.write_str("an animated PNG needs at least one frame"),
            Self::FrameMismatch { index } => write!(
                f,
                "animation frame {index} differs in size or format from the first"
            ),
            Self::OutOfMemory(err) => write!(f, "cannot encode the image: {err}"),
        }
    }
}

impl Error for EncodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::OutOfMemory(err) => Some(err),
            _ => None,
        }
    }
}

impl From<OutOfMemory> for EncodeError {
    fn from(err: OutOfMemory) -> Self {
        Self::OutOfMemory(err)
    }
}

/// An error found while decoding a PNG file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// The data does not start with the PNG signature.
    InvalidSignature,
    /// The data ended inside a chunk or before the `IEND` chunk.
    UnexpectedEof,
    /// A chunk declares a length above 2^31 − 1.
    ChunkTooLong {
        /// The declared length.
        length: u32,
    },
    /// A chunk type contains bytes other than ASCII letters.
    InvalidChunkType(ChunkType),
    /// A chunk's CRC does not match its contents.
    CrcMismatch {
        /// The damaged chunk.
        chunk: ChunkType,
        /// CRC stored in the file.
        stored: u32,
        /// CRC of the chunk as read.
        computed: u32,
    },
    /// The first chunk is not `IHDR`.
    MissingHeader,
    /// A chunk with a fixed layout has the wrong length.
    InvalidChunkLength {
        /// The chunk concerned.
        chunk: ChunkType,
        /// Its actual length.
        length: usize,
    },
    /// Width or height is zero or above 2^31 − 1.
    InvalidDimensions {
        /// The declared width.
        width: u32,
        /// The declared height.
        height: u32,
    },
    /// The colour type / bit depth pair is not allowed by PNG.
    InvalidFormat {
        /// The `IHDR` colour type code.
        color_type: u8,
        /// The `IHDR` bit depth.
        bit_depth: u8,
    },
    /// The compression method is not 0 (DEFLATE).
    UnsupportedCompressionMethod(u8),
    /// The filter method is not 0 (adaptive filtering).
    UnsupportedFilterMethod(u8),
    /// The interlace method is neither 0 (none) nor 1 (Adam7).
    UnsupportedInterlaceMethod(u8),
    /// A chunk that may occur only once occurs again.
    DuplicateChunk(ChunkType),
    /// A critical chunk appears where it is not allowed, e.g. `PLTE` after
    /// `IDAT`, or `IDAT` chunks that are not consecutive.
    ChunkOutOfOrder(ChunkType),
    /// A critical chunk this decoder does not know.
    UnknownCriticalChunk(ChunkType),
    /// An indexed image has no `PLTE` chunk.
    MissingPalette,
    /// A palette is empty, larger than 256 entries, or not a whole number of
    /// RGB triples.
    InvalidPalette {
        /// Length of the `PLTE` data.
        length: usize,
    },
    /// A palette appears in a greyscale image.
    UnexpectedPalette,
    /// A `tRNS` chunk has the wrong size, or appears in an image with an
    /// alpha channel.
    InvalidTransparency,
    /// There is no `IDAT` chunk.
    MissingImageData,
    /// The decoded image would exceed the decoder's size limit.
    ImageTooLarge {
        /// The limit, in bytes.
        limit: usize,
    },
    /// The image data is not a valid zlib stream.
    Zlib(ZlibError),
    /// The image data decompresses to fewer bytes than the image needs.
    ImageDataTooShort {
        /// Bytes the image needs.
        expected: usize,
        /// Bytes found.
        actual: usize,
    },
    /// A scanline uses a filter type other than 0–4.
    InvalidFilterType(u8),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSignature => f.write_str("not a PNG file (bad signature)"),
            Self::UnexpectedEof => f.write_str("PNG data ended unexpectedly"),
            Self::ChunkTooLong { length } => write!(f, "chunk length {length} is too large"),
            Self::InvalidChunkType(chunk) => write!(f, "invalid chunk type {chunk:?}"),
            Self::CrcMismatch {
                chunk,
                stored,
                computed,
            } => write!(
                f,
                "CRC mismatch in {chunk} chunk: stored {stored:#010x}, computed {computed:#010x}"
            ),
            Self::MissingHeader => f.write_str("PNG does not start with an IHDR chunk"),
            Self::InvalidChunkLength { chunk, length } => {
                write!(f, "{chunk} chunk has invalid length {length}")
            }
            Self::InvalidDimensions { width, height } => {
                write!(f, "invalid PNG dimensions {width}×{height}")
            }
            Self::InvalidFormat {
                color_type,
                bit_depth,
            } => write!(
                f,
                "invalid colour type {color_type} with bit depth {bit_depth}"
            ),
            Self::UnsupportedCompressionMethod(method) => {
                write!(f, "unsupported compression method {method}")
            }
            Self::UnsupportedFilterMethod(method) => {
                write!(f, "unsupported filter method {method}")
            }
            Self::UnsupportedInterlaceMethod(method) => {
                write!(f, "unsupported interlace method {method}")
            }
            Self::DuplicateChunk(chunk) => write!(f, "duplicate {chunk} chunk"),
            Self::ChunkOutOfOrder(chunk) => write!(f, "{chunk} chunk out of order"),
            Self::UnknownCriticalChunk(chunk) => write!(f, "unknown critical chunk {chunk}"),
            Self::MissingPalette => f.write_str("indexed image has no palette"),
            Self::InvalidPalette { length } => write!(f, "invalid palette of {length} bytes"),
            Self::UnexpectedPalette => f.write_str("greyscale image has a palette"),
            Self::InvalidTransparency => f.write_str("invalid tRNS chunk"),
            Self::MissingImageData => f.write_str("PNG has no IDAT chunk"),
            Self::ImageTooLarge { limit } => {
                write!(f, "decoded image would exceed the limit of {limit} bytes")
            }
            Self::Zlib(error) => write!(f, "invalid image data: {error}"),
            Self::ImageDataTooShort { expected, actual } => write!(
                f,
                "image data has {actual} bytes after decompression, {expected} needed"
            ),
            Self::InvalidFilterType(filter) => write!(f, "invalid scanline filter type {filter}"),
        }
    }
}

impl Error for DecodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Zlib(error) => Some(error),
            _ => None,
        }
    }
}

impl From<ZlibError> for DecodeError {
    fn from(error: ZlibError) -> Self {
        Self::Zlib(error)
    }
}
