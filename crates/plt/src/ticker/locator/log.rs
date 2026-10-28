//! `LogLocator`: ticks at powers of a base (and multiples of them).

use super::{AutoLocator, Locator};
use crate::ticker::pymath::{arange, ceil_int, floor_int, round_int};
use crate::ticker::{AutolimitMode, LogBase, TickContext, TickerError, TickerSettings};

/// Multiples of each power of the base at which [`LogLocator`] places
/// ticks.
#[derive(Clone, Debug, PartialEq)]
pub enum LogSubs {
    /// `2, 3, ..., base - 1` between powers, but only when fewer than ten
    /// decades are visible (minor ticks).
    Auto,
    /// Like [`LogSubs::Auto`] but including `1`; with ten or more decades
    /// only the powers themselves.
    All,
    /// Explicit multiples, e.g. `[1.0]` for major ticks at the powers.
    Explicit(Vec<f64>),
}

/// The maximum number of major ticks a [`LogLocator`] places.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumTicks {
    /// Derived from the axis' tick space (clipped to `[2, 9]`); 9 without
    /// an axis.
    Auto,
    /// A fixed maximum.
    Fixed(usize),
}

/// Ticks at `subs[j] * base**i` (matplotlib's `LogLocator`).
///
/// ```
/// use plt::ticker::{Locator, LogLocator, TickContext};
/// let locator = LogLocator::new(10.0).unwrap();
/// let ticks = locator.tick_values(1.0, 1e3, &TickContext::default()).unwrap();
/// assert_eq!(ticks, [0.1, 1.0, 10.0, 100.0, 1000.0, 10000.0]);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct LogLocator {
    base: f64,
    subs: LogSubs,
    numticks: NumTicks,
}

impl LogLocator {
    /// Major ticks at powers of `base` (positive and not 1).
    pub fn new(base: f64) -> Result<Self, TickerError> {
        LogBase::new(base)
            .map(Self::from_base)
            .ok_or_else(|| TickerError::invalid("base", "must be positive and not 1"))
    }

    /// Major ticks at powers of `base`.
    pub fn from_base(base: LogBase) -> Self {
        Self {
            base: base.get(),
            subs: LogSubs::Explicit(vec![1.0]),
            numticks: NumTicks::Auto,
        }
    }

    /// The default locator for `settings`: classic mode caps it at 15
    /// ticks instead of deriving the count from the axis.
    pub fn from_settings(base: LogBase, settings: &TickerSettings) -> Self {
        let numticks = if settings.classic_mode {
            NumTicks::Fixed(15)
        } else {
            NumTicks::Auto
        };
        Self::from_base(base).with_numticks(numticks)
    }

    /// Sets the multiples of each power.
    #[must_use]
    pub fn with_subs(mut self, subs: LogSubs) -> Self {
        self.subs = subs;
        self
    }

    /// Sets the maximum number of major ticks.
    #[must_use]
    pub fn with_numticks(mut self, numticks: NumTicks) -> Self {
        self.numticks = numticks;
        self
    }

    /// The logarithm base.
    pub fn base(&self) -> f64 {
        self.base
    }

    /// The multiples of each power.
    pub fn subs(&self) -> &LogSubs {
        &self.subs
    }

    /// The maximum number of major ticks.
    pub fn numticks(&self) -> NumTicks {
        self.numticks
    }

    /// `log_base(x)`, using the more accurate dedicated functions for
    /// bases 10 and 2 like matplotlib.
    fn log_b(&self, x: f64) -> f64 {
        if self.base == 10.0 {
            x.log10()
        } else if self.base == 2.0 {
            x.log2()
        } else {
            x.ln() / self.base.ln()
        }
    }

    /// The decades (exponents) of the major ticks and their stride.
    fn decades(
        n_avail: i64,
        emin: i64,
        emax: i64,
        mid: f64,
        n_request: i64,
        classic: bool,
    ) -> Result<(Vec<i64>, i64), TickerError> {
        let invalid = || TickerError::invalid("numticks", "too small for a log axis");
        if classic {
            let denominator = n_request - 1;
            if denominator == 0 {
                return Err(invalid());
            }
            let stride = ((n_avail - 1) as f64 / denominator as f64).ceil().max(1.0) as i64;
            return Ok((
                (emin - stride..=emax + stride)
                    .step_by(stride as usize)
                    .collect(),
                stride,
            ));
        }
        if n_avail < 0 {
            // A base below 1 turns the decades around; matplotlib divides
            // by zero or fails an assertion.
            return Err(TickerError::invalid("base", "below 1 for a log axis"));
        }
        let mut n_request = n_request;
        let stride = n_avail.div_euclid(n_request.saturating_add(1)) + 1;
        let nr = (n_avail as f64 / stride as f64).ceil() as i64;
        if nr <= n_request {
            n_request = nr;
        }
        Ok(match n_request {
            0 => (vec![emin - 1, emax + 1], emax - emin + 2),
            1 => {
                let mid = round_int(mid, "decade")?;
                let stride = (mid - (emin - 1)).max((emax + 1) - mid);
                (vec![mid - stride, mid, mid + stride], stride)
            }
            _ => {
                let mut stride = (n_avail - 1).div_euclid(n_request - 1);
                if (stride as f64) < n_avail as f64 / n_request as f64 {
                    stride = n_avail.div_euclid(n_request);
                }
                if stride <= 0 {
                    return Err(invalid());
                }
                let olo = (n_avail - stride * n_request).max(0);
                let ohi = (n_avail - stride * (n_request - 1)).min(stride);
                let mut offset = (-emin).rem_euclid(stride);
                if !(olo <= offset && offset < ohi) {
                    offset = olo;
                }
                let decades = (emin + offset - stride..=emax + stride)
                    .step_by(stride as usize)
                    .collect();
                (decades, stride)
            }
        })
    }
}

