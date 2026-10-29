//! The direction of bars, steps, histograms, colorbars, sliders and span
//! selectors.

use std::fmt;
use std::str::FromStr;

/// Which way values grow (matplotlib's `orientation`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Orientation {
    /// Values along y, positions along x.
    #[default]
    Vertical,
    /// Values along x, positions along y.
    Horizontal,
}

impl Orientation {
    /// `(position, value)` laid out as `(x, y)` for this orientation.
    pub fn place(self, position: f64, value: f64) -> (f64, f64) {
        match self {
            Self::Vertical => (position, value),
            Self::Horizontal => (value, position),
        }
    }
}

/// An orientation name other than `"vertical"` or `"horizontal"`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrientationError(pub String);

impl fmt::Display for OrientationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} is not a valid orientation; supported values are 'vertical', 'horizontal'",
            self.0
        )
    }
}

impl std::error::Error for OrientationError {}

impl FromStr for Orientation {
    type Err = OrientationError;

    fn from_str(name: &str) -> Result<Self, OrientationError> {
        match name {
            "vertical" => Ok(Self::Vertical),
            "horizontal" => Ok(Self::Horizontal),
            other => Err(OrientationError(other.to_owned())),
        }
    }
}
