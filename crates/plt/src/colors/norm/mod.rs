//! Data normalisation: mapping data values onto colormap positions
//! (matplotlib's `Normalize` family).
//!
//! Every norm implements [`Norm`]. Data is passed as `Option<f64>` where
//! `None` is a masked (missing) value; results are [`Normalized`] values
//! ready for [`Colormap::lookup`](super::Colormap::lookup). Unset limits
//! are filled from the first data normalised, like matplotlib.
//!
//! ```
//! use plt::colors::{LogNorm, Norm, Normalized};
//! let mut norm = LogNorm::new(1.0, 100.0);
//! assert_eq!(norm.normalize(&[10.0])?, [Normalized::Fraction(0.5)]);
//! assert_eq!(norm.normalize(&[0.0])?, [Normalized::Masked], "outside the log domain");
//! # Ok::<(), plt::colors::NormError>(())
//! ```

#![allow(
    clippy::float_cmp,
    reason = "matplotlib's norms branch on exact limit comparisons (vmin == vmax)"
)]

#[macro_use]
mod macros;

mod boundary;
mod centered;
mod linear;
mod power;
mod scaled;

use std::fmt;

pub use boundary::BoundaryNorm;
pub use centered::{CenteredNorm, TwoSlopeNorm};
pub use linear::{NoNorm, Normalize};
pub use power::PowerNorm;
pub use scaled::{AsinhNorm, FuncNorm, LogNorm, NormTransform, ScaleNorm, SymLogNorm};

use super::Normalized;
use super::npmath::{nan_max, nan_min, np_clip};
use crate::scale::ScaleError;

/// A mapping from data values to colormap positions (matplotlib's `Norm`).
pub trait Norm: fmt::Debug {
    /// The data value mapped to 0, once known.
    fn vmin(&self) -> Option<f64>;

    /// The data value mapped to 1, once known.
    fn vmax(&self) -> Option<f64>;

    /// Sets (or clears) the lower limit.
    fn set_vmin(&mut self, vmin: Option<f64>);

    /// Sets (or clears) the upper limit.
    fn set_vmax(&mut self, vmax: Option<f64>);

    /// Whether out-of-range values are clipped to the limits by default.
    fn clip(&self) -> bool;

    /// Whether both limits are set.
    fn scaled(&self) -> bool {
        self.vmin().is_some() && self.vmax().is_some()
    }

    /// Sets both limits from `data`, discarding the current ones.
    fn autoscale(&mut self, data: &[Option<f64>]) {
        self.set_vmin(None);
        self.set_vmax(None);
        self.autoscale_none(data);
    }

    /// Fills whichever limits are unset from `data` (masked entries are
    /// ignored; a NaN makes the limit NaN, as numpy's `min`/`max` do).
    fn autoscale_none(&mut self, data: &[Option<f64>]);

    /// Normalises masked data; `clip` overrides [`Norm::clip`].
    fn normalize_masked(
        &mut self,
        data: &[Option<f64>],
        clip: Option<bool>,
    ) -> Result<Vec<Normalized>, NormError>;

    /// Normalises plain data with the norm's own clipping.
    fn normalize(&mut self, values: &[f64]) -> Result<Vec<Normalized>, NormError> {
        let data: Vec<Option<f64>> = values.iter().copied().map(Some).collect();
        self.normalize_masked(&data, None)
    }

    /// Maps normalised values back to data values. Entries matplotlib
    /// would mask come back as `None`.
    fn inverse(&self, values: &[f64]) -> Result<Vec<Option<f64>>, NormError>;
}

/// Why normalisation failed.
#[derive(Clone, Debug, PartialEq)]
pub enum NormError {
    /// `vmin > vmax`.
    InvertedLimits {
        /// Lower limit.
        vmin: f64,
        /// Upper limit.
        vmax: f64,
    },
    /// The operation needs both limits, but some are unset (no data yet).
    Unscaled,
    /// The limits fall outside the norm's transform domain (e.g. a log
    /// norm with `vmin <= 0`).
    InvalidLimits,
    /// A transform parameter is invalid.
    Scale(ScaleError),
    /// A two-slope norm needs `vmin <= vcenter <= vmax` (strictly at
    /// construction).
    CenterOrder,
    /// A boundary norm needs at least two boundaries.
    TooFewBoundaries(usize),
    /// Boundaries must not decrease.
    UnsortedBoundaries,
    /// A boundary norm has more bins than colours.
    TooManyBins {
        /// Bins including extensions.
        bins: usize,
        /// Available colours.
        ncolors: usize,
    },
    /// Clipping and extensions cannot be combined.
    ClipWithExtend,
    /// The norm has no inverse.
    NotInvertible,
}

