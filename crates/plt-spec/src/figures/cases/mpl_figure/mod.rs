//! `figures/cases_mpl_figure.py`: matplotlib's own image tests from
//! `test_figure.py` (three submodules: figure-level artists and layouts,
//! mosaics, subfigures), `test_constrainedlayout.py`,
//! `test_tightlayout.py` and `test_bbox_tight.py` (one submodule each).

mod bbox_tight;
mod constrained;
mod figure;
mod mosaic;
mod subfigures;
mod tight;

use plt::axes::{Mesh, PcolorOptions};
use plt::collections::Collection;
use plt::colors::{ColorizerOptions, colormaps};
use plt::numeric::Grid;
use plt::prelude::*;

use super::common::{decorate, example_plot};
use super::mpl_tests::{classic, remove_ticks_and_titles};
use crate::figures::FigureCase;
use crate::rng::SpecRng;

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [
        figure::CASES,
        mosaic::CASES,
        subfigures::CASES,
        constrained::CASES,
        tight::CASES,
        bbox_tight::CASES,
    ]
    .concat()
}

/// What a case's helpers may fail with.
type Fallible<T> = Result<T, Box<dyn std::error::Error>>;

/// `savefig_kwarg={"bbox_inches": "tight"}`.
const TIGHT_BBOX: (&str, &str) = ("savefig.bbox", "tight");

/// `savefig_kwarg={"facecolor": "teal"}`.
const TEAL: (&str, &str) = ("savefig.facecolor", "teal");

/// `np.random.randn(n, n)` drawn from `stream`.
fn randn(stream: &mut SpecRng, n: usize) -> Fallible<Grid<f64>> {
    Ok(Grid::from_vec(n, n, stream.normal(n * n, 0.0, 1.0))?)
}

/// The layout tests' `example_pcolor`: a rasterized `RdBu_r` mesh of
/// `(1 - x/2 + x⁵ + y³) exp(-x² - y²)` on an 11 × 11 grid spaced 0.6
/// apart, labelled in `fontsize` points.
pub(super) fn example_pcolor(ax: &Axes, fontsize: f64) -> Fallible<Collection> {
    const N: usize = 11;
    let at = |i: usize| i as f64 * 0.6 + -3.0;
    let x = Grid::from_fn(N, N, |_, col| at(col));
    let y = Grid::from_fn(N, N, |row, _| at(row));
    let z = Grid::from_fn(N - 1, N - 1, |row, col| {
        let (x, y) = (at(col), at(row));
        (1.0 - x / 2.0 + x.powf(5.0) + y.powf(3.0)) * (-(x * x) - y * y).exp()
    });
    let options = PcolorOptions {
        colorizer: ColorizerOptions::default()
            .cmap(colormaps::get("RdBu_r")?)
            .limits(Some(-1.0), Some(1.0)),
        ..PcolorOptions::default()
    };
    let mesh = ax.pcolormesh(Mesh::from_coords(x, y, z), options)?;
    mesh.set_rasterized(true);
    decorate(ax, fontsize);
    Ok(mesh)
}
