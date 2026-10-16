//! Rust translations of the spec's figure cases, one module per group.

mod axes;
mod bars;
mod basic;
mod hist;
mod layout;
mod lines;
mod scatter;
mod stats;

use super::FigureCase;

/// Every translated case.
pub(crate) fn all() -> Vec<FigureCase> {
    [
        axes::CASES,
        bars::CASES,
        basic::CASES,
        hist::CASES,
        layout::CASES,
        lines::CASES,
        scatter::CASES,
        stats::ERRORBAR,
        stats::EVENTS,
        stats::FILL_BETWEEN,
    ]
    .concat()
}
