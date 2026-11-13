//! numpy's `argsort` of `float64` values, down to the order of ties.
//!
//! numpy's default sort is not stable, so which of two equal values comes
//! first is whatever its algorithm leaves, and matplotlib inherits that
//! order wherever it sorts by `np.argsort`: 3D faces and points drawn by
//! depth, for one. Which algorithm that is depends on the processor
//! ([`CpuLevel`]): numpy's wheels hold three, and choose when numpy is
//! imported.
//!
//! * Without AVX2 (numpy's x86-64 baseline), and on every processor but
//!   x86, numpy sorts the indices with its own introsort ([`generic`]).
//! * With AVX2 (`X86_V3`) or AVX-512 (`X86_V4`) it sorts them with Intel's
//!   x86-simd-sort, compiled once for each:
//!   - an array with NaN goes to C++'s `std::sort` (NaN last), which
//!     [`fallback`] reproduces as GCC's standard library sorts; an array
//!     already in order keeps its indices;
//!   - up to 256 values are sorted by bitonic networks ([`network`]);
//!   - longer arrays are partitioned around a median of one register of
//!     samples, four registers at a time, down to network-sized ranges
//!     ([`partition`]); after `2 log2(n)` levels without progress,
//!     `std::sort` takes over.
//!
//! This module replays those steps lane by lane in scalar code. The two
//! compilations are one algorithm over registers of 4 and of 8 keys
//! ([`width`] holds what differs). The networks come out the same for
//! both, a bitonic sorter being one network however it is cut into
//! registers, so the levels agree on every array of up to 256 values; on
//! longer ones the partitions differ in three ways: the pivot is the third
//! smallest of four samples or the fifth smallest of eight, a step takes
//! 16 keys or 32, and AVX2, lacking a compress instruction, stores a
//! register's keys of at least the pivot from its last lane down.
//!
//! numpy dispatches the same way for the other keys it sorts indices of:
//! 32- and 64-bit integers and `float32` go to x86-simd-sort at both
//! levels (with registers of as many keys as indices), while 8- and 16-bit
//! integers, booleans, `float16`, `longdouble`, strings and objects, and
//! every stable or descending sort, stay with numpy's own code at every
//! level. Highway's `vqsort` sorts values, never indices, and only off
//! x86. pltrs sorts indices of `float64` only.
//!
//! ```
//! use pltrs::numeric::{CpuLevel, argsort, argsort_at};
//!
//! assert_eq!(argsort(&[3.0, 1.0, 2.0]), vec![1, 2, 0]);
//! // Equal values in numpy's (unstable) order at each level.
//! let ties = [1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0];
//! assert_eq!(argsort_at(&ties, CpuLevel::V4), vec![1, 3, 7, 5, 2, 0, 4, 6, 8]);
//! assert_eq!(argsort_at(&ties, CpuLevel::V3), vec![1, 3, 7, 5, 2, 0, 4, 6, 8]);
//! assert_eq!(argsort_at(&ties, CpuLevel::Baseline), vec![1, 3, 5, 7, 0, 2, 4, 6, 8]);
//! ```

#[cfg(test)]
mod digests;
mod fallback;
mod generic;
mod network;
mod partition;
mod width;

use pltrs_core::numeric::CpuLevel;
use width::{Avx2, Avx512, Width};

/// The indices that sort `values` ascending, NaN last: numpy's
/// `np.argsort(values)`, equal values included, as numpy sorts on this
/// processor ([`CpuLevel::current`]; see the module docs).
pub fn argsort(values: &[f64]) -> Vec<usize> {
    argsort_at(values, CpuLevel::current())
}

/// [`argsort`] as numpy sorts at `level`, whatever the processor.
pub fn argsort_at(values: &[f64], level: CpuLevel) -> Vec<usize> {
    match level {
        CpuLevel::V4 => simd_argsort::<Avx512>(values),
        CpuLevel::V3 => simd_argsort::<Avx2>(values),
        CpuLevel::Baseline => {
            let mut indices: Vec<usize> = (0..values.len()).collect();
            generic::sort(values, &mut indices);
            indices
        }
    }
}

