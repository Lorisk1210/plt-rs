//! Tick labels and their placement: aligned and rotated labels, ticks on
//! the top and right, a customised figure, nested tick labels on
//! secondary axes, and string values making categorical ticks
//! (`align_ticklabels`, `ticklabels_rotation`, `ticks_top_right`,
//! `fig_axes_customize_simple`, `multilevel_ticks` and `ticks_too_many`).

use plt::axes::{AxisName, SecondaryLocation, TickLabelProps, TickLevel, TickParams};
use plt::dates::{DateFormatter, Datetime64, RRuleLocator};
use plt::fonts::FontSize;
use plt::layout::{LayoutEngine, LayoutEngineError};
use plt::prelude::*;
use plt::spines::SpineSide;
use plt::text::{HAlign, TextRotationMode as RotationMode};

use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// The seed of the examples' random data.
const SEED: u64 = 19_680_801;

/// One axes in a constrained figure.
fn constrained(fig: &Figure) -> Result<Axes, LayoutEngineError> {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    Ok(fig.subplot())
}

/// Tick parameters of one axis.
fn axis_params(axis: AxisName) -> TickParams {
    TickParams {
        axis: Some(axis),
        ..TickParams::default()
    }
}

fn align_ticklabels(fig: &Figure) -> CaseResult {
    let population = [
        ("Sydney", 5.2),
        ("Mexico City", 8.8),
        ("São Paulo", 12.2),
        ("Istanbul", 15.9),
        ("Lagos", 15.9),
        ("Shanghai", 21.9),
    ];
    let (cities, millions): (Vec<&str>, Vec<f64>) = population.into_iter().unzip();
    let ax = constrained(fig)?;
    ax.barh(cities, millions, BarOptions::default())?;
    ax.set_xlabel("Population (in millions)");
    ax.yaxis().set_ticklabel_props(
        TickLevel::Major,
        TickLabelProps {
            halign: Some(HAlign::Left),
            ..TickLabelProps::default()
        },
    );
    ax.tick_params(&TickParams {
        pad: Some(70.0),
        ..axis_params(AxisName::Y)
    });
    Ok(())
}

fn ticklabels_rotation(fig: &Figure) -> CaseResult {
    let population = [
        ("India", 1417.492),
        ("China", 1404.89),
        ("United States", 341.784_857),
        ("Indonesia", 284.438_782),
        ("Pakistan", 241.499_431),
        ("Nigeria", 223.8),
        ("Brazil", 213.421_037),
        ("Bangladesh", 169.828_911),
        ("Russia", 146.028_325),
        ("Mexico", 131.001_723),
        ("Japan", 122.95),
        ("Philippines", 114.1236),
        ("DR Congo", 112.832),
        ("Ethiopia", 111.652_998),
        ("Egypt", 107.868_296),
        ("Vietnam", 102.3),
    ];
    let (countries, millions): (Vec<&str>, Vec<f64>) = population.into_iter().unzip();
    let ax = constrained(fig)?;
    ax.bar(countries, millions, BarOptions::default())?;
    ax.tick_params(&TickParams {
        labelrotation: Some(45.0),
        labelrotation_mode: Some(RotationMode::XTick),
        ..axis_params(AxisName::X)
    });
    ax.set_ylabel("population (millions)");
    Ok(())
}

fn ticks_top_right(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot_y((0..10).map(f64::from).collect::<Vec<_>>());
    ax.tick_params(&TickParams {
        top: Some(true),
        labeltop: Some(true),
        bottom: Some(false),
        labelbottom: Some(false),
        right: Some(true),
        labelright: Some(true),
        left: Some(false),
        labelleft: Some(false),
        ..TickParams::default()
    });
    ax.xaxis().set_label_position(SpineSide::Top)?;
    ax.yaxis().set_label_position(SpineSide::Right)?;
    ax.set_xlabel("X label");
    ax.set_ylabel("Y label");
    Ok(())
}

