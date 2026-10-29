//! The spacing of subplot grids (matplotlib's `SubplotParams` and the
//! `subplots_adjust` keywords) and the errors of grid specifications.

use std::fmt;

use crate::rc::RcParams;

/// The margins and spacing of a subplot grid, in figure fractions
/// (matplotlib's `SubplotParams` / `figure.subplot.*`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SubplotParams {
    /// Left edge of the grid.
    pub left: f64,
    /// Bottom edge of the grid.
    pub bottom: f64,
    /// Right edge of the grid.
    pub right: f64,
    /// Top edge of the grid.
    pub top: f64,
    /// Horizontal gap between columns, as a fraction of the average column width.
    pub wspace: f64,
    /// Vertical gap between rows, as a fraction of the average row height.
    pub hspace: f64,
}

impl SubplotParams {
    /// The defaults from `figure.subplot.*`.
    pub fn from_rc(rc: &RcParams) -> Self {
        Self {
            left: rc.figure_subplot_left,
            bottom: rc.figure_subplot_bottom,
            right: rc.figure_subplot_right,
            top: rc.figure_subplot_top,
            wspace: rc.figure_subplot_wspace,
            hspace: rc.figure_subplot_hspace,
        }
    }

    /// Applies the given overrides, rejecting an empty grid area
    /// (matplotlib's `SubplotParams.update`).
    pub fn updated(self, overrides: SubplotOverrides) -> Result<Self, GridSpecError> {
        let updated = Self {
            left: overrides.left.unwrap_or(self.left),
            bottom: overrides.bottom.unwrap_or(self.bottom),
            right: overrides.right.unwrap_or(self.right),
            top: overrides.top.unwrap_or(self.top),
            wspace: overrides.wspace.unwrap_or(self.wspace),
            hspace: overrides.hspace.unwrap_or(self.hspace),
        };
        if updated.left >= updated.right {
            return Err(GridSpecError::EmptyWidth);
        }
        if updated.bottom >= updated.top {
            return Err(GridSpecError::EmptyHeight);
        }
        Ok(updated)
    }
}

/// Optional replacements for some [`SubplotParams`] fields.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SubplotOverrides {
    /// Left edge.
    pub left: Option<f64>,
    /// Bottom edge.
    pub bottom: Option<f64>,
    /// Right edge.
    pub right: Option<f64>,
    /// Top edge.
    pub top: Option<f64>,
    /// Column spacing.
    pub wspace: Option<f64>,
    /// Row spacing.
    pub hspace: Option<f64>,
}

/// An invalid grid definition or cell selection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GridSpecError {
    /// A grid needs at least one row and one column.
    EmptyGrid,
    /// The number of width or height ratios does not match the grid.
    RatioCount {
        /// Expected number of ratios.
        expected: usize,
        /// Supplied number of ratios.
        actual: usize,
    },
    /// `left` must be smaller than `right`.
    EmptyWidth,
    /// `bottom` must be smaller than `top`.
    EmptyHeight,
    /// The selected cells fall outside the grid.
    OutOfRange,
}

impl fmt::Display for GridSpecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyGrid => f.write_str("a grid needs at least one row and one column"),
            Self::RatioCount { expected, actual } => {
                write!(f, "expected {expected} ratios, got {actual}")
            }
            Self::EmptyWidth => f.write_str("left cannot be >= right"),
            Self::EmptyHeight => f.write_str("bottom cannot be >= top"),
            Self::OutOfRange => f.write_str("subplot cells outside the grid"),
        }
    }
}

impl std::error::Error for GridSpecError {}
