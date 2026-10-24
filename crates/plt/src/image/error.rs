//! Errors of the image, mesh and hexagonal-binning plotting methods.

use std::fmt;

use plt_raster::resample::ResampleError;

use crate::axes::AspectError;
use crate::colors::multivar::MultivarError;
use crate::colors::{ColorizerError, ColormapError, NORM_WITH_LIMITS, NormError};
use crate::stats::StatsError;

/// Invalid input to `imshow`, `pcolormesh`, `hexbin` and friends, or a
/// failure while turning an image into pixels.
#[derive(Clone, Debug, PartialEq)]
pub enum ImageError {
    /// Two grids that must have the same shape do not.
    ShapeMismatch {
        /// What the grids are (`"mask"`, `"alpha"`, ...).
        what: &'static str,
        /// The image shape `(rows, columns)`.
        expected: (usize, usize),
        /// The offending shape.
        actual: (usize, usize),
    },
    /// The grid of cell corners or centres does not fit the colour array
    /// (matplotlib's `pcolor` dimension checks).
    MeshShape {
        /// The shading the grid was checked for.
        shading: &'static str,
        /// The colour array shape `(rows, columns)`.
        values: (usize, usize),
        /// Number of x coordinates per row.
        nx: usize,
        /// Number of y coordinates per column.
        ny: usize,
    },
    /// The coordinate arrays of a mesh have different shapes.
    IncompatibleCoordinates,
    /// A coordinate array of `pcolorfast` has the wrong length.
    EdgeCount {
        /// The axis (`"X"` or `"Y"`).
        axis: &'static str,
        /// Its length.
        len: usize,
        /// The required length (cells + 1).
        expected: usize,
    },
    /// Non-finite or masked mesh coordinates, which `pcolormesh` rejects.
    InvalidCoordinates,
    /// A shading the plotting method cannot draw (Gouraud for `pcolor`).
    ShadingNotSupported(&'static str),
    /// The filter radius is not a positive number.
    FilterRadius(f64),
    /// A hexbin grid needs at least one hexagon per direction.
    GridSize,
    /// A hexbin grid of this many hexagons along x and y would hold more
    /// than [`MAX_HEXAGONS`](crate::axes::MAX_HEXAGONS).
    TooManyHexagons(usize, usize),
    /// A hexbin extent with `xmin > xmax` or `ymin > ymax`.
    InvertedExtent,
    /// Hexbin values whose number differs from the number of points.
    ValueCount {
        /// Number of values.
        values: usize,
        /// Number of points.
        points: usize,
    },
    /// Both a norm and explicit `vmin`/`vmax` were given.
    NormWithLimits,
    /// An unknown or invalid colormap.
    Colormap(ColormapError),
    /// The norm could not map the data.
    Norm(NormError),
    /// An invalid aspect ratio.
    Aspect(AspectError),
    /// The resampler rejected the image geometry.
    Resample(ResampleError),
    /// Invalid two-dimensional histogram input (`hist2d`).
    Histogram(StatsError),
    /// Multivariate data needs at least two variates.
    TooFewVariates(usize),
    /// A bivariate or multivariate colormap was given for scalar data.
    MultivariateColormap,
    /// Data of several variates was given without a bivariate or
    /// multivariate colormap, or with options for scalar data.
    ScalarColorizer,
    /// The plotting method (or shading) cannot draw data of several
    /// variates, as matplotlib 3.11's cannot.
    MultivariateNotSupported(&'static str),
    /// The multivariate norms or colormap cannot map the data.
    Multivar(MultivarError),
}

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ShapeMismatch {
                what,
                expected,
                actual,
            } => write!(
                f,
                "the {what} has shape {actual:?} but the image has shape {expected:?}"
            ),
            Self::MeshShape {
                shading,
                values,
                nx,
                ny,
            } => write!(
                f,
                "dimensions of C {values:?} are incompatible with X ({nx}) and Y ({ny}) \
                 for shading='{shading}'"
            ),
            Self::IncompatibleCoordinates => f.write_str("incompatible X, Y inputs"),
            Self::EdgeCount {
                axis,
                len,
                expected,
            } => write!(
                f,
                "length of {axis} ({len}) must be one larger than the number of cells \
                 ({})",
                expected - 1
            ),
            Self::InvalidCoordinates => f.write_str(
                "x and y arguments to pcolormesh cannot have non-finite values or masked values",
            ),
            Self::ShadingNotSupported(shading) => {
                write!(f, "shading='{shading}' is not supported here")
            }
            Self::FilterRadius(radius) => {
                write!(
                    f,
                    "the filter radius must be a positive number, got {radius}"
                )
            }
            Self::GridSize => f.write_str("the hexbin grid size must be at least 1"),
            Self::TooManyHexagons(nx, ny) => write!(
                f,
                "a hexbin grid of {nx} × {ny} hexagons is larger than {}",
                crate::axes::MAX_HEXAGONS
            ),
            Self::InvertedExtent => {
                f.write_str("in extent, xmax must be greater than xmin and ymax than ymin")
            }
            Self::ValueCount { values, points } => {
                write!(f, "{values} values given for {points} points")
            }
            Self::NormWithLimits => f.write_str(NORM_WITH_LIMITS),
            Self::Colormap(err) => err.fmt(f),
            Self::Norm(err) => err.fmt(f),
            Self::Aspect(err) => err.fmt(f),
            Self::Resample(err) => err.fmt(f),
            Self::Histogram(err) => err.fmt(f),
            Self::TooFewVariates(n) => {
                write!(f, "multivariate data needs at least two variates, got {n}")
            }
            Self::MultivariateColormap => {
                f.write_str("a bivariate or multivariate colormap needs data of several variates")
            }
            Self::ScalarColorizer => f.write_str(
                "data of several variates needs a bivariate or multivariate colormap, \
                 and no scalar colormap, norm or limits",
            ),
            Self::MultivariateNotSupported(what) => {
                write!(f, "{what} does not support data of several variates")
            }
            Self::Multivar(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for ImageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Colormap(err) => Some(err),
            Self::Norm(err) => Some(err),
            Self::Aspect(err) => Some(err),
            Self::Resample(err) => Some(err),
            Self::Histogram(err) => Some(err),
            Self::Multivar(err) => Some(err),
            _ => None,
        }
    }
}

