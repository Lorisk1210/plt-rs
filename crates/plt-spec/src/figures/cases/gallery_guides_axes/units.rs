//! `axes_units`: dates and strings as coordinates, with the default and
//! the concise date converter, and floats on date and categorical axes.

use plt::dates::{DateFormatter, Datetime64, RRuleLocator};
use plt::figure::Share;
use plt::layout::LayoutEngine;
use plt::prelude::*;
use plt::text::TextProps;

use crate::figures::cases::common::styled_box;
use crate::figures::{CaseResult, FigureCase};

/// The fruit of the categorical figures and their counts.
const FRUIT: [(&str, f64); 4] = [
    ("apple", 10.0),
    ("orange", 15.0),
    ("lemon", 5.0),
    ("lime", 20.0),
];

/// The rc of the figures drawn after the page switches to the concise
/// date converter.
const CONCISE: &[(&str, &str)] = &[("date.converter", "concise")];

/// A numpy `datetime64` of a date string.
fn day(text: &str) -> Result<Datetime64, Box<dyn std::error::Error>> {
    Ok(Datetime64::parse(text)?.ok_or("not a time")?)
}

/// Plots the day count against the days of 1980 up to June 24
/// (`np.arange('1980-01-01', '1980-06-25', dtype='datetime64[D]')`).
fn days_of_1980(ax: &Axes) -> CaseResult {
    let (first, last) = (day("1980-01-01")?.seconds(), day("1980-06-25")?.seconds());
    let time: Vec<Datetime64> = (first..last)
        .step_by(86_400)
        .map(|seconds| Datetime64::new(seconds, 0))
        .collect();
    let count = arange(0.0, time.len() as f64, 1.0);
    ax.plot(time, count)?;
    Ok(())
}

/// One constrained axes.
fn constrained_axes(fig: &Figure) -> Axes {
    fig.set_layout_engine(LayoutEngine::constrained());
    fig.subplot()
}

/// Bars of the fruit counts.
fn fruit_bars(ax: &Axes) -> CaseResult {
    let (names, values): (Vec<&str>, Vec<f64>) = FRUIT.into_iter().unzip();
    ax.bar(names, values, BarOptions::default())?;
    Ok(())
}

fn axes_units_1(fig: &Figure) -> CaseResult {
    days_of_1980(&constrained_axes(fig))
}

fn axes_units_2(fig: &Figure) -> CaseResult {
    let ax = constrained_axes(fig);
    days_of_1980(&ax)?;
    ax.plot_fmt([0.0], [0.0], "d")?;
    ax.text(0.0, 0.0, " Float x=0").set_rotation(45.0);
    Ok(())
}

fn axes_units_3(fig: &Figure) -> CaseResult {
    let ax = constrained_axes(fig);
    days_of_1980(&ax)?;
    ax.xaxis()
        .set_major_locator(RRuleLocator::months(&[1, 3, 5, 7, 9, 11], &[1], 1))
        .set_major_formatter(DateFormatter::new("%b"));
    ax.set_xlabel("1980");
    Ok(())
}

fn axes_units_5(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained());
    let axs = fig.subplots(2, 1)?;
    for ax in axs.as_slice() {
        days_of_1980(ax)?;
    }
    let limits = axs[(0, 0)]
        .xaxis()
        .convert_units(vec![day("1980-02-01")?, day("1980-04-01")?]);
    axs[(0, 0)].set_xlim(limits[0], limits[1]);
    axs[(1, 0)].set_xlim(3683.0, 3683.0 + 60.0);
    Ok(())
}

fn axes_units_6(fig: &Figure) -> CaseResult {
    let (names, values): (Vec<&str>, Vec<f64>) = FRUIT.into_iter().unzip();
    fig.set_layout_engine(LayoutEngine::constrained());
    let axs = fig.subplots_shared(1, 3, Share::None, Share::All)?;
    axs[(0, 0)].bar(names.clone(), values.clone(), BarOptions::default())?;
    axs[(0, 1)].scatter(names.clone(), values.clone())?;
    axs[(0, 2)].plot(names, values)?;
    fig.suptitle("Categorical Plotting");
    Ok(())
}

