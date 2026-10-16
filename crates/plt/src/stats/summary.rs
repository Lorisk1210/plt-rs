//! Descriptive statistics with numpy's exact arithmetic: pairwise sums,
//! means, variances, medians and 'linear' percentiles.

use std::ops::Add;

use super::StatsError;

/// Below this length numpy sums sequentially; up to the block size it uses
/// eight interleaved accumulators, beyond it the halves recursively.
const PAIRWISE_UNROLL: usize = 8;
const PAIRWISE_BLOCK: usize = 128;

/// Sum with numpy's pairwise algorithm (`np.add.reduce` over a contiguous
/// axis), which bounds the rounding error by `O(log n)` and fixes the exact
/// result numpy reports.
pub fn pairwise_sum(values: &[f64]) -> f64 {
    pairwise(values, 0.0)
}

/// [`pairwise_sum`] for any additive type (e.g. complex numbers).
pub(crate) fn pairwise<T: Copy + Add<Output = T>>(values: &[T], zero: T) -> T {
    let n = values.len();
    if n < PAIRWISE_UNROLL {
        values.iter().fold(zero, |acc, &v| acc + v)
    } else if n <= PAIRWISE_BLOCK {
        let mut acc = [zero; PAIRWISE_UNROLL];
        acc.copy_from_slice(&values[..PAIRWISE_UNROLL]);
        let blocks = n - n % PAIRWISE_UNROLL;
        let (chunks, _) = values[PAIRWISE_UNROLL..blocks].as_chunks::<PAIRWISE_UNROLL>();
        for chunk in chunks {
            for (a, &v) in acc.iter_mut().zip(chunk) {
                *a = *a + v;
            }
        }
        let head =
            ((acc[0] + acc[1]) + (acc[2] + acc[3])) + ((acc[4] + acc[5]) + (acc[6] + acc[7]));
        values[blocks..].iter().fold(head, |acc, &v| acc + v)
    } else {
        let half = n / 2;
        let half = half - half % PAIRWISE_UNROLL;
        pairwise(&values[..half], zero) + pairwise(&values[half..], zero)
    }
}

/// Arithmetic mean (numpy `mean`); NaN for empty input.
pub fn mean(values: &[f64]) -> f64 {
    pairwise_sum(values) / values.len() as f64
}

/// Variance with `ddof` delta degrees of freedom (numpy `var`).
///
/// Population variance is `ddof = 0`; the sample variance `ddof = 1`.
pub fn var(values: &[f64], ddof: usize) -> f64 {
    let m = mean(values);
    let squares: Vec<f64> = values
        .iter()
        .map(|&v| {
            let d = v - m;
            d * d
        })
        .collect();
    pairwise_sum(&squares) / values.len().saturating_sub(ddof) as f64
}

/// Standard deviation with `ddof` delta degrees of freedom (numpy `std`).
pub fn std(values: &[f64], ddof: usize) -> f64 {
    var(values, ddof).sqrt()
}

/// Median (numpy `median`): the middle value, or the mean of the two middle
/// values; NaN if the data is empty or contains NaN.
pub fn median(values: &[f64]) -> f64 {
    let Some(sorted) = sorted_without_nan(values) else {
        return f64::NAN;
    };
    let n = sorted.len();
    if n == 0 {
        f64::NAN
    } else if n % 2 == 1 {
        sorted[n / 2]
    } else {
        mean_of_two(sorted[n / 2 - 1], sorted[n / 2])
    }
}

/// `(a + b) / 2` as numpy's `mean` computes it, overflowing like numpy.
#[expect(
    clippy::manual_midpoint,
    reason = "numpy's median overflows to infinity"
)]
fn mean_of_two(a: f64, b: f64) -> f64 {
    (a + b) / 2.0
}

/// The `p`-th percentile, `0 <= p <= 100`, with numpy's default 'linear'
/// interpolation (`np.percentile`).
///
/// NaN in the data makes the result NaN; empty data is NaN as well.
///
/// ```
/// use plt::stats::percentile;
/// assert_eq!(percentile(&[1.0, 2.0, 3.0, 4.0], 50.0), Ok(2.5));
/// assert_eq!(percentile(&[3.0, 1.0, 2.0], 25.0), Ok(1.5));
/// ```
pub fn percentile(values: &[f64], p: f64) -> Result<f64, StatsError> {
    Ok(percentiles(values, &[p])?[0])
}

