//! Power-law normalisation (`PowerNorm`).

use super::{Limits, Norm, NormError, map_unmasked, np_clip, zeros};
use crate::colors::Normalized;

/// Maps `[vmin, vmax]` linearly onto `[0, 1]` and then raises positive
/// results to `gamma` (matplotlib's `PowerNorm`); values below `vmin` stay
/// linear.
///
/// ```
/// use pltrs::colors::{Norm, Normalized::Fraction, PowerNorm};
/// let mut norm = PowerNorm::new(2.0).with_vmin(0.0).with_vmax(2.0);
/// assert_eq!(norm.normalize(&[1.0, -1.0])?, [Fraction(0.25), Fraction(-0.5)]);
/// # Ok::<(), pltrs::colors::NormError>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PowerNorm {
    gamma: f64,
    limits: Limits,
}

impl PowerNorm {
    /// An autoscaling power norm with exponent `gamma`.
    pub fn new(gamma: f64) -> Self {
        Self {
            gamma,
            limits: Limits::default(),
        }
    }

    /// The exponent.
    pub fn gamma(&self) -> f64 {
        self.gamma
    }
}

limit_builders!(PowerNorm);

impl Norm for PowerNorm {
    limit_accessors!();

    fn autoscale_none(&mut self, data: &[Option<f64>]) {
        self.limits.autoscale_none(data);
    }

    fn normalize_masked(
        &mut self,
        data: &[Option<f64>],
        clip: Option<bool>,
    ) -> Result<Vec<Normalized>, NormError> {
        self.autoscale_none(data);
        let (lo, hi) = self.limits.both()?;
        if lo > hi {
            return Err(NormError::InvertedLimits { vmin: lo, vmax: hi });
        }
        if lo == hi {
            return Ok(zeros(data));
        }
        let clip = clip.unwrap_or(self.limits.clip);
        Ok(map_unmasked(data, |v| {
            let v = if clip { np_clip(v, lo, hi) } else { v };
            let t = (v - lo) / (hi - lo);
            Normalized::Fraction(if t > 0.0 { t.powf(self.gamma) } else { t })
        }))
    }

    fn inverse(&self, values: &[f64]) -> Result<Vec<Option<f64>>, NormError> {
        let (lo, hi) = self.limits.both()?;
        Ok(values
            .iter()
            .map(|&v| {
                let t = if v > 0.0 { v.powf(1.0 / self.gamma) } else { v };
                Some(t * (hi - lo) + lo)
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::Normalized::Fraction;

    #[test]
    fn power_law_and_inverse() {
        let mut norm = PowerNorm::new(0.5).with_vmin(1.0).with_vmax(4.0);
        assert_eq!(norm.normalize(&[1.75]), Ok(vec![Fraction(0.5)]));
        assert_eq!(norm.inverse(&[0.5, -0.5]), Ok(vec![Some(1.75), Some(-0.5)]));
        let mut clipped = norm.with_clip(true);
        assert_eq!(
            clipped.normalize(&[9.0, 0.0]),
            Ok(vec![Fraction(1.0), Fraction(0.0)])
        );
    }

    #[test]
    fn unscaled_and_degenerate() {
        assert_eq!(PowerNorm::new(2.0).normalize(&[]), Err(NormError::Unscaled));
        assert_eq!(
            PowerNorm::new(2.0).inverse(&[0.5]),
            Err(NormError::Unscaled)
        );
        let mut flat = PowerNorm::new(2.0).with_vmin(1.0).with_vmax(1.0);
        assert_eq!(flat.normalize(&[2.0]), Ok(vec![Fraction(0.0)]));
        let mut auto = PowerNorm::new(2.0);
        assert_eq!(
            auto.normalize(&[1.0, 3.0, 2.0]),
            Ok(vec![Fraction(0.0), Fraction(1.0), Fraction(0.25)])
        );
    }
}
