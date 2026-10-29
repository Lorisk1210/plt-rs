//! `figures/cases_gallery_lines.py`: the gallery's "Lines, bars and
//! markers" section, one case per figure of each example.

mod bars;
mod fills;
mod lines;
mod markers;
mod scatter;
mod series;

use crate::figures::FigureCase;

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [
        bars::CASES,
        fills::CASES,
        lines::CASES,
        markers::CASES,
        scatter::CASES,
        series::CASES,
    ]
    .concat()
}