/// Several percentiles of the same data (sorting once).
pub fn percentiles(values: &[f64], ps: &[f64]) -> Result<Vec<f64>, StatsError> {
    let qs = ps
        .iter()
        .map(|&p| {
            let q = p / 100.0;
            if (0.0..=1.0).contains(&q) {
                Ok(q)
            } else {
                Err(StatsError::PercentileOutOfRange(p))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(quantiles_unchecked(values, &qs))
}

/// [`percentiles`] for a fixed number of percentiles.
pub(crate) fn percentile_array<const N: usize>(
    values: &[f64],
    ps: [f64; N],
) -> Result<[f64; N], StatsError> {
    let found = percentiles(values, &ps)?;
    let mut out = [f64::NAN; N];
    out.copy_from_slice(&found);
    Ok(out)
}

/// The `q`-quantile, `0 <= q <= 1` (numpy `quantile`, 'linear' method).
pub fn quantile(values: &[f64], q: f64) -> Result<f64, StatsError> {
    if (0.0..=1.0).contains(&q) {
        Ok(quantiles_unchecked(values, &[q])[0])
    } else {
        Err(StatsError::QuantileOutOfRange(q))
    }
}

fn quantiles_unchecked(values: &[f64], qs: &[f64]) -> Vec<f64> {
    match sorted_without_nan(values) {
        Some(sorted) if !sorted.is_empty() => qs.iter().map(|&q| interpolate(&sorted, q)).collect(),
        _ => vec![f64::NAN; qs.len()],
    }
}

/// numpy's `_quantile` for one fraction of sorted, NaN-free data.
fn interpolate(sorted: &[f64], q: f64) -> f64 {
    let last = sorted.len() - 1;
    let virtual_index = last as f64 * q;
    let (below, above) = if virtual_index >= last as f64 {
        (last, last)
    } else {
        let below = virtual_index.floor() as usize;
        (below, below + 1)
    };
    let gamma = virtual_index - below as f64;
    lerp(sorted[below], sorted[above], gamma)
}

/// numpy's `_lerp`, which interpolates from the nearer end for accuracy.
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    let diff = b - a;
    if t >= 0.5 {
        b - diff * (1.0 - t)
    } else {
        a + diff * t
    }
}

/// The values sorted ascending, or `None` if any is NaN.
fn sorted_without_nan(values: &[f64]) -> Option<Vec<f64>> {
    if values.iter().any(|v| v.is_nan()) {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    Some(sorted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairwise_sum_matches_numpy_grouping() {
        assert_eq!(pairwise_sum(&[]), 0.0);
        assert_eq!(pairwise_sum(&[1.0, 2.0, 3.0]), 6.0);
        // Sequentially every `1e16 + 1` rounds back to 1e16; numpy's eight
        // accumulators add the ones among themselves first.
        let mut v = vec![1e16];
        v.extend(std::iter::repeat_n(1.0, 15));
        assert_eq!(pairwise_sum(&v), 1e16 + 14.0);
        let long: Vec<f64> = (0..1000).map(f64::from).collect();
        assert_eq!(pairwise_sum(&long), 499_500.0);
    }

    #[test]
    fn moments() {
        let x = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        assert_eq!(mean(&x), 5.0);
        assert_eq!(var(&x, 0), 4.0);
        assert_eq!(std(&x, 0), 2.0);
        assert_eq!(var(&[1.0, 3.0], 1), 2.0);
        assert!(mean(&[]).is_nan());
    }

    #[test]
    fn medians() {
        assert_eq!(median(&[3.0, 1.0, 2.0]), 2.0);
        assert_eq!(median(&[4.0, 1.0, 2.0, 3.0]), 2.5);
        assert!(median(&[]).is_nan());
        assert!(median(&[1.0, f64::NAN]).is_nan());
    }

    #[test]
    fn percentiles_follow_numpy_linear_method() {
        let x = [1.5, -0.5, 4.0, 2.25, 9.0];
        assert_eq!(
            percentiles(&x, &[0.0, 25.0, 50.0, 75.0, 100.0]),
            Ok(vec![-0.5, 1.5, 2.25, 4.0, 9.0])
        );
        assert_eq!(percentile(&[1.0, 2.0], 30.0), Ok(1.3));
        assert_eq!(quantile(&[0.0, 10.0], 0.25), Ok(2.5));
        assert!(percentile(&[1.0, f64::NAN], 50.0).unwrap().is_nan());
        assert!(percentile(&[], 50.0).unwrap().is_nan());
        assert_eq!(
            percentile(&[1.0], 150.0),
            Err(StatsError::PercentileOutOfRange(150.0))
        );
        assert_eq!(
            quantile(&[1.0], -0.1),
            Err(StatsError::QuantileOutOfRange(-0.1))
        );
    }

    #[test]
    fn infinite_values_interpolate_like_numpy() {
        let x = [1.0, f64::INFINITY, 2.0, 3.0];
        assert_eq!(percentile(&x, 75.0), Ok(f64::INFINITY));
        assert_eq!(percentile(&x, 50.0), Ok(2.5));
    }
}
