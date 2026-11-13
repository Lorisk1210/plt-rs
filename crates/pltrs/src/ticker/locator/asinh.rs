//! `AsinhLocator`: ticks evenly spaced on an inverse-sinh axis.

use pltrs_core::numeric::{Ufunc, np_arcsinh, np_sign, pow, py_max};

use super::Locator;
use crate::ticker::pymath::{linspace, pow10, warn_divide};
use crate::ticker::{TickContext, TickerError};

/// Ticks spaced evenly in `asinh` space and rounded to "nice" multiples of
/// powers of `base` (matplotlib's `AsinhLocator`).
#[derive(Clone, Debug, PartialEq)]
pub struct AsinhLocator {
    linear_width: f64,
    numticks: usize,
    symthresh: f64,
    base: f64,
    subs: Option<Vec<f64>>,
}

pltrs_core::archive_struct!(AsinhLocator {
    linear_width,
    numticks,
    symthresh,
    base,
    subs
});

impl AsinhLocator {
    /// Ticks for an asinh axis whose quasi-linear region has width
    /// `linear_width`; 11 ticks, base 10, no sub-multiples.
    pub fn new(linear_width: f64) -> Self {
        Self {
            linear_width,
            numticks: 11,
            symthresh: 0.2,
            base: 10.0,
            subs: None,
        }
    }

    /// The approximate number of ticks across the axis.
    #[must_use]
    pub fn with_numticks(mut self, numticks: usize) -> Self {
        self.numticks = numticks;
        self
    }

    /// Views that are symmetric about zero up to this fraction get exactly
    /// symmetric ticks.
    #[must_use]
    pub fn with_symthresh(mut self, symthresh: f64) -> Self {
        self.symthresh = symthresh;
        self
    }

    /// Rounds tick values to powers of `base`; a base `<= 1` rounds to
    /// integer multiples of powers of ten instead.
    #[must_use]
    pub fn with_base(mut self, base: f64) -> Self {
        self.base = base;
        self
    }

    /// Multiples of each power (used for minor ticks), in an array or a
    /// `Vec`: `[2.0, 5.0]`. An empty list means none, as by default.
    ///
    /// ```
    /// use pltrs::ticker::AsinhLocator;
    /// let minor = AsinhLocator::new(1.0).with_subs([2.0, 5.0]);
    /// assert_eq!(minor.subs(), Some(&[2.0, 5.0][..]));
    /// assert_eq!(minor.with_subs([]).subs(), None);
    /// ```
    #[must_use]
    pub fn with_subs(mut self, subs: impl Into<Vec<f64>>) -> Self {
        self.subs = Some(subs.into()).filter(|s| !s.is_empty());
        self
    }

    /// The width of the quasi-linear region.
    pub fn linear_width(&self) -> f64 {
        self.linear_width
    }

    /// The approximate number of ticks.
    pub fn numticks(&self) -> usize {
        self.numticks
    }

    /// The symmetry threshold.
    pub fn symthresh(&self) -> f64 {
        self.symthresh
    }

    /// The rounding base.
    pub fn base(&self) -> f64 {
        self.base
    }

    /// The multiples of each power, if any.
    pub fn subs(&self) -> Option<&[f64]> {
        self.subs.as_deref()
    }

    /// Rounds data-space positions to nice values. The logarithms and
    /// powers are numpy's own where numpy is in the process: their last
    /// bit decides which power a position is rounded to.
    fn round_ticks(&self, xs: &[f64], ys: &[f64]) -> Vec<f64> {
        let sizes: Vec<f64> = xs.iter().map(|x| x.abs()).collect();
        if self.base > 1.0 {
            let log_base = self.base.ln();
            let exponents: Vec<f64> = Ufunc::Log
                .map(&sizes, f64::ln)
                .into_iter()
                .map(|log| (log / log_base).floor())
                .collect();
            let powers = Ufunc::PowerOf(self.base).map(&exponents, |e| pow(self.base, e));
            let pows = xs.iter().zip(powers).map(|(&x, power)| np_sign(x) * power);
            match &self.subs {
                Some(subs) => pows.flat_map(|p| subs.iter().map(move |s| p * s)).collect(),
                None => pows.collect(),
            }
        } else {
            let exponents: Vec<f64> = Ufunc::Log10
                .map(&sizes, f64::log10)
                .into_iter()
                .map(f64::floor)
                .collect();
            let powers = Ufunc::PowerOf(10.0).map(&exponents, pow10);
            xs.iter()
                .zip(ys)
                .zip(powers)
                .map(|((&x, &y), power)| {
                    let power = if y == 0.0 { 1.0 } else { power };
                    power * (x / power).round_ties_even()
                })
                .collect()
        }
    }
}

impl Locator for AsinhLocator {
    fn tick_values(
        &self,
        vmin: f64,
        vmax: f64,
        _: &TickContext<'_>,
    ) -> Result<Vec<f64>, TickerError> {
        let width = self.linear_width;
        let ends = Ufunc::Arcsinh.map(&[vmin / width, vmax / width], np_arcsinh);
        let (ymin, ymax) = (width * ends[0], width * ends[1]);
        let mut ys = linspace(ymin, ymax, self.numticks)?;
        // matplotlib divides the ticks by the span first, whatever it is.
        warn_divide(&ys, ymax - ymin);
        if ymin * ymax < 0.0 {
            // Make sure zero is a tick when the axis straddles it.
            let threshold = 0.5 / self.numticks as f64;
            ys.retain(|y| (y / (ymax - ymin)).abs() > threshold);
            ys.push(0.0);
        }
        let scaled: Vec<f64> = ys.iter().map(|y| y / width).collect();
        let xs: Vec<f64> = Ufunc::Sinh
            .map(&scaled, f64::sinh)
            .into_iter()
            .map(|sinh| width * sinh)
            .collect();
        let mut ticks = self.round_ticks(&xs, &ys);
        ticks.sort_by(f64::total_cmp);
        ticks.dedup_by(|a, b| a == b);
        if ticks.len() >= 2 {
            Ok(ticks)
        } else {
            linspace(vmin, vmax, self.numticks)
        }
    }

    fn locate(&self, ctx: &TickContext<'_>) -> Result<Vec<f64>, TickerError> {
        let (vmin, vmax) = ctx.view_interval();
        if vmin * vmax < 0.0 && (1.0 + vmax / vmin).abs() < self.symthresh {
            // Nearly symmetric about zero: make it exactly symmetric.
            let bound = py_max(vmin.abs(), vmax.abs());
            self.tick_values(-bound, bound, ctx)
        } else {
            self.tick_values(vmin, vmax, ctx)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asinh_ticks_include_zero() {
        let ctx = TickContext::default();
        let loc = AsinhLocator::new(1.0);
        let ticks = loc.tick_values(-100.0, 100.0, &ctx).unwrap();
        assert!(ticks.contains(&0.0));
        assert!(ticks.windows(2).all(|w| w[0] < w[1]));
        let flat = loc.tick_values(0.0, 0.0, &ctx).unwrap();
        assert_eq!(flat.len(), 11);
    }

    #[test]
    fn huge_tick_counts_are_refused() {
        // Found by fuzzing: 2^31 ticks asked for 16 GB.
        let ctx = TickContext::default();
        let loc = AsinhLocator::new(1.0).with_numticks(1 << 31);
        assert_eq!(
            loc.tick_values(-100.0, 100.0, &ctx),
            Err(TickerError::TooManyTicks(1 << 31))
        );
    }
}
