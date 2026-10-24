//! Errors reported by the resampler.

use std::error::Error;
use std::fmt;

use crate::error::RasterError;

/// Why a grid could not be built or an image could not be resampled.
#[derive(Clone, Debug, PartialEq)]
pub enum ResampleError {
    /// A grid's data length does not equal `width × height` (or that
    /// product overflows).
    GridSize {
        /// Requested width.
        width: usize,
        /// Requested height.
        height: usize,
        /// Number of pixels supplied.
        len: usize,
    },
    /// A grid side exceeds [`MAX_DIMENSION`](super::MAX_DIMENSION), or the
    /// grid exceeds [`MAX_PIXELS`](crate::MAX_PIXELS) or the memory there
    /// is.
    TooLarge {
        /// Grid width.
        width: usize,
        /// Grid height.
        height: usize,
    },
    /// The source image has no pixels.
    EmptySource,
    /// The affine transform has a non-finite coefficient or is not
    /// invertible.
    SingularTransform,
    /// The lookup mesh's dimensions differ from the destination's.
    MeshSize {
        /// Mesh width and height.
        mesh: (usize, usize),
        /// Destination width and height.
        destination: (usize, usize),
    },
    /// `alpha` is not a finite value in `[0, 1]`.
    InvalidAlpha(f64),
    /// `radius` is not finite and positive, or exceeds
    /// [`MAX_FILTER_RADIUS`](super::MAX_FILTER_RADIUS).
    InvalidRadius(f64),
    /// An interpolation name matplotlib's `_image` module does not know.
    UnknownInterpolation(String),
    /// The rasterizer could not cover the destination.
    Raster(RasterError),
}

impl fmt::Display for ResampleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GridSize { width, height, len } => {
                write!(f, "a {width}x{height} grid cannot hold {len} pixels")
            }
            Self::TooLarge { width, height } => write!(
                f,
                "a {width}x{height} grid exceeds the maximum side of {} or {} pixels",
                super::MAX_DIMENSION,
                crate::MAX_PIXELS
            ),
            Self::EmptySource => f.write_str("the source image is empty"),
            Self::SingularTransform => {
                f.write_str("the pixel transform is not finite and invertible")
            }
            Self::MeshSize { mesh, destination } => write!(
                f,
                "a {}x{} lookup mesh does not match the {}x{} destination",
                mesh.0, mesh.1, destination.0, destination.1
            ),
            Self::InvalidAlpha(alpha) => write!(f, "alpha {alpha} is not in [0, 1]"),
            Self::InvalidRadius(radius) => write!(
                f,
                "filter radius {radius} is not in (0, {}]",
                super::MAX_FILTER_RADIUS
            ),
            Self::UnknownInterpolation(name) => write!(f, "unknown interpolation {name:?}"),
            Self::Raster(err) => err.fmt(f),
        }
    }
}

impl Error for ResampleError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Raster(err) => Some(err),
            _ => None,
        }
    }
}

impl From<RasterError> for ResampleError {
    fn from(err: RasterError) -> Self {
        Self::Raster(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_are_descriptive() {
        let e = ResampleError::GridSize {
            width: 2,
            height: 3,
            len: 5,
        };
        assert_eq!(e.to_string(), "a 2x3 grid cannot hold 5 pixels");
        assert!(
            ResampleError::InvalidAlpha(2.0)
                .to_string()
                .contains("[0, 1]")
        );
        assert!(
            ResampleError::UnknownInterpolation("x".into())
                .to_string()
                .contains("\"x\"")
        );
    }
}
