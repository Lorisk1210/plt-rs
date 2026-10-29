//! The tests plotting `matplotlib.testing.jpl_units` values (the unit
//! classes are in `crate::user_units::jpl`): user labels kept against the
//! labels of units (`test_formatter_ticker`, one case per figure), fills
//! of epochs and degrees mixed with plain numbers, and spans between
//! epochs. matplotlib's `xunits=` and `yunits=` keywords of `plot` become
//! [`Axis::set_units`](pltrs::axes::Axis::set_units) before the call, as
//! `plot` sets them before its values update the units.

use pltrs::dates::{DateTime, get_epoch};
use pltrs::numeric::linspace;
use pltrs::prelude::*;

use crate::figures::CaseResult;
use crate::user_units::jpl::{Duration, Epoch, UnitDbl, UnitName};

/// Figure `figure` (1 to 5) of `test_formatter_ticker`: labels set before
/// and after plotting seconds against kilometres, the last two figures
/// with a second line in hours, which the axis turns to.
fn formatter_ticker(fig: &Figure, figure: u8) -> CaseResult {
    let sec = UnitDbl::new(1.0, UnitName::Seconds);
    let km = UnitDbl::new(1.0, UnitName::Kilometres);
    let steps = linspace(0.0, 9.0, 10);
    let xdata: Vec<UnitDbl> = steps.iter().map(|&x| x * sec).collect();
    let ydata1: Vec<UnitDbl> = steps.iter().map(|&y| (1.5 * y - 0.5) * km).collect();
    let ydata2: Vec<UnitDbl> = steps.iter().map(|&y| (1.75 * y - 1.0) * km).collect();
    let ax = fig.subplot();
    if figure <= 3 {
        ax.set_xlabel("x-label 001");
    }
    if figure >= 2 {
        ax.xaxis().set_units(UnitName::Seconds);
        ax.plot_with(xdata.clone(), ydata1, LineProps::default().color("blue"))?;
    }
    if figure >= 4 {
        ax.xaxis().set_units(UnitName::Hours);
        ax.plot_with(xdata, ydata2, LineProps::default().color("green"))?;
    }
    if figure >= 3 {
        ax.set_xlabel(format!("x-label {figure:03}"));
    }
    if figure == 5 {
        ax.autoscale_view(None);
    }
    Ok(())
}

pub(super) fn formatter_ticker_001(fig: &Figure) -> CaseResult {
    formatter_ticker(fig, 1)
}

pub(super) fn formatter_ticker_002(fig: &Figure) -> CaseResult {
    formatter_ticker(fig, 2)
}

pub(super) fn formatter_ticker_003(fig: &Figure) -> CaseResult {
    formatter_ticker(fig, 3)
}

pub(super) fn formatter_ticker_004(fig: &Figure) -> CaseResult {
    formatter_ticker(fig, 4)
}

pub(super) fn formatter_ticker_005(fig: &Figure) -> CaseResult {
    formatter_ticker(fig, 5)
}

/// `test_fill_units`: an epoch at 10° in red over a blue triangle a day
/// wide and 90° high, given in date numbers and plain numbers, epochs and
/// numbers, date numbers and degrees, then epochs and degrees.
pub(super) fn fill_units(fig: &Figure) -> CaseResult {
    let start = DateTime::new(2009, 4, 27, 0, 0, 0, 0)?;
    let t = Epoch::from_date(start);
    let deg = UnitDbl::new(1.0, UnitName::Degrees);
    let value = 10.0 * deg;
    let day = Duration::seconds(24.0 * 60.0 * 60.0);
    // The date numbers of 2009-04-27 and 2009-04-28, `dtn[[0, 0, 1, 1]]`.
    let first = get_epoch().date2num(start);
    let dtn = vec![first, first, first + 1.0, first + 1.0];
    let epochs = vec![t, t, t + day, t + day];
    let degrees: Vec<UnitDbl> = [0.0, 0.0, 90.0, 0.0].map(|v| v * deg).to_vec();
    let numbers = [0.0, 0.0, 90.0, 0.0];
    let axs = fig.subplots(2, 2)?;
    let blue = || PatchProps::default().facecolor("b");
    for ax in [&axs[(0, 0)], &axs[(0, 1)], &axs[(1, 0)], &axs[(1, 1)]] {
        ax.yaxis().set_units(UnitName::Degrees);
        ax.plot_with(vec![t], vec![value], LineProps::default().color("red"))?;
    }
    axs[(0, 0)].fill(dtn.clone(), numbers, blue())?;
    axs[(0, 1)].fill(epochs.clone(), numbers, blue())?;
    axs[(1, 0)].fill(dtn, degrees.clone(), blue())?;
    axs[(1, 1)].fill(epochs, degrees, PatchProps::default().facecolor("blue"))?;
    Ok(())
}

/// A day-long span between two epochs, 25 % opaque blue, with limits five
/// days beyond it, along x (`test_axvspan_epoch`) or along y
/// (`test_axhspan_epoch`).
fn span_epoch(fig: &Figure, vertical: bool) -> CaseResult {
    let t0 = Epoch::from_date(DateTime::new(2009, 1, 21, 0, 0, 0, 0)?);
    let tf = Epoch::from_date(DateTime::new(2009, 1, 22, 0, 0, 0, 0)?);
    let dt = Duration::seconds(UnitDbl::new(24.0, UnitName::Hours).convert(UnitName::Seconds)?);
    let (low, high) = (t0 - 5.0 * dt, tf + 5.0 * dt);
    let ax = fig.subplot();
    let span = if vertical {
        let span = ax.axvspan(t0, tf);
        ax.set_xlim(low, high)?;
        span
    } else {
        let span = ax.axhspan(t0, tf);
        ax.set_ylim(low, high)?;
        span
    };
    span.set_facecolor("blue").set_alpha(Some(0.25));
    Ok(())
}

pub(super) fn axvspan_epoch(fig: &Figure) -> CaseResult {
    span_epoch(fig, true)
}

pub(super) fn axhspan_epoch(fig: &Figure) -> CaseResult {
    span_epoch(fig, false)
}
