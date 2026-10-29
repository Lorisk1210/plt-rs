//! The polar charts of the section: a nested pie of polar bars, bars,
//! lines with reduced limits, error bars, a legend and scatter plots with
//! moved origins and sectors.

use std::f64::consts::PI;

use pltrs::axes::{BarAlign, MappedColors, TickParams};
use pltrs::colors::{Colormap, colormaps};
use pltrs::figure::Figure;
use pltrs::layout::LayoutEngine;
use pltrs::legend::{BboxToAnchor, LegendOptions};
use pltrs::offsetbox::AnchorLoc;
use pltrs::polar::{Compass, PolarAxes};
use pltrs::prelude::*;
use pltrs::text::VAlign;

use super::SEED;
use crate::figures::cases::common::{cumsum, map};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `nested_pie`'s values: three groups of two (`NESTED_VALS`).
const NESTED_VALS: [[f64; 2]; 3] = [[60.0, 32.0], [37.0, 40.0], [29.0, 10.0]];

/// The width of each ring (`RING`).
const RING: f64 = 0.3;

/// The colours of `cmap` at table entries `indices` (`cmap([...])`).
fn entries(cmap: &Colormap, indices: &[i64]) -> Vec<Color> {
    indices
        .iter()
        .map(|&i| cmap.sample_index(i).into())
        .collect()
}

fn nested_pie_2(fig: &Figure) -> CaseResult {
    let ax = fig.polar_subplot();
    let total: f64 = NESTED_VALS.as_flattened().iter().sum();
    let valsnorm = map(NESTED_VALS.as_flattened(), |v| v / total * 2.0 * PI);
    let mut valsleft = vec![0.0];
    valsleft.extend(cumsum(&valsnorm[..valsnorm.len() - 1]));
    let cmap = colormaps::get("tab20c")?;
    let ring = |width: Vec<f64>, bottom: f64, color: Vec<Color>| BarOptions {
        thickness: width.into(),
        base: vec![bottom].into(),
        align: BarAlign::Edge,
        color,
        edgecolor: vec!["w".into()],
        linewidth: vec![1.0],
        ..BarOptions::default()
    };
    let outer_left: Vec<f64> = valsleft.iter().copied().step_by(2).collect();
    let outer_width: Vec<f64> = valsnorm.chunks(2).map(|pair| pair[0] + pair[1]).collect();
    let outer_colors = entries(&cmap, &[0, 4, 8]);
    ax.bar(
        outer_left,
        [RING; 3],
        ring(outer_width, 1.0 - RING, outer_colors),
    )?;
    let inner_colors = entries(&cmap, &[1, 2, 5, 6, 9, 10]);
    ax.bar(
        valsleft,
        [RING; 6],
        ring(valsnorm, 1.0 - 2.0 * RING, inner_colors),
    )?;
    ax.set_title("Pie plot with `ax.bar` and polar coordinates");
    ax.set_axis_off();
    Ok(())
}

fn polar_bar(fig: &Figure) -> CaseResult {
    let mut g = SpecRng::new(SEED);
    let n = 20;
    let theta = pltrs::numeric::linspace_with(0.0, 2.0 * PI, n, false);
    let radii = map(&g.uniform(n, 0.0, 1.0), |u| 10.0 * u);
    let width = map(&g.uniform(n, 0.0, 1.0), |u| PI / 4.0 * u);
    let viridis = colormaps::get("viridis")?;
    let options = BarOptions {
        thickness: width.into(),
        base: vec![0.0].into(),
        color: radii
            .iter()
            .map(|r| viridis.sample(r / 10.0).into())
            .collect(),
        props: PatchProps::default().alpha(0.5),
        ..BarOptions::default()
    };
    fig.polar_subplot().bar(theta, &radii, options)?;
    Ok(())
}

/// Plots `polar_demo`'s spiral out to r = 2 into `ax`.
fn spiral(ax: &PolarAxes) -> CaseResult {
    let r = arange(0.0, 2.0, 0.01);
    ax.plot(map(&r, |r| 2.0 * PI * r), &r)?;
    ax.set_rmax(2.0);
    Ok(())
}

/// Finishes a panel of `polar_demo`: fewer radial ticks, their labels
/// moved off the line, the grid and the title.
fn finish_panel(ax: &PolarAxes, rticks: &[f64], title: &str) {
    ax.set_rticks(rticks).set_rlabel_position(-22.5);
    ax.grid(true);
    ax.set_title(title).set_valign(VAlign::Bottom);
}

fn polar_demo(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let top = fig.add_polar_subplot(2, 1, 1)?;
    let bottom = fig.add_polar_subplot(2, 1, 2)?;
    spiral(&top)?;
    finish_panel(&top, &[0.5, 1.0, 1.5, 2.0], "A line plot on a polar axis");
    spiral(&bottom)?;
    bottom
        .set_rmin(1.0)
        .set_rorigin(Some(0.0))
        .set_thetamin(0.0)
        .set_thetamax(225.0);
    finish_panel(
        &bottom,
        &[1.0, 1.5, 2.0],
        "Same plot, but with reduced axis limits",
    );
    Ok(())
}

