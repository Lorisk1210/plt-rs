//! The gallery's "Units" section. matplotlib converts the values of the
//! gallery's `basic_units` module (centimetres and inches, radians and
//! degrees, seconds, hertz and minutes) through a unit registry, which
//! picks each axis' unit, label, locator and formatter. plt-rs has no unit
//! registry: these cases convert the values into each axis' unit and set
//! what the registry would have set, as `basic_units` defines it and as
//! matplotlib ends up showing it (`annotate_with_units`, `artist_tests`,
//! `bar_demo2`, `bar_unit_demo`, `ellipse_with_units`, `evans_test`,
//! `radian_demo`, `units_sample` and `units_scatter`).

use std::f64::consts::PI;

use plt::axes::{AxisName, ErrorSize, TickParams};
use plt::layout::{LayoutEngine, TightLayout};
use plt::legend::LegendOptions;
use plt::patches::{Arc, Ellipse, PatchShape};
use plt::prelude::*;
use plt::rc::Aspect;
use plt::text::{AnnotateOptions, HAlign, TextRotationMode as RotationMode, VAlign};
use plt::ticker::{AutoLocator, FormatStrFormatter, FuncFormatter, IndexLocator, MultipleLocator};

use crate::figures::cases::common::{black_arrow, map};
use crate::figures::{CaseResult, FigureCase};

/// `basic_units`' factor from centimetres to inches.
const INCHES_PER_CM: f64 = 1.0 / 2.54;

/// An axis' unit, as `basic_units` names it on the axis label.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Length {
    Centimeters,
    Inches,
}

impl Length {
    /// `cm` lengths in this unit.
    fn of(self, cm: &[f64]) -> Vec<f64> {
        match self {
            Self::Centimeters => cm.to_vec(),
            Self::Inches => map(cm, |v| v * INCHES_PER_CM),
        }
    }

    /// The axis label (`basic_units`' full name of the unit).
    fn label(self) -> &'static str {
        match self {
            Self::Centimeters => "centimeters",
            Self::Inches => "inches",
        }
    }
}

/// Labels the axes with their units.
fn label_units(ax: &Axes, x: Length, y: Length) {
    ax.set_xlabel(x.label()).set_ylabel(y.label());
}

fn annotate_with_units(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.annotate("Note 01", (0.5, 0.5), AnnotateOptions::default())?;
    // The text at (0.8 cm, 0.95 cm), then at (0.8, 0.95) of the axes.
    for textcoords in ["data", "axes fraction"] {
        ax.annotate(
            "local max",
            (3.0, 1.0),
            AnnotateOptions {
                xytext: Some((0.8, 0.95).into()),
                textcoords: Some(textcoords.parse()?),
                arrowprops: Some(black_arrow()),
                ..AnnotateOptions::default()
            },
        )?
        .set_halign(HAlign::Right)
        .set_valign(VAlign::Top);
    }
    ax.set_xlim(0.0, 4.0).set_ylim(0.0, 4.0);
    label_units(&ax, Length::Centimeters, Length::Centimeters);
    Ok(())
}

/// matplotlib finds no converter when the units are set before any value
/// is converted, so these axes are not labelled.
fn artist_tests(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.add_line(
        &[0.0, 1.5],
        &[0.0, 2.5],
        LineProps::default().linewidth(2.0).color("black"),
    )?;
    // A `Text` given to `add_artist`, which clips it to the axes box.
    ax.text(3.0, 2.5, "text label")
        .set_halign(HAlign::Left)
        .set_valign(VAlign::Bottom)
        .set_clip_on(true);
    ax.set_xlim(-1.0, 10.0).set_ylim(-1.0, 10.0);
    ax.grid(true);
    ax.set_title("Artists with units");
    Ok(())
}

