//! Norms built on a scale transform (`LogNorm`, `SymLogNorm`, `AsinhNorm`,
//! `FuncNorm`): matplotlib's `make_norm_from_scale`.

use pltrs_core::numeric::np_clip;

use super::{Limits, Norm, NormError, NormKind, map_unmasked, zeros};
use crate::colors::Normalized;
use crate::scale::{
    AsinhScale, AsinhTransform, FuncScale, FuncTransform, LinearScale, LogScale, LogTransform,
    LogitScale, LogitTransform, NonPositive, Scale, SymmetricalLogScale, SymmetricalLogTransform,
};
use crate::ticker::LogBase;
use crate::transforms::AxisMap;

/// A scale transform a [`ScaleNorm`] can normalise through.
pub trait NormTransform: AxisMap + Clone + 'static {
    /// The inverse of one value, or `None` where matplotlib's masked-array
    /// arithmetic masks it (transforms that divide their input mask
    /// non-finite values).
    fn inverse_masked(&self, value: f64) -> Option<f64> {
        Some(self.inverse(value))
    }

    /// The scale the norm was made from (matplotlib's `norm._scale`).
    fn scale(&self) -> Box<dyn Scale>;
}

impl NormTransform for LogTransform {
    fn scale(&self) -> Box<dyn Scale> {
        Box::new(LogScale::from_base(self.log_base()).with_nonpositive(self.nonpositive()))
    }
}

/// The identity map of the linear scale: a norm made from it is linear
/// but, unlike [`Normalize`](super::Normalize), masks NaN and infinite
/// values and leaves them out of autoscaling (matplotlib's norm for the
/// scale name `"linear"`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct LinearTransform;

pltrs_core::archive_struct!(LinearTransform);

impl AxisMap for LinearTransform {
    fn forward(&self, value: f64) -> f64 {
        value
    }

    fn inverse(&self, value: f64) -> f64 {
        value
    }
}

impl NormTransform for LinearTransform {
    fn scale(&self) -> Box<dyn Scale> {
        Box::new(LinearScale)
    }
}

impl NormTransform for LogitTransform {
    fn scale(&self) -> Box<dyn Scale> {
        Box::new(LogitScale::new(self.nonpositive()))
    }
}

impl NormTransform for FuncTransform {
    fn scale(&self) -> Box<dyn Scale> {
        Box::new(FuncScale::from_transform(self.clone()))
    }
}

impl NormTransform for SymmetricalLogTransform {
    fn inverse_masked(&self, value: f64) -> Option<f64> {
        value.is_finite().then(|| self.inverse(value))
    }

    fn scale(&self) -> Box<dyn Scale> {
        let scale = SymmetricalLogScale::new(self.base(), self.linthresh(), self.linscale())
            .expect("the transform has valid parameters");
        Box::new(scale)
    }
}

impl NormTransform for AsinhTransform {
    fn inverse_masked(&self, value: f64) -> Option<f64> {
        value.is_finite().then(|| self.inverse(value))
    }

    fn scale(&self) -> Box<dyn Scale> {
        let scale =
            AsinhScale::new(self.linear_width(), 10.0).expect("the transform has a valid width");
        Box::new(scale)
    }
}

/// Normalises in a transformed space: `(T(x) - T(vmin)) / (T(vmax) -
/// T(vmin))`. Results that are not finite (outside the transform's domain)
/// are masked.
#[derive(Clone, Debug)]
pub struct ScaleNorm<T> {
    transform: T,
    limits: Limits,
}

pltrs_core::archive_struct!(ScaleNorm<T> { transform, limits });

/// Logarithmic normalisation (base 10); non-positive values are masked.
///
/// ```
/// use pltrs::colors::{LogNorm, Norm, Normalized::Fraction};
/// assert_eq!(LogNorm::new(1.0, 100.0).normalize(&[10.0])?, [Fraction(0.5)]);
/// # Ok::<(), pltrs::colors::NormError>(())
/// ```
pub type LogNorm = ScaleNorm<LogTransform>;

