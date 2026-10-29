//! `figures/cases_mpl_rest.py`: matplotlib's own image tests of
//! `test_multivariate_colormaps.py`, `test_sankey.py`, `test_png.py`,
//! `test_ft2font.py`, `test_category.py`, `test_axis.py`,
//! `test_animation.py`, `test_preprocess_data.py`, `test_backend_cairo.py`
//! and the `fig_ref` side of `test_pickle.py`, drawn under the conditions
//! of matplotlib's test decorators: the test's style (by default
//! "classic" plus `_classic_test_patch`, on an 8 × 6 in canvas saved at
//! 100 dpi), and `remove_ticks_and_titles` for `remove_text=True`.
//! Multivariate colormaps, PNG files, plotting methods, texts and the
//! pickled figure each have a submodule.

mod colormaps;
mod pickle;
mod plots;
mod png;
mod text;

use super::mpl_tests::{classic, remove_ticks_and_titles as remove_text};
use crate::figures::FigureCase;

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [
        colormaps::CASES,
        png::CASES,
        plots::CASES,
        text::CASES,
        pickle::CASES,
    ]
    .concat()
}
