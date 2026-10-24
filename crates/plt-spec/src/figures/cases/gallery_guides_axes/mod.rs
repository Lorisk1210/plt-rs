//! `figures/cases_gallery_guides_axes.py`: matplotlib's user guide pages
//! on axes and artists, one case per figure of each page.

mod arranging;
mod artists;
mod autoscale;
mod colorbars;
mod constrained;
mod constrained_grids;
mod extents;
mod handlers;
mod legends;
mod mosaics;
mod scales;
mod ticks;
mod tight;
mod transforms;
mod units;

use plt::numeric::Grid;
use plt::prelude::*;
use plt::text::{HAlign, Text, TextProps, VAlign};

use crate::figures::cases::common::example_plot;
use crate::figures::{CaseResult, FigureCase};

/// The seed of the pages' `np.random.seed(19680801)`.
const SEED: u64 = 19_680_801;

/// The rcParams the constrained and tight layout guides set first.
const LAYOUT_RC: &[(&str, &str)] = &[("savefig.facecolor", "0.8")];

/// The layout guides' `arr`: `np.arange(100).reshape((10, 10))`.
fn arange_grid() -> Grid<f64> {
    Grid::from_fn(10, 10, |row, col| (row * 10 + col) as f64)
}

/// `plt.subplot2grid`'s four axes of the layout guides on a 3 × 3 grid,
/// each an `example_plot`.
fn subplot2grid(fig: &Figure) -> CaseResult {
    let gs = fig.add_gridspec(3, 3)?;
    for (rows, cols) in [(0..1, 0..1), (0..1, 1..3), (1..3, 0..2), (1..3, 2..3)] {
        example_plot(&fig.add_subplot_spec(&gs.span(rows, cols)?)?, 12.0, false)?;
    }
    Ok(())
}

/// Centres `text` on its position in dark grey at `points` (the labels of
/// `arranging_axes` and `mosaic`).
fn centered(text: &Text, points: f64) {
    text.update_props(
        TextProps::default()
            .halign(HAlign::Center)
            .valign(VAlign::Center)
            .color("darkgrey"),
    )
    .set_fontsize(points);
}

/// `arranging_axes`' `annotate_axes`: `text` in the middle of `ax`.
fn annotate_axes(ax: &Axes, text: &str, points: f64) {
    centered(
        ax.text(0.5, 0.5, text).set_transform(ax.trans_axes()),
        points,
    );
}

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [
        arranging::CASES,
        artists::CASES,
        autoscale::CASES,
        colorbars::CASES,
        constrained::CASES,
        constrained_grids::CASES,
        extents::CASES,
        legends::CASES,
        mosaics::CASES,
        scales::CASES,
        ticks::CASES,
        tight::CASES,
        transforms::CASES,
        units::CASES,
    ]
    .concat()
}
