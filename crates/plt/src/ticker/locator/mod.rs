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
pub(crate) use log::{decade_greater, decade_greater_equal, decade_less, decade_less_equal};
pub use log::{LogLocator, LogSubs, NumTicks};
pub use logit::LogitLocator;
pub use maxn::{AutoLocator, MaxNLocator, Nbins, Prune, scale_range};
pub use minor::AutoMinorLocator;
pub use symlog::SymmetricalLogLocator;

use super::{TickContext, TickerError};

/// Number of ticks beyond which matplotlib warns that a locator is
/// misbehaving (`Locator.MAXTICKS`).
pub const MAXTICKS: usize = 1000;

/// Whether `locs` is too long to be drawn sensibly (matplotlib logs a
/// warning in `raise_if_exceeds`); callers decide how to report it.
pub fn exceeds_max_ticks(locs: &[f64]) -> bool {
    locs.len() >= MAXTICKS
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

    /// Adjusts a data range to avoid singularities during autoscaling.
    ///
    /// Called with the data limits, or `(-inf, inf)` when there are no
    /// data. Returns an increasing interval: expanded around `v0` if
    /// `v0 == v1` (up to rounding), default limits for infinite input,
    /// and the input otherwise.
    fn nonsingular(&self, v0: f64, v1: f64, _ctx: &TickContext<'_>) -> (f64, f64) {
        nonsingular(v0, v1, 0.05, 1e-15)
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
        Ok(nonsingular(dmin, dmax, 0.001, 1e-15))
    }
}

/// Modifies the endpoints of a range to avoid singularities
/// (matplotlib's `transforms._nonsingular`, always increasing).
///
/// * Infinite or NaN endpoints give `(-expander, expander)`.
/// * Endpoints are swapped into increasing order.
/// * If both are tiny (≲ 1e-290) the result is `(-expander, expander)`.
/// * If the interval is smaller than `tiny` times the larger magnitude,
///   each endpoint moves outwards by `expander` times its magnitude
///   (or to `±expander` if both are zero).
pub fn nonsingular(vmin: f64, vmax: f64, expander: f64, tiny: f64) -> (f64, f64) {
    if !vmin.is_finite() || !vmax.is_finite() {
        return (-expander, expander);
    }
    let (mut vmin, mut vmax) = if vmax < vmin {
        (vmax, vmin)
    } else {
        (vmin, vmax)
    };
    let maxabs = vmin.abs().max(vmax.abs());
    if maxabs < (1e6 / tiny) * f64::MIN_POSITIVE {
        vmin = -expander;
        vmax = expander;
    } else if vmax - vmin <= maxabs * tiny {
        if vmax == 0.0 && vmin == 0.0 {
            vmin = -expander;
            vmax = expander;
        } else {
            vmin -= expander * vmin.abs();
            vmax += expander * vmax.abs();
        }
    }
    (vmin, vmax)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonsingular_matches_matplotlib() {
        assert_eq!(nonsingular(0.0, 1.0, 0.05, 1e-15), (0.0, 1.0));
        assert_eq!(nonsingular(1.0, 0.0, 0.05, 1e-15), (0.0, 1.0));
        assert_eq!(nonsingular(0.0, 0.0, 0.05, 1e-15), (-0.05, 0.05));
        assert_eq!(nonsingular(-1.0, -1.0, 0.05, 1e-15), (-1.05, -0.95));
        assert_eq!(
            nonsingular(f64::NEG_INFINITY, 1.0, 0.05, 1e-15),
            (-0.05, 0.05)
        );
        assert_eq!(nonsingular(f64::NAN, 1.0, 0.05, 1e-15), (-0.05, 0.05));
        assert_eq!(nonsingular(1e-300, 1e-300, 0.05, 1e-15), (-0.05, 0.05));
    }

    #[test]
    fn max_ticks_threshold() {
        assert!(!exceeds_max_ticks(&[0.0; 999]));
        assert!(exceeds_max_ticks(&[0.0; 1000]));
    }
}
