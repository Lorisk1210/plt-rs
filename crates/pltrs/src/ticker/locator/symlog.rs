//! `SymmetricalLogLocator`: log ticks on both sides of a linear region.

use pltrs_core::numeric::{floor_div, py_max, py_min};

use super::Locator;
use super::log::{decade_greater, decade_greater_equal, decade_less, decade_less_equal};
use crate::ticker::pymath::arange;
use crate::ticker::{AutolimitMode, TickContext, TickerError};
use crate::transforms::nonsingular;

/// Ticks at `±subs[j] * base**i` beyond `linthresh` and at zero in the
/// linear region (matplotlib's `SymmetricalLogLocator`).
#[derive(Clone, Debug, PartialEq)]
pub struct SymmetricalLogLocator {
    base: f64,
    linthresh: f64,
    subs: Vec<f64>,
    numticks: usize,
}

pltrs_core::archive_struct!(SymmetricalLogLocator {
    base,
    linthresh,
    subs,
    numticks
});

impl SymmetricalLogLocator {
    /// Ticks for a symlog axis with the given `base` and `linthresh`.
    pub fn new(base: f64, linthresh: f64) -> Self {
        Self {
            base,
            linthresh,
            subs: vec![1.0],
            numticks: 15,
        }
    }

    /// Multiples of each power (default `[1.0]`), in an array or a `Vec`.
    ///
    /// ```
    /// use pltrs::ticker::SymmetricalLogLocator;
    /// let minor = SymmetricalLogLocator::new(10.0, 1.0).with_subs([2.0, 5.0]);
    /// assert_eq!(minor.subs(), [2.0, 5.0]);
    /// ```
    #[must_use]
    pub fn with_subs(mut self, subs: impl Into<Vec<f64>>) -> Self {
        self.subs = subs.into();
        self
    }

    /// The approximate maximum number of ticks (default 15).
    #[must_use]
    pub fn with_numticks(mut self, numticks: usize) -> Self {
        self.numticks = numticks;
        self
    }

    /// The logarithm base.
    pub fn base(&self) -> f64 {
        self.base
    }

    /// The extent of the linear region around zero.
    pub fn linthresh(&self) -> f64 {
        self.linthresh
    }

    /// The multiples of each power.
    pub fn subs(&self) -> &[f64] {
        &self.subs
    }

    /// The approximate maximum number of ticks.
    pub fn numticks(&self) -> usize {
        self.numticks
    }

    /// `(floor(log_b(lo)), ceil(log_b(hi)))`.
    fn log_range(&self, lo: f64, hi: f64) -> (f64, f64) {
        let log_base = self.base.ln();
        ((lo.ln() / log_base).floor(), (hi.ln() / log_base).ceil())
    }
}

impl Locator for SymmetricalLogLocator {
    fn tick_values(
        &self,
        vmin: f64,
        vmax: f64,
        _: &TickContext<'_>,
    ) -> Result<Vec<f64>, TickerError> {
        let linthresh = self.linthresh;
        let (vmin, vmax) = if vmax < vmin {
            (vmax, vmin)
        } else {
            (vmin, vmax)
        };
        if -linthresh <= vmin && vmin < vmax && vmax <= linthresh {
            // Only the linear region is visible.
            let mut ticks = vec![vmin, 0.0, vmax];
            ticks.sort_by(f64::total_cmp);
            ticks.dedup_by(|a, b| a == b);
            return Ok(ticks);
        }
        if self.numticks < 2 {
            return Err(TickerError::invalid("numticks", "must be at least 2"));
        }
        let has_a = vmin < -linthresh;
        let has_c = vmax > linthresh;
        let has_b = (has_a && vmax > -linthresh) || (has_c && vmin < linthresh);

        let (a_lo, a_hi) = if has_a {
            self.log_range(py_min(-linthresh, vmax).abs(), vmin.abs() + 1.0)
        } else {
            (0.0, 0.0)
        };
        let (c_lo, c_hi) = if has_c {
            self.log_range(py_max(linthresh, vmin), vmax + 1.0)
        } else {
            (0.0, 0.0)
        };
        let total_ticks = (a_hi - a_lo) + (c_hi - c_lo) + if has_b { 1.0 } else { 0.0 };
        let stride = py_max(floor_div(total_ticks, (self.numticks - 1) as f64), 1.0);

        let mut decades = Vec::new();
        if has_a {
            let exponents = arange(a_lo, a_hi, stride)?;
            decades.extend(
                exponents
                    .iter()
                    .rev()
                    .map(|&e| -pltrs_core::numeric::pow(self.base, e)),
            );
        }
        if has_b {
            decades.push(0.0);
        }
        if has_c {
            let exponents = arange(c_lo, c_hi, stride)?;
            decades.extend(
                exponents
                    .iter()
                    .map(|&e| pltrs_core::numeric::pow(self.base, e)),
            );
        }

        match self.subs.as_slice() {
            [] => Err(TickerError::invalid("subs", "must not be empty")),
            [only] if *only == 1.0 => Ok(decades),
            subs => Ok(decades
                .iter()
                .flat_map(|&decade| {
                    let multiples: Vec<f64> = if decade == 0.0 {
                        vec![decade]
                    } else {
                        subs.iter().map(|s| s * decade).collect()
                    };
                    multiples
                })
                .collect()),
        }
    }

    fn view_limits(
        &self,
        vmin: f64,
        vmax: f64,
        ctx: &TickContext<'_>,
    ) -> Result<(f64, f64), TickerError> {
        let b = self.base;
        let (mut vmin, mut vmax) = if vmax < vmin {
            (vmax, vmin)
        } else {
            (vmin, vmax)
        };
        if ctx.settings().axes_autolimit_mode == AutolimitMode::RoundNumbers {
            vmin = decade_less_equal(vmin, b);
            vmax = decade_greater_equal(vmax, b);
            if vmin == vmax {
                vmin = decade_less(vmin, b);
                vmax = decade_greater(vmax, b);
            }
        }
        Ok(nonsingular(vmin, vmax, 0.001, 1e-15, true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticks(locator: &SymmetricalLogLocator, vmin: f64, vmax: f64) -> Vec<f64> {
        locator
            .tick_values(vmin, vmax, &TickContext::default())
            .unwrap()
    }

    #[test]
    fn symlog_ticks() {
        let loc = SymmetricalLogLocator::new(10.0, 1.0);
        assert_eq!(
            ticks(&loc, -100.0, 100.0),
            [-100.0, -10.0, -1.0, 0.0, 1.0, 10.0, 100.0]
        );
        assert_eq!(ticks(&loc, -0.5, 0.5), [-0.5, 0.0, 0.5]);
        assert_eq!(ticks(&loc, 0.0, 0.5), [0.0, 0.5]);
        assert_eq!(ticks(&loc, 0.0, 0.0), Vec::<f64>::new());
        let minor = loc.with_subs([2.0, 5.0]);
        assert_eq!(ticks(&minor, 0.0, 20.0), [0.0, 2.0, 5.0, 20.0, 50.0]);
    }
}
