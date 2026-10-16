//! `figures/cases_stats.py`, split by plot family.

mod errorbar;
mod events;
mod fill_between;

pub(crate) use errorbar::CASES as ERRORBAR;
pub(crate) use events::CASES as EVENTS;
pub(crate) use fill_between::CASES as FILL_BETWEEN;

/// `values` mapped through `f`.
fn map(values: &[f64], f: impl Fn(f64) -> f64) -> Vec<f64> {
    values.iter().copied().map(f).collect()
}