impl Default for LogLocator {
    /// Base 10, major ticks at the powers.
    fn default() -> Self {
        Self::from_base(LogBase::TEN)
    }
}

impl Locator for LogLocator {
    fn tick_values(
        &self,
        vmin: f64,
        vmax: f64,
        ctx: &TickContext<'_>,
    ) -> Result<Vec<f64>, TickerError> {
        let n_request = match (self.numticks, ctx.axis()) {
            (NumTicks::Fixed(n), _) => n,
            (NumTicks::Auto, Some(axis)) => axis.tick_space.clamp(2, 9),
            (NumTicks::Auto, None) => 9,
        };
        let n_request = i64::try_from(n_request).unwrap_or(i64::MAX);
        let b = self.base;
        let mut vmin = vmin;
        let mut vmax = vmax;
        if vmin <= 0.0 {
            if let Some(axis) = ctx.axis() {
                vmin = axis.minpos;
            }
            if vmin <= 0.0 || !vmin.is_finite() {
                return Err(TickerError::NonPositiveLogData);
            }
        }
        if vmax < vmin {
            std::mem::swap(&mut vmin, &mut vmax);
        }
        let (efmin, efmax) = (self.log_b(vmin), self.log_b(vmax));
        let emin = ceil_int(efmin, "log of the lower limit")?;
        let emax = floor_int(efmax, "log of the upper limit")?;
        let n_avail = emax - emin + 1;

        let subs = match &self.subs {
            LogSubs::Explicit(subs) => subs.clone(),
            generated if n_avail >= 10 || b < 3.0 => {
                if *generated == LogSubs::Auto {
                    return Ok(Vec::new());
                }
                vec![1.0]
            }
            generated => {
                let first = if *generated == LogSubs::Auto {
                    2.0
                } else {
                    1.0
                };
                arange(first, b, 1.0)?
            }
        };

        let classic = ctx.settings().classic_mode;
        let mid = efmin.midpoint(efmax);
        let (decades, stride) = Self::decades(n_avail, emin, emax, mid, n_request, classic)?;

        let is_minor = subs.len() > 1 || (subs.len() == 1 && subs[0] != 1.0);
        let ticklocs: Vec<f64> = if !is_minor {
            decades.iter().map(|&d| b.powf(d as f64)).collect()
        } else if stride == 1 || n_avail <= 1 {
            // Minor ticks start in the decade preceding the first major.
            (emin - 1..=emax)
                .flat_map(|decade| {
                    let power = b.powf(decade as f64);
                    subs.iter().map(move |s| s * power)
                })
                .collect()
        } else {
            Vec::new()
        };

        let in_view = ticklocs.iter().filter(|&&t| vmin <= t && t <= vmax).count();
        if subs.len() > 1 && stride == 1 && decades.len() + in_view <= 3 {
            // A minor locator expecting two ticks per decade that would
            // show at most one tick: fall back to linear ticks.
            let settings = ctx.settings();
            return AutoLocator::from_settings(settings).tick_values(
                vmin,
                vmax,
                &TickContext::detached(settings),
            );
        }
        Ok(ticklocs)
    }

    fn nonsingular(&self, vmin: f64, vmax: f64, ctx: &TickContext<'_>) -> (f64, f64) {
        let (mut vmin, mut vmax) = if vmin > vmax {
            (vmax, vmin)
        } else {
            (vmin, vmax)
        };
        if !vmin.is_finite() || !vmax.is_finite() || vmax <= 0.0 {
            // No (positive) data yet: matplotlib's initial log range.
            return (1.0, 10.0);
        }
        let minpos = ctx.axis().map_or(f64::INFINITY, |axis| axis.minpos);
        let minpos = if minpos.is_finite() { minpos } else { 1e-300 };
        if vmin <= 0.0 {
            vmin = minpos;
        }
        if vmin == vmax {
            vmin = decade_less(vmin, self.base);
            vmax = decade_greater(vmax, self.base);
        }
        (vmin, vmax)
    }

