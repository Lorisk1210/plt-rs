//! `figures/cases_mpl_toolkits.py`: matplotlib's own image tests of
//! `mpl_toolkits` — `axes_grid1/tests/test_axes_grid1.py` (inset axes in
//! one submodule, twins, anchored artists and image grids in another) and
//! `axisartist/tests/` (axis lines in a third, axis artists and tick
//! marks on plain axes in a fourth, curvilinear grids and floating axes
//! in a fifth).
//!
//! Cases keep the test's style: the test suite's default `classic` plus
//! `_classic_test_patch` (8 × 6 in figures saved at 100 dpi), or the
//! reference configuration for tests asking for `mpl20`/`default`.
//! `remove_text=True` becomes [`remove_ticks_and_titles`] at the end of
//! the build, and `savefig(bbox_inches="tight")` the case's
//! `savefig.bbox`.

mod axis_artist;
mod axislines;
mod curvelinear;
mod grids;
mod insets;

use plt::numeric::Grid;

use super::mpl_tests::{classic, remove_ticks_and_titles};
use crate::figures::FigureCase;

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [
        insets::CASES,
        grids::CASES,
        axislines::CASES,
        axis_artist::CASES,
        curvelinear::CASES,
    ]
    .concat()
}

/// `cbook.get_sample_data("axes_grid/bivariate_normal.npy")`: ten times
/// the difference of two bivariate normal densities, the standard one and
/// one centred at `(1, 1)` with deviations 1.5 and 0.5, on the 15 × 15
/// grid `x = -3, -2.5, ..., 4`, `y = -4, -3.5, ..., 3`.
fn bivariate_normal() -> Grid<f64> {
    let density = |x: f64, y: f64, (sigma_x, sigma_y): (f64, f64), (mu_x, mu_y): (f64, f64)| {
        let z = (x - mu_x).powi(2) / sigma_x.powi(2) + (y - mu_y).powi(2) / sigma_y.powi(2);
        (-z / 2.0).exp() / (2.0 * std::f64::consts::PI * sigma_x * sigma_y)
    };
    Grid::from_fn(15, 15, |r, c| {
        let (x, y) = (-3.0 + 0.5 * c as f64, -4.0 + 0.5 * r as f64);
        (density(x, y, (1.0, 1.0), (0.0, 0.0)) - density(x, y, (1.5, 0.5), (1.0, 1.0))) * 10.0
    })
}
