//! `MaxNLocator` and `AutoLocator`: nice evenly spaced ticks.

use pltrs_core::numeric::{divmod, floor_div, py_max};

use super::Locator;
use crate::ticker::pymath::{arange, pow10};
use crate::ticker::{AutolimitMode, TickContext, TickerError, TickerSettings};
use crate::transforms::nonsingular;

/// Maximum number of intervals between ticks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nbins {
    /// Derived from the axis' tick space (clipped to `[1, 9]`); 9 without
    /// an axis.
    Auto,
    /// A fixed number of intervals.
    Fixed(usize),
}

/// Which edge ticks [`MaxNLocator`] drops.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prune {
    /// The lowest tick.
    Lower,
    /// The highest tick.
    Upper,
    /// Both edge ticks.
    Both,
}

/// `scale_range`: the power of ten close to `|vmax - vmin| / n` and an
/// offset (a power of ten with the sign of the mean) when the mean is
/// large compared to the range (`|mean| / range >= threshold`).
pub fn scale_range(vmin: f64, vmax: f64, n: f64, threshold: f64) -> (f64, f64) {
    let dv = (vmax - vmin).abs();
    let meanv = vmax.midpoint(vmin);
    let offset = if meanv.abs() / dv < threshold {
        0.0
    } else {
        pow10(floor_div(meanv.abs().log10(), 1.0)).copysign(meanv)
    };
    let scale = pow10(floor_div((dv / n).log10(), 1.0));
    (scale, offset)
}

/// Integer multiples of a step, tolerant of floating-point error when a
/// large offset has been subtracted (matplotlib's `_Edge_integer`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct EdgeInteger {
    step: f64,
    offset: f64,
}

impl EdgeInteger {
    /// Multiples of `step > 0` after subtracting `offset`.
    pub(crate) fn new(step: f64, offset: f64) -> Result<Self, TickerError> {
        if step > 0.0 && step.is_finite() {
            Ok(Self {
                step,
                offset: offset.abs(),
            })
        } else {
            Err(TickerError::invalid("step", "must be positive"))
        }
    }

    /// The step.
    pub(crate) fn step(self) -> f64 {
        self.step
    }

    fn close_to(self, ms: f64, edge: f64) -> bool {
        let tol = if self.offset > 0.0 {
            let digits = (self.offset / self.step).log10();
            1e-10_f64.max(pow10(digits - 12.0)).min(0.4999)
        } else {
            1e-10
        };
        (ms - edge).abs() < tol
    }

    /// The largest `n` with `n * step <= x`.
    pub(crate) fn le(self, x: f64) -> f64 {
        let (d, m) = divmod(x, self.step);
        if self.close_to(m / self.step, 1.0) {
            d + 1.0
        } else {
            d
        }
    }

    /// The smallest `n` with `n * step >= x`.
    pub(crate) fn ge(self, x: f64) -> f64 {
        let (d, m) = divmod(x, self.step);
        if self.close_to(m / self.step, 0.0) {
            d
        } else {
            d + 1.0
        }
    }
}

/// Places at most `nbins + 1` evenly spaced ticks at nice multiples of a
/// step within the view, plus one beyond each limit (matplotlib's
/// `MaxNLocator`).
///
/// ```
/// use pltrs::ticker::{Locator, MaxNLocator, Nbins, TickContext};
/// let locator = MaxNLocator::new(Nbins::Fixed(4));
/// let ticks = locator.tick_values(0.1, 0.9, &TickContext::default()).unwrap();
/// assert_eq!(ticks, [0.0, 0.2, 0.4, 0.600_000_000_000_000_1, 0.8, 1.0]);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct MaxNLocator {
    nbins: Nbins,
    steps: Vec<f64>,
    extended_steps: Vec<f64>,
    integer: bool,
    symmetric: bool,
    prune: Option<Prune>,
    min_n_ticks: usize,
}

impl MaxNLocator {
    /// The steps used when none are given.
    pub const DEFAULT_STEPS: [f64; 10] = [1.0, 1.5, 2.0, 2.5, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0];

    /// A locator with at most `nbins` intervals and default parameters.
    pub fn new(nbins: Nbins) -> Self {
        Self {
            nbins,
            steps: Self::DEFAULT_STEPS.to_vec(),
            extended_steps: staircase(&Self::DEFAULT_STEPS),
            integer: false,
            symmetric: false,
            prune: None,
            min_n_ticks: 2,
        }
    }

