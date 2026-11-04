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
    let n = values.len();
    if n == 0 {
        return f64::NAN;
    }
    // One rank for odd lengths, the two middle ones for even lengths.
    match ranked_without_nan(values, &[(n - 1) / 2, n / 2]) {
        Some(ranked) if n % 2 == 1 => ranked[n / 2],
        Some(ranked) => mean_of_two(ranked[n / 2 - 1], ranked[n / 2]),
        None => f64::NAN,
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
/// use pltrs::stats::percentile;
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
    let Some(last) = values.len().checked_sub(1) else {
        return vec![f64::NAN; qs.len()];
    };
    let positions: Vec<Position> = qs.iter().map(|&q| Position::of(last, q)).collect();
    let ranks: Vec<usize> = positions.iter().flat_map(|p| [p.below, p.above]).collect();
    match ranked_without_nan(values, &ranks) {
        Some(ranked) => positions
            .iter()
            .map(|p| lerp(ranked[p.below], ranked[p.above], p.gamma))
            .collect(),
        None => vec![f64::NAN; qs.len()],
    }
}

/// Where numpy's `_quantile` finds a fraction among sorted data: between
/// the values of ranks `below` and `above`, `gamma` of the way.
struct Position {
    below: usize,
    above: usize,
    gamma: f64,
}

impl Position {
    /// The position of the fraction `q` among data whose last rank is
    /// `last`.
    fn of(last: usize, q: f64) -> Self {
        let virtual_index = last as f64 * q;
        let (below, above) = if virtual_index >= last as f64 {
            (last, last)
        } else {
            let below = virtual_index.floor() as usize;
            (below, below + 1)
        };
        Self {
            below,
            above,
            gamma: virtual_index - below as f64,
        }
    }
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

/// A copy of the values in which each of `ranks` holds what it would
/// hold sorted ascending (numpy's `partition` on them, linear in the
/// length rather than a sort), or `None` if any value is NaN.
fn ranked_without_nan(values: &[f64], ranks: &[usize]) -> Option<Vec<f64>> {
    if values.iter().any(|v| v.is_nan()) {
        return None;
    }
    let mut ranks = ranks.to_vec();
    ranks.sort_unstable();
    ranks.dedup();
    let mut ranked = values.to_vec();
    // Each selection leaves the larger values after the rank, among which
    // the next, larger rank is selected.
    let mut start = 0;
    for rank in ranks {
        ranked[start..].select_nth_unstable_by(rank - start, f64::total_cmp);
        start = rank + 1;
    }
    Some(ranked)
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
    fn selected_ranks_are_those_of_a_sort() {
        let mut rng = pltrs_core::rng::SplitMix64::new(7);
        for len in [1, 2, 3, 10, 101, 1000] {
            // Few distinct values, and zeros of both signs.
            let values: Vec<f64> = (0..len)
                .map(|_| ((rng.below(9) as f64 - 4.0) * 0.5).copysign(rng.next_f64() - 0.5))
                .collect();
            let mut sorted = values.clone();
            sorted.sort_by(f64::total_cmp);
            let ranks = [len - 1, 0, len / 2, len / 3, len / 2];
            let ranked = ranked_without_nan(&values, &ranks).unwrap();
            for rank in ranks {
                assert_eq!(
                    ranked[rank].to_bits(),
                    sorted[rank].to_bits(),
                    "rank {rank} of {len}"
                );
            }
        }
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
