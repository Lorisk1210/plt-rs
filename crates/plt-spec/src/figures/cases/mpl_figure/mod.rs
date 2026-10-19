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

use plt::axes::{LocatorParams, Mesh, PcolorOptions};
use plt::collections::Collection;
use plt::colors::colormaps;
use plt::fonts::FontSize;
use plt::image::ColorizerOptions;
use plt::layout::{ConstrainedLayout, LayoutEngine};
use plt::numeric::Grid;
use plt::prelude::*;
use plt::ticker::Nbins;

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

/// `fig.set_layout_engine("constrained")` (or `"compressed"`).
fn constrained(fig: &Figure, compress: bool) {
    fig.set_layout_engine(Some(LayoutEngine::Constrained(ConstrainedLayout {
        compress,
        ..ConstrainedLayout::from_rc(&plt::rc::params())
    })));
}

/// `np.random.randn(n, n)` drawn from `stream`.
fn randn(stream: &mut SpecRng, n: usize) -> Fallible<Grid<f64>> {
    Ok(Grid::from_vec(n, n, stream.normal(n * n, 0.0, 1.0))?)
}

/// `ax.set_xlabel`, `ax.set_ylabel` and `ax.set_title` in `fontsize`
/// points, as both layout tests' helpers label their axes.
fn decorate(ax: &Axes, fontsize: f64) {
    ax.set_xlabel("x-label").set_ylabel("y-label");
    for axis in [ax.xaxis(), ax.yaxis()] {
        axis.set_label_fontsize(FontSize::Points(fontsize));
    }
    ax.set_title("Title").set_fontsize(fontsize);
}

/// The layout tests' `example_plot`: a line with at most three tick
/// intervals, labelled in `fontsize` points unless `nodec`, in which case
/// the tick labels are blanked instead.
fn example_plot(ax: &Axes, fontsize: f64, nodec: bool) -> Fallible<()> {
    ax.plot_y([1.0, 2.0]);
    ax.locator_params(&LocatorParams {
        nbins: Some(Nbins::Fixed(3)),
        ..LocatorParams::default()
    })?;
    if nodec {
        ax.set_xticklabels(Vec::<String>::new())?;
        ax.set_yticklabels(Vec::<String>::new())?;
    } else {
        decorate(ax, fontsize);
    }
    Ok(())
}

/// The layout tests' `example_pcolor`: a rasterized `RdBu_r` mesh of
/// `(1 - x/2 + x⁵ + y³) exp(-x² - y²)` on an 11 × 11 grid spaced 0.6
/// apart, labelled in `fontsize` points.
fn example_pcolor(ax: &Axes, fontsize: f64) -> Fallible<Collection> {
    const N: usize = 11;
    let at = |i: usize| i as f64 * 0.6 + -3.0;
    let x = Grid::from_fn(N, N, |_, col| at(col));
    let y = Grid::from_fn(N, N, |row, _| at(row));
    let z = Grid::from_fn(N - 1, N - 1, |row, col| {
        let (x, y) = (at(col), at(row));
        (1.0 - x / 2.0 + x.powf(5.0) + y.powf(3.0)) * (-(x * x) - y * y).exp()
    });
    let options = PcolorOptions {
        colors: ColorizerOptions::default()
            .cmap(colormaps::get("RdBu_r")?)
            .limits(Some(-1.0), Some(1.0)),
        ..PcolorOptions::default()
    };
    let mesh = ax.pcolormesh(Mesh::with_coords(x, y, z), options)?;
    mesh.set_rasterized(true);
    decorate(ax, fontsize);
    Ok(mesh)
}
