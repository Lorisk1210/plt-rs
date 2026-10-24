//! Invalid streamplot input.

use std::fmt;

use plt_core::Point;

use super::GridAxis;
use crate::colors::{ColorizerError, NormError};

/// Why a streamplot could not be drawn.
#[derive(Clone, Debug, PartialEq)]
pub enum StreamplotError {
    /// A grid coordinate has fewer than two values.
    TooFewPoints(GridAxis),
    /// A grid coordinate is not strictly increasing.
    NotIncreasing(GridAxis),
    /// A grid coordinate is not evenly spaced.
    UnevenSpacing(GridAxis),
    /// The rows of a 2-D `x` differ.
    MeshRowsDiffer,
    /// The columns of a 2-D `y` differ.
    MeshColumnsDiffer,
    /// A field does not have the grid's shape.
    ShapeMismatch {
        /// The field (`"u"`, `"v"`, `"color"`, `"linewidth"`).
        field: &'static str,
        /// Its `(rows, columns)`.
        shape: (usize, usize),
        /// The grid's `(rows, columns)`.
        grid: (usize, usize),
    },
    /// A negative density.
    InvalidDensity(f64),
    /// Densities whose grid of streamline cells would have more than
    /// [`MAX_CELLS`](super::MAX_CELLS) cells.
    DensityTooHigh(f64, f64),
    /// Tracing the streamlines took more than
    /// [`MAX_STEPS`](super::MAX_STEPS) integration steps.
    TooManySteps,
    /// More arrows (this many, or per streamline) than
    /// [`MAX_ARROWS`](super::MAX_ARROWS).
    TooManyArrows(usize),
    /// An integration scale that is not positive.
    InvalidScale {
        /// The option.
        name: &'static str,
        /// Its value.
        value: f64,
    },
    /// A start point outside the grid.
    StartPointOutside(Point),
    /// An unknown colormap, or a norm given with limits.
    Colorizer(ColorizerError),
    /// Colour values the norm cannot map.
    Norm(NormError),
}

impl fmt::Display for StreamplotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewPoints(axis) => {
                write!(f, "'{}' must have at least two values", axis.name())
            }
            Self::NotIncreasing(axis) => write!(f, "'{}' must be strictly increasing", axis.name()),
            Self::UnevenSpacing(axis) => {
                write!(f, "'{}' values must be equally spaced", axis.name())
            }
            Self::MeshRowsDiffer => f.write_str("The rows of 'x' must be equal"),
            Self::MeshColumnsDiffer => f.write_str("The columns of 'y' must be equal"),
            Self::ShapeMismatch { field, shape, grid } => write!(
                f,
                "'{field}' has shape {shape:?} but must match the shape {grid:?} of the (x, y) grid"
            ),
            Self::InvalidDensity(density) => {
                write!(f, "'density' must be positive, got {density}")
            }
            Self::DensityTooHigh(x, y) => write!(
                f,
                "a density of ({x}, {y}) needs more than {} streamline cells",
                super::MAX_CELLS
            ),
            Self::TooManyArrows(count) => write!(
                f,
                "{count} streamline arrows are more than {}",
                super::MAX_ARROWS
            ),
            Self::TooManySteps => write!(
                f,
                "the streamlines need more than {} integration steps",
                super::MAX_STEPS
            ),
            Self::InvalidScale { name, value } => {
                write!(f, "The value of {name} must be > 0, got {value}")
            }
            Self::StartPointOutside(p) => {
                write!(
                    f,
                    "Starting point ({}, {}) outside of data boundaries",
                    p.x, p.y
                )
            }
            Self::Colorizer(err) => err.fmt(f),
            Self::Norm(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for StreamplotError {}

impl From<ColorizerError> for StreamplotError {
    fn from(err: ColorizerError) -> Self {
        Self::Colorizer(err)
    }
}

impl From<NormError> for StreamplotError {
    fn from(err: NormError) -> Self {
        Self::Norm(err)
    }
}
