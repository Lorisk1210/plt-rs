//! Errors raised by triangular-grid operations.

use std::fmt;

/// Why a triangulation operation failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TriError {
    /// `x` and `y` have different lengths.
    CoordinateLengths {
        /// Length of `x`.
        x: usize,
        /// Length of `y`.
        y: usize,
    },
    /// A triangle references a point that does not exist.
    PointIndex {
        /// The offending index.
        index: usize,
        /// Number of points.
        npoints: usize,
    },
    /// The mask does not have one entry per triangle.
    MaskLength {
        /// Number of triangles.
        expected: usize,
        /// Length of the mask.
        actual: usize,
    },
    /// A per-point array (field values, derivatives) has the wrong length.
    ValueLength {
        /// Number of points.
        expected: usize,
        /// Length of the array.
        actual: usize,
    },
    /// Delaunay triangulation needs at least three points.
    TooFewPoints,
    /// Delaunay triangulation needs at least three distinct points.
    TooFewUniquePoints,
    /// All points are collinear, so no triangle can be formed.
    Collinear,
    /// A coordinate is NaN or infinite.
    NonFinite,
    /// Triangles overlap or are duplicated, so points cannot be located.
    InvalidTriangulation,
}

impl fmt::Display for TriError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CoordinateLengths { x, y } => {
                write!(f, "x and y must be equal-length arrays, got {x} and {y}")
            }
            Self::PointIndex { index, npoints } => write!(
                f,
                "triangles are indices into the points and must be in the range \
                 0 <= i < {npoints} but found value {index}"
            ),
            Self::MaskLength { expected, actual } => write!(
                f,
                "mask array must have same length as triangles array ({expected}), got {actual}"
            ),
            Self::ValueLength { expected, actual } => write!(
                f,
                "array must have same length as triangulation x and y arrays \
                 ({expected}), got {actual}"
            ),
            Self::TooFewPoints => f.write_str("at least 3 points are required for triangulation"),
            Self::TooFewUniquePoints => {
                f.write_str("at least 3 unique points are required for triangulation")
            }
            Self::Collinear => f.write_str("all points are collinear (singular input)"),
            Self::NonFinite => f.write_str("point coordinates must be finite"),
            Self::InvalidTriangulation => f.write_str("triangulation is invalid"),
        }
    }
}

impl std::error::Error for TriError {}