/// Symmetric logarithmic normalisation, linear within `±linthresh`.
pub type SymLogNorm = ScaleNorm<SymmetricalLogTransform>;

/// Inverse-hyperbolic-sine normalisation, linear near zero.
pub type AsinhNorm = ScaleNorm<AsinhTransform>;

/// Normalisation through a user-supplied monotonic function and its
/// inverse.
pub type FuncNorm = ScaleNorm<FuncTransform>;

impl<T: NormTransform> ScaleNorm<T> {
    /// An autoscaling norm through `transform`.
    pub fn from_transform(transform: T) -> Self {
        Self {
            transform,
            limits: Limits::default(),
        }
    }

    /// The transform.
    pub fn transform(&self) -> &T {
        &self.transform
    }

    /// `T(vmin)` and `T(vmax)`, which must be finite.
    fn transformed_limits(&self, lo: f64, hi: f64) -> Result<(f64, f64), NormError> {
        let (t_lo, t_hi) = (self.transform.forward(lo), self.transform.forward(hi));
        if t_lo.is_finite() && t_hi.is_finite() {
            Ok((t_lo, t_hi))
        } else {
            Err(NormError::InvalidLimits)
        }
    }

    /// Both limits, checked to be in order.
    fn ordered_limits(&self) -> Result<(f64, f64), NormError> {
        let (lo, hi) = self.limits.both()?;
        if lo > hi {
            Err(NormError::InvertedLimits { vmin: lo, vmax: hi })
        } else {
            Ok((lo, hi))
        }
    }
}

limit_builders!(for<T: NormTransform> ScaleNorm<T>);

impl LogNorm {
    /// A base-10 log norm with both limits set; [`LogNorm::default`]
    /// autoscales.
    pub fn new(vmin: f64, vmax: f64) -> Self {
        Self::default().with_vmin(vmin).with_vmax(vmax)
    }
}

impl Default for LogNorm {
    fn default() -> Self {
        Self::from_transform(LogTransform::from_base(LogBase::TEN, NonPositive::Mask))
    }
}

impl SymLogNorm {
    /// A symlog norm: linear within `±linthresh`, where the linear part
    /// spans `linscale` decades of the logarithmic part in `base`.
    pub fn new(linthresh: f64, linscale: f64, base: f64) -> Result<Self, NormError> {
        let transform = SymmetricalLogTransform::new(base, linthresh, linscale)?;
        Ok(Self::from_transform(transform))
    }
}

impl AsinhNorm {
    /// An asinh norm, quasi-linear within about `±linear_width`.
    pub fn new(linear_width: f64) -> Result<Self, NormError> {
        Ok(Self::from_transform(AsinhTransform::new(linear_width)?))
    }
}

impl FuncNorm {
    /// A norm through `forward` (monotonic) and its `inverse`, which run
    /// while figures are drawn: they are `Send`, so they cannot use a
    /// figure's handles.
    pub fn new(
        forward: impl Fn(f64) -> f64 + Send + 'static,
        inverse: impl Fn(f64) -> f64 + Send + 'static,
    ) -> Self {
        Self::from_transform(FuncTransform::new(forward, inverse))
    }
}

impl<T: NormTransform> Norm for ScaleNorm<T> {
    limit_accessors!();

    /// Fills unset limits from the values the transform maps to finite
    /// numbers; like matplotlib, data with no such values sets them to 0.
    fn autoscale_none(&mut self, data: &[Option<f64>]) {
        let in_domain: Vec<Option<f64>> = data
            .iter()
            .flatten()
            .filter(|&&v| self.transform.forward(v).is_finite())
            .map(|&v| Some(v))
            .collect();
        if in_domain.is_empty() {
            self.limits.autoscale_none(&[Some(0.0)]);
        } else {
            self.limits.autoscale_none(&in_domain);
        }
    }