    /// Changes the maximum number of intervals.
    #[must_use]
    pub fn with_nbins(mut self, nbins: Nbins) -> Self {
        self.nbins = nbins;
        self
    }

    /// Acceptable tick multiples: an increasing sequence within `[1, 10]`
    /// (1 and 10 are added if missing).
    pub fn with_steps(mut self, steps: &[f64]) -> Result<Self, TickerError> {
        let invalid = || {
            TickerError::invalid(
                "steps",
                "must be an increasing sequence of numbers between 1 and 10 inclusive",
            )
        };
        let (&first, &last) = steps.first().zip(steps.last()).ok_or_else(invalid)?;
        let increasing = steps.windows(2).all(|w| w[1] - w[0] > 0.0);
        if !increasing || last > 10.0 || first < 1.0 || first.is_nan() || last.is_nan() {
            return Err(invalid());
        }
        let mut validated = Vec::with_capacity(steps.len() + 2);
        if first != 1.0 {
            validated.push(1.0);
        }
        validated.extend_from_slice(steps);
        if last != 10.0 {
            validated.push(10.0);
        }
        self.extended_steps = staircase(&validated);
        self.steps = validated;
        Ok(self)
    }

    /// Only integer ticks (when at least `min_n_ticks` integers are in view).
    #[must_use]
    pub fn with_integer(mut self, integer: bool) -> Self {
        self.integer = integer;
        self
    }

    /// Ticks (and autoscaled limits) symmetric about zero.
    #[must_use]
    pub fn with_symmetric(mut self, symmetric: bool) -> Self {
        self.symmetric = symmetric;
        self
    }

    /// Drops edge ticks.
    #[must_use]
    pub fn with_prune(mut self, prune: Option<Prune>) -> Self {
        self.prune = prune;
        self
    }

    /// Relaxes `nbins`/`integer` if needed to get this many ticks in view
    /// (at least 1).
    #[must_use]
    pub fn with_min_n_ticks(mut self, min_n_ticks: usize) -> Self {
        self.min_n_ticks = min_n_ticks.max(1);
        self
    }

    /// The configured number of bins.
    pub fn nbins(&self) -> Nbins {
        self.nbins
    }

    /// The acceptable steps, including the implied 1 and 10.
    pub fn steps(&self) -> &[f64] {
        &self.steps
    }

    /// Whether ticks are restricted to integers.
    pub fn integer(&self) -> bool {
        self.integer
    }

    /// Whether ticks are symmetric about zero.
    pub fn symmetric(&self) -> bool {
        self.symmetric
    }

    /// Which edge ticks are dropped.
    pub fn prune(&self) -> Option<Prune> {
        self.prune
    }

    /// The minimum number of ticks in view.
    pub fn min_n_ticks(&self) -> usize {
        self.min_n_ticks
    }

    /// The effective number of bins for `ctx`.
    pub(crate) fn resolved_nbins(&self, ctx: &TickContext<'_>) -> Result<usize, TickerError> {
        let nbins = match (self.nbins, ctx.axis()) {
            (Nbins::Fixed(n), _) => n,
            // np.clip(space, lo, 9) == min(max(space, lo), 9), even if lo > 9.
            (Nbins::Auto, Some(axis)) => axis
                .tick_space
                .max(self.min_n_ticks.saturating_sub(1).max(1))
                .min(9),
            (Nbins::Auto, None) => 9,
        };
        if nbins == 0 {
            return Err(TickerError::invalid("nbins", "must be positive"));
        }
        Ok(nbins)
    }

