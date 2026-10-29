//! `figures/cases_mpl_rendering.py`: matplotlib's own image tests of
//! `test_simplification.py`, `test_quiver.py`, `test_streamplot.py`,
//! `test_patheffects.py`, `test_arrow_patches.py` and `test_widgets.py`,
//! one submodule each.

mod arrow_patches;
mod patheffects;
mod quiver;
mod simplification;
mod streamplot;
mod widgets;

use crate::figures::FigureCase;

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [
        simplification::CASES,
        quiver::CASES,
        widgets::CASES,
        streamplot::CASES,
        patheffects::CASES,
        arrow_patches::CASES,
    ]
    .concat()
}