fn fig_axes_customize_simple(fig: &Figure) -> CaseResult {
    fig.set_facecolor(Color::from("lightgoldenrodyellow").resolve()?);
    let ax1 = fig.add_axes([0.1, 0.3, 0.4, 0.4]);
    ax1.set_facecolor("lightslategray");
    ax1.tick_params(&TickParams {
        labelcolor: Some("tab:red".into()),
        labelrotation: Some(45.0),
        labelsize: Some(FontSize::Points(16.0)),
        ..axis_params(AxisName::X)
    });
    ax1.tick_params(&TickParams {
        color: Some("tab:green".into()),
        length: Some(25.0),
        width: Some(3.0),
        ..axis_params(AxisName::Y)
    });
    Ok(())
}

/// A secondary x axis on top of the host's own, at its bottom.
fn bottom_axis(ax: &Axes, fraction: f64) -> Result<Axes, Box<dyn std::error::Error>> {
    Ok(ax.secondary_xaxis(SecondaryLocation::Fraction(fraction), None)?)
}

fn multilevel_ticks_1(fig: &Figure) -> CaseResult {
    let ax = constrained(fig)?;
    ax.plot_y((0..30).map(f64::from).collect::<Vec<_>>());
    let sec = bottom_axis(&ax, 0.0)?;
    sec.set_xticks([5.0, 15.0, 25.0])
        .set_xticklabels(["\nOughts", "\nTeens", "\nTwenties"])?;
    Ok(())
}

fn multilevel_ticks_2(fig: &Figure) -> CaseResult {
    let ax = constrained(fig)?;
    let animals = [
        "cats", "dogs", "pigs", "snakes", "lizards", "chickens", "eagles", "herons", "buzzards",
    ];
    ax.plot_fmt(animals, SpecRng::new(SEED).normal(9, 0.0, 1.0), "o")?;
    let sec = bottom_axis(&ax, 0.0)?;
    sec.set_xticks([1.0, 3.5, 6.5]).set_xticklabels([
        "\n\nMammals",
        "\n\nReptiles",
        "\n\nBirds",
    ])?;
    sec.tick_params(&TickParams {
        length: Some(0.0),
        ..axis_params(AxisName::X)
    });
    let sec2 = bottom_axis(&ax, 0.0)?;
    sec2.set_xticks([-0.5, 2.5, 4.5, 8.5])
        .set_xticklabels(Vec::<String>::new())?;
    sec2.tick_params(&TickParams {
        length: Some(40.0),
        width: Some(1.5),
        ..axis_params(AxisName::X)
    });
    ax.set_xlim(-0.6, 8.6);
    Ok(())
}

fn multilevel_ticks_3(fig: &Figure) -> CaseResult {
    let ax = constrained(fig)?;
    // Every day from 2020-01-01 (day 18262 of the epoch) to 2020-03-30.
    let time: Vec<Datetime64> = (18_262..18_352)
        .map(|day| Datetime64::new(day * 86_400, 0))
        .collect();
    let values = SpecRng::new(SEED).uniform(time.len(), 0.0, 1.0);
    ax.plot(time, values)?;
    ax.xaxis().set_major_formatter(DateFormatter::new("%d"));
    let sec = bottom_axis(&ax, -0.075)?;
    sec.xaxis()
        .set_major_locator(RRuleLocator::months(&[], &[1], 1))
        .set_major_formatter(DateFormatter::new("  %b"));
    sec.tick_params(&TickParams {
        length: Some(0.0),
        ..axis_params(AxisName::X)
    });
    sec.spines().get(SpineSide::Bottom).set_linewidth(0.0);
    sec.set_xlabel("Dates (2020)");
    Ok(())
}

