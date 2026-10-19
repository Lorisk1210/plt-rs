//! `markevery` in all its forms (strides, offsets, indices, slices and
//! distances along the line) on linear, zoomed, log and polar axes and
//! with NaN gaps, and marker edges of several widths.

use std::f64::consts::PI;

use plt::gridspec::GridSpec;
use plt::legend::LegendOptions;
use plt::numeric::linspace;
use plt::prelude::*;

use super::remove_ticks_and_titles;
use crate::figures::CaseResult;

/// `_markevery_styles`: four marker styles (after `line`, `""` or `"-"`)
/// with no, default, every-10th and every-20th-from-5 markers, and a
/// legend.
fn markevery_styles(fig: &Figure, line: &str) -> CaseResult {
    let x = linspace(0.0, 10.0, 100);
    let y: Vec<f64> = x
        .iter()
        .map(|x| x.sin() * (x / 10.0 + 0.5).sqrt())
        .collect();
    let ax = fig.subplot();
    let curves = [
        ("o", 0.0, None, "default"),
        ("d", 1.0, None, "mark all"),
        (
            "s",
            2.0,
            Some(MarkEvery::Every { start: 0, step: 10 }),
            "mark every 10",
        ),
        (
            "+",
            3.0,
            Some(MarkEvery::Every { start: 5, step: 20 }),
            "mark every 20 starting at 5",
        ),
    ];
    for (marker, shift, markevery, label) in curves {
        let shifted: Vec<f64> = y.iter().map(|y| y + shift).collect();
        let style = LineProps::fmt(&format!("{line}{marker}"))?
            .markevery(markevery)
            .label(label);
        ax.plot_with(&x, shifted, style)?;
    }
    ax.legend(LegendOptions::default())?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn markevery(fig: &Figure) -> CaseResult {
    markevery_styles(fig, "")
}

pub(super) fn markevery_line(fig: &Figure) -> CaseResult {
    markevery_styles(fig, "-")
}

/// The `markevery` values of the grid tests, in order.
fn markevery_cases() -> [Option<MarkEvery>; 11] {
    let distance = |start, step| Some(MarkEvery::Distance { start, step });
    [
        None,
        Some(MarkEvery::Every { start: 0, step: 8 }),
        Some(MarkEvery::Every { start: 30, step: 8 }),
        Some(MarkEvery::Indices(vec![16, 24, 30])),
        Some(MarkEvery::Indices(vec![0, -1])),
        Some(MarkEvery::Slice {
            start: Some(100),
            stop: Some(200),
            step: Some(3),
        }),
        distance(0.0, 0.1),
        distance(0.0, 0.3),
        distance(0.0, 1.5),
        distance(0.0, 0.1),
        distance(0.45, 0.1),
    ]
}

/// How the grid tests set up each axes.
#[derive(Clone, Copy)]
enum GridAxes {
    /// Plain rectilinear axes.
    Linear,
    /// Rectilinear axes zoomed to `x` 6–6.7, `y` 1.1–1.7 after plotting.
    Zoomed,
    /// Log scales on both axes before plotting.
    Log,
    /// Polar axes.
    Polar,
}

/// `_markevery_grid`: one subplot per `markevery` value on a 4×3 grid,
/// each plotting `y` against `x` as `o-` with 4 pt markers (the titles
/// naming the values are removed with the tick labels).
fn markevery_grid(fig: &Figure, x: &[f64], y: &[f64], kind: GridAxes) -> CaseResult {
    let cases = markevery_cases();
    let grid = GridSpec::new(cases.len() / 3 + 1, 3)?;
    for (i, markevery) in cases.into_iter().enumerate() {
        let spec = grid.cell(i / 3, i % 3)?;
        let ax = match kind {
            GridAxes::Polar => (*fig.add_polar_subplot_spec(&spec)?).clone(),
            _ => fig.add_subplot_spec(&spec)?,
        };
        if matches!(kind, GridAxes::Log) {
            ax.set_xscale("log")?.set_yscale("log")?;
        }
        let style = LineProps::fmt("o")?
            .linestyle(LineStyle::Solid)
            .markersize(4.0)
            .markevery(markevery);
        ax.plot_with(x, y, style)?;
        if matches!(kind, GridAxes::Zoomed) {
            ax.set_xlim(6.0, 6.7).set_ylim(1.1, 1.7);
        }
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `_markevery_sine`: 200 points of a raised sine.
fn markevery_sine() -> (Vec<f64>, Vec<f64>) {
    let delta = 0.11;
    let x: Vec<f64> = linspace(0.0, 10.0 - 2.0 * delta, 200)
        .into_iter()
        .map(|x| x + delta)
        .collect();
    let y = x.iter().map(|x| x.sin() + 1.0 + delta).collect();
    (x, y)
}

pub(super) fn markevery_linear_scales(fig: &Figure) -> CaseResult {
    let (x, y) = markevery_sine();
    markevery_grid(fig, &x, &y, GridAxes::Linear)
}

pub(super) fn markevery_linear_scales_zoomed(fig: &Figure) -> CaseResult {
    let (x, y) = markevery_sine();
    markevery_grid(fig, &x, &y, GridAxes::Zoomed)
}

pub(super) fn markevery_log_scales(fig: &Figure) -> CaseResult {
    let (x, y) = markevery_sine();
    markevery_grid(fig, &x, &y, GridAxes::Log)
}

pub(super) fn markevery_polar(fig: &Figure) -> CaseResult {
    let r = linspace(0.0, 3.0, 200);
    let theta: Vec<f64> = r.iter().map(|r| 2.0 * PI * r).collect();
    markevery_grid(fig, &theta, &r, GridAxes::Polar)
}

pub(super) fn markevery_linear_scales_nans(fig: &Figure) -> CaseResult {
    let (x, mut y) = markevery_sine();
    let n = y.len();
    for gap in [0..10, n - 20..n, 50..70] {
        y[gap].fill(f64::NAN);
    }
    markevery_grid(fig, &x, &y, GridAxes::Linear)
}

pub(super) fn marker_edges(fig: &Figure) -> CaseResult {
    let x = linspace(0.0, 1.0, 10);
    let y: Vec<f64> = x.iter().map(|x| x.sin()).collect();
    let ax = fig.subplot();
    for (shift, width, color) in [(0.0, 0.0, "r"), (0.1, 1.0, "r"), (0.2, 2.0, "b")] {
        let style = LineProps::fmt("y.")?
            .markersize(30.0)
            .markeredgewidth(width)
            .markeredgecolor(color);
        ax.plot_with(x.iter().map(|x| x + shift).collect::<Vec<_>>(), &y, style)?;
    }
    remove_ticks_and_titles(fig);
    Ok(())
}
