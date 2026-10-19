//! `figures/cases_text.py`, split by section.

mod annotate;
mod boxes;
mod effects;
mod gallery;
mod layout;
mod math;
mod table_cells;
mod tables;

pub(crate) use annotate::CASES as ANNOTATE;
pub(crate) use boxes::CASES as BOXES;
pub(crate) use effects::CASES as EFFECTS;
pub(crate) use gallery::CASES as GALLERY;
pub(crate) use layout::CASES as LAYOUT;
pub(crate) use math::CASES as MATH;
pub(crate) use table_cells::CASES as TABLE_CELLS;
pub(crate) use tables::CASES as TABLES;

use crate::figures::cases::common::{blank, styled_box};

/// `LOREM`: the filler paragraph of the wrapping cases.
const LOREM: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod \
                     tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim \
                     veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea \
                     commodo consequat.";
