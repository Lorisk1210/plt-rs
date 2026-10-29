//! `figures/cases_mpl_small.py`: matplotlib's own image tests from its
//! smaller test modules: spines and scales in one submodule, markers and
//! artists in another, subplot layouts and transforms in a third.

mod layouts;
mod markers;
mod spines;

pub(super) use super::mpl_tests::{CLASSIC_TEST_PATCH, remove_ticks_and_titles as remove_text};
use crate::figures::FigureCase;

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [spines::CASES, markers::CASES, layouts::CASES].concat()
}
