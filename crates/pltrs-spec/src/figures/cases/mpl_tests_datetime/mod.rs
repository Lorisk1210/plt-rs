//! `figures/cases_mpl_tests_datetime.py`: matplotlib's own tests of
//! plotting dates (`test_datetime.py`), one case per figure, in the
//! `default` style the tests pick (no rc), with `date.converter` set to
//! `concise` where the tests set it. Lines, markers and fills; reference
//! lines, spans and segments; bars, histograms and event rows; and fields,
//! images and statistics each have a submodule.
//!
//! The plotting methods take the dates themselves; option fields, which
//! hold axis numbers, get them converted on their axis
//! (`Axis::convert_value`), as matplotlib converts them, and `timedelta`
//! extents become days.

mod bars;
mod fields;
mod lines;
mod spans;

use pltrs::axes::Axes;
use pltrs::dates::{DateError, DateTime, Datetime64, TimeDelta};
use pltrs::figure::Figure;
use pltrs::gridspec::GridSpecError;
use pltrs::layout::{LayoutEngine, LayoutEngineError};

use crate::figures::{CaseResult, FigureCase};

/// What the tests setting `date.converter` change.
const CONCISE: &[(&str, &str)] = &[("date.converter", "concise")];

/// A case in the `default` style on the default canvas.
const fn datetime_case(name: &'static str, build: fn(&Figure) -> CaseResult) -> FigureCase {
    FigureCase::new(name, build)
}

/// A case of a test setting `date.converter` to `concise`.
const fn concise(name: &'static str, build: fn(&Figure) -> CaseResult) -> FigureCase {
    datetime_case(name, build).with_rc(CONCISE)
}

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [lines::CASES, spans::CASES, bars::CASES, fields::CASES].concat()
}

/// `layout="constrained"`.
fn constrained(fig: &Figure) -> Result<(), LayoutEngineError> {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    Ok(())
}

/// The axes of `fig.subplots(rows, columns)`, row by row.
fn axes(fig: &Figure, rows: usize, columns: usize) -> Result<Vec<Axes>, GridSpecError> {
    Ok(fig.subplots(rows, columns)?.as_slice().to_vec())
}

/// `datetime.datetime(year, month, day, hour)`.
fn hour(year: i32, month: u32, day: u32, hour: u32) -> Result<DateTime, DateError> {
    DateTime::new(year, month, day, hour, 0, 0, 0)
}

/// `datetime.datetime(year, month, day)`.
fn day(year: i32, month: u32, day: u32) -> Result<DateTime, DateError> {
    hour(year, month, day, 0)
}

/// `datetime.timedelta(days=n)`.
fn days(n: i64) -> Result<TimeDelta, DateError> {
    TimeDelta::new(n, 0, 0)
}

/// `start + timedelta(days=i)` for every `i` of `range`.
fn daily(
    start: DateTime,
    range: impl IntoIterator<Item = i64>,
) -> Result<Vec<DateTime>, DateError> {
    range
        .into_iter()
        .map(|i| start.checked_add(days(i)?))
        .collect()
}

/// `numpy.datetime64(text)`.
fn datetime64(text: &str) -> Result<Datetime64, Box<dyn std::error::Error>> {
    Datetime64::parse(text)?.ok_or_else(|| format!("{text} is not a time").into())
}

/// The numbers `range` as `f64`, like `range(...)` passed to matplotlib.
fn numbers(range: impl IntoIterator<Item = i32>) -> Vec<f64> {
    range.into_iter().map(f64::from).collect()
}
