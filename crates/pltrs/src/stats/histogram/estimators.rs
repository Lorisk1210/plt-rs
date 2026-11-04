//! numpy's bin-width estimators (`numpy.lib._histograms_impl`).

use std::f64::consts::PI;

use super::counting;
use super::{BinEstimator, BinSample, Uniform};
use crate::numeric::linspace;
use crate::stats::summary::{mean, percentile_array, std};

/// The raw bin width `estimator` proposes for `data`, whose equal-width
/// bins will span `range` (only Stone's rule uses the range).
///
/// This is numpy's `_hist_bin_<name>(x, range)`: no rounding to a whole
/// number of bins and no integer clamp is applied. A width of zero means
/// "use a single bin".
pub fn bin_width<T: BinSample>(data: &[T], estimator: BinEstimator, range: (f64, f64)) -> f64 {
    let x = T::floats(data);
    match estimator {
        BinEstimator::Sqrt => sqrt_width(&x),
        BinEstimator::Sturges => sturges_width(&x),
        BinEstimator::Rice => ptp(&x) / (2.0 * size(&x).powf(1.0 / 3.0)),
        BinEstimator::Scott => (24.0 * PI.powf(0.5) / size(&x)).powf(1.0 / 3.0) * std(&x, 0),
        BinEstimator::Stone => stone_width(data, &x, range),
        BinEstimator::Doane => doane_width(&x),
        BinEstimator::Fd => fd_width(&x),
        BinEstimator::Auto => {
            let fd = fd_width(&x);
            let half_sqrt = sqrt_width(&x) / 2.0;
            let corrected = if half_sqrt > fd { half_sqrt } else { fd };
            let sturges = sturges_width(&x);
            if sturges < corrected {
                sturges
            } else {
                corrected
            }
        }
    }
}

fn size(x: &[f64]) -> f64 {
    x.len() as f64
}

/// Peak to peak (`max - min`); NaN when any value is NaN.
fn ptp(x: &[f64]) -> f64 {
    if x.iter().any(|v| v.is_nan()) {
        return f64::NAN;
    }
    let max = x.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let min = x.iter().copied().fold(f64::INFINITY, f64::min);
    max - min
}

fn sqrt_width(x: &[f64]) -> f64 {
    ptp(x) / size(x).sqrt()
}

fn sturges_width(x: &[f64]) -> f64 {
    ptp(x) / (size(x).log2() + 1.0)
}

fn fd_width(x: &[f64]) -> f64 {
    let iqr = percentile_array(x, [75.0, 25.0]).map_or(f64::NAN, |[q3, q1]| q3 - q1);
    2.0 * iqr * size(x).powf(-1.0 / 3.0)
}

fn doane_width(x: &[f64]) -> f64 {
    let n = size(x);
    if x.len() > 2 {
        let sg1 = (6.0 * (n - 2.0) / ((n + 1.0) * (n + 3.0))).sqrt();
        let sigma = std(x, 0);
        if sigma > 0.0 {
            let m = mean(x);
            let cubes: Vec<f64> = x.iter().map(|&v| ((v - m) / sigma).powf(3.0)).collect();
            let g1 = mean(&cubes);
            return ptp(x) / (1.0 + n.log2() + (1.0 + g1.abs() / sg1).log2());
        }
    }
    0.0
}

/// Stone's rule: the number of bins in `1..=max(100, √n)` minimising the
/// leave-one-out risk estimate, first minimum winning.
fn stone_width<T: BinSample>(data: &[T], x: &[f64], (first, last): (f64, f64)) -> f64 {
    let n = x.len();
    let ptp_x = ptp(x);
    if n <= 1 || ptp_x == 0.0 {
        return 0.0;
    }
    let upper = 100.max(size(x).sqrt() as usize);
    let risk = |bins: usize| {
        let edges = linspace(first, last, bins + 1);
        let uniform = Uniform {
            first,
            last,
            count: bins,
        };
        let sum_sq: f64 = counting::uniform(data, None, &edges, uniform)
            .iter()
            .map(|&c| {
                let p = c / n as f64;
                p * p
            })
            .sum();
        (2.0 - (n as f64 + 1.0) * sum_sq) / (ptp_x / bins as f64)
    };
    let mut best = (1, risk(1));
    for bins in 2..=upper {
        let r = risk(bins);
        if r < best.1 {
            best = (bins, r);
        }
    }
    ptp_x / best.0 as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn width(data: &[f64], estimator: BinEstimator) -> f64 {
        let lo = data.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = data.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        bin_width(data, estimator, (lo, hi))
    }

    #[test]
    fn closed_form_widths() {
        let data: Vec<f64> = (0..16).map(f64::from).collect();
        assert_eq!(width(&data, BinEstimator::Sqrt), 15.0 / 4.0);
        assert_eq!(width(&data, BinEstimator::Sturges), 3.0);
        assert_eq!(
            width(&data, BinEstimator::Rice),
            15.0 / (2.0 * 16_f64.powf(1.0 / 3.0))
        );
        assert_eq!(
            width(&data, BinEstimator::Fd),
            2.0 * 7.5 * 16_f64.powf(-1.0 / 3.0)
        );
        let auto = width(&data, BinEstimator::Auto);
        assert_eq!(auto, 3.0);
    }

    #[test]
    fn degenerate_widths_are_zero() {
        assert_eq!(width(&[2.0, 2.0], BinEstimator::Doane), 0.0);
        assert_eq!(width(&[2.0, 2.0, 2.0], BinEstimator::Doane), 0.0);
        assert_eq!(width(&[2.0], BinEstimator::Stone), 0.0);
        assert_eq!(width(&[2.0, 2.0], BinEstimator::Scott), 0.0);
    }

    #[test]
    fn symmetric_data_has_no_doane_correction() {
        let data = [-2.0, -1.0, 0.0, 1.0, 2.0];
        let expected = 4.0 / (1.0 + 5_f64.log2());
        assert!((width(&data, BinEstimator::Doane) - expected).abs() < 1e-15);
    }

    #[test]
    fn stone_prefers_separating_clusters() {
        let data = [0.0, 0.0, 0.0, 10.0, 10.0, 10.0];
        let w = width(&data, BinEstimator::Stone);
        assert!(w <= 10.0 && w > 0.0);
    }
}
