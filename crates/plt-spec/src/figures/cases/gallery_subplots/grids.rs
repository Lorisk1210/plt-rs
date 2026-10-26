//! Axes placed by hand and on grids: `axes_demo`,
//! `gridspec_customization`, `gridspec_multicolumn`, `gridspec_nested`,
//! `gridspec_and_subplots` and `subplot2grid`.

use std::ops::Range;

use plt::axes::{HistPlotOptions, TickParams};
use plt::gridspec::{GridSpec, GridSpecError, SubplotOverrides};
use plt::layout::{LayoutEngine, TightLayout};
use plt::prelude::*;
use plt::stats::Bins;
use plt::text::{AnnotateOptions, HAlign, VAlign};

use super::{SEED, convolve};
use crate::figures::cases::common::{map, tickless};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `_label_axes`: names every axes in its middle (`ax1`, `ax2`, ...) and
/// hides the tick labels.
fn label_axes(fig: &Figure) {
    for (i, ax) in fig.axes().iter().enumerate() {
        ax.text(0.5, 0.5, format!("ax{}", i + 1))
            .set_valign(VAlign::Center)
            .set_halign(HAlign::Center);
        ax.tick_params(&TickParams {
            labelbottom: Some(false),
            labelleft: Some(false),
            ..TickParams::default()
        });
    }
}

/// Adds one axes per `(rows, cols)` block of `grid`.
fn add_blocks(
    fig: &Figure,
    grid: &GridSpec,
    blocks: &[(Range<usize>, Range<usize>)],
) -> CaseResult {
    for (rows, cols) in blocks {
        fig.add_subplot_spec(&grid.span(rows.clone(), cols.clone())?)?;
    }
    Ok(())
}

/// The block layout of `gridspec_customization`'s and `gridspec_nested`'s
/// left grid: a wide top block over a wide and a small cell.
const WIDE_TOP: [(Range<usize>, Range<usize>); 3] = [(0..2, 0..3), (2..3, 0..2), (2..3, 2..3)];

/// Their right grid: a tall left block beside a tall and a small cell.
const TALL_LEFT: [(Range<usize>, Range<usize>); 3] = [(0..3, 0..2), (0..2, 2..3), (2..3, 2..3)];

/// The blocks of `gridspec_multicolumn` and `subplot2grid` on a 3 × 3
/// grid: a full row, two thirds of a row beside a tall cell, two cells.
const MULTICOLUMN: [(Range<usize>, Range<usize>); 5] = [
    (0..1, 0..3),
    (1..2, 0..2),
    (1..3, 2..3),
    (2..3, 0..1),
    (2..3, 1..2),
];

fn axes_demo(fig: &Figure) -> CaseResult {
    let dt = 0.001;
    let t = arange(0.0, 10.0, dt);
    let r = map(&t[..1000], |t| (-t / 0.05).exp());
    let x = SpecRng::new(SEED).normal(t.len(), 0.0, 1.0);
    let s: Vec<f64> = convolve(&x, &r)[..x.len()].iter().map(|v| v * dt).collect();
    let main_ax = fig.subplot();
    main_ax.plot(&t, &s)?;
    let (low, high) = s
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
            (lo.min(v), hi.max(v))
        });
    main_ax
        .set_xlim(0.0, 1.0)?
        .set_ylim(1.1 * low, 2.0 * high)?;
    main_ax.set_xlabel("time (s)").set_ylabel("current (nA)");
    main_ax.set_title("Gaussian colored noise");
    let right_inset_ax = fig.add_axes([0.65, 0.6, 0.2, 0.2])?;
    right_inset_ax.set_facecolor("k");
    let density = HistPlotOptions {
        bins: Some(Bins::Count(400)),
        density: true,
        ..HistPlotOptions::default()
    };
    right_inset_ax.hist(&[&s], density)?;
    right_inset_ax.set_title("Probability");
    tickless(&right_inset_ax);
    let left_inset_ax = fig.add_axes([0.2, 0.6, 0.2, 0.2])?;
    left_inset_ax.set_facecolor("k");
    left_inset_ax.plot(&t[..r.len()], &r)?;
    left_inset_ax.set_title("Impulse response");
    left_inset_ax.set_xlim(0.0, 0.2)?;
    tickless(&left_inset_ax);
    Ok(())
}