/// The two axes of `ticks_too_many`, the left one plotting `y` over the
/// strings `x` as categories with its ticks in red.
fn out_of_order(
    fig: &Figure,
    x: &[&str],
    y: &[f64],
    props: LineProps,
) -> Result<(Axes, Axes), Box<dyn std::error::Error>> {
    let axs = fig.subplots(1, 2)?;
    let (left, right) = (axs[(0, 0)].clone(), axs[(0, 1)].clone());
    left.plot_with(x.to_vec(), y.to_vec(), props)?;
    left.tick_params(&TickParams {
        color: Some("r".into()),
        labelcolor: Some("r".into()),
        ..axis_params(AxisName::X)
    });
    Ok((left, right))
}

/// `x` parsed as numbers (`np.asarray(x, dtype='float')`).
fn floats(x: &[&str]) -> Result<Vec<f64>, std::num::ParseFloatError> {
    x.iter().map(|s| s.parse()).collect()
}

fn ticks_too_many_1(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let x = ["1", "5", "2", "3"];
    let y = [1.0, 4.0, 2.0, 3.0];
    let (left, right) = out_of_order(fig, &x, &y, LineProps::fmt("d")?)?;
    left.set_xlabel("Categories");
    left.set_title("Ticks seem out of order / misplaced");
    right.plot_fmt(floats(&x)?, y, "d")?;
    right.set_xlabel("Floats");
    right.set_title("Ticks as expected");
    Ok(())
}

fn ticks_too_many_2(fig: &Figure) -> CaseResult {
    let numbers: Vec<String> = (0..100).map(|i| i.to_string()).collect();
    let x: Vec<&str> = numbers.iter().map(String::as_str).collect();
    let y: Vec<f64> = (0..100).map(f64::from).collect();
    let (left, right) = out_of_order(fig, &x, &y, LineProps::default())?;
    left.set_title("Too many ticks");
    left.set_xlabel("Categories");
    right.plot(floats(&x)?, y)?;
    right.set_title("x converted to numbers");
    right.set_xlabel("Floats");
    Ok(())
}

fn ticks_too_many_3(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let x = ["2021-10-01", "2021-11-02", "2021-12-03", "2021-09-01"];
    let y = [0.0, 2.0, 3.0, 1.0];
    let (left, right) = out_of_order(fig, &x, &y, LineProps::fmt("d")?)?;
    let upright = TickParams {
        labelrotation: Some(90.0),
        ..axis_params(AxisName::X)
    };
    left.tick_params(&upright);
    left.set_title("Dates out of order");
    let dates = x
        .iter()
        .map(|day| Datetime64::parse(day))
        .collect::<Result<Option<Vec<_>>, _>>()?
        .ok_or("not-a-time in the dates")?;
    right.plot_fmt(dates, y, "d")?;
    right.tick_params(&upright);
    right.set_title("x converted to datetimes");
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_ticks/align_ticklabels", align_ticklabels),
    FigureCase::new("gallery_ticks/ticklabels_rotation", ticklabels_rotation),
    FigureCase::new("gallery_ticks/ticks_top_right", ticks_top_right),
    FigureCase::new(
        "gallery_ticks/fig_axes_customize_simple",
        fig_axes_customize_simple,
    ),
    FigureCase::new("gallery_ticks/multilevel_ticks_1", multilevel_ticks_1).sized(4.0, 4.0, 100.0),
    FigureCase::new("gallery_ticks/multilevel_ticks_2", multilevel_ticks_2).sized(7.0, 4.0, 100.0),
    FigureCase::new("gallery_ticks/multilevel_ticks_3", multilevel_ticks_3).sized(7.0, 4.0, 100.0),
    FigureCase::new("gallery_ticks/ticks_too_many_1", ticks_too_many_1).sized(6.0, 2.5, 100.0),
    FigureCase::new("gallery_ticks/ticks_too_many_2", ticks_too_many_2).sized(6.0, 2.5, 100.0),
    FigureCase::new("gallery_ticks/ticks_too_many_3", ticks_too_many_3).sized(6.0, 2.75, 100.0),
];
