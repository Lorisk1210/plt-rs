//! Evenly spaced ticks: a fixed count (`LinearLocator`) or a fixed step
//! (`MultipleLocator`).

use plt_core::numeric::{divmod, floor_div};

use super::Locator;
use super::maxn::EdgeInteger;
use crate::ticker::pymath::{arange, ceil_int, floor_int, linspace};
use crate::ticker::{AutolimitMode, TickContext, TickerError};
use crate::transforms::nonsingular;

/// `numticks` evenly spaced ticks spanning the view (matplotlib's
/// `LinearLocator`); `presets` override the ticks for specific views.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LinearLocator {
    numticks: Option<usize>,
    presets: Vec<((f64, f64), Vec<f64>)>,
}

impl LinearLocator {
    /// A locator with `numticks` ticks (`None` means 11).
    pub fn new(numticks: Option<usize>) -> Self {
        Self {
            numticks,
            presets: Vec::new(),
        }
    }

    /// Uses `locs` whenever the (nonsingular) view is exactly
    /// `(vmin, vmax)`.
    #[must_use]
    pub fn with_preset(mut self, vmin: f64, vmax: f64, locs: Vec<f64>) -> Self {
        self.presets.push(((vmin, vmax), locs));
        self
    }

    /// The number of ticks.
    pub fn numticks(&self) -> usize {
        self.numticks.unwrap_or(11)
    }
}

impl Locator for LinearLocator {
    fn tick_values(
        &self,
        vmin: f64,
        vmax: f64,
        _: &TickContext<'_>,
    ) -> Result<Vec<f64>, TickerError> {
        let view = nonsingular(vmin, vmax, 0.05, 1e-15, true);
        if let Some((_, locs)) = self.presets.iter().find(|(key, _)| *key == view) {
            return Ok(locs.clone());
        }
        linspace(view.0, view.1, self.numticks())
    }

    fn view_limits(
        &self,
        vmin: f64,
        vmax: f64,
        ctx: &TickContext<'_>,
    ) -> Result<(f64, f64), TickerError> {
        let (mut vmin, mut vmax) = if vmax < vmin {
            (vmax, vmin)
        } else {
            (vmin, vmax)
        };
        if vmin == vmax {
            vmin -= 1.0;
            vmax += 1.0;
        }
        if ctx.settings().axes_autolimit_mode == AutolimitMode::RoundNumbers {
            let intervals = self.numticks().saturating_sub(1).max(1) as f64;
            let log_intervals = intervals.log10();
            if log_intervals == 0.0 {
                return Err(TickerError::invalid(
                    "numticks",
                    "round_numbers view limits need more than 2 ticks",
                ));
            }
            let (mut exponent, remainder) = divmod((vmax - vmin).log10(), log_intervals);
            if remainder < 0.5 {
                exponent -= 1.0;
            }
            let scale = intervals.powf(-exponent);
            vmin = floor_int(scale * vmin, "view limit")? as f64 / scale;
            vmax = ceil_int(scale * vmax, "view limit")? as f64 / scale;
        }
        Ok(nonsingular(vmin, vmax, 0.001, 1e-15, true))
    }
}

/// Ticks at every integer multiple of `base`, shifted by `offset`
/// (matplotlib's `MultipleLocator`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MultipleLocator {
    edge: EdgeInteger,
    offset: f64,
}

impl MultipleLocator {
    /// Ticks at multiples of `base`, which must be positive.
    pub fn new(base: f64) -> Result<Self, TickerError> {
        Ok(Self {
            edge: EdgeInteger::new(base, 0.0)?,
            offset: 0.0,
        })
    }

    /// Adds `offset` to every multiple.
    #[must_use]
    pub fn with_offset(mut self, offset: f64) -> Self {
        self.offset = offset;
        self
    }

    /// The interval between ticks.
    pub fn base(&self) -> f64 {
        self.edge.step()
    }

    /// The offset added to each multiple.
    pub fn offset(&self) -> f64 {
        self.offset
    }
}

