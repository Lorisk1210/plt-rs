//! `figures/cases_axes.py`, split by section.

mod formatting;
mod grids;
mod labels;
mod limits;
mod scales;
mod spines;
mod ticks;
mod twins;

pub(crate) use formatting::CASES as FORMATTING;
pub(crate) use grids::CASES as GRIDS;
pub(crate) use labels::CASES as LABELS;
pub(crate) use limits::CASES as LIMITS;
pub(crate) use scales::CASES as SCALES;
pub(crate) use spines::CASES as SPINES;
pub(crate) use ticks::CASES as TICKS;
pub(crate) use twins::CASES as TWINS;

use pltrs::prelude::*;

/// `T = np.linspace(0, 10, 50)`.
fn t() -> Vec<f64> {
    linspace(0.0, 10.0, 50)
}

/// `T` mapped through `f`.
fn of_t(f: impl Fn(f64) -> f64) -> Vec<f64> {
    t().into_iter().map(f).collect()
}

/// `_line(ax)`: `ax.plot(T, np.sin(T))`.
fn line(ax: &Axes) -> Result<(), pltrs::axes::PlotError> {
    ax.plot(t(), of_t(f64::sin)).map(drop)
}
