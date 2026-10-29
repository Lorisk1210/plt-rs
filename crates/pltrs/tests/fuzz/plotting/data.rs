//! The data of fuzzed plots: series of random lengths (sometimes
//! mismatched, sometimes empty) whose values are ordinary, special or at
//! the edges of `f64`, and small grids of them.

use crate::harness::Gen;

/// A series length: empty, single, short, or a few hundred points.
pub(crate) fn length(g: &mut Gen) -> usize {
    match g.below(10) {
        0 => 0,
        1 => 1,
        2 => 300,
        _ => 2 + g.below(20),
    }
}

/// A series of `len` values.
pub(crate) fn series(g: &mut Gen, len: usize) -> Vec<f64> {
    g.numbers(len)
}

/// A second series for the same points: usually as long as the first,
/// sometimes not.
pub(crate) fn partner(g: &mut Gen, len: usize) -> Vec<f64> {
    let len = if g.chance(0.1) { length(g) } else { len };
    g.numbers(len)
}

/// Values in `[low, high]` mostly, for options such as widths and alphas.
pub(crate) fn values_in(g: &mut Gen, len: usize, low: f64, high: f64) -> Vec<f64> {
    (0..len).map(|_| g.number_in(low, high)).collect()
}

/// Fewer than `max` values in `[low, high]` mostly.
pub(crate) fn few_in(g: &mut Gen, max: usize, low: f64, high: f64) -> Vec<f64> {
    let len = g.below(max);
    values_in(g, len, low, high)
}

/// One value per point (of `n`) or a single one, in `[low, high]` mostly.
pub(crate) fn each_or_one_in(g: &mut Gen, n: usize, low: f64, high: f64) -> Vec<f64> {
    let len = if g.chance(0.5) { n } else { 1 };
    values_in(g, len, low, high)
}

/// Small datasets for statistics (box plots, violins, histograms).
pub(crate) fn datasets(g: &mut Gen) -> Vec<Vec<f64>> {
    let count = g.len(4);
    (0..count)
        .map(|_| {
            let len = length(g);
            series(g, len)
        })
        .collect()
}

/// A grid of rows (ragged now and then).
pub(crate) fn grid(g: &mut Gen) -> Vec<Vec<f64>> {
    let (rows, cols) = (g.below(12), g.below(12));
    let ragged = g.chance(0.05);
    (0..rows)
        .map(|_| {
            let cols = if ragged { g.below(12) } else { cols };
            series(g, cols)
        })
        .collect()
}

/// A count option: small mostly, zero or huge now and then.
pub(crate) fn count(g: &mut Gen, usual: usize) -> usize {
    match g.below(20) {
        0 => 0,
        1 => g.pick(&[1 << 20, 1 << 31, usize::MAX]),
        2 => 1,
        _ => 1 + g.below(usual),
    }
}
