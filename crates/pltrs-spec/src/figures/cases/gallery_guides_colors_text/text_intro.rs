//! The text tutorial: axis labels and their padding, position and fonts,
//! titles and their location and padding, and tick locators and
//! formatters, for numbers and dates (`text_intro`).

use std::f64::consts::PI;

use pltrs::axes::{AxisName, LabelLoc, PlotError, TickParams, TitleLocation};
use pltrs::dates::{DateFormatter, DateTime, RRuleLocator, TimeDelta};
use pltrs::fonts::{FontProperties, FontSize, FontStyle, FontWeight, RelativeSize};
use pltrs::gridspec::SubplotOverrides;
use pltrs::layout::LayoutEngine;
use pltrs::prelude::*;
use pltrs::ticker::{
    AutoLocator, FixedLocator, FormatStrFormatter, FuncFormatter, MaxNLocator, Nbins,
    StrMethodFormatter,
};

use crate::figures::cases::common::{cumsum, map};
use crate::figures::{CaseResult, FigureCase};

/// The tutorial's damped oscillation: `cos(2πx) e^{-x}` at 100 points of
/// `[0, 5]` (`x1`, `y1`).
fn oscillation() -> (Vec<f64>, Vec<f64>) {
    let x = linspace(0.0, 5.0, 100);
    let y = map(&x, |x| (2.0 * PI * x).cos() * (-x).exp());
    (x, y)
}

/// The oscillation times `scale` on the only axes of the figure, with
/// room left for the labels.
fn oscillation_axes(fig: &Figure, scale: f64) -> Result<Axes, Box<dyn std::error::Error>> {
    let (x, y) = oscillation();
    let ax = fig.subplot();
    fig.subplots_adjust(SubplotOverrides {
        bottom: Some(0.15),
        left: Some(0.2),
        ..SubplotOverrides::default()
    })?;
    ax.plot(x, map(&y, |y| y * scale))?;
    Ok(ax)
}

/// `oscillation_axes` labelled with time and voltage, the y label `pad`
/// points from the tick labels when given.
fn labelled(fig: &Figure, scale: f64, pad: Option<f64>) -> CaseResult {
    let ax = oscillation_axes(fig, scale)?;
    ax.set_xlabel("Time (s)");
    ax.set_ylabel("Damped oscillation (V)");
    if let Some(pad) = pad {
        ax.yaxis().set_label_pad(pad);
    }
    Ok(())
}

fn text_intro_2(fig: &Figure) -> CaseResult {
    labelled(fig, 1.0, None)
}

fn text_intro_3(fig: &Figure) -> CaseResult {
    labelled(fig, 10000.0, None)
}

fn text_intro_4(fig: &Figure) -> CaseResult {
    labelled(fig, 10000.0, Some(18.0))
}

fn text_intro_5(fig: &Figure) -> CaseResult {
    let ax = oscillation_axes(fig, 1.0)?;
    ax.set_xlabel("Time (s)");
    ax.xaxis().set_label_loc(LabelLoc::Start);
    ax.set_ylabel("Damped oscillation (V)");
    Ok(())
}

fn text_intro_6(fig: &Figure) -> CaseResult {
    let font = FontProperties {
        families: vec!["Times New Roman".to_owned()],
        style: FontStyle::Italic,
        ..FontProperties::from_rc(&pltrs::rc::params())
    };
    let ax = oscillation_axes(fig, 1.0)?;
    ax.set_xlabel("Time (s)");
    ax.xaxis()
        .set_label_fontsize(FontSize::Relative(RelativeSize::Large))
        .set_label_weight(FontWeight::BOLD);
    ax.set_ylabel("Damped oscillation (V)");
    ax.yaxis().set_label_font(font);
    Ok(())
}

