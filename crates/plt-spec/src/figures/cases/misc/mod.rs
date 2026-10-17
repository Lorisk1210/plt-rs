//! `figures/cases_axes_misc.py`, split by section.

mod artists;
mod axes_grid;
mod axisartist;
mod dates;
mod insets;
mod sankey;
mod spectral;
mod units;
mod widgets;

pub(crate) use artists::CASES as ARTISTS;
pub(crate) use axes_grid::CASES as AXES_GRID;
pub(crate) use axisartist::CASES as AXISARTIST;
pub(crate) use dates::CASES as DATES;
pub(crate) use insets::CASES as INSETS;
pub(crate) use sankey::CASES as SANKEY;
pub(crate) use spectral::CASES as SPECTRAL;
pub(crate) use units::CASES as UNITS;
pub(crate) use widgets::CASES as WIDGETS;

use plt::numeric::{Grid, linspace};

/// `_wave(n)`: n points spanning a smooth deterministic wave.
fn wave(n: usize) -> Vec<f64> {
    linspace(0.0, 3.0 * std::f64::consts::PI, n)
        .into_iter()
        .zip(linspace(0.0, 1.0, n))
        .map(|(t, ramp)| t.sin() + ramp)
        .collect()
}

/// `_field(n)`: `sin(x) * cos(y)` on an `n × n` grid over `[-3, 3]²`.
fn field(n: usize) -> Grid<f64> {
    let x = linspace(-3.0, 3.0, n);
    Grid::from_fn(n, n, |r, c| x[c].sin() * x[r].cos())
}
