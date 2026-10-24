//! `figures/cases_mpl_tests_units.py`: matplotlib's own tests of unit
//! conversion (`test_units.py`) that draw dates and only check that
//! drawing works, one case per figure, in the test suite's default style.

use plt::axes::ErrorbarOptions;
use plt::dates::{DateTime, Datetime64, TimeDelta};
use plt::figure::Figure;

use super::mpl_tests::classic;
use crate::figures::{CaseResult, FigureCase};

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    vec![
        classic("mpl_tests_units/empty_arrays", empty_arrays),
        classic(
            "mpl_tests_units/scatter_element0_masked",
            scatter_element0_masked,
        ),
        classic("mpl_tests_units/errorbar_mixed_units", errorbar_mixed_units),
    ]
}

/// Seconds in a day.
const SECONDS_PER_DAY: i64 = 86_400;

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
