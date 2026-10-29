//! `autoscale`: margins, sticky edges, explicit limits, tight autoscaling
//! and the `round_numbers` limit mode.

use std::f64::consts::PI;

use pltrs::axes::ImshowOptions;
use pltrs::numeric::Grid;
use pltrs::prelude::*;
use pltrs::ticker::{AutolimitMode, AxisName};

use crate::figures::cases::common::map;
use crate::figures::{CaseResult, FigureCase};

/// numpy's `sinc`: `sin(πx) / (πx)`, with `x = 0` replaced by `1e-20`.
fn sinc(x: f64) -> f64 {
    let y = PI * if x == 0.0 { 1.0e-20 } else { x };
    y.sin() / y
}

/// The page's curve: `sinc` at 201 points of `[-6, 6]`.
fn sinc_curve() -> (Vec<f64>, Vec<f64>) {
    let x = linspace(-6.0, 6.0, 201);
    let y = map(&x, sinc);
    (x, y)
}

/// The page's image: a sinc bump centred at `(1, 1)` over the curve's
/// grid.
fn sinc_field() -> Grid<f64> {
    let x = linspace(-6.0, 6.0, 201);
    Grid::from_fn(x.len(), x.len(), |row, col| {
        sinc(((x[col] - 1.0).powi(2) + (x[row] - 1.0).powi(2)).sqrt())
    })
}

/// One axes with the curve on it.
fn sinc_axes(fig: &Figure) -> Result<Axes, Box<dyn std::error::Error>> {
    let ax = fig.subplot();
    let (x, y) = sinc_curve();
    ax.plot(x, y)?;
    Ok(ax)
}

fn autoscale_1(fig: &Figure) -> CaseResult {
    sinc_axes(fig)?;
    Ok(())
}

fn autoscale_2(fig: &Figure) -> CaseResult {
    sinc_axes(fig)?.set_margins(0.2, 0.2)?;
    Ok(())
}

fn autoscale_3(fig: &Figure) -> CaseResult {
    sinc_axes(fig)?.set_margins(None, -0.2)?;
    Ok(())
}

/// `ncols` axes side by side, each showing the image and titled.
fn image_row(fig: &Figure, titles: &[&str]) -> Result<Vec<Axes>, Box<dyn std::error::Error>> {
    let zz = sinc_field();
    let axs = fig.subplots(1, titles.len())?;
    for (ax, title) in axs.as_slice().iter().zip(titles) {
        ax.imshow(zz.clone(), ImshowOptions::default())?;
        ax.set_title(*title);
    }
    Ok(axs.as_slice().to_vec())
}

fn autoscale_4(fig: &Figure) -> CaseResult {
    let ax = image_row(fig, &["default margins", "margins(0.2)"])?;
    ax[1].set_margins(0.2, 0.2)?;
    Ok(())
}

fn autoscale_5(fig: &Figure) -> CaseResult {
    let ax = image_row(
        fig,
        &[
            "default use_sticky_edges\nmargins(0.2)",
            "use_sticky_edges=False\nmargins(0.2)",
            "default use_sticky_edges\nmargins(-0.2)",
        ],
    )?;
    ax[0].set_margins(0.2, 0.2)?;
    ax[1].set_margins(0.2, 0.2)?;
    ax[1].set_use_sticky_edges(false);
    ax[2].set_margins(-0.2, -0.2)?;
    Ok(())
}

fn autoscale_6(fig: &Figure) -> CaseResult {
    let (x, y) = sinc_curve();
    let ax = fig.subplots(1, 2)?;
    ax[(0, 0)].plot(&x, &y)?;
    ax[(0, 0)].set_title("Single curve");
    ax[(0, 1)].plot(&x, &y)?;
    ax[(0, 1)].plot(map(&x, |x| x * 2.0), &y)?;
    ax[(0, 1)].set_title("Two curves");
    Ok(())
}

fn autoscale_7(fig: &Figure) -> CaseResult {
    let (x, y) = sinc_curve();
    let shifted = map(&x, |x| x + PI * 0.5);
    let ax = fig.subplots(1, 2)?;
    for (col, title) in [
        "set_xlim(left=-1, right=1)\n",
        "set_xlim(left=-1, right=1)\nautoscale()",
    ]
    .into_iter()
    .enumerate()
    {
        let ax = &ax[(0, col)];
        ax.plot(&x, &y)?;
        ax.set_xlim(-1.0, 1.0)?;
        ax.plot(&shifted, &y)?;
        if col == 1 {
            ax.autoscale(None, None);
        }
        ax.set_title(title);
    }
    Ok(())
}

fn autoscale_8(fig: &Figure) -> CaseResult {
    let ax = sinc_axes(fig)?;
    ax.set_margins(0.2, 0.2)?;
    ax.autoscale_with(None, Some(AxisName::X), Some(true));
    Ok(())
}

fn autoscale_9(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = linspace(-6.0, 6.0, 201);
    ax.plot(&x, map(&x, f64::sin))?;
    Ok(())
}

fn autoscale_10(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    fig.suptitle("Margins are visually constant, even with different data limits and axis scales");
    ax1.plot([0.0, 10.0], [0.0, 1.0])?;
    ax1.set_margins(0.2, 0.2)?;
    let x = linspace(1.0, 20.0, 50);
    ax2.semilogy(&x, map(&x, f64::exp))?;
    ax2.set_margins(0.2, 0.2)?;
    Ok(())
}

fn autoscale_11(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    ax1.plot([0.3, 4.7], [0.3, 4.7])?;
    ax1.set_title("autolimit_mode='data' (default)");
    // The page autoscales the right axes once under `round_numbers`,
    // whose limits the axes then keeps.
    ax2.plot([0.3, 4.7], [0.3, 4.7])?;
    ax2.set_title("autolimit_mode='round_numbers'");
    ax2.set_autolimit_mode(AutolimitMode::RoundNumbers);
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_guides_axes/autoscale_1", autoscale_1),
    FigureCase::new("gallery_guides_axes/autoscale_2", autoscale_2),
    FigureCase::new("gallery_guides_axes/autoscale_3", autoscale_3),
    FigureCase::new("gallery_guides_axes/autoscale_4", autoscale_4).sized(12.0, 8.0, 100.0),
    FigureCase::new("gallery_guides_axes/autoscale_5", autoscale_5).sized(16.0, 10.0, 100.0),
    FigureCase::new("gallery_guides_axes/autoscale_6", autoscale_6).sized(12.0, 8.0, 100.0),
    FigureCase::new("gallery_guides_axes/autoscale_7", autoscale_7).sized(12.0, 8.0, 100.0),
    FigureCase::new("gallery_guides_axes/autoscale_8", autoscale_8),
    FigureCase::new("gallery_guides_axes/autoscale_9", autoscale_9),
    FigureCase::new("gallery_guides_axes/autoscale_10", autoscale_10).sized(9.0, 4.0, 100.0),
    FigureCase::new("gallery_guides_axes/autoscale_11", autoscale_11).sized(10.0, 4.0, 100.0),
];
