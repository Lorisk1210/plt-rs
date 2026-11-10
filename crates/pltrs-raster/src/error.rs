//! The crate's error type.

use std::fmt;

use pltrs_core::alloc::OutOfMemory;

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
    /// A path too large for matplotlib's Agg rasterizer (its cell block
    /// limit, which an infinite sketch exceeds when the path is filled or
    /// hatched), which matplotlib's `RendererAgg.draw_path` cannot split
    /// into pieces for the reasons given (its `OverflowError`).
    UnsplittablePath {
        /// The path is filled.
        filled: bool,
        /// The path is hatched.
        hatched: bool,
        /// The path qualifies for simplification (`should_simplify`).
        simplifiable: bool,
    },
    /// A dash pattern contains a negative or non-finite length, or no
    /// positive length at all.
    InvalidDashPattern,
    /// A buffer the system cannot allocate, or larger than
    /// [`MAX_PIXELS`](crate::MAX_PIXELS) (matplotlib's `MemoryError`).
    OutOfMemory(OutOfMemory),
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
            Self::UnsplittablePath {
                filled,
                hatched,
                simplifiable,
            } => {
                f.write_str(
                    "Exceeded cell block limit in Agg, however for the following reasons:\n\n",
                )?;
                if *filled {
                    f.write_str("- cannot split filled path\n")?;
                }
                if *hatched {
                    f.write_str("- cannot split hatched path\n")?;
                }
                if !simplifiable {
                    f.write_str("- path.should_simplify is False\n")?;
                }
                f.write_str(
                    "\nwe cannot automatically split up this path to draw.\n\n\
                     Please manually simplify your path.",
                )
            }
            Self::InvalidDashPattern => f.write_str(
                "dash lengths must be finite and non-negative with at least one positive",
            ),
            Self::OutOfMemory(err) => write!(f, "out of memory: {err}"),
        }
    }
}

impl std::error::Error for RasterError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::OutOfMemory(err) => Some(err),
            _ => None,
        }
    }
}

impl From<OutOfMemory> for RasterError {
    fn from(err: OutOfMemory) -> Self {
        Self::OutOfMemory(err)
    }
}
