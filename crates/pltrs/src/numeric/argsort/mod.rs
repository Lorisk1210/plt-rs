//! numpy's `argsort` of `float64` values, down to the order of ties.
//!
//! numpy's default sort is not stable, so which of two equal values comes
//! first is whatever its algorithm leaves, and matplotlib inherits that
//! order wherever it sorts by `np.argsort`: 3D faces and points drawn by
//! depth, for one. numpy 2 sorts the indices of `float64` arrays on x86-64
//! CPUs with AVX-512 (`X86_V4`, as on the machine the goldens are made on)
//! with Intel's x86-simd-sort:
//!
//! * an array with NaN goes to C++'s `std::sort` (NaN last), which
//!   [`fallback`] reproduces as GCC's standard library sorts; an array
//!   already in order keeps its indices;
//! * up to 256 values are sorted by bitonic networks ([`network`]);
//! * longer arrays are partitioned around a median of eight samples, four
//!   registers at a time, down to network-sized ranges ([`partition`]);
//!   after `2 log2(n)` levels without progress, `std::sort` takes over.
//!
//! This module replays those steps lane by lane in scalar code.
//!
//! ```
//! use pltrs::numeric::argsort;
//!
//! assert_eq!(argsort(&[3.0, 1.0, 2.0]), vec![1, 2, 0]);
//! // Equal values in numpy's (unstable) order.
//! assert_eq!(argsort(&[1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0]), vec![1, 3, 7, 5, 2, 0, 4, 6, 8]);
//! ```

mod fallback;
mod network;
mod partition;

/// Keys per AVX-512 register of `f64`s.
const LANES: usize = 8;

/// The indices that sort `values` ascending, NaN last: numpy's
/// `np.argsort(values)`, equal values included (see the module docs).
pub fn argsort(values: &[f64]) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..values.len()).collect();
    if values.len() < 2 {
        return indices;
    }
    if values.iter().any(|v| v.is_nan()) {
        fallback::sort(&mut indices, |a, b| nan_last_less(values[a], values[b]));
    } else if !values.is_sorted() {
        let max_iters = 2 * values.len().ilog2() as usize;
        partition::sort(values, &mut indices, max_iters);
    }
    indices
}

/// `a < b` with NaN after every number (x86-simd-sort's comparator for
/// arrays with NaN).
fn nan_last_less(a: f64, b: f64) -> bool {
    match (a.is_nan(), b.is_nan()) {
        (false, false) => a < b,
        (a_nan, b_nan) => !a_nan && b_nan,
    }
}

#[cfg(test)]
mod tests {
    use pltrs_core::rng::SplitMix64;

    use super::*;

    /// Whether `order` is a permutation that sorts `values`, NaN last.
    fn sorts(values: &[f64], order: &[usize]) -> bool {
        let mut seen = vec![false; values.len()];
        order.len() == values.len()
            && order
                .iter()
                .all(|&i| !std::mem::replace(&mut seen[i], true))
            && order
                .windows(2)
                .all(|w| !nan_last_less(values[w[1]], values[w[0]]))
    }

    /// `n` values drawn (by `SplitMix64`) from `levels` equally spaced ones.
    fn ties(seed: u64, n: usize, levels: u64) -> Vec<f64> {
        let mut random = SplitMix64::new(seed);
        (0..n)
            .map(|_| (random.next_u64() % levels) as f64 / 4.0)
            .collect()
    }

    #[test]
    fn small_arrays_tie_like_numpy() {
        let values = [
            2.0, 1.0, 1.0, 2.0, 1.0, 1.0, 2.0, 1.0, 1.0, 2.0, 1.0, 1.0, 0.0,
        ];
        assert_eq!(argsort(&values), [12, 1, 5, 2, 7, 11, 8, 4, 10, 6, 0, 3, 9]);
        let values = [2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 2.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        assert_eq!(argsort(&values), [1, 2, 3, 4, 9, 10, 8, 7, 11, 6, 0, 5]);
    }

    #[test]
    fn nan_goes_last_through_std_sort() {
        let nan = f64::NAN;
        assert_eq!(
            argsort(&[1.0, 0.0, nan, 0.0, 1.0, nan, 0.0]),
            [1, 3, 6, 0, 4, 2, 5]
        );
        let mut values = ties(7, 500, 5);
        values[17] = nan;
        values[300] = nan;
        let order = argsort(&values);
        assert!(sorts(&values, &order));
        assert!(matches!(order[498..], [17, 300] | [300, 17]));
    }

    #[test]
    fn sorted_and_tiny_arrays_keep_their_order() {
        assert_eq!(argsort(&[]), Vec::<usize>::new());
        assert_eq!(argsort(&[f64::NAN]), [0]);
        assert_eq!(argsort(&[1.0, 1.0, 2.0, 2.0]), [0, 1, 2, 3]);
    }

    #[test]
    fn every_path_sorts() {
        for (seed, n) in [2, 7, 8, 9, 63, 64, 65, 256, 257, 300, 1000, 2600]
            .into_iter()
            .enumerate()
        {
            for levels in [2, 9, 1000] {
                let values = ties(seed as u64 * 31 + levels, n, levels);
                assert!(
                    sorts(&values, &argsort(&values)),
                    "{n} values, {levels} levels"
                );
            }
        }
        // Mostly the minimum: the partitions make no progress and
        // `std::sort` takes over.
        let values: Vec<f64> = (0..2000)
            .map(|i| if i % 10 == 3 { f64::from(i) } else { 0.0 })
            .collect();
        assert!(sorts(&values, &argsort(&values)));
        let infinite: Vec<f64> = ties(5, 700, 3)
            .into_iter()
            .map(|v| if v > 0.4 { f64::INFINITY } else { v })
            .collect();
        assert!(sorts(&infinite, &argsort(&infinite)));
    }
}
