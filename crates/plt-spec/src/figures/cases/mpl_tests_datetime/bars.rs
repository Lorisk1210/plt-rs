//! The bar, histogram and event cases: `test_bar`, `test_barh`,
//! `test_bar_label`, `test_broken_barh`, `test_hist` (two figures),
//! `test_stairs` and `test_eventplot`.

use plt::axes::{
    BandAlign, BarLabelOptions, BarLabelType, BarOptions, EventplotOptions, HistPlotOptions,
    StairsOptions,
};
use plt::dates::{DateError, DateTime, TimeDelta};
use plt::patches::StepBaseline;
use plt::prelude::*;
use plt::stats::{Bins, HistOptions};

use super::{axes, concise, constrained, daily, datetime_case, day, days, numbers};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    concise("mpl_tests_datetime/bar", bar),
    concise("mpl_tests_datetime/barh", barh),
    datetime_case("mpl_tests_datetime/bar_label", bar_label).sized(10.0, 8.0, 100.0),
    concise("mpl_tests_datetime/broken_barh", broken_barh),
    concise("mpl_tests_datetime/hist_1", hist_1),
    concise("mpl_tests_datetime/hist_2", hist_2),
    concise("mpl_tests_datetime/stairs", stairs),
    concise("mpl_tests_datetime/eventplot", eventplot),
];

/// The four dates of `test_bar` and `test_eventplot`.
fn summer_dates() -> Result<Vec<DateTime>, DateError> {
    [(6, 30), (7, 22), (8, 3), (9, 14)]
        .into_iter()
        .map(|(month, d)| day(2020, month, d))
        .collect()
}

/// Whole days from `from` to `to` (`to - from` as a `timedelta`, in the
/// days a date axis counts).
fn days_between(from: DateTime, to: DateTime) -> f64 {
    (to.timestamp_micros() - from.timestamp_micros()) as f64 / 86_400_000_000.0
}

/// `test_bar`: bars at dates four days wide, and bars from a date as
/// high as durations.
fn bar(fig: &Figure) -> CaseResult {
    constrained(fig);
    let ax = axes(fig, 2, 1)?;
    let x_dates = summer_dates()?;
    let width = BarOptions {
        thickness: vec![4.0],
        ..BarOptions::default()
    };
    ax[0].bar(x_dates.clone(), [8800.0, 2600.0, 8500.0, 7400.0], width)?;
    let x = day(2020, 6, 1)?;
    let heights: Vec<f64> = x_dates.iter().map(|&d| days_between(x, d)).collect();
    let from_x = BarOptions {
        base: vec![ax[1].yaxis().convert_value(x)],
        ..BarOptions::default()
    };
    ax[1].bar(numbers(0..4), heights, from_x)?;
    Ok(())
}

/// `test_barh`: bars at birth dates ten days high, and bars from the new
/// year as long as the time to the birth dates.
fn barh(fig: &Figure) -> CaseResult {
    constrained(fig);
    let ax = axes(fig, 2, 1)?;
    let birth_dates = [(4, 10), (5, 30), (10, 12), (11, 15)]
        .into_iter()
        .map(|(month, d)| day(2020, month, d))
        .collect::<Result<Vec<_>, _>>()?;
    let (year_start, year_end) = (day(2020, 1, 1)?, day(2020, 12, 31)?);
    ax[0].set_xlabel("Age");
    ax[0].set_ylabel("Birth Date");
    let ten_days = BarOptions {
        thickness: vec![10.0],
        ..BarOptions::default()
    };
    ax[0].barh(birth_dates.clone(), [21.0, 53.0, 20.0, 24.0], ten_days)?;
    ax[1].set_xlim(year_start, year_end);
    ax[1].set_xlabel("Birth Date");
    ax[1].set_ylabel("Order of Birth Dates");
    let widths: Vec<f64> = birth_dates
        .iter()
        .map(|&d| days_between(year_start, d))
        .collect();
    let from_new_year = BarOptions {
        base: vec![ax[1].xaxis().convert_value(year_start)],
        ..BarOptions::default()
    };
    ax[1].barh(numbers(0..4), widths, from_new_year)?;
    Ok(())
}

/// `test_bar_label`: bars at five days labelled with percentages.
fn bar_label(fig: &Figure) -> CaseResult {
    constrained(fig);
    let ax = fig.subplot();
    let dates = daily(day(2023, 1, 1)?, 0..5)?;
    let values = [10.0, 20.0, 15.0, 25.0, 30.0];
    let bars = ax.bar(dates, values, BarOptions::default())?;
    let options = BarLabelOptions {
        labels: values.iter().map(|value| format!("{value}%")).collect(),
        label_type: BarLabelType::Edge,
        ..BarLabelOptions::default()
    };
    ax.bar_label(&bars, options)?;
    Ok(())
}

/// `test_broken_barh`: bands of date ranges.
fn broken_barh(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let ranges = |spans: [(u32, i64); 2]| -> Result<Vec<(DateTime, TimeDelta)>, DateError> {
        spans
            .into_iter()
            .map(|(d, length)| Ok((day(2023, 1, d)?, days(length)?)))
            .collect()
    };
    ax.broken_barh(&ranges([(4, 2), (8, 3)])?, (10.0, 9.0), BandAlign::Bottom)
        .set_facecolor("tab:blue");
    ax.broken_barh(&ranges([(2, 1), (4, 4)])?, (20.0, 9.0), BandAlign::Bottom)
        .set_facecolor("tab:red");
    Ok(())
}

