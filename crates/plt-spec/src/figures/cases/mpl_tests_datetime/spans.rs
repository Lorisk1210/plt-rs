//! The reference line, span and segment cases: `test_axhline`,
//! `test_axvline`, `test_axhspan`, `test_axvspan`, `test_hlines` and
//! `test_vlines`.

use plt::axes::Axes;
use plt::dates::DateError;
use plt::prelude::*;
use plt::units::{AxisValue, AxisValues};

use super::{axes, concise, constrained, daily, datetime64, day, days, numbers};
use crate::figures::{CaseResult, FigureCase};

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    concise("mpl_tests_datetime/axhline", axhline),
    concise("mpl_tests_datetime/axvline", axvline),
    concise("mpl_tests_datetime/axhspan", axhspan).sized(10.0, 12.0, 100.0),
    concise("mpl_tests_datetime/axvspan", axvspan).sized(10.0, 12.0, 100.0),
    concise("mpl_tests_datetime/hlines", hlines),
    concise("mpl_tests_datetime/vlines", vlines),
];

/// `test_axhline`: lines at dates, `datetime64` included, over part of
/// the width of axes with date limits.
fn axhline(fig: &Figure) -> CaseResult {
    constrained(fig);
    let ax = axes(fig, 3, 1)?;
    ax[0].set_ylim(day(2020, 4, 1)?, day(2020, 8, 1)?);
    ax[1].set_ylim(datetime64("2005-01-01")?, datetime64("2005-04-01")?);
    ax[2].set_ylim(day(2023, 9, 1)?, day(2023, 11, 1)?);
    let props = LineProps::default;
    ax[0].axhline_between(day(2020, 6, 3)?, 0.5, 0.7, props());
    ax[1].axhline_between(datetime64("2005-02-25T03:30")?, 0.1, 0.9, props());
    ax[2].axhline_between(day(2023, 10, 24)?, 0.4, 0.7, props());
    Ok(())
}

/// `test_axvline`.
fn axvline(fig: &Figure) -> CaseResult {
    constrained(fig);
    let ax = axes(fig, 3, 1)?;
    ax[0].set_xlim(day(2020, 4, 1)?, day(2020, 8, 1)?);
    ax[1].set_xlim(datetime64("2005-01-01")?, datetime64("2005-04-01")?);
    ax[2].set_xlim(day(2023, 9, 1)?, day(2023, 11, 1)?);
    let props = LineProps::default;
    ax[0].axvline_between(day(2020, 6, 3)?, 0.5, 0.7, props());
    ax[1].axvline_between(datetime64("2005-02-25T03:30")?, 0.1, 0.9, props());
    ax[2].axvline_between(day(2023, 10, 24)?, 0.4, 0.7, props());
    Ok(())
}

/// A band between two values of one axis.
type Span = (AxisValue, AxisValue);

/// `test_axhspan` (`horizontal`) and `test_axvspan`: dates and numbers
/// plotted against each other, with bands every other day or unit.
fn spans(fig: &Figure, horizontal: bool) -> CaseResult {
    constrained(fig);
    let start = day(2023, 1, 1)?;
    let dates = daily(start, 0..31)?;
    let values = numbers(1..32);
    let after = |n: i64| start.checked_add(days(n)?);
    let date_spans = (0..31_i64)
        .step_by(2)
        .map(|i| Ok((after(i)?.into(), after(i + 1)?.into())))
        .collect::<Result<Vec<Span>, DateError>>()?;
    let number_spans: Vec<Span> = (0..31_i32)
        .step_by(2)
        .map(|i| (f64::from(i + 1).into(), f64::from(i + 2).into()))
        .collect();
    let ax = axes(fig, 3, 1)?;
    let (first, second) = if horizontal {
        (&number_spans, &date_spans)
    } else {
        (&date_spans, &number_spans)
    };
    let band = |ax: &Axes, (low, high): &Span| {
        let patch = if horizontal {
            ax.axhspan(low.clone(), high.clone())
        } else {
            ax.axvspan(low.clone(), high.clone())
        };
        patch
            .set_facecolor(if horizontal { "green" } else { "red" })
            .set_alpha(Some(0.5));
    };
    let panel = |ax: &Axes,
                 (x, y): (AxisValues, AxisValues),
                 bands: &[Span],
                 [title, xlabel, ylabel]: [&str; 3]|
     -> CaseResult {
        let marked = LineProps::default()
            .marker("o".parse::<plt::markers::Marker>()?)
            .color("blue");
        ax.plot_with(x, y, marked)?;
        for span in bands {
            band(ax, span);
        }
        ax.set_title(title);
        ax.set_xlabel(xlabel);
        ax.set_ylabel(ylabel);
        Ok(())
    };
    let (dates, values) = (AxisValues::from(dates), AxisValues::from(values));
    panel(
        &ax[0],
        (dates.clone(), values.clone()),
        first,
        ["Datetime vs. Number", "Date", "Number"],
    )?;
    panel(
        &ax[1],
        (values, dates.clone()),
        second,
        ["Number vs. Datetime", "Number", "Date"],
    )?;
    panel(
        &ax[2],
        (dates.clone(), dates),
        &date_spans,
        ["Datetime vs. Datetime", "Date", "Date"],
    )
}

