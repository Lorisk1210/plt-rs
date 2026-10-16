//! Rust translations of the spec's figure cases, one module per group.

mod axes;
mod bars;
mod basic;
mod contour;
mod hist;
mod layout;
mod lines;
mod misc;
mod scatter;
mod stats;

use super::FigureCase;

/// Every translated case.
pub(crate) fn all() -> Vec<FigureCase> {
    [
        axes::FORMATTING,
        axes::GRIDS,
        axes::LABELS,
        axes::LIMITS,
        axes::SCALES,
        axes::SPINES,
        axes::TICKS,
        axes::TWINS,
        bars::CASES,
        basic::CASES,
        contour::CASES,
        hist::CASES,
        layout::CASES,
        lines::CASES,
        misc::ARTISTS,
        misc::DATES,
        misc::SPECTRAL,
        misc::UNITS,
        scatter::CASES,
        stats::BOXPLOT,
        stats::ERRORBAR,
        stats::EVENTS,
        stats::FILL_BETWEEN,
        stats::VIOLIN,
    ]
    .concat()
}
