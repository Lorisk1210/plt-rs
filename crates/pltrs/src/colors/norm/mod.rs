//! Data normalisation: mapping data values onto colormap positions
//! (matplotlib's `Normalize` family).
//!
//! Every norm implements [`Norm`]. Data is passed as `Option<f64>` where
//! `None` is a masked (missing) value; results are [`Normalized`] values
//! ready for [`Colormap::lookup`](super::Colormap::lookup). Unset limits
//! are filled from the first data normalised, like matplotlib.
//!
//! ```
//! use pltrs::colors::{LogNorm, Norm, Normalized};
//! let mut norm = LogNorm::new(1.0, 100.0);
//! assert_eq!(norm.normalize(&[10.0])?, [Normalized::Fraction(0.5)]);
//! assert_eq!(norm.normalize(&[0.0])?, [Normalized::Masked], "outside the log domain");
//! # Ok::<(), pltrs::colors::NormError>(())
//! ```

#![expect(
    clippy::float_cmp,
    reason = "matplotlib's norms branch on exact limit comparisons (vmin == vmax)"
)]

#[macro_use]
mod macros;

mod boundary;
mod centered;
mod linear;
mod multi;
mod power;
mod scaled;

use std::fmt;

pub use boundary::BoundaryNorm;
pub use centered::{CenteredNorm, TwoSlopeNorm};
pub use linear::{NoNorm, Normalize};
pub use multi::{MultiNorm, NormName, NormSpec};
pub use power::PowerNorm;
pub(crate) use scaled::LinearTransform;
pub use scaled::{AsinhNorm, FuncNorm, LogNorm, NormTransform, ScaleNorm, SymLogNorm};

use pltrs_core::numeric::np_clip;

use super::Normalized;
use crate::scale::{Scale, ScaleError};

/// The family a norm belongs to, for code that treats norm classes
/// differently (matplotlib's `isinstance` checks and a norm's `_scale`),
/// such as colorbars choosing their axis scale and ticks.
#[derive(Debug)]
pub enum NormKind<'a> {
    /// Exactly the linear [`Normalize`].
    Linear,
    /// A [`BoundaryNorm`] binning data into colour indices.
    Boundary(&'a BoundaryNorm),
    /// The identity [`NoNorm`].
    Identity,
    /// A norm made from a scale ([`LogNorm`], [`SymLogNorm`],
    /// [`AsinhNorm`], [`FuncNorm`]): axes showing it use that scale.
    Scaled(Box<dyn Scale>),
    /// Any other norm.
    Other,
}

/// Boxed copies of norms, implemented for every norm that is `Clone`.
pub trait NormClone {
    /// A boxed copy of this norm.
    fn clone_box(&self) -> Box<dyn Norm>;
}

impl<T: Norm + Clone + 'static> NormClone for T {
    fn clone_box(&self) -> Box<dyn Norm> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn Norm> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// A mapping from data values to colormap positions (matplotlib's `Norm`).
pub trait Norm: fmt::Debug + NormClone + std::any::Any {
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

    /// Sets whether out-of-range values are clipped by default
    /// (matplotlib's `clip` attribute, which is not validated).
    fn set_clip(&mut self, clip: bool);

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

    /// [`autoscale_none`](Self::autoscale_none) of `values` without the
    /// entries `mask` flags. Norms that only need the range of the data
    /// find it without building the masked data.
    fn autoscale_none_unmasked(&mut self, values: &[f64], mask: &[bool]) {
        let data: Vec<Option<f64>> = values
            .iter()
            .zip(mask)
            .map(|(&v, &masked)| (!masked).then_some(v))
            .collect();
        self.autoscale_none(&data);
    }

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

    /// [`normalize_masked`](Self::normalize_masked) of data handed over
    /// as the array `form` says, as matplotlib hands it: a mappable's
    /// array or a mesh's coordinates as masked arrays, a colorbar's
    /// boundaries or an axis' values as plain ones. Only a norm running
    /// the program's code sees the array: the default normalises the
    /// values as `normalize_masked` does.
    fn normalize_shaped(
        &mut self,
        data: &[Option<f64>],
        clip: Option<bool>,
        form: ArrayForm<'_>,
    ) -> Result<Vec<Normalized>, NormError> {
        let _ = form;
        self.normalize_masked(data, clip)
    }

    /// Normalises one value handed over alone, as matplotlib's colorbar
    /// normalises the value of an extension (a scalar, not an array).
    /// Only a norm running the program's code sees the difference.
    fn normalize_scalar(&mut self, value: f64) -> Result<Normalized, NormError> {
        let normalized = self.normalize(&[value])?;
        Ok(normalized.into_iter().next().unwrap_or(Normalized::Masked))
    }

    /// Maps normalised values back to data values. Entries matplotlib
    /// would mask come back as `None`.
    fn inverse(&self, values: &[f64]) -> Result<Vec<Option<f64>>, NormError>;

    /// Whether the norm hands data through unchanged, so that integer
    /// data indexes the colormap directly (matplotlib's `NoNorm`).
    fn is_identity(&self) -> bool {
        matches!(self.kind(), NormKind::Identity)
    }

