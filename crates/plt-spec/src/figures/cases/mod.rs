//! Rust translations of the spec's figure cases, one module per group.

mod basic;

use super::FigureCase;

/// Every translated case.
pub(crate) fn all() -> Vec<FigureCase> {
    [basic::CASES].concat()
}
