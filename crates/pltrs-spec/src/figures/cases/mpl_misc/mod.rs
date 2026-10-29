//! `figures/cases_mpl_misc.py`: matplotlib's own image tests of
//! `test_legend.py`, `test_offsetbox.py`, `test_dates.py`,
//! `test_table.py`, `test_colors.py`, `test_triangulation.py`,
//! `test_agg.py`, `test_agg_filter.py`, `test_skew.py` and
//! `test_units.py`, drawn under the conditions of matplotlib's test
//! decorators: the test's style (by default "classic" plus
//! `_classic_test_patch`, on an 8 × 6 in canvas saved at 100 dpi), and
//! `remove_ticks_and_titles` for `remove_text=True`. Legends, offset boxes
//! and tables, dates and units, colours, triangulations, and artist
//! filters and transforms each have a submodule.

mod artists;
mod boxes;
mod colors;
mod dates;
mod legend;
mod tri;

use super::mpl_tests::{classic, remove_ticks_and_titles as remove_text};
use crate::figures::FigureCase;

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [
        legend::CASES,
        boxes::CASES,
        dates::CASES,
        colors::CASES,
        tri::CASES,
        artists::CASES,
    ]
    .concat()
}