fn gridspec_customization_1(fig: &Figure) -> CaseResult {
    fig.suptitle("Controlling subplot sizes with width_ratios and height_ratios");
    let grid = GridSpec::with_ratios(
        2,
        2,
        Some(vec![1.0, 2.0]),
        Some(vec![4.0, 1.0]),
        SubplotOverrides::default(),
    )?;
    for i in 1..=4 {
        fig.add_subplot_spec(&grid.numbered(i)?)?;
    }
    label_axes(fig);
    Ok(())
}

/// A 3 × 3 grid between `left` and `right` with the given spacing.
fn margined_grid(
    left: f64,
    right: f64,
    wspace: Option<f64>,
    hspace: Option<f64>,
) -> Result<GridSpec, GridSpecError> {
    GridSpec::with_ratios(
        3,
        3,
        None,
        None,
        SubplotOverrides {
            left: Some(left),
            right: Some(right),
            wspace,
            hspace,
            ..SubplotOverrides::default()
        },
    )
}

fn gridspec_customization_2(fig: &Figure) -> CaseResult {
    fig.suptitle("Controlling spacing around and between subplots");
    add_blocks(
        fig,
        &margined_grid(0.05, 0.48, Some(0.05), None)?,
        &WIDE_TOP,
    )?;
    add_blocks(
        fig,
        &margined_grid(0.55, 0.98, None, Some(0.05))?,
        &TALL_LEFT,
    )?;
    label_axes(fig);
    Ok(())
}

fn gridspec_multicolumn(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(Some(LayoutEngine::constrained()))?;
    add_blocks(fig, &GridSpec::new(3, 3)?, &MULTICOLUMN)?;
    fig.suptitle("GridSpec");
    label_axes(fig);
    Ok(())
}

fn gridspec_nested(fig: &Figure) -> CaseResult {
    let outer = GridSpec::new(1, 2)?;
    for (col, blocks) in [WIDE_TOP, TALL_LEFT].iter().enumerate() {
        let inner = GridSpec::nested(&outer.cell(0, col)?, 3, 3, None, None, (None, None))?;
        add_blocks(fig, &inner, blocks)?;
    }
    fig.suptitle("GridSpec Inside GridSpec");
    label_axes(fig);
    Ok(())
}

fn gridspec_and_subplots(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(3, 3)?;
    let grid = axs[(1, 2)]
        .subplot_spec()
        .ok_or("subplots lie on a grid")?
        .gridspec()
        .clone();
    for row in 1..3 {
        axs[(row, 2)].remove();
    }
    let axbig = fig.add_subplot_spec(&grid.span(1..3, 2..3)?)?;
    axbig
        .annotate(
            "Big Axes \nGridSpec[1:, -1]",
            (0.1, 0.5),
            AnnotateOptions {
                xycoords: "axes fraction".parse()?,
                ..AnnotateOptions::default()
            },
        )?
        .set_valign(VAlign::Center);
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

fn subplot2grid(fig: &Figure) -> CaseResult {
    add_blocks(fig, &GridSpec::new(3, 3)?, &MULTICOLUMN)?;
    label_axes(fig);
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_subplots/axes_demo", axes_demo),
    FigureCase::new(
        "gallery_subplots/gridspec_customization_1",
        gridspec_customization_1,
    ),
    FigureCase::new(
        "gallery_subplots/gridspec_customization_2",
        gridspec_customization_2,
    ),
    FigureCase::new(
        "gallery_subplots/gridspec_multicolumn",
        gridspec_multicolumn,
    ),
    FigureCase::new("gallery_subplots/gridspec_nested", gridspec_nested),
    FigureCase::new(
        "gallery_subplots/gridspec_and_subplots",
        gridspec_and_subplots,
    ),
    FigureCase::new("gallery_subplots/subplot2grid", subplot2grid),
];