/// `test_axhspan`.
fn axhspan(fig: &Figure) -> CaseResult {
    spans(fig, true)
}

/// `test_axvspan`.
fn axvspan(fig: &Figure) -> CaseResult {
    spans(fig, false)
}

/// `test_hlines`: lines at dates and `datetime64` values spanning axes
/// fractions or dates.
fn hlines(fig: &Figure) -> CaseResult {
    constrained(fig);
    let axs = fig.subplots(2, 4)?;
    let dates = (1..6)
        .map(|m| day(2023, m * 2, 10))
        .collect::<Result<Vec<_>, _>>()?;
    let date_start = (1..6)
        .map(|i| day(2023, 6, 5 * i))
        .collect::<Result<Vec<_>, _>>()?;
    let date_end = (1..6)
        .map(|i| day(2023, 7, 5 * i))
        .collect::<Result<Vec<_>, _>>()?;
    let np_dates = [
        "2023-03-08",
        "2023-04-09",
        "2023-05-13",
        "2023-07-28",
        "2023-12-24",
    ]
    .into_iter()
    .map(datetime64)
    .collect::<Result<Vec<_>, _>>()?;
    let (may_10, may_31) = (vec![day(2020, 5, 10)?], vec![day(2020, 5, 31)?]);
    let rising = (vec![0.1, 0.2, 0.3, 0.4, 0.5], vec![0.5, 0.6, 0.7, 0.8, 0.9]);
    axs[(0, 0)].hlines(dates.clone(), rising.0.clone(), rising.1.clone())?;
    axs[(0, 1)].hlines(dates.clone(), may_10.clone(), may_31.clone())?;
    axs[(0, 2)].hlines(dates.clone(), date_start.clone(), date_end.clone())?;
    axs[(0, 3)].hlines(dates, [0.45], [0.65])?;
    axs[(1, 0)].hlines(np_dates.clone(), rising.1, rising.0)?;
    axs[(1, 2)].hlines(np_dates.clone(), date_start, date_end)?;
    axs[(1, 1)].hlines(np_dates.clone(), may_10, may_31)?;
    axs[(1, 3)].hlines(np_dates, [0.45], [0.65])?;
    Ok(())
}

/// `test_vlines`: lines at dates over axes fractions, at numbers between
/// `datetime64` values, and at dates between dates.
fn vlines(fig: &Figure) -> CaseResult {
    constrained(fig);
    let ax = axes(fig, 3, 1)?;
    ax[0].set_xlim(day(2023, 1, 1)?, day(2023, 6, 30)?);
    ax[0].vlines(
        vec![day(2023, 2, 10)?, day(2023, 5, 18)?, day(2023, 6, 6)?],
        [0.0, 0.25, 0.5],
        [0.25, 0.5, 0.75],
    )?;
    ax[1].set_xlim(0.0, 0.5);
    ax[1].vlines(
        [0.3, 0.35],
        vec![datetime64("2023-03-20")?, datetime64("2023-03-31")?],
        vec![datetime64("2023-05-01")?, datetime64("2023-05-16")?],
    )?;
    ax[2].set_xlim(day(2023, 7, 1)?, day(2023, 12, 31)?);
    ax[2].vlines(
        vec![day(2023, 9, 1)?, day(2023, 12, 10)?],
        vec![day(2023, 1, 15)?],
        vec![day(2023, 1, 30)?],
    )?;
    Ok(())
}