impl From<ColormapError> for ImageError {
    fn from(err: ColormapError) -> Self {
        Self::Colormap(err)
    }
}

impl From<ColorizerError> for ImageError {
    fn from(err: ColorizerError) -> Self {
        match err {
            ColorizerError::Colormap(err) => Self::Colormap(err),
            ColorizerError::NormWithLimits => Self::NormWithLimits,
        }
    }
}

impl From<NormError> for ImageError {
    fn from(err: NormError) -> Self {
        Self::Norm(err)
    }
}

impl From<AspectError> for ImageError {
    fn from(err: AspectError) -> Self {
        Self::Aspect(err)
    }
}

impl From<ResampleError> for ImageError {
    fn from(err: ResampleError) -> Self {
        Self::Resample(err)
    }
}

impl From<MultivarError> for ImageError {
    fn from(err: MultivarError) -> Self {
        Self::Multivar(err)
    }
}

impl From<StatsError> for ImageError {
    fn from(err: StatsError) -> Self {
        Self::Histogram(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_name_the_problem() {
        let err = ImageError::MeshShape {
            shading: "flat",
            values: (3, 4),
            nx: 4,
            ny: 3,
        };
        assert!(err.to_string().contains("shading='flat'"));
        let err = ImageError::EdgeCount {
            axis: "X",
            len: 3,
            expected: 5,
        };
        assert_eq!(
            err.to_string(),
            "length of X (3) must be one larger than the number of cells (4)"
        );
        let err = ImageError::from(NormError::Unscaled);
        assert!(std::error::Error::source(&err).is_some());
    }
}
