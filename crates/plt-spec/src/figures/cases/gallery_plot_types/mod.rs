//! `figures/cases_gallery_plot_types.py`: the gallery's "Plot types"
//! overview, one 2 × 2 inch thumbnail per example under matplotlib's
//! gallery style sheets. The basic plots, the plots of arrays and fields,
//! the statistics plots, the unstructured grids and the 3-D plots each
//! have a submodule.

mod arrays;
mod basic;
mod stats;
mod three_d;
mod unstructured;

use plt::axes::Axes;
use plt::contour::{ContourOptions, Levels};
use plt::figure::Figure;
use plt::numeric::{Grid, arange, linspace};

use crate::figures::cases::common::func3;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// A case of the group on its 2 × 2 inch canvas, styled by `sheet`.
const fn thumbnail(
    name: &'static str,
    sheet: &'static str,
    build: fn(&Figure) -> CaseResult,
) -> FigureCase {
    FigureCase::new(name, build)
        .sized(2.0, 2.0, 100.0)
        .with_style(sheet)
}

/// A case under the gallery sheet with a grid (`_mpl-gallery`).
const fn gridded(name: &'static str, build: fn(&Figure) -> CaseResult) -> FigureCase {
    thumbnail(name, "_mpl-gallery", build)
}

/// A case under the gallery sheet without a grid
/// (`_mpl-gallery-nogrid`).
const fn plain(name: &'static str, build: fn(&Figure) -> CaseResult) -> FigureCase {
    thumbnail(name, "_mpl-gallery-nogrid", build)
}

/// The limits and ticks most thumbnails share: 0 to 8 on both axes,
/// ticked at every integer in between.
fn unit_square(ax: &Axes) {
    ax.set_xlim(0.0, 8.0).set_xticks(arange(1.0, 8.0, 1.0));
    ax.set_ylim(0.0, 8.0).set_yticks(arange(1.0, 8.0, 1.0));
}

/// `func3` over an `n × n` grid of `[-3, 3]²`: the coordinates along
/// either axis and the values, one row per y.
fn func3_grid(n: usize) -> (Vec<f64>, Grid<f64>) {
    let t = linspace(-3.0, 3.0, n);
    let z = Grid::from_fn(n, n, |i, j| func3(t[j], t[i]));
    (t, z)
}

/// Seven contour levels from the minimum to the maximum of `z`.
fn extrema_levels(z: &[f64]) -> ContourOptions {
    let (low, high) = z
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), &v| {
            (low.min(v), high.max(v))
        });
    ContourOptions {
        levels: Levels::Values(linspace(low, high, 7)),
        ..ContourOptions::default()
    }
}

/// `np.random.normal(loc, scale, (rows, 3))` read as its three columns.
fn normal_columns(rng: &mut SpecRng, rows: usize, loc: [f64; 3], scale: [f64; 3]) -> Vec<Vec<f64>> {
    let draws = rng.normal(rows * 3, 0.0, 1.0);
    (0..3)
        .map(|column| {
            draws
                .iter()
                .skip(column)
                .step_by(3)
                .map(|z| z * scale[column] + loc[column])
                .collect()
        })
        .collect()
}

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [
        basic::CASES,
        arrays::CASES,
        stats::CASES,
        unstructured::CASES,
        three_d::CASES,
    ]
    .concat()
}
