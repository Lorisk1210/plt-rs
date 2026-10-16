//! Norms with a fixed centre (`TwoSlopeNorm`, `CenteredNorm`).

use super::{Limits, Norm, NormError, data_range, linear, linear_inverse, map_unmasked};
use crate::colors::Normalized;
use crate::ticker::pymath::py_max;

/// Maps `vmin → 0`, `vcenter → 0.5` and `vmax → 1` with a different slope
/// on each side (matplotlib's `TwoSlopeNorm`); values beyond the limits
/// extrapolate to `∓∞`.
///
/// ```
/// use plt::colors::{Norm, Normalized::Fraction, TwoSlopeNorm};
/// let mut norm = TwoSlopeNorm::new(0.0, Some(-4000.0), Some(10000.0))?;
/// assert_eq!(norm.normalize(&[-2000.0, 5000.0])?, [Fraction(0.25), Fraction(0.75)]);
/// # Ok::<(), plt::colors::NormError>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TwoSlopeNorm {
    vcenter: f64,
    vmin: Option<f64>,
    vmax: Option<f64>,
}

impl TwoSlopeNorm {
    /// A norm centred on `vcenter`; unset limits autoscale. Given limits
    /// must lie strictly on either side of the centre.
    pub fn new(vcenter: f64, vmin: Option<f64>, vmax: Option<f64>) -> Result<Self, NormError> {
        if vmax.is_some_and(|hi| vcenter >= hi) || vmin.is_some_and(|lo| vcenter <= lo) {
            return Err(NormError::CenterOrder);
        }
        Ok(Self {
            vcenter,
            vmin,
            vmax,
        })
    }

    /// The data value mapped to 0.5.
    pub fn vcenter(&self) -> f64 {
        self.vcenter
    }
}

impl Norm for TwoSlopeNorm {
    fn vmin(&self) -> Option<f64> {
        self.vmin
    }

    fn vmax(&self) -> Option<f64> {
        self.vmax
    }

    fn set_vmin(&mut self, vmin: Option<f64>) {
        self.vmin = vmin;
    }

    fn set_vmax(&mut self, vmax: Option<f64>) {
        self.vmax = vmax;
    }

    /// Two-slope norms never clip.
    fn clip(&self) -> bool {
        false
    }

    /// Fills unset limits from `data`, then moves a limit on the wrong side
    /// of the centre to mirror the other one.
    fn autoscale_none(&mut self, data: &[Option<f64>]) {
        let mut limits = Limits::new(self.vmin, self.vmax);
        limits.autoscale_none(data);
        let center = self.vcenter;
        if let (Some(lo), Some(hi)) = (limits.vmin, limits.vmax) {
            let lo = if lo >= center {
                center - (hi - center)
            } else {
                lo
            };
            let hi = if hi <= center {
                center + (center - lo)
            } else {
                hi
            };
            (self.vmin, self.vmax) = (Some(lo), Some(hi));
        } else {
            (self.vmin, self.vmax) = (limits.vmin, limits.vmax);
        }
    }

    fn normalize_masked(
        &mut self,
        data: &[Option<f64>],
        _clip: Option<bool>,
    ) -> Result<Vec<Normalized>, NormError> {
        self.autoscale_none(data);
        let (lo, hi) = self.vmin.zip(self.vmax).ok_or(NormError::Unscaled)?;
        if !(lo <= self.vcenter && self.vcenter <= hi) {
            return Err(NormError::CenterOrder);
        }
        let xp = [lo, self.vcenter, hi];
        Ok(map_unmasked(data, |v| {
            Normalized::Fraction(np_interp(v, xp, [0.0, 0.5, 1.0]))
        }))
    }

    fn inverse(&self, values: &[f64]) -> Result<Vec<Option<f64>>, NormError> {
        let (lo, hi) = self.vmin.zip(self.vmax).ok_or(NormError::Unscaled)?;
        let fp = [lo, self.vcenter, hi];
        Ok(values
            .iter()
            .map(|&v| Some(np_interp(v, [0.0, 0.5, 1.0], fp)))
            .collect())
    }
}

/// `np.interp(x, xp, fp, left=-inf, right=inf)` for three points.
fn np_interp(x: f64, xp: [f64; 3], fp: [f64; 3]) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x < xp[0] {
        return f64::NEG_INFINITY;
    }
    if x > xp[2] {
        return f64::INFINITY;
    }
    // numpy's linear search for short tables: the last j with xp[j] <= x.
    let j = (1..3).take_while(|&i| x >= xp[i]).last().unwrap_or(0);
    if j == 2 || xp[j] == x {
        return fp[j];
    }
    let slope = (fp[j + 1] - fp[j]) / (xp[j + 1] - xp[j]);
    let y = slope * (x - xp[j]) + fp[j];
    if !y.is_nan() {
        return y;
    }
    let y = slope * (x - xp[j + 1]) + fp[j + 1];
    if y.is_nan() && fp[j] == fp[j + 1] {
        fp[j]
    } else {
        y
    }
}

/// Maps `[vcenter - halfrange, vcenter + halfrange]` linearly onto
/// `[0, 1]` (matplotlib's `CenteredNorm`); an unset half-range autoscales
/// to the largest distance of the data from the centre.
///
/// ```
/// use plt::colors::{CenteredNorm, Norm, Normalized::Fraction};
/// let mut norm = CenteredNorm::new(0.0, Some(4.0));
/// assert_eq!(norm.normalize(&[-2.0, 4.0])?, [Fraction(0.25), Fraction(1.0)]);
/// # Ok::<(), plt::colors::NormError>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CenteredNorm {
    vcenter: f64,
    limits: Limits,
}

