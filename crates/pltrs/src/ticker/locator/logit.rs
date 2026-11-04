//! `LogitLocator`: ticks for probabilities on a logit axis.

use super::{Locator, MaxNLocator, Nbins};
use crate::ticker::pymath::{ceil_int, floor_int, np_log10, pow10};
use crate::ticker::{TickContext, TickerError};

/// The steps of the `MaxNLocator` used when zoomed in.
const LOGIT_STEPS: [f64; 4] = [1.0, 2.0, 5.0, 10.0];

/// The smallest probability assumed when the data give none.
const STANDARD_MINPOS: f64 = 1e-7;

/// Ticks at `..., 0.01, 0.1, 1/2, 0.9, 0.99, ...` (or linear-style ticks
/// when zoomed in) for a logit axis (matplotlib's `LogitLocator`).
#[derive(Clone, Debug, PartialEq)]
pub struct LogitLocator {
    max_n: MaxNLocator,
    minor: bool,
}

pltrs_core::archive_struct!(LogitLocator { max_n, minor });

/// The `index`-th "ideal" logit tick: `10^index` below one half,
/// `1 - 10^-index` above.
fn ideal_tick(index: i64) -> f64 {
    match index {
        i if i < 0 => pow10(i as f64),
        0 => 0.5,
        i => 1.0 - pow10(-i as f64),
    }
}

impl LogitLocator {
    /// A major (`minor = false`) or minor tick locator with automatic bins.
    pub fn new(minor: bool) -> Self {
        Self {
            max_n: MaxNLocator::new(Nbins::Auto)
                .with_steps(&LOGIT_STEPS)
                .expect("logit steps are valid"),
            minor,
        }
    }

    /// Sets the number of bins.
    #[must_use]
    pub fn with_nbins(mut self, nbins: Nbins) -> Self {
        self.max_n = MaxNLocator::new(nbins)
            .with_steps(&LOGIT_STEPS)
            .expect("logit steps are valid");
        self
    }

    /// Whether this locator places minor ticks.
    pub fn minor(&self) -> bool {
        self.minor
    }

    /// The `MaxNLocator` configuration used when zoomed in.
    pub fn as_max_n(&self) -> &MaxNLocator {
        &self.max_n
    }

    /// Minor ticks between the ideal ticks `binf..bsup`.
    fn minor_ticks(binf: i64, bsup: i64) -> Vec<f64> {
        let mut ticks = Vec::new();
        for b in binf..bsup {
            match b {
                b if b < -1 => {
                    let power = pow10(b as f64);
                    ticks.extend((2..10).map(|k| f64::from(k) * power));
                }
                -1 => ticks.extend((2..5).map(|k| f64::from(k) / 10.0)),
                0 => ticks.extend((6..9).map(|k| f64::from(k) / 10.0)),
                b => {
                    let power = pow10((-b - 1) as f64);
                    ticks.extend((2..10).rev().map(|k| 1.0 - f64::from(k) * power));
                }
            }
        }
        ticks
    }
}

impl Default for LogitLocator {
    fn default() -> Self {
        Self::new(false)
    }
}

impl Locator for LogitLocator {
    fn tick_values(
        &self,
        vmin: f64,
        vmax: f64,
        ctx: &TickContext<'_>,
    ) -> Result<Vec<f64>, TickerError> {
        let nbins = match (self.max_n.nbins(), ctx.axis()) {
            (Nbins::Fixed(n), _) => n,
            (Nbins::Auto, Some(axis)) => axis.tick_space.max(2),
            (Nbins::Auto, None) => 9,
        } as i64;
        let (vmin, vmax) = self.nonsingular(vmin, vmax, ctx);
        let binf = if vmin < 0.5 {
            floor_int(np_log10(vmin), "log10 of the lower limit")?
        } else if vmin < 0.9 {
            0
        } else {
            -ceil_int(np_log10(1.0 - vmin), "log10 of the lower limit")?
        };
        let bsup = if vmax <= 0.5 {
            ceil_int(np_log10(vmax), "log10 of the upper limit")?
        } else if vmax <= 0.9 {
            1
        } else {
            -floor_int(np_log10(1.0 - vmax), "log10 of the upper limit")?
        };
        let numideal = bsup - binf - 1;
        if numideal >= 2 {
            if numideal > nbins {
                // Too many ideal ticks: subsample them for major ticks and
                // use the others as minor ticks.
                let factor = (numideal as f64 / nbins as f64).ceil() as i64;
                return Ok((binf..=bsup)
                    .filter(|b| (b.rem_euclid(factor) != 0) == self.minor)
                    .map(ideal_tick)
                    .collect());
            }
            if self.minor {
                return Ok(Self::minor_ticks(binf, bsup));
            }
            return Ok((binf..=bsup).map(ideal_tick).collect());
        }
        // Zoomed in: the same ticks as a linear scale.
        if self.minor {
            return Ok(Vec::new());
        }
        self.max_n.tick_values(vmin, vmax, ctx)
    }