    fn view_limits(
        &self,
        vmin: f64,
        vmax: f64,
        ctx: &TickContext<'_>,
    ) -> Result<(f64, f64), TickerError> {
        let (vmin, vmax) = self.nonsingular(vmin, vmax, ctx);
        if ctx.settings().axes_autolimit_mode == AutolimitMode::RoundNumbers {
            Ok((
                decade_less_equal(vmin, self.base),
                decade_greater_equal(vmax, self.base),
            ))
        } else {
            Ok((vmin, vmax))
        }
    }
}

/// The largest integer power of `base` that is `<= x` (for negative `x`
/// the exponent is larger); zero stays zero.
pub(crate) fn decade_less_equal(x: f64, base: f64) -> f64 {
    if x == 0.0 {
        x
    } else if x < 0.0 {
        -decade_greater_equal(-x, base)
    } else {
        base.powf((x.ln() / base.ln()).floor())
    }
}

/// The smallest integer power of `base` that is `>= x`.
pub(crate) fn decade_greater_equal(x: f64, base: f64) -> f64 {
    if x == 0.0 {
        x
    } else if x < 0.0 {
        -decade_less_equal(-x, base)
    } else {
        base.powf((x.ln() / base.ln()).ceil())
    }
}

/// The largest integer power of `base` that is `< x`.
pub(crate) fn decade_less(x: f64, base: f64) -> f64 {
    if x < 0.0 {
        return -decade_greater(-x, base);
    }
    let less = decade_less_equal(x, base);
    if less == x { less / base } else { less }
}

/// The smallest integer power of `base` that is `> x`.
pub(crate) fn decade_greater(x: f64, base: f64) -> f64 {
    if x < 0.0 {
        return -decade_less(-x, base);
    }
    let greater = decade_greater_equal(x, base);
    if greater == x {
        greater * base
    } else {
        greater
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ticker::AxisInfo;

    fn ticks(locator: &LogLocator, vmin: f64, vmax: f64) -> Vec<f64> {
        locator
            .tick_values(vmin, vmax, &TickContext::default())
            .unwrap()
    }

    #[test]
    fn major_and_minor_ticks() {
        let major = LogLocator::default();
        assert_eq!(
            ticks(&major, 1.0, 1e3),
            [0.1, 1.0, 10.0, 100.0, 1000.0, 10000.0]
        );
        let minor = LogLocator::default().with_subs(LogSubs::Auto);
        let t = ticks(&minor, 1.0, 100.0);
        assert_eq!(t.len(), 32);
        assert_eq!(t[0], 0.2);
        assert_eq!(ticks(&minor, 1.0, 1e12), Vec::<f64>::new());
    }

    #[test]
    fn non_positive_limits_use_minpos() {
        let locator = LogLocator::default();
        let ctx = TickContext::default();
        assert_eq!(
            locator.tick_values(0.0, 100.0, &ctx),
            Err(TickerError::NonPositiveLogData)
        );
        let axis = AxisInfo {
            minpos: 2.0,
            ..AxisInfo::DUMMY
        };
        let settings = TickerSettings::default();
        let ctx = TickContext::new(&axis, &settings);
        assert_eq!(
            locator.tick_values(-5.0, 100.0, &ctx).unwrap(),
            [1.0, 10.0, 100.0, 1000.0]
        );
        assert_eq!(locator.nonsingular(0.0, 70.0, &ctx), (2.0, 70.0));
        assert_eq!(locator.nonsingular(10.0, 10.0, &ctx), (1.0, 100.0));
        assert_eq!(locator.nonsingular(-10.0, -1.0, &ctx), (1.0, 10.0));
    }

    #[test]
    fn decades() {
        assert_eq!(decade_less_equal(0.3, 10.0), 0.1);
        assert_eq!(decade_greater_equal(70.0, 10.0), 100.0);
        assert_eq!(decade_less(10.0, 10.0), 1.0);
        assert_eq!(decade_greater(-10.0, 10.0), -1.0);
        assert_eq!(decade_less_equal(0.0, 10.0), 0.0);
    }

    #[test]
    fn invalid_base() {
        assert!(LogLocator::new(1.0).is_err());
        assert!(LogLocator::new(0.0).is_err());
        assert!(LogLocator::new(f64::NAN).is_err());
    }

    #[test]
    fn bases_below_one_and_huge_tick_counts_do_not_overflow() {
        // Found by fuzzing: base 0.76 overflowed the decade arithmetic.
        let ctx = TickContext::default();
        let below_one = LogLocator::new(0.76).unwrap();
        assert!(below_one.tick_values(1e-5, 1e5, &ctx).is_err());
        let many = LogLocator::new(10.0)
            .unwrap()
            .with_numticks(NumTicks::Fixed(usize::MAX));
        assert!(many.tick_values(1.0, 1e5, &ctx).is_ok());
    }
}
