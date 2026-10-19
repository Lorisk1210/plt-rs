//! `figures/cases_mpl_axes_c.py`: matplotlib's own image tests of
//! `test_axes.py` from `test_bxp_customwidths` to `test_eb_line_zorder`,
//! drawn under the conditions of matplotlib's test decorators: the
//! test's style (by default "classic" plus `_classic_test_patch`, on an
//! 8 × 6 in canvas saved at 100 dpi), its savefig dpi, and
//! `remove_ticks_and_titles` for `remove_text=True`.

mod boxplots;
mod errorbars;
mod events;
mod hists;
mod markers;
mod violins;

use plt::figure::Figure;

use super::mpl_tests::{classic, remove_ticks_and_titles};
use crate::figures::{CaseResult, FigureCase};

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [
        boxplots::CASES,
        errorbars::CASES,
        events::CASES,
        hists::CASES,
        markers::CASES,
        violins::CASES,
    ]
    .concat()
}

/// A case in the default style saved at `savefig_kwarg={'dpi': 40}`.
const fn at_40_dpi(name: &'static str, build: fn(&Figure) -> CaseResult) -> FigureCase {
    FigureCase::new(name, build).sized(6.4, 4.8, 40.0)
}

/// `values` mapped through `f`.
fn map(values: &[f64], f: impl Fn(f64) -> f64) -> Vec<f64> {
    values.iter().copied().map(f).collect()
}