/// [`argsort`] as x86-simd-sort sorts with registers of width `W`.
fn simd_argsort<W: Width>(values: &[f64]) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..values.len()).collect();
    if values.len() < 2 {
        return indices;
    }
    if values.iter().any(|v| v.is_nan()) {
        fallback::sort(&mut indices, |a, b| nan_last_less(values[a], values[b]));
    } else if !values.is_sorted() {
        let max_iters = 2 * values.len().ilog2() as usize;
        partition::sort::<W>(values, &mut indices, max_iters);
    }
    indices
}

/// `a < b` with NaN after every number (x86-simd-sort's comparator for
/// arrays with NaN, and numpy's own for floats).
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

    /// The levels x86-simd-sort sorts at.
    const SIMD: [CpuLevel; 2] = [CpuLevel::V3, CpuLevel::V4];

    #[test]
    fn small_arrays_tie_like_numpy() {
        // numpy 2.5.4 with AVX2 and with AVX-512: one network.
        for level in SIMD {
            let values = [
                2.0, 1.0, 1.0, 2.0, 1.0, 1.0, 2.0, 1.0, 1.0, 2.0, 1.0, 1.0, 0.0,
            ];
            assert_eq!(
                argsort_at(&values, level),
                [12, 1, 5, 2, 7, 11, 8, 4, 10, 6, 0, 3, 9]
            );
            let values = [2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 2.0, 1.0, 0.0, 0.0, 0.0, 1.0];
            assert_eq!(
                argsort_at(&values, level),
                [1, 2, 3, 4, 9, 10, 8, 7, 11, 6, 0, 5]
            );
        }
        // numpy 2.5.4 without AVX2: its own introsort.
        let values = [2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 2.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        assert_eq!(
            argsort_at(&values, CpuLevel::Baseline),
            [1, 2, 3, 4, 8, 9, 10, 7, 11, 0, 5, 6]
        );
    }

    #[test]
    fn nan_goes_last_through_std_sort() {
        let nan = f64::NAN;
        for level in SIMD {
            assert_eq!(
                argsort_at(&[1.0, 0.0, nan, 0.0, 1.0, nan, 0.0], level),
                [1, 3, 6, 0, 4, 2, 5]
            );
            let mut values = ties(7, 500, 5);
            values[17] = nan;
            values[300] = nan;
            let order = argsort_at(&values, level);
            assert!(sorts(&values, &order));
            assert!(matches!(order[498..], [17, 300] | [300, 17]));
        }
    }

    #[test]
    fn sorted_and_tiny_arrays_keep_their_order() {
        for level in CpuLevel::ALL {
            assert_eq!(argsort_at(&[], level), Vec::<usize>::new());
            assert_eq!(argsort_at(&[f64::NAN], level), [0]);
            assert_eq!(argsort_at(&[1.0, 1.0, 2.0, 2.0], level), [0, 1, 2, 3]);
        }
    }

    #[test]
    fn every_path_sorts_at_every_level() {
        // Mostly the minimum: the partitions make no progress and
        // `std::sort` takes over.
        let stalled: Vec<f64> = (0..2000)
            .map(|i| if i % 10 == 3 { f64::from(i) } else { 0.0 })
            .collect();
        let infinite: Vec<f64> = ties(5, 700, 3)
            .into_iter()
            .map(|v| if v > 0.4 { f64::INFINITY } else { v })
            .collect();
        for level in CpuLevel::ALL {
            for (seed, n) in [2, 7, 8, 9, 63, 64, 65, 256, 257, 300, 1000, 2600]
                .into_iter()
                .enumerate()
            {
                for levels in [2, 9, 1000] {
                    let values = ties(seed as u64 * 31 + levels, n, levels);
                    let order = argsort_at(&values, level);
                    assert!(sorts(&values, &order), "{n} values, {levels} levels");
                }
            }
            assert!(sorts(&stalled, &argsort_at(&stalled, level)));
            assert!(sorts(&infinite, &argsort_at(&infinite, level)));
        }
    }

    #[test]
    fn the_processors_level_is_the_default() {
        let values = ties(11, 900, 3);
        assert_eq!(argsort(&values), argsort_at(&values, CpuLevel::current()));
    }
}
