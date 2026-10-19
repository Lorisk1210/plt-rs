//! `figures/cases_mpl_axes_b.py`: matplotlib's own image tests of
//! `test_axes.py` (bars, grouped bars, histograms, stairs, contours,
//! hist2d, scatter, stackplot and `bxp`), split by plot family.

mod bars;
mod bxp;
mod contour;
mod hist;
mod scatter;
mod stackplot;
mod stairs;

use plt::axes::Limits;

use super::mpl_tests::{classic, remove_ticks_and_titles};
use crate::figures::FigureCase;

/// New view limits from `low` on, keeping the other limit
/// (`set_ylim(bottom=low)` and the like).
fn from(low: f64) -> Limits {
    Limits {
        low: Some(low),
        ..Limits::default()
    }
}

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [
        bars::CASES,
        hist::CASES,
        stairs::CASES,
        contour::CASES,
        scatter::CASES,
        stackplot::CASES,
        bxp::CASES,
    ]
    .concat()
}