    /// The family of the norm ([`NormKind::Other`] unless overridden).
    fn kind(&self) -> NormKind<'_> {
        NormKind::Other
    }

    /// Whether the norm runs code of the program's that observes its
    /// calls (a host language's norm): the colorbar axes it scales are
    /// then ticked as matplotlib ticks them (see
    /// [`TickUpdater`](crate::axes::TickUpdater)). `false` by default.
    fn calls_program(&self) -> bool {
        false
    }
}

/// How values are handed to a norm of the program's (see
/// [`Norm::normalize_shaped`]): as an array of `shape`, their order
/// row-major, masked or plain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArrayForm<'a> {
    /// The shape of the array.
    pub shape: &'a [usize],
    /// Whether it is a masked array (its masked values those of the data).
    pub masked: bool,
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
    /// No scale of this name makes a norm.
    UnknownName(String),
    /// The scale needs forward and inverse functions, which a name alone
    /// does not give (`"function"`, `"functionlog"`).
    NeedsFunctions(String),
    /// A [`MultiNorm`] needs at least one norm.
    NoNorms,
    /// A [`MultiNorm`] input does not have one entry per norm.
    ComponentCount {
        /// What was given (`"vmin"`, `"vmax"`, `"clip"` or `"data"`).
        what: &'static str,
        /// The number of norms.
        expected: usize,
        /// The number of entries given.
        given: usize,
    },
}

impl fmt::Display for NormError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // matplotlib's message, which names neither limit.
            Self::InvertedLimits { .. } => {
                f.write_str("minvalue must be less than or equal to maxvalue")
            }
            Self::Unscaled => f.write_str("vmin and vmax must both be set"),
            Self::InvalidLimits => f.write_str("Invalid vmin or vmax"),
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
            Self::UnknownName(name) => write!(f, "{name:?} is not a valid value for norm"),
            Self::NeedsFunctions(name) => write!(
                f,
                "the {name:?} scale needs forward and inverse functions to make a norm"
            ),
            Self::NoNorms => f.write_str("a MultiNorm needs at least one norm"),
            Self::ComponentCount {
                what,
                expected,
                given,
            } => write!(
                f,
                "{what} must have one component for each of the {expected} norms, \
                 got {given}"
            ),
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

pltrs_core::archive_struct!(Limits { vmin, vmax, clip });

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
        self.fill_from(data_range(data.iter().flatten()));
    }

    /// [`autoscale_none`](Self::autoscale_none) of the unmasked `values`.
    fn autoscale_none_unmasked(&mut self, values: &[f64], mask: &[bool]) {
        let unmasked = values
            .iter()
            .zip(mask)
            .filter_map(|(v, &masked)| (!masked).then_some(v));
        self.fill_from(data_range(unmasked));
    }

    /// Sets the unset limits to the data's `range`, if there is one.
    fn fill_from(&mut self, range: Option<(f64, f64)>) {
        if let Some((min, max)) = range {
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
/// or `None` when nothing is unmasked. Any NaN makes both NaN, so the
/// running extremes skip the NaN checks numpy's reduction makes at every
/// step (`np_minimum` and `np_maximum` otherwise reduce to `min` and `max`).
fn data_range<'a>(mut values: impl Iterator<Item = &'a f64>) -> Option<(f64, f64)> {
    let first = *values.next()?;
    let (mut lo, mut hi, mut nan) = (first, first, first.is_nan());
    for &v in values {
        nan |= v.is_nan();
        lo = lo.min(v);
        hi = hi.max(v);
    }
    Some(if nan { (f64::NAN, f64::NAN) } else { (lo, hi) })
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

pltrs_core::archive_kinds!(dyn Norm {
    "linear" => Normalize,
    "none" => NoNorm,
    "boundary" => BoundaryNorm,
    "two-slope" => TwoSlopeNorm,
    "centered" => CenteredNorm,
    "power" => PowerNorm,
    "log" => ScaleNorm<crate::scale::LogTransform>,
    "symlog" => ScaleNorm<crate::scale::SymmetricalLogTransform>,
    "asinh" => ScaleNorm<crate::scale::AsinhTransform>,
    "logit" => ScaleNorm<crate::scale::LogitTransform>,
    "func" => ScaleNorm<crate::scale::FuncTransform>,
    "scaled-linear" => ScaleNorm<LinearTransform>,
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_range_follows_numpy() {
        assert_eq!(data_range([3.0, -1.0].iter()), Some((-1.0, 3.0)));
        assert_eq!(data_range([].iter()), None);
        let (lo, hi) = data_range([f64::NAN, 1.0].iter()).unwrap();
        assert!(lo.is_nan() && hi.is_nan());
    }

    #[test]
    fn unmasked_autoscaling_matches_masked_data() {
        let values = [3.0, 7.0, -1.0, 2.0];
        let mask = [false, true, false, false];
        let data = [Some(3.0), None, Some(-1.0), Some(2.0)];
        let mut direct = Limits::new(None, Some(5.0));
        direct.autoscale_none_unmasked(&values, &mask);
        let mut masked = Limits::new(None, Some(5.0));
        masked.autoscale_none(&data);
        assert_eq!(direct, masked);
        assert_eq!((direct.vmin, direct.vmax), (Some(-1.0), Some(5.0)));
        let mut nothing = Limits::new(None, None);
        nothing.autoscale_none_unmasked(&values, &[true; 4]);
        assert_eq!((nothing.vmin, nothing.vmax), (None, None));
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