/// Bars of `cm` heights at `cm` positions from a zero bottom, `width`
/// wide, shown in the units `x` and `y`.
fn length_bars(ax: &Axes, cm: &[f64], width: f64, x: Length, y: Length) -> CaseResult {
    ax.bar(
        x.of(cm),
        y.of(cm),
        BarOptions {
            thickness: vec![width],
            base: vec![0.0],
            ..BarOptions::default()
        },
    )?;
    label_units(ax, x, y);
    Ok(())
}

fn bar_demo2(fig: &Figure) -> CaseResult {
    use Length::{Centimeters as Cm, Inches as In};
    let cms = [0.0, 2.0, 4.0, 6.0, 8.0];
    // `width = 0.8 * cm` in the axis' unit; a plain 0.8 without it.
    let width = |x: Length| x.of(&[0.8])[0];
    let axs = fig.subplots(2, 2)?;
    length_bars(&axs[(0, 0)], &cms, 0.8, Cm, Cm)?;
    length_bars(&axs[(0, 1)], &cms, width(Cm), Cm, In)?;
    length_bars(&axs[(1, 0)], &cms, width(In), In, Cm)?;
    // Plain numbers are in the axis' unit, centimetres are converted.
    axs[(1, 0)].set_xlim(2.0, 6.0);
    length_bars(&axs[(1, 1)], &cms, width(In), In, In)?;
    let limits = In.of(&[2.0, 6.0]);
    axs[(1, 1)].set_xlim(limits[0], limits[1]);
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

/// The y axis is in inches but, its units set before any value was
/// converted, not labelled.
fn bar_unit_demo(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let ind: Vec<f64> = (0..5).map(f64::from).collect();
    let width = 0.35;
    let groups = [
        (
            "Tea",
            [15.0, 10.0, 8.0, 12.0, 5.0],
            [2.0, 1.0, 1.0, 4.0, 2.0],
            0.0,
        ),
        (
            "Coffee",
            [14.0, 19.0, 7.0, 5.0, 10.0],
            [3.0, 5.0, 2.0, 1.0, 2.0],
            width,
        ),
    ];
    for (label, means, std, shift) in groups {
        ax.bar(
            map(&ind, |i| i + shift),
            Length::Inches.of(&means),
            BarOptions {
                thickness: vec![width],
                base: vec![0.0],
                label: Some(label.to_owned()),
                errors: ErrorbarOptions {
                    yerr: Some(ErrorSize::Symmetric(Length::Inches.of(&std))),
                    ..ErrorbarOptions::default()
                },
                ..BarOptions::default()
            },
        )?;
    }
    ax.set_title("Cup height by group and beverage choice");
    ax.set_xticks(map(&ind, |i| i + width / 2.0))
        .set_xticklabels(["G1", "G2", "G3", "G4", "G5"])?;
    ax.legend(LegendOptions::default())?;
    ax.autoscale_view(None);
    Ok(())
}

/// A figure of `ellipse_with_units`: the ellipse drawn by `outline` over
/// the polygon approximating it, with automatic and equal aspect. The
/// example's arithmetic drops the centimetres before matplotlib sees the
/// values, so they are plain numbers here too.
fn ellipse_compare<S: PatchShape>(fig: &Figure, outline: impl Fn() -> S) -> CaseResult {
    let (xcenter, ycenter) = (0.38, 0.52);
    let (width, height) = (1e-1, 3e-1);
    let angle: f64 = -30.0;
    let theta = map(&arange(0.0, 360.0, 1.0), f64::to_radians);
    let rtheta = angle.to_radians();
    let (cos, sin) = (rtheta.cos(), rtheta.sin());
    let (x, y): (Vec<f64>, Vec<f64>) = theta
        .iter()
        .map(|t| {
            let (x, y) = (0.5 * width * t.cos(), 0.5 * height * t.sin());
            (cos * x + -sin * y + xcenter, sin * x + cos * y + ycenter)
        })
        .unzip();
    let outlined = PatchProps::default().linewidth(2.0).fill(false).zorder(2.0);
    let ax = fig.add_subplot(2, 1, 1)?;
    ax.set_aspect(Aspect::Auto)?;
    ax.fill(
        &x,
        &y,
        PatchProps::default()
            .alpha(0.2)
            .facecolor("yellow")
            .edgecolor("yellow")
            .linewidth(1.0)
            .zorder(1.0),
    )?;
    ax.add_patch_with(outline(), outlined.clone());
    let ax = fig.add_subplot(2, 1, 2)?;
    ax.set_aspect(Aspect::Equal)?;
    ax.fill(
        &x,
        &y,
        PatchProps::default()
            .alpha(0.2)
            .facecolor("green")
            .edgecolor("green")
            .zorder(1.0),
    )?;
    ax.add_patch_with(outline(), outlined);
    Ok(())
}

fn ellipse_with_units_1(fig: &Figure) -> CaseResult {
    ellipse_compare(fig, || Ellipse {
        angle: -30.0,
        ..Ellipse::new((0.38, 0.52), 1e-1, 3e-1)
    })
}

fn ellipse_with_units_2(fig: &Figure) -> CaseResult {
    ellipse_compare(fig, || Arc::new((0.38, 0.52), 1e-1, 3e-1).with_angle(-30.0))
}

/// `evans_test`'s `Foo` values `0, 2, ..., 48` in the unit `factor`
/// (their value divided by it) on an axis ticked every eighth value and
/// labelled `foo`, as `FooConverter.axisinfo` sets it, against their
/// index.
fn foo_plot(ax: &Axes, factor: f64, line: LineProps) -> CaseResult {
    let values: Vec<f64> = (0..50)
        .step_by(2)
        .map(|v| f64::from(v) * 1.0 / factor)
        .collect();
    let index: Vec<f64> = (0..values.len()).map(|i| i as f64).collect();
    ax.plot_with(values, index, line)?;
    ax.xaxis()
        .set_major_locator(IndexLocator::new(8.0, 0.0))
        .set_major_formatter(FormatStrFormatter::new("VAL: %s")?);
    ax.set_xlabel("foo");
    ax.tick_params(&TickParams {
        axis: Some(AxisName::X),
        labelrotation: Some(30.0),
        labelrotation_mode: Some(RotationMode::XTick),
        ..TickParams::default()
    });
    Ok(())
}

fn evans_test(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    fig.suptitle("Custom units");
    fig.subplots_adjust(&plt::gridspec::SubplotOverrides {
        bottom: Some(0.2),
        ..plt::gridspec::SubplotOverrides::default()
    })?;
    foo_plot(ax2, 2.0, LineProps::fmt("o")?)?;
    ax2.set_title("xunits = 2.0");
    foo_plot(ax1, 1.0, LineProps::default())?;
    ax1.set_title("default units");
    Ok(())
}

/// `basic_units.rad_fn`: multiples of π/2 in math text.
fn rad_fn(x: f64) -> String {
    let half_turns = (x / PI) * 2.0;
    // `int()` truncates towards zero.
    let n = if x >= 0.0 {
        (half_turns + 0.25).trunc()
    } else {
        (half_turns - 0.25).trunc()
    } as i64;
    match n {
        0 => "0".to_owned(),
        1 => r"$\pi/2$".to_owned(),
        2 => r"$\pi$".to_owned(),
        -1 => r"$-\pi/2$".to_owned(),
        -2 => r"$-\pi$".to_owned(),
        n if n % 2 == 0 => format!(r"${}\pi$", n.div_euclid(2)),
        n => format!(r"${n}\pi/2$"),
    }
}

fn radian_demo(fig: &Figure) -> CaseResult {
    let x = arange(0.0, 15.0, 0.01);
    let cos = map(&x, f64::cos);
    let axs = fig.subplots(2, 1)?;
    let (radians, degrees) = (&axs[(0, 0)], &axs[(1, 0)]);
    radians.plot(&x, &cos)?;
    radians
        .xaxis()
        .set_major_locator(MultipleLocator::new(PI / 2.0)?)
        .set_major_formatter(FuncFormatter::new(|x, _| rad_fn(x)));
    radians.set_xlabel("radians");
    degrees.plot(map(&x, |x| x * (180.0 / PI)), &cos)?;
    degrees
        .xaxis()
        .set_major_locator(AutoLocator::new())
        .set_major_formatter(FormatStrFormatter::new(r"$%i^\circ$")?);
    degrees.set_xlabel("degrees");
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

fn units_sample(fig: &Figure) -> CaseResult {
    use Length::{Centimeters as Cm, Inches as In};
    let cms = [0.0, 2.0, 4.0, 6.0, 8.0];
    fig.set_layout_engine(LayoutEngine::constrained());
    let axs = fig.subplots(2, 2)?;
    for ((row, col), x, y) in [
        ((0, 0), Cm, Cm),
        ((0, 1), Cm, In),
        ((1, 0), In, Cm),
        ((1, 1), In, In),
    ] {
        axs[(row, col)].plot(x.of(&cms), y.of(&cms))?;
        label_units(&axs[(row, col)], x, y);
    }
    // Plain numbers are in the axis' unit, centimetres are converted.
    axs[(1, 0)].set_xlim(-1.0, 4.0);
    let limits = In.of(&[3.0, 6.0]);
    axs[(1, 1)].set_xlim(limits[0], limits[1]);
    Ok(())
}

/// `secs * MaskedArray(1..=8)`: the masked seconds are left out (NaN).
fn masked_seconds() -> Vec<f64> {
    let mask = [true, false, true, false, false, false, true, false];
    (1..=8)
        .zip(mask)
        .map(|(v, masked)| if masked { f64::NAN } else { f64::from(v) })
        .collect()
}

/// The time units of `units_scatter`, as `basic_units` converts seconds
/// into them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Time {
    Seconds,
    Hertz,
    Minutes,
}

impl Time {
    /// `seconds` in this unit.
    fn of(self, seconds: &[f64]) -> Vec<f64> {
        match self {
            Self::Seconds => seconds.to_vec(),
            Self::Hertz => map(seconds, |s| 1.0 / s),
            Self::Minutes => map(seconds, |s| s * (1.0 / 60.0)),
        }
    }

    /// The axis label (`basic_units`' full name of the unit).
    fn label(self) -> &'static str {
        match self {
            Self::Seconds => "seconds",
            Self::Hertz => "Hertz",
            Self::Minutes => "minutes",
        }
    }
}

