//! Unit conversion of plotting values (matplotlib's `units` and
//! `category` modules): numbers pass through unchanged and category names
//! are placed at 0, 1, 2, ... in order of their first appearance on an
//! axis.
//!
//! ```
//! use plt::prelude::*;
//! let ax = Figure::new().subplot();
//! let bars = ax.bar(["apple", "banana", "cherry"], [4.0, 6.0, 3.0], Default::default())?;
//! assert_eq!(bars.patches[1].shape().xy.x, 0.6);
//! # Ok::<(), plt::axes::PlotError>(())
//! ```

mod category;

pub use category::{Categories, CategoryFormatter, CategoryLocator};

/// Extents along an axis re-expressed through the axis origin, the first
/// finite position (or the first position when none is finite):
/// `(origin + delta) - origin`. This is matplotlib's `_convert_dx` for
/// numbers, which rounds the extents of bars and error bars; without
/// positions the deltas are kept.
///
/// ```
/// use plt::units::convert_dx;
/// // 1e16 + 1 rounds back to 1e16.
/// assert_eq!(convert_dx(&[f64::NAN, 1e16], &[1.0]), [0.0]);
/// assert_eq!(convert_dx(&[], &[0.1]), [0.1]);
/// ```
pub fn convert_dx(positions: &[f64], deltas: &[f64]) -> Vec<f64> {
    let origin = positions
        .iter()
        .copied()
        .find(|p| p.is_finite())
        .or_else(|| positions.first().copied());
    match origin {
        Some(origin) => deltas.iter().map(|&d| (origin + d) - origin).collect(),
        None => deltas.to_vec(),
    }
}

/// Plotting values along one axis before unit conversion.
#[derive(Clone, Debug, PartialEq)]
pub enum AxisValues {
    /// Plain numbers.
    Numbers(Vec<f64>),
    /// Category names.
    Categories(Vec<String>),
}

impl AxisValues {
    /// The number of values.
    pub fn len(&self) -> usize {
        match self {
            Self::Numbers(values) => values.len(),
            Self::Categories(names) => names.len(),
        }
    }

    /// Whether there are no values.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl From<Vec<f64>> for AxisValues {
    fn from(values: Vec<f64>) -> Self {
        Self::Numbers(values)
    }
}

impl From<&Vec<f64>> for AxisValues {
    fn from(values: &Vec<f64>) -> Self {
        Self::Numbers(values.clone())
    }
}

impl From<&[f64]> for AxisValues {
    fn from(values: &[f64]) -> Self {
        Self::Numbers(values.to_vec())
    }
}

impl<const N: usize> From<[f64; N]> for AxisValues {
    fn from(values: [f64; N]) -> Self {
        Self::Numbers(values.to_vec())
    }
}

impl<const N: usize> From<&[f64; N]> for AxisValues {
    fn from(values: &[f64; N]) -> Self {
        Self::Numbers(values.to_vec())
    }
}

impl From<Vec<String>> for AxisValues {
    fn from(names: Vec<String>) -> Self {
        Self::Categories(names)
    }
}

impl From<&[String]> for AxisValues {
    fn from(names: &[String]) -> Self {
        Self::Categories(names.to_vec())
    }
}

impl From<Vec<&str>> for AxisValues {
    fn from(names: Vec<&str>) -> Self {
        names.as_slice().into()
    }
}

impl From<&[&str]> for AxisValues {
    fn from(names: &[&str]) -> Self {
        Self::Categories(names.iter().map(|&name| name.to_owned()).collect())
    }
}

impl<const N: usize> From<[&str; N]> for AxisValues {
    fn from(names: [&str; N]) -> Self {
        names.as_slice().into()
    }
}
