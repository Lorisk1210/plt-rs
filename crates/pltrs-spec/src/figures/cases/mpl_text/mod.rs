//! `figures/cases_mpl_text.py`: matplotlib's own image tests of
//! `test_text.py` and `test_mathtext.py`, drawn under the conditions of
//! matplotlib's test decorators: the test's style ("mpl20", the reference
//! configuration, or by default "classic" plus `_classic_test_patch` on an
//! 8 × 6 in canvas saved at 100 dpi), its savefig dpi, and
//! `remove_ticks_and_titles` for `remove_text=True`. Fonts, lines and
//! alignment; annotations, clipping and opacity; wrapping and tick label
//! rotation; shaping; mathtext; and the expressions of the parametrised
//! mathtext tests each have a submodule.

mod annotations;
mod layout;
mod math;
mod math_tests;
mod shaping;
mod wrapping;

use super::mpl_tests::{classic, remove_ticks_and_titles};
use crate::figures::FigureCase;

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [
        layout::CASES,
        annotations::CASES,
        wrapping::CASES,
        shaping::CASES,
        math::CASES,
        &math_tests::cases(),
    ]
    .concat()
}