fn axes_units_7(fig: &Figure) -> CaseResult {
    let ax = constrained_axes(fig);
    fruit_bars(&ax)?;
    ax.scatter(vec!["lemon", "apple"], vec![7.0, 12.0])?;
    ax.plot_with(
        vec!["pear", "orange", "apple", "lemon"],
        vec![13.0, 10.0, 7.0, 12.0],
        LineProps::default().color("C1"),
    )?;
    Ok(())
}

fn axes_units_8(fig: &Figure) -> CaseResult {
    let ax = constrained_axes(fig);
    fruit_bars(&ax)?;
    let marker = LineProps::fmt("d")?.color("C1");
    for x in [0.0, 2.0, 4.0, 2.5] {
        ax.plot_with([x], [2.0], marker.clone())?;
        ax.text(x, 3.0, format!("Float x={x}"))
            .update_props(TextProps::default().rotation(70.0).color("C1"))
            .set_bbox(styled_box(
                "round",
                PatchProps::default().color("white").alpha(0.7),
            )?);
    }
    Ok(())
}

fn axes_units_9(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained());
    let axs = fig.subplots(2, 1)?;
    let ax = &axs[(0, 0)];
    fruit_bars(ax)?;
    let limits = ax.xaxis().convert_units(vec!["orange", "lemon"]);
    ax.set_xlim(limits[0], limits[1]);
    ax.set_xlabel("limits set with categories");
    let ax = &axs[(1, 0)];
    fruit_bars(ax)?;
    ax.set_xlim(0.5, 2.5);
    ax.set_xlabel("limits set with floats");
    Ok(())
}

fn axes_units_10(fig: &Figure) -> CaseResult {
    let ax = constrained_axes(fig);
    let x: Vec<String> = (0..100).map(|i| i.to_string()).collect();
    ax.plot(x, arange(0.0, 100.0, 1.0))?;
    ax.set_xlabel("x is list of strings");
    Ok(())
}

fn axes_units_11(fig: &Figure) -> CaseResult {
    let ax = constrained_axes(fig);
    let x = arange(0.0, 100.0, 1.0);
    ax.plot(&x, &x)?;
    ax.set_xlabel("x is array of floats");
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_guides_axes/axes_units_1", axes_units_1).sized(5.4, 2.0, 100.0),
    FigureCase::new("gallery_guides_axes/axes_units_2", axes_units_2).sized(5.4, 2.0, 100.0),
    FigureCase::new("gallery_guides_axes/axes_units_3", axes_units_3).sized(5.4, 2.0, 100.0),
    FigureCase::new("gallery_guides_axes/axes_units_4", axes_units_1)
        .sized(5.4, 2.0, 100.0)
        .with_rc(CONCISE),
    FigureCase::new("gallery_guides_axes/axes_units_5", axes_units_5)
        .sized(5.4, 3.0, 100.0)
        .with_rc(CONCISE),
    FigureCase::new("gallery_guides_axes/axes_units_6", axes_units_6)
        .sized(7.0, 3.0, 100.0)
        .with_rc(CONCISE),
    FigureCase::new("gallery_guides_axes/axes_units_7", axes_units_7)
        .sized(5.0, 3.0, 100.0)
        .with_rc(CONCISE),
    FigureCase::new("gallery_guides_axes/axes_units_8", axes_units_8)
        .sized(5.0, 3.0, 100.0)
        .with_rc(CONCISE),
    FigureCase::new("gallery_guides_axes/axes_units_9", axes_units_9)
        .sized(5.0, 5.0, 100.0)
        .with_rc(CONCISE),
    FigureCase::new("gallery_guides_axes/axes_units_10", axes_units_10)
        .sized(5.4, 2.5, 100.0)
        .with_rc(CONCISE),
    FigureCase::new("gallery_guides_axes/axes_units_11", axes_units_11)
        .sized(5.4, 2.5, 100.0)
        .with_rc(CONCISE),
];