impl fmt::Display for NormError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvertedLimits { vmin, vmax } => {
                write!(
                    f,
                    "vmin ({vmin}) must be less than or equal to vmax ({vmax})"
                )
            }
            Self::Unscaled => f.write_str("vmin and vmax must both be set"),
            Self::InvalidLimits => f.write_str("invalid vmin or vmax for this norm"),
            Self::Scale(err) => write!(f, "{err}"),
            Self::CenterOrder => f.write_str("vmin, vcenter and vmax must be in ascending order"),
            Self::TooFewBoundaries(n) => {
                write!(f, "at least 2 boundaries (1 region) are needed, got {n}")
            }
            Self::UnsortedBoundaries => f.write_str("boundaries must increase monotonically"),
            Self::TooManyBins { bins, ncolors } => write!(
                f,
                "there are {bins} color bins including extensions, but ncolors = {ncolors}"
            ),
            Self::ClipWithExtend => f.write_str("clip=True is not compatible with extend"),
            Self::NotInvertible => f.write_str("this norm is not invertible"),
        }
    }
}

impl std::error::Error for NormError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Scale(err) => Some(err),
            _ => None,
        }
    }
}

impl From<ScaleError> for NormError {
    fn from(err: ScaleError) -> Self {
        Self::Scale(err)
    }
}

/// The limits and clipping flag most norms share.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Limits {
    vmin: Option<f64>,
    vmax: Option<f64>,
    clip: bool,
}

impl Limits {
    fn new(vmin: Option<f64>, vmax: Option<f64>) -> Self {
        Self {
            vmin,
            vmax,
            clip: false,
        }
    }

    /// matplotlib's `Normalize.autoscale_None`.
    fn autoscale_none(&mut self, data: &[Option<f64>]) {
        if let Some((min, max)) = data_range(data) {
            self.vmin.get_or_insert(min);
            self.vmax.get_or_insert(max);
        }
    }

    /// Both limits, or [`NormError::Unscaled`].
    fn both(&self) -> Result<(f64, f64), NormError> {
        self.vmin.zip(self.vmax).ok_or(NormError::Unscaled)
    }
}

/// numpy's `(min(), max())` over the unmasked entries (NaN propagates),
/// or `None` when nothing is unmasked.
fn data_range(data: &[Option<f64>]) -> Option<(f64, f64)> {
    data.iter().flatten().fold(None, |range, &v| {
        Some(match range {
            None => (v, v),
            Some((lo, hi)) => (nan_min(lo, v), nan_max(hi, v)),
        })
    })
}

/// Every entry 0 (masked entries stay masked): matplotlib's result when
/// `vmin == vmax`.
fn zeros(data: &[Option<f64>]) -> Vec<Normalized> {
    data.iter()
        .map(|v| v.map_or(Normalized::Masked, |_| Normalized::Fraction(0.0)))
        .collect()
}

/// matplotlib's linear `Normalize.__call__` arithmetic for given limits.
fn linear(
    data: &[Option<f64>],
    vmin: Option<f64>,
    vmax: Option<f64>,
    clip: bool,
) -> Result<Vec<Normalized>, NormError> {
    let (lo, hi) = match (vmin, vmax) {
        (None, None) => return Ok(zeros(data)),
        (Some(lo), Some(hi)) => (lo, hi),
        _ => return Err(NormError::Unscaled),
    };
    if lo == hi {
        return Ok(zeros(data));
    }
    if lo > hi {
        return Err(NormError::InvertedLimits { vmin: lo, vmax: hi });
    }
    Ok(map_unmasked(data, |v| {
        let v = if clip { np_clip(v, lo, hi) } else { v };
        Normalized::Fraction((v - lo) / (hi - lo))
    }))
}

/// matplotlib's linear `Normalize.inverse`: `vmin + v * (vmax - vmin)`.
fn linear_inverse(values: &[f64], limits: &Limits) -> Result<Vec<Option<f64>>, NormError> {
    let (lo, hi) = limits.both()?;
    Ok(values.iter().map(|&v| Some(lo + v * (hi - lo))).collect())
}

/// Applies `f` to unmasked entries; masked ones stay masked.
fn map_unmasked(data: &[Option<f64>], f: impl Fn(f64) -> Normalized) -> Vec<Normalized> {
    data.iter()
        .map(|v| v.map_or(Normalized::Masked, &f))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_range_follows_numpy() {
        assert_eq!(
            data_range(&[Some(3.0), None, Some(-1.0)]),
            Some((-1.0, 3.0))
        );
        assert_eq!(data_range(&[None]), None);
        let (lo, hi) = data_range(&[Some(f64::NAN), Some(1.0)]).unwrap();
        assert!(lo.is_nan() && hi.is_nan());
    }

    #[test]
    fn linear_core() {
        let data = [Some(-1.0), Some(0.5), None, Some(f64::NAN)];
        let out = linear(&data, Some(0.0), Some(1.0), true).unwrap();
        assert_eq!(
            &out[..3],
            [
                Normalized::Fraction(0.0),
                Normalized::Fraction(0.5),
                Normalized::Masked
            ]
        );
        assert!(out[3].fraction().unwrap().is_nan());
        assert_eq!(
            linear(&data[..1], Some(2.0), Some(2.0), false).unwrap(),
            [Normalized::Fraction(0.0)]
        );
        assert!(linear(&data, Some(2.0), Some(1.0), false).is_err());
        assert_eq!(linear(&[], None, None, false), Ok(vec![]));
    }
}
