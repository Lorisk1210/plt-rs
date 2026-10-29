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