impl Locator for MultipleLocator {
    fn tick_values(
        &self,
        vmin: f64,
        vmax: f64,
        _: &TickContext<'_>,
    ) -> Result<Vec<f64>, TickerError> {
        let (vmin, vmax) = if vmax < vmin {
            (vmax, vmin)
        } else {
            (vmin, vmax)
        };
        let step = self.edge.step();
        let vmax = vmax - self.offset;
        let vmin = self.edge.ge(vmin - self.offset) * step;
        let n = floor_div(vmax - vmin + 0.001 * step, step);
        Ok(arange(0.0, n + 3.0, 1.0)?
            .into_iter()
            .map(|i| vmin - step + i * step + self.offset)
            .collect())
    }

    fn view_limits(
        &self,
        dmin: f64,
        dmax: f64,
        ctx: &TickContext<'_>,
    ) -> Result<(f64, f64), TickerError> {
        let (mut vmin, mut vmax) = (dmin, dmax);
        if ctx.settings().axes_autolimit_mode == AutolimitMode::RoundNumbers {
            let step = self.edge.step();
            vmin = self.edge.le(dmin - self.offset) * step + self.offset;
            vmax = self.edge.ge(dmax - self.offset) * step + self.offset;
            if vmin == vmax {
                vmin -= 1.0;
                vmax += 1.0;
            }
        }
        Ok(nonsingular(vmin, vmax, 0.001, 1e-15, true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ticker::TickerSettings;

    #[test]
    fn huge_tick_counts_are_refused() {
        // Found by fuzzing: 2^31 ticks asked for 16 GB.
        let ctx = TickContext::default();
        assert_eq!(
            LinearLocator::new(Some(1 << 31)).tick_values(0.0, 1.0, &ctx),
            Err(TickerError::TooManyTicks(1 << 31))
        );
    }

    #[test]
    fn linear_locator() {
        let ctx = TickContext::default();
        let loc = LinearLocator::new(Some(5));
        assert_eq!(
            loc.tick_values(0.0, 1.0, &ctx).unwrap(),
            [0.0, 0.25, 0.5, 0.75, 1.0]
        );
        assert_eq!(loc.tick_values(5.0, 5.0, &ctx).unwrap()[0], 4.75);
        assert_eq!(
            LinearLocator::new(Some(0))
                .tick_values(0.0, 1.0, &ctx)
                .unwrap(),
            Vec::<f64>::new()
        );
        let preset = LinearLocator::default().with_preset(0.0, 1.0, vec![0.5]);
        assert_eq!(preset.tick_values(0.0, 1.0, &ctx).unwrap(), [0.5]);
    }

    #[test]
    fn linear_round_numbers() {
        let settings = TickerSettings {
            axes_autolimit_mode: AutolimitMode::RoundNumbers,
            ..TickerSettings::DEFAULT
        };
        let ctx = TickContext::detached(&settings);
        let loc = LinearLocator::default();
        assert_eq!(loc.view_limits(0.13, 0.87, &ctx).unwrap(), (0.1, 0.9));
        assert!(
            LinearLocator::new(Some(2))
                .view_limits(0.0, 1.0, &ctx)
                .is_err()
        );
    }

    #[test]
    fn multiple_locator() {
        let ctx = TickContext::default();
        let loc = MultipleLocator::new(2.5).unwrap();
        assert_eq!(
            loc.tick_values(0.0, 10.0, &ctx).unwrap(),
            [-2.5, 0.0, 2.5, 5.0, 7.5, 10.0, 12.5]
        );
        let shifted = MultipleLocator::new(1.0).unwrap().with_offset(0.25);
        assert_eq!(
            shifted.tick_values(0.0, 2.0, &ctx).unwrap(),
            [-0.75, 0.25, 1.25, 2.25]
        );
        assert!(MultipleLocator::new(0.0).is_err());
        assert!(MultipleLocator::new(-1.0).is_err());
    }
}
