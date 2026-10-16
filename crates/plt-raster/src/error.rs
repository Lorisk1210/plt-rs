//! The crate's error type.

use std::fmt;

/// Why a raster operation could not be performed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RasterError {
    /// A pixmap or mask dimension is zero-sized where that is not allowed,
    /// or too large to address (matplotlib caps each side below 2²³).
    InvalidDimensions {
        /// Requested width in pixels.
        width: u32,
        /// Requested height in pixels.
        height: u32,
    },
    /// A raw pixel buffer does not have `width × height × channels` bytes.
    DataLength {
        /// Bytes required by the dimensions.
        expected: usize,
        /// Bytes supplied.
        actual: usize,
    },
    /// A clip mask does not cover the target it is applied to.
    MaskSize {
        /// Mask dimensions.
        mask: (u32, u32),
        /// Target dimensions.
        target: (u32, u32),
    },
    /// The geometry produced more coverage cells than the rasterizer's
    /// limit (matplotlib raises `OverflowError: Exceeded cell block limit`).
    CellLimitExceeded {
        /// The configured maximum number of cells.
        limit: usize,
    },
    /// A dash pattern contains a negative or non-finite length, or no
    /// positive length at all.
    InvalidDashPattern,
}

impl fmt::Display for RasterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDimensions { width, height } => {
                write!(f, "invalid raster dimensions {width}x{height}")
            }
            Self::DataLength { expected, actual } => {
                write!(f, "pixel buffer has {actual} bytes, expected {expected}")
            }
            Self::MaskSize { mask, target } => write!(
                f,
                "clip mask is {}x{} but the target is {}x{}",
                mask.0, mask.1, target.0, target.1
            ),
            Self::CellLimitExceeded { limit } => {
                write!(
                    f,
                    "path is too complex: exceeded the limit of {limit} raster cells"
                )
            }
            Self::InvalidDashPattern => f.write_str(
                "dash lengths must be finite and non-negative with at least one positive",
            ),
        }
    }
}

impl std::error::Error for RasterError {}
