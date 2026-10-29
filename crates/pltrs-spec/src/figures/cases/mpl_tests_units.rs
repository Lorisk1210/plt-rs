//! `figures/cases_mpl_tests_units.py`: matplotlib's own tests of unit
//! conversion (`test_units.py`) that draw, one case per figure: dates,
//! the `Quantity` facade with its converter, a masked array with no
//! converter, and `jpl_units` bars (the unit classes are in
//! `crate::user_units`).

use pltrs::axes::ErrorbarOptions;
use pltrs::dates::{DateTime, Datetime64, TimeDelta};
use pltrs::figure::Figure;
use pltrs::gridspec::SubplotOverrides;
use pltrs::numeric::linspace;
use pltrs::orientation::Orientation;
use pltrs::prelude::*;

use super::mpl_tests::{classic, remove_ticks_and_titles};
use crate::figures::{CaseResult, FigureCase};
use crate::user_units::jpl::{Duration, Epoch, UnitDbl, UnitName};
use crate::user_units::quantity::{Quantity, QuantityUnit, quantities};

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    vec![
        classic("mpl_tests_units/empty_arrays", empty_arrays),
        classic(
            "mpl_tests_units/scatter_element0_masked",
            scatter_element0_masked,
        ),
        classic("mpl_tests_units/errorbar_mixed_units", errorbar_mixed_units),
        FigureCase::new("mpl_tests_units/numpy_facade", numpy_facade)
            .with_rc(&[("axes.formatter.limits", "-7, 7")]),
        FigureCase::new("mpl_tests_units/plot_masked_units", plot_masked_units),
        jpl("mpl_tests_units/jpl_bar_units", jpl_bar_units),
        jpl("mpl_tests_units/jpl_barh_units", jpl_barh_units),
    ]
}

/// Seconds in a day.
const SECONDS_PER_DAY: i64 = 86_400;

/// A `jpl_units` case: the reference figure saved at 120 dpi.
fn jpl(name: &'static str, build: fn(&Figure) -> CaseResult) -> FigureCase {
    FigureCase::new(name, build).sized(6.4, 4.8, 120.0)
}

/// `test_empty_arrays`: a scatter of no dates.
fn empty_arrays(fig: &Figure) -> CaseResult {
    fig.subplot()
        .scatter(Vec::<Datetime64>::new(), Vec::<f64>::new())?;
    Ok(())
}

/// `test_scatter_element0_masked`: the days of February 2005 against 0 to
/// 27, the first value NaN.
fn scatter_element0_masked(fig: &Figure) -> CaseResult {
    // 2005-02-01 is day 12 815 since 1970-01-01.
    let first = 12_815;
    let times: Vec<Datetime64> = (first..first + 28)
        .map(|day| Datetime64::new(day * SECONDS_PER_DAY, 0))
        .collect();
    let y: Vec<f64> = (0..28)
        .map(|i| if i == 0 { f64::NAN } else { f64::from(i) })
        .collect();
    fig.subplot().scatter(times, y)?;
    Ok(())
}

/// `test_errorbar_mixed_units`: dates of May 2020 against 0 to 9 with error
/// bars of half a day.
fn errorbar_mixed_units(fig: &Figure) -> CaseResult {
    let x: Vec<f64> = (0..10).map(f64::from).collect();
    let y = (0..10)
        .map(|i| DateTime::new(2020, 5, 2 * i + 1, 0, 0, 0, 0))
        .collect::<Result<Vec<_>, _>>()?;
    let options = ErrorbarOptions {
        yerr: Some(TimeDelta::new(0, SECONDS_PER_DAY / 2, 0)?.into()),
        ..ErrorbarOptions::default()
    };
    fig.subplot().errorbar(x, y, options)?;
    Ok(())
}

/// `test_numpy_facade`: hours against miles with a line at 26400 feet and
/// one at 120 minutes, then the axes in seconds and inches, which convert
/// the lines again.
fn numpy_facade(fig: &Figure) -> CaseResult {
    let y = quantities(linspace(0.0, 30.0, 50), QuantityUnit::Miles);
    let x = quantities(linspace(0.0, 5.0, 50), QuantityUnit::Hours);
    let ax = fig.subplot();
    fig.subplots_adjust(SubplotOverrides {
        left: Some(0.15),
        ..SubplotOverrides::default()
    })?;
    ax.plot_with(x, y, LineProps::default().color("tab:blue"))?;
    let feet = Quantity::new(26_400.0, QuantityUnit::Feet);
    ax.axhline(feet, LineProps::default().color("tab:red"));
    let minutes = Quantity::new(120.0, QuantityUnit::Minutes);
    ax.axvline(minutes, LineProps::default().color("tab:green"));
    ax.yaxis().set_units(QuantityUnit::Inches);
    ax.xaxis().set_units(QuantityUnit::Seconds);
    Ok(())
}

/// `test_plot_masked_units`: a masked array in a `Quantity` facade with no
/// converter registered for it (the test fixture restores the registry)
/// plots as the bare masked array, its masked values as gaps.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn plot_masked_units(fig: &Figure) -> CaseResult {
    let masked: Vec<f64> = linspace(-5.0, 5.0, 50)
        .into_iter()
        .map(|x| if -2.0 < x && x < 2.0 { f64::NAN } else { x })
        .collect();
    fig.subplot().plot_y(masked);
    remove_ticks_and_titles(fig);
    Ok(())
}

/// The `jpl_units` tests' bars: at 0, 1 and 2 km, one, two and three days
/// long from 2009-04-26 (ephemeris time), along y (`test_jpl_bar_units`)
/// or along x (`test_jpl_barh_units`), with limits from a day before the
/// bars to a little over a day after the longest one.
fn jpl_bars(fig: &Figure, orientation: Orientation) -> CaseResult {
    let day = Duration::seconds(24.0 * 60.0 * 60.0);
    let km = UnitDbl::new(1.0, UnitName::Kilometres);
    let x = vec![0.0 * km, 1.0 * km, 2.0 * km];
    let w = vec![1.0 * day, 2.0 * day, 3.0 * day];
    let b = Epoch::from_date(DateTime::new(2009, 4, 26, 0, 0, 0, 0)?);
    let (low, high) = (b - 1.0 * day, b + w[2] + 1.001 * day);
    let from_b = BarOptions {
        base: vec![b].into(),
        ..BarOptions::default()
    };
    let ax = fig.subplot();
    match orientation {
        Orientation::Vertical => {
            ax.bar(x, w, from_b)?;
            ax.set_ylim(low, high)?;
        }
        Orientation::Horizontal => {
            ax.barh(x, w, from_b)?;
            ax.set_xlim(low, high)?;
        }
    }
    Ok(())
}

/// `test_jpl_bar_units`: vertical bars.
fn jpl_bar_units(fig: &Figure) -> CaseResult {
    jpl_bars(fig, Orientation::Vertical)
}

/// `test_jpl_barh_units`: horizontal bars.
fn jpl_barh_units(fig: &Figure) -> CaseResult {
    jpl_bars(fig, Orientation::Horizontal)
}
