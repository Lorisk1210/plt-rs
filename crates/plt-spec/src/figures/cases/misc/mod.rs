//! `figures/cases_axes_misc.py`, split by section.

mod artists;
mod dates;
mod sankey;
mod spectral;
mod units;
mod widgets;

pub(crate) use artists::CASES as ARTISTS;
pub(crate) use dates::CASES as DATES;
pub(crate) use sankey::CASES as SANKEY;
pub(crate) use spectral::CASES as SPECTRAL;
pub(crate) use units::CASES as UNITS;
pub(crate) use widgets::CASES as WIDGETS;

use plt::numeric::linspace;

/// `_wave(n)`: n points spanning a smooth deterministic wave.
fn wave(n: usize) -> Vec<f64> {
    linspace(0.0, 3.0 * std::f64::consts::PI, n)
        .into_iter()
        .zip(linspace(0.0, 1.0, n))
        .map(|(t, ramp)| t.sin() + ramp)
        .collect()
}