/// One figure of `test_hist`: the same 30 days, weighted three ways, in
/// `bins` (dates are converted on the x axis first).
fn hist(fig: &Figure, bins: impl Fn(&plt::axes::Axes) -> Result<Bins, DateError>) -> CaseResult {
    constrained(fig);
    let days_of_month = daily(day(2023, 10, 1)?, 0..30)?;
    let mut rng = SpecRng::new(7);
    let weights: Vec<Vec<f64>> = (0..3)
        .map(|_| {
            rng.integers(30, 1, 10)
                .into_iter()
                .map(|w| w as f64)
                .collect()
        })
        .collect();
    for (ax, weights) in axes(fig, 3, 1)?.iter().zip(weights) {
        let x = ax.xaxis().convert_units(days_of_month.clone());
        let weights = [weights];
        let options = HistPlotOptions {
            layout: HistOptions {
                bins: Some(bins(ax)?),
                weights: Some(&weights),
                ..HistOptions::default()
            },
            ..HistPlotOptions::default()
        };
        ax.hist(&[x], options)?;
    }
    Ok(())
}

/// `test_hist`, its first figure: ten bins.
fn hist_1(fig: &Figure) -> CaseResult {
    hist(fig, |_| Ok(Bins::Count(10)))
}

/// `test_hist`, its second figure: one bin per day.
fn hist_2(fig: &Figure) -> CaseResult {
    hist(fig, |ax| {
        let edges = daily(day(2023, 10, 1)?, 0..31)?;
        Ok(Bins::Edges(ax.xaxis().convert_units(edges)))
    })
}

/// `test_stairs`: steps between daily edges, from a 1980 baseline for
/// the steps of dates.
fn stairs(fig: &Figure) -> CaseResult {
    constrained(fig);
    let start = day(2023, 12, 1)?;
    let baseline = day(1980, 1, 1)?;
    let bin_edges = daily(start, 0..31)?;
    let mut rng = SpecRng::new(123_456);
    let values1: Vec<f64> = rng
        .integers(30, 1, 100)
        .into_iter()
        .map(|v| v as f64)
        .collect();
    let values2 = daily(start, rng.integers(30, 1, 10_000))?;
    let values3 = daily(start, rng.integers(30, -10_000, 10_000))?;
    let ax = axes(fig, 3, 1)?;
    let date_edges = |ax: &plt::axes::Axes| Some(ax.xaxis().convert_units(bin_edges.clone()));
    let from_1980 = |ax: &plt::axes::Axes| StepBaseline::Value(ax.yaxis().convert_value(baseline));
    ax[0].stairs(
        values1,
        StairsOptions {
            edges: date_edges(&ax[0]),
            ..StairsOptions::default()
        },
    )?;
    let values2 = ax[1].yaxis().convert_units(values2);
    ax[1].stairs(
        values2,
        StairsOptions {
            edges: Some(numbers(0..31)),
            baseline: from_1980(&ax[1]),
            ..StairsOptions::default()
        },
    )?;
    let values3 = ax[2].yaxis().convert_units(values3);
    ax[2].stairs(
        values3,
        StairsOptions {
            edges: date_edges(&ax[2]),
            baseline: from_1980(&ax[2]),
            ..StairsOptions::default()
        },
    )?;
    Ok(())
}

/// `test_eventplot`: one row of dates, then three rows of random dates
/// at numbers and at dates.
fn eventplot(fig: &Figure) -> CaseResult {
    constrained(fig);
    let ax = axes(fig, 3, 1)?;
    ax[0].eventplot(&[summer_dates()?], EventplotOptions::default())?;
    let mut rng = SpecRng::new(19_680_801);
    let start = day(2020, 7, 1)?;
    let seconds = (day(2020, 10, 15)?.timestamp_micros() - start.timestamp_micros()) / 1_000_000;
    let rows = [30, 10, 50]
        .into_iter()
        .map(|n| {
            rng.integers(n, 0, seconds)
                .into_iter()
                .map(|s| start.checked_add(TimeDelta::new(0, s, 0)?))
                .collect::<Result<Vec<_>, DateError>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    let rows_at = |lineoffsets: Vec<f64>| EventplotOptions {
        colors: vec!["C1".into(), "C2".into(), "C3".into()],
        lineoffsets,
        linelengths: vec![5.0, 2.0, 3.0],
        ..EventplotOptions::default()
    };
    ax[1].eventplot(&rows, rows_at(vec![1.0, 6.0, 8.0]))?;
    let date_offsets = [(7, 1), (7, 15), (8, 1)]
        .into_iter()
        .map(|(month, d)| day(2020, month, d))
        .collect::<Result<Vec<_>, _>>()?;
    let offsets = ax[2].yaxis().convert_units(date_offsets);
    ax[2].eventplot(&rows, rows_at(offsets))?;
    Ok(())
}