    /// Tick locations covering `[vmin, vmax]` (matplotlib's `_raw_ticks`).
    fn raw_ticks(
        &self,
        vmin: f64,
        vmax: f64,
        ctx: &TickContext<'_>,
    ) -> Result<Vec<f64>, TickerError> {
        let nbins = self.resolved_nbins(ctx)? as f64;
        let (scale, offset) = scale_range(vmin, vmax, nbins, 100.0);
        let (lo, hi) = (vmin - offset, vmax - offset);
        let mut steps: Vec<f64> = self.extended_steps.iter().map(|s| s * scale).collect();
        if self.integer {
            steps.retain(|&s| s < 1.0 || (s - s.round_ties_even()).abs() < 0.001);
        }
        let mut raw_step = (hi - lo) / nbins;
        if ctx.axis().is_some_and(|axis| axis.three_d) {
            // matplotlib keeps the appearance of 3-D ticks from before its
            // 3.9 automargin change (a zoom of 2/48).
            raw_step = raw_step * 23.0 / 24.0;
        }
        let round_numbers = ctx.settings().axes_autolimit_mode == AutolimitMode::RoundNumbers;
        let large_enough = |step: f64| {
            step >= raw_step && (!round_numbers || floor_div(lo, step) * step + step * nbins >= hi)
        };
        let last = steps
            .len()
            .checked_sub(1)
            .ok_or_else(|| TickerError::invalid("steps", "no usable step"))?;
        let istep = steps.iter().position(|&s| large_enough(s)).unwrap_or(last);

        let min_ticks = self.min_n_ticks as f64;
        let mut ticks = Vec::new();
        for &candidate in steps[..=istep].iter().rev() {
            let step = if self.integer && hi.floor() - lo.ceil() >= min_ticks - 1.0 {
                py_max(1.0, candidate)
            } else {
                candidate
            };
            let best_vmin = floor_div(lo, step) * step;
            let edge = EdgeInteger::new(step, offset)?;
            let low = edge.le(lo - best_vmin);
            let high = edge.ge(hi - best_vmin);
            ticks = arange(low, high + 1.0, 1.0)?
                .into_iter()
                .map(|t| t * step + best_vmin)
                .collect();
            let in_view = ticks.iter().filter(|&&t| t <= hi && t >= lo).count();
            if in_view >= self.min_n_ticks {
                break;
            }
        }
        Ok(ticks.into_iter().map(|t| t + offset).collect())
    }

    fn symmetrized(&self, vmin: f64, vmax: f64) -> (f64, f64) {
        if self.symmetric {
            let m = py_max(vmin.abs(), vmax.abs());
            (-m, m)
        } else {
            (vmin, vmax)
        }
    }
}

impl Default for MaxNLocator {
    /// `MaxNLocator()`: 10 bins.
    fn default() -> Self {
        Self::new(Nbins::Fixed(10))
    }
}

/// matplotlib's `_staircase`: the steps extended a decade down and up.
fn staircase(steps: &[f64]) -> Vec<f64> {
    let mut out: Vec<f64> = steps[..steps.len() - 1].iter().map(|s| 0.1 * s).collect();
    out.extend_from_slice(steps);
    out.push(10.0 * steps[1]);
    out
}

impl Locator for MaxNLocator {
    fn tick_values(
        &self,
        vmin: f64,
        vmax: f64,
        ctx: &TickContext<'_>,
    ) -> Result<Vec<f64>, TickerError> {
        let (vmin, vmax) = self.symmetrized(vmin, vmax);
        let (vmin, vmax) = nonsingular(vmin, vmax, 1e-13, 1e-14, true);
        let mut locs = self.raw_ticks(vmin, vmax, ctx)?;
        let n = locs.len();
        match self.prune {
            Some(Prune::Lower) => {
                locs.drain(..n.min(1));
            }
            Some(Prune::Upper) => locs.truncate(n.saturating_sub(1)),
            Some(Prune::Both) => {
                locs.truncate(n.saturating_sub(1));
                locs.drain(..locs.len().min(1));
            }
            None => {}
        }
        Ok(locs)
    }

    fn view_limits(
        &self,
        dmin: f64,
        dmax: f64,
        ctx: &TickContext<'_>,
    ) -> Result<(f64, f64), TickerError> {
        let (dmin, dmax) = self.symmetrized(dmin, dmax);
        let (dmin, dmax) = nonsingular(dmin, dmax, 1e-12, 1e-13, true);
        if ctx.settings().axes_autolimit_mode == AutolimitMode::RoundNumbers {
            let ticks = self.raw_ticks(dmin, dmax, ctx)?;
            match (ticks.first(), ticks.last()) {
                (Some(&lo), Some(&hi)) => Ok((lo, hi)),
                _ => Err(TickerError::invalid("view", "no ticks to round to")),
            }
        } else {
            Ok((dmin, dmax))
        }
    }
}

/// [`MaxNLocator`] with automatic bins and steps `[1, 2, 2.5, 5, 10]`
/// (classic mode: 9 bins, steps `[1, 2, 5, 10]`) — the default major
/// locator of linear axes.
#[derive(Clone, Debug, PartialEq)]
pub struct AutoLocator(MaxNLocator);

impl AutoLocator {
    /// An `AutoLocator` for matplotlib's default rcParams.
    pub fn new() -> Self {
        Self::from_settings(&TickerSettings::DEFAULT)
    }