fn text_intro_7(fig: &Figure) -> CaseResult {
    let (x, y) = oscillation();
    let ax = fig.subplot();
    fig.subplots_adjust(SubplotOverrides {
        bottom: Some(0.2),
        left: Some(0.2),
        ..SubplotOverrides::default()
    })?;
    ax.plot(x, cumsum(&map(&y, |y| y * y)))?;
    ax.set_xlabel("Time (s) \n This was a long experiment");
    ax.set_ylabel(r"$\int\ Y^2\ dt\ \ (V^2 s)$");
    Ok(())
}

fn text_intro_8(fig: &Figure) -> CaseResult {
    let (x, y) = oscillation();
    fig.set_layout_engine(LayoutEngine::tight())?;
    let axs = fig.subplots(3, 1)?;
    for (ax, (location, name)) in axs.as_slice().iter().zip([
        (TitleLocation::Center, "center"),
        (TitleLocation::Left, "left"),
        (TitleLocation::Right, "right"),
    ]) {
        ax.plot(&x, &y)?;
        ax.set_title_at(location, format!("Title with loc at {name}"));
    }
    Ok(())
}

fn text_intro_9(fig: &Figure) -> CaseResult {
    let (x, y) = oscillation();
    let ax = fig.subplot();
    fig.subplots_adjust(SubplotOverrides {
        top: Some(0.8),
        ..SubplotOverrides::default()
    })?;
    ax.plot(x, y)?;
    ax.set_title("Vertically offset title");
    ax.set_title_pad(30.0);
    Ok(())
}

/// The ticks the tutorial sets: `np.arange(0., 8.1, 2.)`.
fn ticks() -> Vec<f64> {
    arange(0.0, 8.1, 2.0)
}

/// Two tight rows of the oscillation; returns their axes.
fn oscillation_pair(fig: &Figure) -> Result<(Axes, Axes), Box<dyn std::error::Error>> {
    let (x, y) = oscillation();
    fig.set_layout_engine(LayoutEngine::tight())?;
    let axs = fig.subplots(2, 1)?;
    for ax in axs.as_slice() {
        ax.plot(&x, &y)?;
    }
    Ok((axs[(0, 0)].clone(), axs[(1, 0)].clone()))
}

/// Gives `ax` the x limits of `like`.
fn same_xlim(ax: &Axes, like: &Axes) -> Result<(), PlotError> {
    let (left, right) = like.xlim();
    ax.set_xlim(left, right)?;
    Ok(())
}

fn text_intro_10(fig: &Figure) -> CaseResult {
    let (_, lower) = oscillation_pair(fig)?;
    lower.xaxis().set_ticks(ticks());
    Ok(())
}

fn text_intro_11(fig: &Figure) -> CaseResult {
    let (upper, lower) = oscillation_pair(fig)?;
    let tickla: Vec<String> = ticks().iter().map(|tick| format!("{tick:1.2}")).collect();
    lower.xaxis().set_ticks(ticks());
    lower.xaxis().set_ticklabels(tickla)?;
    same_xlim(&lower, &upper)?;
    Ok(())
}

fn text_intro_12(fig: &Figure) -> CaseResult {
    let (upper, lower) = oscillation_pair(fig)?;
    lower.xaxis().set_ticks(ticks());
    lower
        .xaxis()
        .set_major_formatter(StrMethodFormatter::new("{x:1.1f}")?);
    same_xlim(&lower, &upper)?;
    Ok(())
}

fn text_intro_13(fig: &Figure) -> CaseResult {
    let (_, lower) = oscillation_pair(fig)?;
    lower
        .xaxis()
        .set_major_locator(FixedLocator::new(ticks()))
        .set_major_formatter(StrMethodFormatter::new("±{x}°")?);
    Ok(())
}

