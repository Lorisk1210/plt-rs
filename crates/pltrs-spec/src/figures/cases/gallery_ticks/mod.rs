//! `figures/cases_gallery_ticks.py`: the gallery's ticks, scales, units
//! and colour sections, one case per figure. Tick labels, locators and
//! formatters, reference strips, date ticks, scales, colours, colormaps
//! and units each have a submodule.

mod colormaps;
mod colors;
mod concise;
mod dates;
mod formatters;
mod labels;
mod log_scales;
mod mercator;
mod scales;
mod strips;
mod units;

use crate::figures::FigureCase;

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [
        labels::CASES,
        formatters::CASES,
        strips::CASES,
        dates::CASES,
        concise::CASES,
        scales::CASES,
        log_scales::CASES,
        mercator::CASES,
        colors::CASES,
        colormaps::CASES,
        units::CASES,
    ]
    .concat()
}
