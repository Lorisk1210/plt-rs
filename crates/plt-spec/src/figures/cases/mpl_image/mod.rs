//! `figures/cases_mpl_image.py`: matplotlib's own image tests of
//! `test_image.py`, drawn under the conditions of matplotlib's test
//! decorators: the test's style (by default "classic" plus
//! `_classic_test_patch`, on an 8 × 6 in canvas saved at 100 dpi), its
//! savefig dpi, and `remove_ticks_and_titles` for `remove_text=True`.
//! Interpolation and alpha, placement and clipping, masks and norms,
//! resampling, and pixel alignment each have a submodule.

mod alignment;
mod alpha;
mod placement;
mod resampling;
mod values;

use std::f64::consts::PI;

use plt::axes::ImshowOptions;
use plt::colors::{ColormapError, colormaps};
use plt::image::{ColorizerOptions, ImageData, ScalarField};
use plt::numeric::{Grid, arange};

use super::mpl_tests::{classic, remove_ticks_and_titles};
use crate::figures::FigureCase;
use crate::rng::SpecRng;

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [
        alpha::CASES,
        placement::CASES,
        values::CASES,
        resampling::CASES,
        alignment::CASES,
    ]
    .concat()
}

/// `np.random.rand(rows, cols)` after `np.random.seed(seed)`: the uniform
/// draws of `spec_support.rng(seed)`, row by row.
fn uniform(seed: u64, rows: usize, cols: usize) -> Grid<f64> {
    let values = SpecRng::new(seed).uniform(rows * cols, 0.0, 1.0);
    Grid::from_vec(rows, cols, values).expect("rows × cols draws")
}

/// `imshow` options with only the colormap `name` set.
fn colormapped(name: &str) -> Result<ImshowOptions, ColormapError> {
    Ok(ImshowOptions {
        colors: ColorizerOptions::default().cmap(colormaps::get(name)?),
        ..ImshowOptions::default()
    })
}

/// `np.arange(rows * cols).reshape(rows, cols)`: integers counting row by
/// row.
fn counting(rows: usize, cols: usize) -> ImageData {
    let values = Grid::from_fn(rows, cols, |r, c| (r * cols + c) as i64);
    ScalarField::integers(&values).into()
}

/// The difference of Gaussians of the rotate and mask tests, sampled
/// every `delta` over `[-3, 3)` and scaled by `scale`.
fn gaussians(delta: f64, scale: f64) -> Grid<f64> {
    let x = arange(-3.0, 3.0, delta);
    Grid::from_fn(x.len(), x.len(), |r, c| {
        let (x, y) = (x[c], x[r]);
        let z1 = (-(x * x + y * y) / 2.0).exp() / (2.0 * PI);
        let z2 = (-(((x - 1.0) / 1.5).powi(2) + ((y - 1.0) / 0.5).powi(2)) / 2.0).exp()
            / (2.0 * PI * 0.5 * 1.5);
        scale * (z2 - z1)
    })
}