impl CenteredNorm {
    /// A norm centred on `vcenter`; `halfrange` (its absolute value) sets
    /// the limits, `None` autoscales.
    pub fn new(vcenter: f64, halfrange: Option<f64>) -> Self {
        let mut norm = Self {
            vcenter,
            limits: Limits::default(),
        };
        norm.set_halfrange(halfrange);
        norm
    }

    /// The same norm, clipping out-of-range values when `clip` is set.
    #[must_use]
    pub fn with_clip(mut self, clip: bool) -> Self {
        self.limits.clip = clip;
        self
    }

    /// The data value mapped to 0.5.
    pub fn vcenter(&self) -> f64 {
        self.vcenter
    }

    /// Half the distance between the limits, once known.
    pub fn halfrange(&self) -> Option<f64> {
        let (lo, hi) = self.limits.both().ok()?;
        Some((hi - lo) / 2.0)
    }

    /// Sets the limits to `vcenter ∓ |halfrange|`, or clears them.
    pub fn set_halfrange(&mut self, halfrange: Option<f64>) {
        match halfrange {
            None => {
                self.set_vmin(None);
                self.set_vmax(None);
            }
            Some(h) => {
                self.set_vmin(Some(self.vcenter - h.abs()));
                self.set_vmax(Some(self.vcenter + h.abs()));
            }
        }
    }
}

/// matplotlib's `!=` between optional limits (NaN differs from itself).
fn differs(a: Option<f64>, b: Option<f64>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a != b,
        (a, b) => a.is_some() != b.is_some(),
    }
}

impl Norm for CenteredNorm {
    fn vmin(&self) -> Option<f64> {
        self.limits.vmin
    }

    fn vmax(&self) -> Option<f64> {
        self.limits.vmax
    }

    /// Sets the lower limit and mirrors the upper one about the centre.
    fn set_vmin(&mut self, vmin: Option<f64>) {
        if differs(vmin, self.limits.vmin) {
            self.limits.vmin = vmin;
            self.limits.vmax = vmin.map(|lo| 2.0 * self.vcenter - lo);
        }
    }

    /// Sets the upper limit and mirrors the lower one about the centre.
    fn set_vmax(&mut self, vmax: Option<f64>) {
        if differs(vmax, self.limits.vmax) {
            self.limits.vmax = vmax;
            self.limits.vmin = vmax.map(|hi| 2.0 * self.vcenter - hi);
        }
    }

    fn clip(&self) -> bool {
        self.limits.clip
    }

    /// Sets the half-range to the data's largest distance from the centre
    /// (no-op for data without unmasked values).
    fn autoscale(&mut self, data: &[Option<f64>]) {
        if let Some((min, max)) = data_range(data) {
            let halfrange = py_max(self.vcenter - min, max - self.vcenter);
            self.set_halfrange(Some(halfrange));
        }
    }

    fn autoscale_none(&mut self, data: &[Option<f64>]) {
        if self.halfrange().is_none() && !data.is_empty() {
            self.autoscale(data);
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
        let limits = self.limits;
        linear(data, limits.vmin, limits.vmax, clip.unwrap_or(limits.clip))
    }

    fn inverse(&self, values: &[f64]) -> Result<Vec<Option<f64>>, NormError> {
        linear_inverse(values, &self.limits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::Normalized::Fraction;

    #[test]
    fn two_slope_validation_and_autoscale() {
        assert_eq!(
            TwoSlopeNorm::new(1.0, Some(0.0), Some(1.0)),
            Err(NormError::CenterOrder)
        );
        let mut norm = TwoSlopeNorm::new(0.0, None, None).unwrap();
        norm.autoscale_none(&[Some(2.0), Some(5.0)]);
        assert_eq!((norm.vmin(), norm.vmax()), (Some(-5.0), Some(5.0)));
        let mut norm = TwoSlopeNorm::new(0.0, Some(-1.0), Some(1.0)).unwrap();
        let out = norm.normalize(&[-2.0, 2.0, 1.0]).unwrap();
        assert_eq!(
            out,
            [
                Fraction(f64::NEG_INFINITY),
                Fraction(f64::INFINITY),
                Fraction(1.0)
            ]
        );
        assert_eq!(
            norm.inverse(&[0.75, 1.5]),
            Ok(vec![Some(0.5), Some(f64::INFINITY)])
        );
        assert_eq!(
            TwoSlopeNorm::new(0.0, None, None).unwrap().inverse(&[0.5]),
            Err(NormError::Unscaled)
        );
    }

    #[test]
    fn centered_limits_mirror() {
        let mut norm = CenteredNorm::new(1.0, Some(-2.0));
        assert_eq!((norm.vmin(), norm.vmax()), (Some(-1.0), Some(3.0)));
        norm.autoscale(&[Some(-2.0), Some(6.0)]);
        assert_eq!((norm.vmin(), norm.vmax()), (Some(-4.0), Some(6.0)));
        let mut auto = CenteredNorm::new(0.0, None);
        assert_eq!(
            auto.normalize(&[-2.0, 4.0]),
            Ok(vec![Fraction(0.25), Fraction(1.0)])
        );
        assert_eq!(auto.halfrange(), Some(4.0));
        assert_eq!(
            CenteredNorm::new(0.0, None).inverse(&[0.5]),
            Err(NormError::Unscaled)
        );
    }

    #[test]
    fn interpolation_edges() {
        let xp = [0.0, 1.0, 2.0];
        let fp = [0.0, 0.5, 1.0];
        assert_eq!(np_interp(0.5, xp, fp), 0.25);
        assert_eq!(np_interp(2.0, xp, fp), 1.0);
        assert_eq!(np_interp(1.0, xp, fp), 0.5);
        assert!(np_interp(f64::NAN, xp, fp).is_nan());
        assert_eq!(np_interp(-1.0, xp, fp), f64::NEG_INFINITY);
    }
}
