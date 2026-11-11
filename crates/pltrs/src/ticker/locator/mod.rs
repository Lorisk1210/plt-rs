//! Tick locators: where ticks go.

mod asinh;
mod fixed;
mod linear;
mod log;
mod logit;
mod maxn;
mod minor;
mod symlog;

use std::any::Any;
use std::fmt;

pub use asinh::AsinhLocator;
pub use fixed::{FixedLocator, IndexLocator, NullLocator};
pub use linear::{LinearLocator, MultipleLocator};
pub use log::{LogLocator, LogSubs, NumTicks};
pub use logit::LogitLocator;
pub use maxn::{AutoLocator, MaxNLocator, Nbins, Prune, scale_range};
pub use minor::AutoMinorLocator;
pub use symlog::SymmetricalLogLocator;

use super::{TickContext, TickerError};
use crate::transforms::nonsingular;

/// Number of ticks beyond which matplotlib warns that a locator is
/// misbehaving (`Locator.MAXTICKS`).
pub const MAXTICKS: usize = 1000;

/// Whether `locs` is too long to be drawn sensibly (matplotlib logs a
/// warning in `raise_if_exceeds`); callers decide how to report it.
pub fn exceeds_max_ticks(locs: &[f64]) -> bool {
    locs.len() >= MAXTICKS
}

/// The most ticks of one level an axis draws: ten times [`MAXTICKS`].
/// matplotlib only warns and goes on to lay out and draw every label (for
/// minutes, or until memory runs out); pltrs reports
/// [`TickerError::TooManyTicks`] instead.
pub const MAX_DRAWN_TICKS: usize = 10 * MAXTICKS;

/// `locs` unless they are more than [`MAX_DRAWN_TICKS`].
pub(crate) fn drawable_ticks(locs: Vec<f64>) -> Result<Vec<f64>, TickerError> {
    if locs.len() > MAX_DRAWN_TICKS {
        Err(TickerError::TooManyTicks(locs.len()))
    } else {
        Ok(locs)
    }
}

/// How close a minor tick may be to a major one, in an axis' scaled
/// coordinates, before it is dropped as a duplicate (matplotlib's
/// `get_minorticklocs`, which calls `np.isclose(atol=span * 1e-5,
/// rtol=0)` with the span of the scaled view).
#[derive(Clone, Copy, Debug)]
pub(crate) struct OverlapTolerance(f64);

impl OverlapTolerance {
    /// The tolerance of the view `view` scaled through `forward`.
    pub(crate) fn of_view(view: (f64, f64), forward: impl Fn(f64) -> f64) -> Self {
        let (low, high) = (forward(view.0), forward(view.1));
        Self((high - low).abs() * 1e-5)
    }

    /// Warns as numpy does when the tolerance is not finite (a view whose
    /// limits are outside the domain of the scale).
    pub(crate) fn warn_if_invalid(self) {
        if !self.0.is_finite() {
            let atol = if self.0.is_nan() { "nan" } else { "inf" };
            crate::warnings::warn_runtime(&format!(
                "One of rtol or atol is not valid, atol: {atol}, rtol: 0"
            ));
        }
    }

    /// The locations `minor` without those on a `major` one, compared
    /// through `forward`.
    pub(crate) fn remove_overlaps(
        self,
        minor: Vec<f64>,
        major: &[f64],
        forward: impl Fn(f64) -> f64,
    ) -> Vec<f64> {
        let major: Vec<f64> = major.iter().map(|&m| forward(m)).collect();
        minor
            .into_iter()
            .filter(|&m| {
                let t = forward(m);
                !major.iter().any(|&j| (t - j).abs() <= self.0)
            })
            .collect()
    }
}

/// Decides where ticks go (matplotlib's `Locator`).
///
/// Locators are pure: everything besides their own parameters comes from
/// the [`TickContext`] (the attached axis, if any, and the rcParams).
pub trait Locator: fmt::Debug + Any {
    /// Tick locations for the interval `[vmin, vmax]`.
    fn tick_values(
        &self,
        vmin: f64,
        vmax: f64,
        ctx: &TickContext<'_>,
    ) -> Result<Vec<f64>, TickerError>;

    /// Tick locations for the context's axis (matplotlib's `__call__`);
    /// by default [`Locator::tick_values`] over the view interval.
    fn locate(&self, ctx: &TickContext<'_>) -> Result<Vec<f64>, TickerError> {
        let (vmin, vmax) = ctx.view_interval();
        self.tick_values(vmin, vmax, ctx)
    }

    /// What matplotlib's `nonsingular` warns of when the data range from
    /// `v0` to `v1` is one the locator's scale cannot take (data without
    /// positive values on a log scale), which autoscaling an axis to its
    /// data gives; `None` for the ranges, and the locators, that take any.
    fn unscalable(&self, _v0: f64, _v1: f64) -> Option<&'static str> {
        None
    }

    /// Adjusts a data range to avoid singularities during autoscaling.
    ///
    /// Called with the data limits, or `(-inf, inf)` when there are no
    /// data. Returns an increasing interval: expanded around `v0` if
    /// `v0 == v1` (up to rounding), default limits for infinite input,
    /// and the input otherwise.
    fn nonsingular(&self, v0: f64, v1: f64, _ctx: &TickContext<'_>) -> (f64, f64) {
        nonsingular(v0, v1, 0.05, 1e-15, true)
    }

    /// Chooses view limits for data spanning `[dmin, dmax]` during
    /// autoscaling; with `axes.autolimit_mode = "round_numbers"` many
    /// locators expand them to tick positions.
    fn view_limits(
        &self,
        dmin: f64,
        dmax: f64,
        _ctx: &TickContext<'_>,
    ) -> Result<(f64, f64), TickerError> {
        Ok(nonsingular(dmin, dmax, 0.001, 1e-15, true))
    }

    /// Days per tick unit of a date locator for the context's view
    /// (matplotlib's `_get_unit`, which date formatters ask their
    /// locator); `None` for locators that are not date locators.
    fn date_unit(&self, _ctx: &TickContext<'_>) -> Option<f64> {
        None
    }

    /// Whether the locator runs code of the program's that observes its
    /// calls (a host language's locator): the axes it ticks are then
    /// ticked as matplotlib ticks them, as often (see
    /// [`TickUpdater`](crate::axes::TickUpdater)). `false` by default.
    fn calls_program(&self) -> bool {
        false
    }
}

pltrs_core::archive_kinds!(dyn Locator {
    "null" => NullLocator,
    "auto" => AutoLocator,
    "maxn" => MaxNLocator,
    "fixed" => FixedLocator,
    "index" => IndexLocator,
    "linear" => LinearLocator,
    "multiple" => MultipleLocator,
    "logit" => LogitLocator,
    "auto-minor" => AutoMinorLocator,
    "symlog" => SymmetricalLogLocator,
    "log" => LogLocator,
    "asinh" => AsinhLocator,
    "category" => crate::units::CategoryLocator,
    "theta" => crate::polar::ThetaLocator,
    "radial" => crate::polar::RadialLocator,
    "auto-date" => crate::dates::AutoDateLocator,
    "chosen-date" => crate::dates::ChosenLocator,
    "microsecond" => crate::dates::MicrosecondLocator,
    "rrule" => crate::dates::RRuleLocator,
    "year" => crate::dates::YearLocator,
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_ticks_threshold() {
        assert!(!exceeds_max_ticks(&[0.0; 999]));
        assert!(exceeds_max_ticks(&[0.0; 1000]));
    }
}
