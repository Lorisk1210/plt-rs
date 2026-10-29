//! `figures/cases_gallery_subplots.py`: matplotlib's gallery section
//! "Subplots, axes and figures", one case per figure of each example.

mod aspects;
mod figures;
mod grids;
mod labels;
mod layouts;
mod secondary;
mod subfigures;
mod subplots;
mod twins;
mod zooms;

use crate::figures::FigureCase;

/// The seed of the examples' `np.random.seed(19680801)`.
const SEED: u64 = 19_680_801;

/// The discrete linear convolution of `a` and `v` over every overlap
/// (`np.convolve(a, v)`, mode `"full"`): `a.len() + v.len() - 1` values.
fn convolve(a: &[f64], v: &[f64]) -> Vec<f64> {
    let mut out = vec![0.0; (a.len() + v.len()).saturating_sub(1)];
    for (i, &ai) in a.iter().enumerate() {
        for (o, &vj) in out[i..].iter_mut().zip(v) {
            *o += ai * vj;
        }
    }
    out
}

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [
        aspects::CASES,
        figures::CASES,
        grids::CASES,
        labels::CASES,
        layouts::CASES,
        secondary::CASES,
        subfigures::CASES,
        subplots::CASES,
        twins::CASES,
        zooms::CASES,
    ]
    .concat()
}
