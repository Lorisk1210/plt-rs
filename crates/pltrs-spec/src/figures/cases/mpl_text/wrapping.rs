//! The wrapping and tick label rotation cases of `test_text.py`.

use crate::figures::cases::common::blank;
use pltrs::axes::{AxisName, TickLabelProps, TickLevel};
use pltrs::fonts::{FontStyle, FontWeight};
use pltrs::gridspec::SubplotOverrides;
use pltrs::prelude::*;
use pltrs::text::{HAlign, RotationMode, Text, VAlign};

use crate::figures::{CaseResult, FigureCase};

/// The long string of the wrapping tests.
const WRAPPED: &str = "This is a really long string that I'd rather have wrapped so that it \
                       doesn't go outside of the figure, but if it's long enough it will go off \
                       the top or bottom!";

/// The long string at `(x, y)`, aligned by `halign` and wrapped.
fn wrapped(ax: &Axes, x: f64, y: f64, halign: HAlign) -> Text {
    let text = ax.text(x, y, WRAPPED);
    text.set_halign(halign).set_wrap(true);
    text
}

/// `test_basic_wrap`: wrapped texts rotated, aligned and styled in
/// different ways, some running off the figure.
fn basic_wrap(fig: &Figure) -> CaseResult {
    use HAlign::{Center, Left, Right};
    let ax = blank(fig)?;
    wrapped(&ax, 4.0, 1.0, Left).set_rotation(15.0);
    wrapped(&ax, 6.0, 5.0, Left).set_rotation(15.0);
    wrapped(&ax, 5.0, 5.0, Right).set_rotation(-15.0);
    wrapped(&ax, 5.0, 10.0, Center)
        .set_fontsize(18.0)
        .set_style(FontStyle::Oblique)
        .set_valign(VAlign::Top);
    wrapped(&ax, 3.0, 4.0, Right)
        .set_family(["serif"])
        .set_style(FontStyle::Italic);
    wrapped(&ax, -1.0, 0.0, Left).set_rotation(-15.0);
    Ok(())
}

/// `test_font_wrap`: wrapped texts in different families, weights and
/// sizes.
fn font_wrap(fig: &Figure) -> CaseResult {
    use HAlign::{Center, Left, Right};
    let ax = blank(fig)?;
    wrapped(&ax, 4.0, -1.0, Left)
        .set_fontsize(18.0)
        .set_family(["serif"])
        .set_rotation(15.0);
    wrapped(&ax, 6.0, 5.0, Left)
        .set_family(["sans serif"])
        .set_rotation(15.0);
    wrapped(&ax, 5.0, 10.0, Center)
        .set_weight("heavy".parse::<FontWeight>()?)
        .set_valign(VAlign::Top);
    wrapped(&ax, 3.0, 4.0, Right).set_family(["monospace"]);
    wrapped(&ax, -1.0, 0.0, Left)
        .set_fontsize(14.0)
        .set_style(FontStyle::Italic)
        .set_rotation(-15.0);
    Ok(())
}

/// 37 `foo` tick labels on axis `name` of `ax` and its twin, turned by
/// 0°, 10°, ..., 360° in rotation mode `mode`.
fn rotated_tick_labels(ax: &Axes, twin: &Axes, name: AxisName, mode: RotationMode) -> CaseResult {
    let ticks: Vec<f64> = (0..37).map(f64::from).collect();
    let angles = linspace(0.0, 360.0, 37);
    for axis in [ax.axis_of(name), twin.axis_of(name)] {
        axis.set_ticks(ticks.clone());
        axis.set_ticklabels(vec!["foo"; ticks.len()])?;
        let props = TickLabelProps {
            rotation_mode: Some(mode),
            ..TickLabelProps::default()
        };
        axis.set_ticklabel_props(TickLevel::Major, props);
        for (index, &angle) in angles.iter().enumerate() {
            let props = TickLabelProps {
                rotation: Some(angle),
                ..TickLabelProps::default()
            };
            axis.set_major_ticklabel_props(index, props);
        }
    }
    Ok(())
}

/// `test_xtick_rotation_mode`: x tick labels below and above a thin axes,
/// aligned by their angle.
fn xtick_rotation_mode(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_yticks(Vec::<f64>::new());
    let twin = ax.twiny()?;
    rotated_tick_labels(&ax, &twin, AxisName::X, RotationMode::XTick)?;
    fig.subplots_adjust(SubplotOverrides {
        left: Some(0.01),
        right: Some(0.99),
        top: Some(0.6),
        bottom: Some(0.4),
        ..SubplotOverrides::default()
    })?;
    Ok(())
}

/// `test_ytick_rotation_mode`: y tick labels left and right of a narrow
/// axes, aligned by their angle.
fn ytick_rotation_mode(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_xticks(Vec::<f64>::new());
    let twin = ax.twinx()?;
    rotated_tick_labels(&ax, &twin, AxisName::Y, RotationMode::YTick)?;
    fig.subplots_adjust(SubplotOverrides {
        left: Some(0.4),
        right: Some(0.6),
        top: Some(0.99),
        bottom: Some(0.01),
        ..SubplotOverrides::default()
    })?;
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_text/basictext_wrap", basic_wrap).sized(8.0, 6.0, 100.0),
    FigureCase::new("mpl_text/fonttext_wrap", font_wrap).sized(8.0, 6.0, 100.0),
    FigureCase::new("mpl_text/xtick_rotation_mode", xtick_rotation_mode).sized(12.0, 1.0, 100.0),
    FigureCase::new("mpl_text/ytick_rotation_mode", ytick_rotation_mode).sized(1.0, 12.0, 100.0),
];
