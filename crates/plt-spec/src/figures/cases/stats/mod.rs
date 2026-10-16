//! `figures/cases_stats.py`, split by plot family.

mod boxplot;
mod errorbar;
mod events;
mod fill_between;
mod violin;

pub(crate) use boxplot::CASES as BOXPLOT;
pub(crate) use errorbar::CASES as ERRORBAR;
pub(crate) use events::CASES as EVENTS;
pub(crate) use fill_between::CASES as FILL_BETWEEN;
pub(crate) use violin::CASES as VIOLIN;

use crate::rng::SpecRng;

/// `SAMPLES`: three normal samples, the last with outliers appended.
fn samples() -> Vec<Vec<f64>> {
    let mut third = SpecRng::new(202).normal(60, -0.5, 1.5);
    third.extend([6.0, -5.5, 5.0]);
    vec![
        SpecRng::new(200).normal(100, 0.0, 1.0),
        SpecRng::new(201).normal(80, 1.0, 0.5),
        third,
    ]
}

/// `values` mapped through `f`.
fn map(values: &[f64], f: impl Fn(f64) -> f64) -> Vec<f64> {
    values.iter().copied().map(f).collect()
}