/// `polar_error_caps`: eight points with capped theta and r errors
/// (`polar_errorbars`).
fn polar_errorbars(fig: &Figure, xerr: f64, yerr: f64, color: &str, title: &str) -> CaseResult {
    let theta = arange(0.0, 2.0 * PI, PI / 4.0);
    let r = map(&theta, |t| t / PI / 2.0 + 0.5);
    let ax = fig.polar_subplot();
    let options = ErrorbarOptions {
        xerr: Some(xerr.into()),
        yerr: Some(yerr.into()),
        capsize: Some(7.0),
        fmt: "o".parse()?,
        line: LineProps::default().color(color),
        ..ErrorbarOptions::default()
    };
    ax.errorbar(&theta, &r, options)?;
    ax.set_title(title);
    Ok(())
}

fn polar_error_caps_1(fig: &Figure) -> CaseResult {
    polar_errorbars(fig, 0.25, 0.1, "seagreen", "Pretty polar error bars")
}

fn polar_error_caps_2(fig: &Figure) -> CaseResult {
    polar_errorbars(fig, 5.25, 0.1, "darkred", "Overlapping theta error bars")
}

fn polar_error_caps_3(fig: &Figure) -> CaseResult {
    polar_errorbars(fig, 0.25, 10.1, "orangered", "Large radius error bars")
}

fn polar_legend(fig: &Figure) -> CaseResult {
    let ax = fig.polar_subplot();
    ax.set_facecolor("lightgoldenrodyellow");
    let r = linspace(0.0, 3.0, 301);
    let theta = map(&r, |r| 2.0 * PI * r);
    ax.plot_with(
        &theta,
        &r,
        LineProps::default()
            .color("tab:orange")
            .linewidth(3.0)
            .label("a line"),
    )?;
    ax.plot_with(
        map(&theta, |t| 0.5 * t),
        &r,
        LineProps::default()
            .color("tab:blue")
            .linestyle(LineStyle::Dashed)
            .linewidth(3.0)
            .label("another line"),
    )?;
    ax.tick_params(TickParams {
        grid_color: Some("palegoldenrod".into()),
        ..TickParams::default()
    });
    let angle = 67.5_f64.to_radians();
    ax.legend(
        LegendOptions::default()
            .loc(AnchorLoc::LowerLeft)
            .bbox_to_anchor(BboxToAnchor::Point(
                0.5 + angle.cos() / 2.0,
                0.5 + angle.sin() / 2.0,
            )),
    )?;
    Ok(())
}

/// `polar_scatter`'s points: size grows with r, colour with theta.
fn polar_scatter(fig: &Figure) -> Result<PolarAxes, Box<dyn std::error::Error>> {
    let mut g = SpecRng::new(SEED);
    let n = 150;
    let r = map(&g.uniform(n, 0.0, 1.0), |u| 2.0 * u);
    let theta = map(&g.uniform(n, 0.0, 1.0), |u| 2.0 * PI * u);
    let ax = fig.polar_subplot();
    let options = ScatterOptions {
        sizes: Some(map(&r, |r| 200.0 * r * r)),
        colors: MappedColors::new(theta.clone())
            .cmap(colormaps::get("hsv")?)
            .into(),
        alpha: 0.75.into(),
        ..ScatterOptions::default()
    };
    ax.scatter_with(theta, r, options)?;
    Ok(ax)
}

fn polar_scatter_1(fig: &Figure) -> CaseResult {
    polar_scatter(fig)?;
    Ok(())
}

fn polar_scatter_2(fig: &Figure) -> CaseResult {
    polar_scatter(fig)?
        .set_rorigin(Some(-2.5))
        .set_theta_zero_location(Compass::W, 10.0);
    Ok(())
}

fn polar_scatter_3(fig: &Figure) -> CaseResult {
    polar_scatter(fig)?.set_thetamin(45.0).set_thetamax(135.0);
    Ok(())
}

/// Every case of the module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_stats/nested_pie_2", nested_pie_2),
    FigureCase::new("gallery_stats/polar_bar", polar_bar),
    FigureCase::new("gallery_stats/polar_demo", polar_demo).sized(5.0, 8.0, 100.0),
    FigureCase::new("gallery_stats/polar_error_caps_1", polar_error_caps_1)
        .sized(10.0, 10.0, 100.0),
    FigureCase::new("gallery_stats/polar_error_caps_2", polar_error_caps_2)
        .sized(10.0, 10.0, 100.0),
    FigureCase::new("gallery_stats/polar_error_caps_3", polar_error_caps_3)
        .sized(10.0, 10.0, 100.0),
    FigureCase::new("gallery_stats/polar_legend", polar_legend),
    FigureCase::new("gallery_stats/polar_scatter_1", polar_scatter_1),
    FigureCase::new("gallery_stats/polar_scatter_2", polar_scatter_2),
    FigureCase::new("gallery_stats/polar_scatter_3", polar_scatter_3),
];