fn text_intro_14(fig: &Figure) -> CaseResult {
    let (x, y) = oscillation();
    fig.set_layout_engine(LayoutEngine::tight())?;
    let axs = fig.subplots(2, 2)?;
    for ax in axs.as_slice() {
        ax.plot(map(&x, |x| x * 10.0), &y)?;
    }
    axs[(0, 1)]
        .xaxis()
        .set_major_locator(MaxNLocator::new(Nbins::Auto).with_steps(&[1.0, 4.0, 10.0])?)
        .set_major_formatter(FormatStrFormatter::new("%1.1f")?);
    axs[(1, 0)]
        .xaxis()
        .set_major_formatter(FormatStrFormatter::new("%1.5f")?)
        .set_major_locator(AutoLocator::new());
    axs[(1, 1)]
        .xaxis()
        .set_major_formatter(FormatStrFormatter::new("%1.5f")?)
        .set_major_locator(MaxNLocator::new(Nbins::Fixed(4)));
    Ok(())
}

/// `formatoddticks`: odd tick positions with two decimals, the others
/// unlabelled.
fn formatoddticks(x: f64, _pos: Option<usize>) -> String {
    if x.rem_euclid(2.0) == 0.0 {
        String::new()
    } else {
        format!("{x:1.2}")
    }
}

fn text_intro_15(fig: &Figure) -> CaseResult {
    let (x, y) = oscillation();
    fig.set_layout_engine(LayoutEngine::tight())?;
    let ax = fig.subplot();
    ax.plot(x, y)?;
    ax.xaxis()
        .set_major_formatter(FuncFormatter::new(formatoddticks))
        .set_major_locator(MaxNLocator::new(Nbins::Fixed(6)));
    Ok(())
}

/// The oscillation over 100 days of 2017, from a second past midnight on
/// New Year's Day, on tight axes prepared by `setup`, its x tick labels
/// turned by 70°.
fn daily(fig: &Figure, setup: impl FnOnce(&Axes) -> CaseResult) -> CaseResult {
    let (x, y) = oscillation();
    let base = DateTime::new(2017, 1, 1, 0, 0, 1, 0)?;
    let time = (0..x.len())
        .map(|day| base.checked_add(TimeDelta::new(day as i64, 0, 0)?))
        .collect::<Result<Vec<_>, _>>()?;
    fig.set_layout_engine(LayoutEngine::tight())?;
    let ax = fig.subplot();
    setup(&ax)?;
    ax.plot(time, y)?;
    ax.tick_params(TickParams {
        axis: Some(AxisName::X),
        labelrotation: Some(70.0),
        ..TickParams::default()
    });
    Ok(())
}

fn text_intro_16(fig: &Figure) -> CaseResult {
    daily(fig, |_| Ok(()))
}

fn text_intro_17(fig: &Figure) -> CaseResult {
    daily(fig, |ax| {
        ax.xaxis()
            .set_major_locator(RRuleLocator::days(&[1, 15], 1)?)
            .set_major_formatter(DateFormatter::new("%b %d"));
        Ok(())
    })
}

/// The cases of this module; the first figure of `text_intro` is
/// `gallery_text/text_commands`.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_guides_colors_text/text_intro_2", text_intro_2).sized(5.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/text_intro_3", text_intro_3).sized(5.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/text_intro_4", text_intro_4).sized(5.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/text_intro_5", text_intro_5).sized(5.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/text_intro_6", text_intro_6).sized(5.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/text_intro_7", text_intro_7).sized(5.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/text_intro_8", text_intro_8).sized(5.0, 6.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/text_intro_9", text_intro_9).sized(5.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/text_intro_10", text_intro_10)
        .sized(5.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/text_intro_11", text_intro_11)
        .sized(5.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/text_intro_12", text_intro_12)
        .sized(5.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/text_intro_13", text_intro_13)
        .sized(5.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/text_intro_14", text_intro_14)
        .sized(8.0, 5.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/text_intro_15", text_intro_15)
        .sized(5.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/text_intro_16", text_intro_16)
        .sized(5.0, 3.0, 100.0),
    FigureCase::new("gallery_guides_colors_text/text_intro_17", text_intro_17)
        .sized(5.0, 3.0, 100.0),
];
