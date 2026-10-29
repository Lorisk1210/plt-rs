//! Why a mosaic cannot be laid out.

use std::convert::Infallible;
use std::fmt;

use crate::gridspec::GridSpecError;
use crate::projections::ProjectionError;

/// Why a mosaic cannot be laid out.
#[derive(Clone, Debug, PartialEq)]
pub enum MosaicError {
    /// The mosaic has no rows or no columns.
    Empty,
    /// Rows of different lengths.
    Ragged {
        /// Length of the first row.
        expected: usize,
        /// Length of the offending row.
        actual: usize,
    },
    /// A label covers cells that do not form a rectangle.
    NonRectangular(String),
    /// A label appears in two separate places (for example inside and
    /// outside a nested mosaic).
    Duplicate(String),
    /// Options name a label the mosaic does not have (matplotlib's
    /// `per_subplot_kw` keys).
    UnknownLabel(String),
    /// Options give one label two projections (a label in two of
    /// matplotlib's `per_subplot_kw` keys).
    RepeatedLabel(String),
    /// The grid itself is invalid (for example mismatched ratios).
    Grid(GridSpecError),
    /// Axes cannot be made with a projection the options ask for.
    Projection(ProjectionError),
}

impl fmt::Display for MosaicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("a mosaic needs at least one row and one column"),
            Self::Ragged { expected, actual } => write!(
                f,
                "all mosaic rows must be the same length: the first has {expected} cells, \
                 another {actual}"
            ),
            Self::NonRectangular(label) => {
                write!(f, "the label {label:?} specifies a non-rectangular area")
            }
            Self::Duplicate(label) => write!(f, "the label {label:?} appears more than once"),
            Self::UnknownLabel(label) => write!(f, "the label {label:?} is not in the mosaic"),
            Self::RepeatedLabel(label) => {
                write!(
                    f,
                    "the label {label:?} appears multiple times in per_subplot"
                )
            }
            Self::Grid(err) => err.fmt(f),
            Self::Projection(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for MosaicError {}

impl From<GridSpecError> for MosaicError {
    fn from(err: GridSpecError) -> Self {
        Self::Grid(err)
    }
}

impl From<Infallible> for MosaicError {
    /// A mosaic given as a [`Mosaic`](super::Mosaic), which converts without
    /// failing.
    fn from(never: Infallible) -> Self {
        match never {}
    }
}

impl From<ProjectionError> for MosaicError {
    fn from(err: ProjectionError) -> Self {
        match err {
            ProjectionError::Grid(err) => Self::Grid(err),
            err => Self::Projection(err),
        }
    }
}