    /// `MaxNLocator.view_limits`, which matplotlib's `LogitLocator`
    /// inherits: with `axes.autolimit_mode: round_numbers` the view widens
    /// to the outer ticks a `MaxNLocator` would place, typically `(0, 1)`,
    /// which the logit scale then limits to `(minpos, 1 - minpos)`.
    fn view_limits(
        &self,
        dmin: f64,
        dmax: f64,
        ctx: &TickContext<'_>,
    ) -> Result<(f64, f64), TickerError> {
        self.max_n.view_limits(dmin, dmax, ctx)
    }

    fn nonsingular(&self, vmin: f64, vmax: f64, ctx: &TickContext<'_>) -> (f64, f64) {
        let initial = (STANDARD_MINPOS, 1.0 - STANDARD_MINPOS);
        let (mut vmin, mut vmax) = if vmin > vmax {
            (vmax, vmin)
        } else {
            (vmin, vmax)
        };
        if !vmin.is_finite() || !vmax.is_finite() || vmax <= 0.0 || vmin >= 1.0 {
            // No data, or no data within (0, 1).
            return initial;
        }
        let minpos = ctx.axis().map_or(STANDARD_MINPOS, |axis| axis.minpos);
        let minpos = if minpos.is_finite() {
            minpos
        } else {
            STANDARD_MINPOS
        };
        if vmin <= 0.0 {
            vmin = minpos;
        }
        // matplotlib uses 1 - minpos for want of a "max below one" datum.
        if vmax >= 1.0 {
            vmax = 1.0 - minpos;
        }
        if vmin == vmax {
            (vmin, vmax) = (0.1 * vmin, 1.0 - 0.1 * vmin);
        }
        (vmin, vmax)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticks(locator: &LogitLocator, vmin: f64, vmax: f64) -> Vec<f64> {
        locator
            .tick_values(vmin, vmax, &TickContext::default())
            .unwrap()
    }

    #[test]
    fn ideal_ticks() {
        let major = LogitLocator::new(false);
        assert_eq!(ticks(&major, 0.01, 0.99), [0.01, 0.1, 0.5, 0.9, 0.99]);
        let minor = LogitLocator::new(true);
        let t = ticks(&minor, 0.01, 0.99);
        assert_eq!(&t[..3], [0.02, 0.03, 0.04]);
        assert_eq!(ticks(&minor, 0.45, 0.55), Vec::<f64>::new());
        assert_ne!(ticks(&major, 0.45, 0.55), Vec::<f64>::new());
    }

    #[test]
    fn nonsingular_defaults() {
        let loc = LogitLocator::default();
        let ctx = TickContext::default();
        assert_eq!(loc.nonsingular(0.0, 1.0, &ctx), (1e-7, 1.0 - 1e-7));
        assert_eq!(loc.nonsingular(2.0, 3.0, &ctx), (1e-7, 1.0 - 1e-7));
        assert_eq!(loc.nonsingular(0.7, 0.2, &ctx), (0.2, 0.7));
        assert_eq!(
            loc.nonsingular(0.3, 0.3, &ctx),
            (0.1 * 0.3, 1.0 - 0.1 * 0.3)
        );
    }

    #[test]
    fn view_limits_round_like_max_n_locator() {
        let loc = LogitLocator::default();
        let round = crate::ticker::TickerSettings {
            axes_autolimit_mode: crate::ticker::AutolimitMode::RoundNumbers,
            ..crate::ticker::TickerSettings::default()
        };
        let ctx = TickContext::detached(&round);
        assert_eq!(loc.view_limits(1e-7, 1.0 - 1e-7, &ctx).unwrap(), (0.0, 1.0));
        let data = crate::ticker::TickerSettings::default();
        let ctx = TickContext::detached(&data);
        assert_eq!(loc.view_limits(0.2, 0.7, &ctx).unwrap(), (0.2, 0.7));
    }
}
