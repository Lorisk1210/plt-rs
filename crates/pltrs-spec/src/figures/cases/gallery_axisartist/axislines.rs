//! Axis lines through the data and on the edges: `demo_axisline_style`,
//! `simple_axisartist1`, `simple_axisline` and `simple_axisline3`.

use std::f64::consts::PI;

use pltrs::axes::AxisName;
use pltrs::axisartist::{self, AxislineStyle, Toggle};
use pltrs::gridspec::SubplotOverrides;
use pltrs::layout::LayoutEngine;
use pltrs::prelude::*;
use pltrs::spines::SpineSide;

use crate::figures::cases::common::{axis_artist, map};
use crate::figures::{CaseResult, FigureCase};

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new(
        "gallery_axisartist/demo_axisline_style",
        demo_axisline_style,
    ),
    FigureCase::new("gallery_axisartist/simple_axisartist1", simple_axisartist1)
        .sized(6.0, 3.0, 100.0),
    FigureCase::new("gallery_axisartist/simple_axisline", simple_axisline),
    FigureCase::new("gallery_axisartist/simple_axisline3", simple_axisline3).sized(3.0, 3.0, 100.0),
];

/// Hides the axis artists `names` of `ax`.
fn hide(ax: &axisartist::AxisArtistAxes, names: &[&str]) -> CaseResult {
    for name in names {
        axis_artist(ax, name)?.set_visible(false);
    }
    Ok(())
}

fn demo_axisline_style(fig: &Figure) -> CaseResult {
    let ax = axisartist::AxisArtistAxes::zero(fig.subplot());
    for name in ["xzero", "yzero"] {
        let axis = axis_artist(&ax, name)?;
        axis.set_axisline_style(Some("-|>".parse::<AxislineStyle>()?));
        axis.set_visible(true);
    }
    hide(&ax, &["left", "right", "bottom", "top"])?;
    let x = linspace(-0.5, 1.0, 100);
    let y = map(&x, |x| (x * PI).sin());
    ax.ax().plot(x, y)?;
    Ok(())
}

fn simple_axisartist1(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let grid = fig.add_gridspec(1, 2)?;
    let ax0 = axisartist::AxisArtistAxes::new(fig.add_subplot_spec(&grid.cell(0, 0)?)?);
    let y0 = ax0.new_floating_axis_facing("y=0", AxisName::X, 0.0, SpineSide::Bottom);
    y0.toggle(Toggle {
        all: Some(true),
        ..Toggle::default()
    });
    y0.label().set_text("y = 0");
    hide(&ax0, &["bottom", "top", "right"])?;
    let ax1 = axisartist::AxisArtistAxes::zero(fig.add_subplot_spec(&grid.cell(0, 1)?)?);
    let xzero = axis_artist(&ax1, "xzero")?;
    xzero.set_visible(true);
    xzero.label().set_text("Axis Zero");
    hide(&ax1, &["bottom", "top", "right"])?;
    let x = arange(0.0, 2.0 * PI, 0.01);
    let y = map(&x, f64::sin);
    ax0.ax().plot(x.clone(), y.clone())?;
    ax1.ax().plot(x, y)?;
    Ok(())
}

fn simple_axisline(fig: &Figure) -> CaseResult {
    fig.subplots_adjust(SubplotOverrides {
        right: Some(0.85),
        ..SubplotOverrides::default()
    })?;
    let ax = axisartist::AxisArtistAxes::zero(fig.subplot());
    hide(&ax, &["right", "top"])?;
    let xzero = axis_artist(&ax, "xzero")?;
    xzero.set_visible(true);
    xzero.label().set_text("Axis Zero");
    ax.ax()
        .set_ylim(-2.0, 4.0)?
        .set_xlabel("Label X")
        .set_ylabel("Label Y");
    ax.new_fixed_axis("right2", SpineSide::Right, (20.0, 0.0))
        .label()
        .set_text("Label Y2");
    ax.ax().plot_y([-2.0, 3.0, 2.0]);
    Ok(())
}

fn simple_axisline3(fig: &Figure) -> CaseResult {
    let ax = axisartist::AxisArtistAxes::new(fig.subplot());
    hide(&ax, &["right", "top"])
}