/// The y values in seconds, hertz and minutes; matplotlib leaves the x
/// axes unlabelled.
fn units_scatter(fig: &Figure) -> CaseResult {
    let xsecs = masked_seconds();
    let axs = fig.subplots_shared(3, 1, plt::figure::Share::All, plt::figure::Share::None)?;
    for (ax, unit) in axs
        .as_slice()
        .iter()
        .zip([Time::Seconds, Time::Hertz, Time::Minutes])
    {
        ax.scatter_with(&xsecs, unit.of(&xsecs), ScatterOptions::default())?;
        ax.set_ylabel(unit.label());
    }
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_ticks/annotate_with_units", annotate_with_units),
    FigureCase::new("gallery_ticks/artist_tests", artist_tests),
    FigureCase::new("gallery_ticks/bar_demo2", bar_demo2),
    FigureCase::new("gallery_ticks/bar_unit_demo", bar_unit_demo),
    FigureCase::new("gallery_ticks/ellipse_with_units_1", ellipse_with_units_1),
    FigureCase::new("gallery_ticks/ellipse_with_units_2", ellipse_with_units_2),
    FigureCase::new("gallery_ticks/evans_test", evans_test),
    FigureCase::new("gallery_ticks/radian_demo", radian_demo),
    FigureCase::new("gallery_ticks/units_sample", units_sample),
    FigureCase::new("gallery_ticks/units_scatter", units_scatter),
];
