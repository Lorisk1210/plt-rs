//! `figures/cases_gallery_stats.py`: the gallery's "Statistics" and "Pie
//! and polar charts" sections, one case per figure of each example.

mod binned;
mod bootstrap;
mod boxplots;
mod curves;
mod errorbars;
mod histograms;
mod normalization;
mod pies;
mod polar;
mod spectra;
mod violins;

use crate::figures::FigureCase;

/// The seed of most examples (`np.random.seed(19680801)`).
const SEED: u64 = 19_680_801;

/// The columns of `values` read as rows of `width` values: the datasets
/// of a 2-D array (`values.reshape(-1, width)` passed to `boxplot` or
/// `hist`).
fn columns(values: &[f64], width: usize) -> Vec<Vec<f64>> {
    (0..width)
        .map(|column| values.iter().skip(column).step_by(width).copied().collect())
        .collect()
}

/// Owned copies of `names`.
fn strings(names: &[&str]) -> Vec<String> {
    names.iter().map(|&name| name.to_owned()).collect()
}

/// Every case of the group.
pub(crate) fn cases() -> Vec<FigureCase> {
    [
        binned::CASES,
        bootstrap::CASES,
        boxplots::CASES,
        curves::CASES,
        errorbars::CASES,
        histograms::CASES,
        normalization::CASES,
        pies::CASES,
        polar::CASES,
        spectra::CASES,
        violins::CASES,
    ]
    .concat()
}
