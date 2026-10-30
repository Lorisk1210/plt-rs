//! Linear normalisation (`Normalize`) and the identity (`NoNorm`).

use super::{Limits, Norm, NormError, NormKind, linear, linear_inverse, map_unmasked};
use crate::colors::Normalized;

/// Maps `[vmin, vmax]` linearly onto `[0, 1]` (matplotlib's `Normalize`).
///
/// ```
/// use pltrs::colors::{Norm, Normalize, Normalized::Fraction};
/// let mut norm = Normalize::new(-1.0, 1.0);
/// assert_eq!(norm.normalize(&[-2.0, 0.0, 2.0])?, [Fraction(-0.5), Fraction(0.5), Fraction(1.5)]);
/// let mut norm = norm.with_clip(true);
/// assert_eq!(norm.normalize(&[2.0])?, [Fraction(1.0)]);
/// # Ok::<(), pltrs::colors::NormError>(())
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Normalize {
    limits: Limits,
}

pltrs_core::archive_struct!(Normalize { limits });

impl Normalize {
    /// A norm with both limits set; [`Normalize::default`] autoscales.
    pub fn new(vmin: f64, vmax: f64) -> Self {
        Self {
            limits: Limits::new(Some(vmin), Some(vmax)),
        }
    }
}

limit_builders!(Normalize);

impl Norm for Normalize {
    limit_accessors!();

    fn autoscale_none(&mut self, data: &[Option<f64>]) {
        self.limits.autoscale_none(data);
    }

    fn autoscale_none_unmasked(&mut self, values: &[f64], mask: &[bool]) {
        self.limits.autoscale_none_unmasked(values, mask);
    }

    fn normalize_masked(
        &mut self,
        data: &[Option<f64>],
        clip: Option<bool>,
    ) -> Result<Vec<Normalized>, NormError> {
        if !self.scaled() {
            self.autoscale_none(data);
        }
        let limits = self.limits;
        linear(data, limits.vmin, limits.vmax, clip.unwrap_or(limits.clip))
    }

    fn inverse(&self, values: &[f64]) -> Result<Vec<Option<f64>>, NormError> {
        linear_inverse(values, &self.limits)
    }

    fn kind(&self) -> NormKind<'_> {
        NormKind::Linear
    }
}

/// Passes values through unchanged (matplotlib's `NoNorm`), for data that
/// already holds colormap positions.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NoNorm {
    limits: Limits,
}

pltrs_core::archive_struct!(NoNorm { limits });

limit_builders!(NoNorm);

impl Norm for NoNorm {
    limit_accessors!();

    fn autoscale_none(&mut self, data: &[Option<f64>]) {
        self.limits.autoscale_none(data);
    }

    fn normalize_masked(
        &mut self,
        data: &[Option<f64>],
        _clip: Option<bool>,
    ) -> Result<Vec<Normalized>, NormError> {
        Ok(map_unmasked(data, Normalized::Fraction))
    }

    fn inverse(&self, values: &[f64]) -> Result<Vec<Option<f64>>, NormError> {
        Ok(values.iter().copied().map(Some).collect())
    }

    fn kind(&self) -> NormKind<'_> {
        NormKind::Identity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::Normalized::{Fraction, Masked};

    #[test]
    fn autoscales_from_first_data() {
        let mut norm = Normalize::default();
        assert_eq!(
            norm.normalize_masked(&[Some(0.0), Some(100.0), None, Some(5.0)], None),
            Ok(vec![Fraction(0.0), Fraction(1.0), Masked, Fraction(0.05)])
        );
        assert_eq!((norm.vmin(), norm.vmax()), (Some(0.0), Some(100.0)));
        let mut half = Normalize::default().with_vmin(0.0);
        half.normalize(&[-1.0, 2.0, 4.0]).unwrap();
        assert_eq!(half.vmax(), Some(4.0));
    }

    #[test]
    fn degenerate_limits_and_inverse() {
        assert_eq!(
            Normalize::new(2.0, 2.0).normalize(&[1.0, 3.0]),
            Ok(vec![Fraction(0.0), Fraction(0.0)])
        );
        assert!(Normalize::new(2.0, 1.0).normalize(&[1.5]).is_err());
        assert_eq!(
            Normalize::new(0.0, 4.0).inverse(&[0.5]),
            Ok(vec![Some(2.0)])
        );
        assert_eq!(
            Normalize::default().inverse(&[0.5]),
            Err(NormError::Unscaled)
        );
        let mut norm = Normalize::new(-1.0, 1.0).with_clip(true);
        assert_eq!(
            norm.normalize_masked(&[Some(2.0)], Some(false)),
            Ok(vec![Fraction(1.5)])
        );
    }

    #[test]
    fn no_norm_is_identity() {
        let mut norm = NoNorm::default();
        assert_eq!(
            norm.normalize(&[5.0, -3.0]),
            Ok(vec![Fraction(5.0), Fraction(-3.0)])
        );
        assert_eq!(norm.inverse(&[3.0]), Ok(vec![Some(3.0)]));
        assert_eq!(norm.vmin(), None);
        assert!(norm.is_identity() && !Normalize::default().is_identity());
    }
}