    /// An `AutoLocator` honouring `_internal.classic_mode`.
    pub fn from_settings(settings: &TickerSettings) -> Self {
        let (nbins, steps): (Nbins, &[f64]) = if settings.classic_mode {
            (Nbins::Fixed(9), &[1.0, 2.0, 5.0, 10.0])
        } else {
            (Nbins::Auto, &[1.0, 2.0, 2.5, 5.0, 10.0])
        };
        Self(
            MaxNLocator::new(nbins)
                .with_steps(steps)
                .expect("built-in steps are valid"),
        )
    }

    /// The underlying `MaxNLocator` configuration.
    pub fn as_max_n(&self) -> &MaxNLocator {
        &self.0
    }
}

impl Default for AutoLocator {
    fn default() -> Self {
        Self::new()
    }
}

impl Locator for AutoLocator {
    fn tick_values(
        &self,
        vmin: f64,
        vmax: f64,
        ctx: &TickContext<'_>,
    ) -> Result<Vec<f64>, TickerError> {
        self.0.tick_values(vmin, vmax, ctx)
    }

    fn view_limits(
        &self,
        dmin: f64,
        dmax: f64,
        ctx: &TickContext<'_>,
    ) -> Result<(f64, f64), TickerError> {
        self.0.view_limits(dmin, dmax, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticks(locator: &impl Locator, vmin: f64, vmax: f64) -> Vec<f64> {
        locator
            .tick_values(vmin, vmax, &TickContext::default())
            .unwrap()
    }

    #[test]
    fn auto_locator_matches_matplotlib() {
        let auto = AutoLocator::new();
        assert_eq!(ticks(&auto, 0.0, 10.0), [0.0, 2.0, 4.0, 6.0, 8.0, 10.0]);
        assert_eq!(
            ticks(&auto, 0.0, 1.0),
            [0.0, 0.2, 0.4, 0.600_000_000_000_000_1, 0.8, 1.0]
        );
        assert_eq!(
            ticks(&auto, 1e5, 1e5 + 5.0),
            [1e5, 1e5 + 1.0, 1e5 + 2.0, 1e5 + 3.0, 1e5 + 4.0, 1e5 + 5.0]
        );
    }

    #[test]
    fn steps_validation() {
        assert!(MaxNLocator::default().with_steps(&[0.5, 1.0]).is_err());
        assert!(MaxNLocator::default().with_steps(&[1.0, 11.0]).is_err());
        assert!(MaxNLocator::default().with_steps(&[5.0, 2.0]).is_err());
        assert!(MaxNLocator::default().with_steps(&[]).is_err());
        let loc = MaxNLocator::default().with_steps(&[2.0, 5.0]).unwrap();
        assert_eq!(loc.steps(), [1.0, 2.0, 5.0, 10.0]);
    }

    #[test]
    fn prune_and_integer() {
        let base = MaxNLocator::new(Nbins::Fixed(5));
        assert_eq!(
            ticks(&base, 0.0, 1.0),
            [0.0, 0.2, 0.4, 0.600_000_000_000_000_1, 0.8, 1.0]
        );
        let pruned = base.clone().with_prune(Some(Prune::Both));
        assert_eq!(
            ticks(&pruned, 0.0, 1.0),
            [0.2, 0.4, 0.600_000_000_000_000_1, 0.8]
        );
        let integer = base.with_integer(true);
        assert_eq!(ticks(&integer, 0.0, 3.0), [0.0, 1.0, 2.0, 3.0]);
    }

    #[test]
    fn round_numbers_view_limits() {
        let settings = TickerSettings {
            axes_autolimit_mode: AutolimitMode::RoundNumbers,
            ..TickerSettings::DEFAULT
        };
        let ctx = TickContext::detached(&settings);
        let auto = AutoLocator::new();
        assert_eq!(auto.view_limits(0.13, 0.87, &ctx).unwrap(), (0.1, 0.9));
        assert_eq!(
            auto.view_limits(0.13, 0.87, &TickContext::default())
                .unwrap(),
            (0.13, 0.87)
        );
    }

    #[test]
    fn degenerate_input_does_not_panic() {
        let auto = AutoLocator::new();
        assert_ne!(ticks(&auto, f64::NAN, 1.0), Vec::<f64>::new());
        assert_ne!(ticks(&auto, 1.5, 1.5), Vec::<f64>::new());
        assert!(
            MaxNLocator::new(Nbins::Fixed(0))
                .tick_values(0.0, 1.0, &TickContext::default())
                .is_err()
        );
    }
}