    fn normalize_masked(
        &mut self,
        data: &[Option<f64>],
        clip: Option<bool>,
    ) -> Result<Vec<Normalized>, NormError> {
        if !self.scaled() {
            self.autoscale_none(data);
        }
        let (lo, hi) = self.ordered_limits()?;
        if lo == hi {
            return Ok(zeros(data));
        }
        let clip = clip.unwrap_or(self.limits.clip);
        let (t_lo, t_hi) = self.transformed_limits(lo, hi)?;
        Ok(map_unmasked(data, |v| {
            let v = if clip { np_clip(v, lo, hi) } else { v };
            let t = (self.transform.forward(v) - t_lo) / (t_hi - t_lo);
            if t.is_finite() {
                Normalized::Fraction(t)
            } else {
                Normalized::Masked
            }
        }))
    }

    fn inverse(&self, values: &[f64]) -> Result<Vec<Option<f64>>, NormError> {
        let (lo, hi) = self.ordered_limits()?;
        let (t_lo, t_hi) = self.transformed_limits(lo, hi)?;
        Ok(values
            .iter()
            .map(|&v| self.transform.inverse_masked(v * (t_hi - t_lo) + t_lo))
            .collect())
    }

    fn kind(&self) -> NormKind<'_> {
        NormKind::Scaled(self.transform.scale())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::Normalized::{Fraction, Masked};

    #[test]
    fn log_domain_and_limits() {
        let mut norm = LogNorm::new(1.0, 100.0);
        assert_eq!(
            norm.normalize(&[0.0, -1.0, 1.0, f64::NAN]),
            Ok(vec![Masked, Masked, Fraction(0.0), Masked])
        );
        let mut clipped = LogNorm::new(1.0, 100.0).with_clip(true);
        assert_eq!(
            clipped.normalize(&[0.5, 1e3]),
            Ok(vec![Fraction(0.0), Fraction(1.0)])
        );
        assert_eq!(
            LogNorm::new(0.0, 10.0).normalize(&[1.0]),
            Err(NormError::InvalidLimits)
        );
        assert!(matches!(
            LogNorm::new(10.0, 1.0).normalize(&[1.0]),
            Err(NormError::InvertedLimits { .. })
        ));
        assert!(LogNorm::new(10.0, 10.0).normalize(&[100.0]).is_ok());
    }

    #[test]
    fn log_autoscale_uses_the_domain() {
        let mut norm = LogNorm::default();
        norm.autoscale_none(&[
            Some(-1.0),
            Some(0.0),
            Some(0.5),
            Some(20.0),
            Some(f64::INFINITY),
        ]);
        assert_eq!((norm.vmin(), norm.vmax()), (Some(0.5), Some(20.0)));
        let mut empty = LogNorm::default();
        empty.autoscale_none(&[Some(-1.0)]);
        assert_eq!((empty.vmin(), empty.vmax()), (Some(0.0), Some(0.0)));
    }

    #[test]
    fn symlog_asinh_and_inverse_masking() {
        let norm = SymLogNorm::new(1.0, 1.0, 10.0)
            .unwrap()
            .with_vmin(-100.0)
            .with_vmax(100.0);
        assert_eq!(norm.clone().normalize(&[0.0]), Ok(vec![Fraction(0.5)]));
        assert_eq!(norm.inverse(&[0.5, f64::NAN]), Ok(vec![Some(0.0), None]));
        assert!(SymLogNorm::new(0.0, 1.0, 10.0).is_err());
        assert!(AsinhNorm::new(-1.0).is_err());
        let log = LogNorm::new(1.0, 100.0);
        assert!(log.inverse(&[f64::NAN]).unwrap()[0].unwrap().is_nan());
    }

    #[test]
    fn function_norm() {
        let mut norm = FuncNorm::new(f64::sqrt, |y| y * y)
            .with_vmin(0.0)
            .with_vmax(16.0);
        assert_eq!(norm.normalize(&[4.0]), Ok(vec![Fraction(0.5)]));
        assert_eq!(norm.inverse(&[0.5]), Ok(vec![Some(4.0)]));
    }
}
