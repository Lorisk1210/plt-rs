//! Rust translations of the spec's figure cases, one module per group.

mod bars;
mod basic;
mod hist;
mod lines;
mod scatter;
mod stats;

use super::FigureCase;

/// Every translated case.
pub(crate) fn all() -> Vec<FigureCase> {
    [
        bars::CASES,
        basic::CASES,
        hist::CASES,
        lines::CASES,
        scatter::CASES,
        stats::CASES,
    ]
    .concat()
}
