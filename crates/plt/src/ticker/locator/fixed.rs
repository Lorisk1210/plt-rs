//! Locators that ignore the view: fixed positions, none, every n-th index.

use super::Locator;
use crate::ticker::pymath::arange;
use crate::ticker::{TickContext, TickerError};

/// Ticks at fixed positions, optionally subsampled to at most
/// `nbins + 1` ticks while keeping the smallest-magnitude position
/// (matplotlib's `FixedLocator`).
#[derive(Clone, Debug, PartialEq)]
pub struct FixedLocator {
    locs: Vec<f64>,
    nbins: Option<usize>,
}

impl FixedLocator {
    /// Ticks exactly at `locs`.
    pub fn new(locs: Vec<f64>) -> Self {
        Self { locs, nbins: None }
    }

    /// Subsamples to at most `nbins + 1` ticks (`nbins` is at least 2).
    #[must_use]
    pub fn with_nbins(mut self, nbins: usize) -> Self {
        self.nbins = Some(nbins.max(2));
        self
    }

    /// The candidate positions.
    pub fn locs(&self) -> &[f64] {
        &self.locs
    }

    /// The subsampling bound, if any.
    pub fn nbins(&self) -> Option<usize> {
        self.nbins
    }
}

/// numpy's `np.abs(values).min()` (NaN-propagating).
fn min_abs(values: impl Iterator<Item = f64>) -> f64 {
    values
        .map(f64::abs)
        .fold(f64::INFINITY, |acc, v| if v.is_nan() || v < acc { v } else { acc })
}

impl Locator for FixedLocator {
    fn tick_values(&self, _: f64, _: f64, _: &TickContext<'_>) -> Result<Vec<f64>, TickerError> {
        let Some(nbins) = self.nbins else {
            return Ok(self.locs.clone());
        };
        let step = self.locs.len().div_ceil(nbins).max(1);
        let every = |start: usize| self.locs.iter().copied().skip(start).step_by(step);
        let mut best = 0;
        for start in 1..step {
            if min_abs(every(start)) < min_abs(every(best)) {
                best = start;
            }
        }
        Ok(every(best).collect())
    }

    fn locate(&self, ctx: &TickContext<'_>) -> Result<Vec<f64>, TickerError> {
        self.tick_values(f64::NAN, f64::NAN, ctx)
    }
}

/// No ticks at all (matplotlib's `NullLocator`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NullLocator;

impl Locator for NullLocator {
    fn tick_values(&self, _: f64, _: f64, _: &TickContext<'_>) -> Result<Vec<f64>, TickerError> {
        Ok(Vec::new())
    }

    fn locate(&self, _: &TickContext<'_>) -> Result<Vec<f64>, TickerError> {
        Ok(Vec::new())
    }
}

/// A tick every `base` data points starting at `offset`, for index plots
/// (matplotlib's `IndexLocator`); located over the data interval.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IndexLocator {
    base: f64,
    offset: f64,
}

impl IndexLocator {
    /// Ticks at `vmin + offset + k * base`.
    pub fn new(base: f64, offset: f64) -> Self {
        Self { base, offset }
    }

    /// The spacing.
    pub fn base(&self) -> f64 {
        self.base
    }

    /// The offset of the first tick.
    pub fn offset(&self) -> f64 {
        self.offset
    }
}

impl Locator for IndexLocator {
    fn tick_values(
        &self,
        vmin: f64,
        vmax: f64,
        _: &TickContext<'_>,
    ) -> Result<Vec<f64>, TickerError> {
        // `+ 1e-12` makes the interval closed at `vmax`.
        arange(vmin + self.offset, vmax + 1e-12, self.base)
    }

    fn locate(&self, ctx: &TickContext<'_>) -> Result<Vec<f64>, TickerError> {
        let (dmin, dmax) = ctx.axis_or_dummy().data_interval;
        self.tick_values(dmin, dmax, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticks(locator: &impl Locator) -> Vec<f64> {
        locator.locate(&TickContext::default()).unwrap()
    }

    #[test]
    fn fixed_subsampling_keeps_smallest_magnitude() {
        let locs: Vec<f64> = (-5..=5).map(f64::from).collect();
        assert_eq!(ticks(&FixedLocator::new(locs.clone())), locs);
        assert_eq!(
            ticks(&FixedLocator::new(locs.clone()).with_nbins(3)),
            [-4.0, 0.0, 4.0]
        );
        assert_eq!(
            ticks(&FixedLocator::new(locs).with_nbins(0)).len(),
            6,
            "nbins is at least 2"
        );
        assert!(ticks(&FixedLocator::new(Vec::new()).with_nbins(2)).is_empty());
    }

    #[test]
    fn index_and_null() {
        let ctx = TickContext::default();
        let index = IndexLocator::new(2.5, 0.5);
        assert_eq!(
            index.tick_values(0.0, 10.0, &ctx).unwrap(),
            [0.5, 3.0, 5.5, 8.0]
        );
        assert!(index.tick_values(10.0, 0.0, &ctx).unwrap().is_empty());
        assert!(ticks(&NullLocator).is_empty());
    }
}
