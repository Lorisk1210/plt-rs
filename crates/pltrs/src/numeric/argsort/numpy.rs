//! numpy's own `argsort`, where numpy is in the process.
//!
//! The order of equal values is whatever the numpy at hand leaves, and
//! the x86-simd-sort numpy vendors changed between its releases: numpy
//! 1.25 and 1.26 leave ranges of up to 64 keys to the networks and
//! partition longer ones (and have no AVX2 sort), numpy 2.0 to 2.2 have
//! networks for 256 keys, and numpy 2.3 to 2.5, which this module
//! replays, also leave an array already in order alone. All of that is
//! above 64 values: up to there every release sorts with one network, or
//! with numpy's own introsort where it has no SIMD sort.
//!
//! So pltrs (Python) hands numpy's `argsort` over ([`sort_as_numpy`]),
//! and from then on:
//!
//! * an array of more than [`LONGEST_REPLAYED`] values is sorted by numpy
//!   itself: its order whatever its release, and sooner than the replay
//!   (numpy sorts with the processor's registers, the replay lane by
//!   lane);
//! * a shorter one is replayed, which costs less than the call. numpy is
//!   asked nothing about those until one comes out with equal values,
//!   whose order numpy's sort decides. Then, once per process, numpy
//!   sorts a few seeded arrays ([`PROBES`]): if the replay at the
//!   processor's level orders them as numpy does, it goes on; if numpy's
//!   own introsort does (numpy 1 with AVX2), that is replayed; and if
//!   neither does (a build with another sort), numpy sorts the short
//!   arrays too. Arrays with NaN take another path of the sort (C++'s
//!   `std::sort`, which is the C++ library's), so they are asked about,
//!   and routed, on their own.
//!
//! pltrs (Rust) alone has no numpy and replays at every length.

use std::sync::OnceLock;

use pltrs_core::numeric::CpuLevel;
use pltrs_core::rng::SplitMix64;

use super::{argsort_at, nan_last_less};

/// numpy's `argsort` of `float64` values; `None` where the call fails.
pub type NumpyArgsort = fn(&[f64]) -> Option<Vec<usize>>;

/// The longest array replayed where numpy can sort: as many values as
/// x86-simd-sort has left to one network in every release, and about
/// where a call of numpy's `argsort` starts to cost less than the replay.
pub const LONGEST_REPLAYED: usize = 64;

/// How arrays of up to [`LONGEST_REPLAYED`] values are sorted beside a
/// numpy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortRoute {
    /// By the replay of numpy's sort at this level.
    Replay(CpuLevel),
    /// By numpy itself, whose sort is none replayed.
    Numpy,
}

impl SortRoute {
    /// The route's name: its level's (`v4`), or `numpy`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Replay(level) => level.name(),
            Self::Numpy => "numpy",
        }
    }
}

/// numpy's `argsort`, once handed over.
static NUMPY: OnceLock<NumpyArgsort> = OnceLock::new();

/// The route of short arrays without NaN, once decided.
static PLAIN: OnceLock<SortRoute> = OnceLock::new();

/// The route of short arrays with NaN, once decided.
static WITH_NAN: OnceLock<SortRoute> = OnceLock::new();

/// Makes [`argsort`](super::argsort) follow the numpy whose `argsort`
/// this is, for the rest of the process; the first one handed over stays.
pub fn sort_as_numpy(numpy: NumpyArgsort) {
    NUMPY.get_or_init(|| numpy);
}

/// The route decided for short arrays with NaN or for those without;
/// `None` until a sort of that kind came out with equal values.
pub fn sort_route(with_nan: bool) -> Option<SortRoute> {
    route_of(with_nan).get().copied()
}

/// The numpy handed over, if any.
pub(super) fn handed_over() -> Option<NumpyArgsort> {
    NUMPY.get().copied()
}

/// Where the route of short arrays with or without NaN is kept.
fn route_of(with_nan: bool) -> &'static OnceLock<SortRoute> {
    if with_nan { &WITH_NAN } else { &PLAIN }
}

/// `n` draws from `levels` multiples of 1/4 (the families `ties*` of
/// `crates/pltrs/tools/argsort_digests.py`).
pub(super) fn ties(seed: u64, n: usize, levels: u64) -> Vec<f64> {
    let mut random = SplitMix64::new(seed);
    (0..n)
        .map(|_| (random.next_u64() % levels) as f64 / 4.0)
        .collect()
}

/// `values` with every fifth of them NaN (the tool's family `nan`, of
/// draws from four levels).
pub(super) fn with_nans(mut values: Vec<f64>) -> Vec<f64> {
    values.iter_mut().step_by(5).for_each(|v| *v = f64::NAN);
    values
}

/// The seed of a probe array of `n` values is this plus `n`.
const PROBE_SEED: u64 = 20_261_114;

/// The lengths of the arrays numpy is asked to sort: one register of
/// either width and less, several registers, and the longest replayed.
const PROBES: [usize; 4] = [9, 20, 40, LONGEST_REPLAYED];

/// The probe arrays with or without NaN.
fn probes(with_nan: bool) -> impl Iterator<Item = Vec<f64>> {
    PROBES.into_iter().map(move |n| {
        let seed = PROBE_SEED + n as u64;
        if with_nan {
            with_nans(ties(seed, n, 4))
        } else {
            ties(seed, n, 5)
        }
    })
}

/// Whether `order`, which sorts `values`, holds equal values side by
/// side (two NaN among them): only then can another sort order them
/// otherwise.
fn has_ties(values: &[f64], order: &[usize]) -> bool {
    order
        .windows(2)
        .any(|pair| !nan_last_less(values[pair[0]], values[pair[1]]))
}

/// numpy's `argsort` of `values`, if it is one: as many indices, each of
/// one of the values.
fn by_numpy(values: &[f64], numpy: NumpyArgsort) -> Option<Vec<usize>> {
    numpy(values)
        .filter(|order| order.len() == values.len() && order.iter().all(|&i| i < values.len()))
}

/// Asks `numpy` how it sorts short arrays at `level`: as the replay at
/// that level does, as its own introsort does, or as neither. A numpy
/// that cannot be asked is taken for the one replayed.
fn decide(level: CpuLevel, with_nan: bool, numpy: NumpyArgsort) -> SortRoute {
    let arrays: Vec<Vec<f64>> = probes(with_nan).collect();
    let Some(sorted) = arrays
        .iter()
        .map(|values| by_numpy(values, numpy))
        .collect::<Option<Vec<_>>>()
    else {
        return SortRoute::Replay(level);
    };
    let agrees = |replayed: &CpuLevel| {
        arrays
            .iter()
            .zip(&sorted)
            .all(|(values, order)| argsort_at(values, *replayed) == *order)
    };
    [level, CpuLevel::Baseline]
        .into_iter()
        .find(agrees)
        .map_or(SortRoute::Numpy, SortRoute::Replay)
}

/// The indices that sort `values` as `numpy` sorts them at `level`.
pub(super) fn sort(values: &[f64], level: CpuLevel, numpy: NumpyArgsort) -> Vec<usize> {
    let replay = || argsort_at(values, level);
    if values.len() > LONGEST_REPLAYED {
        return by_numpy(values, numpy).unwrap_or_else(replay);
    }
    let with_nan = values.iter().any(|value| value.is_nan());
    let route = route_of(with_nan);
    let decided = if let Some(route) = route.get() {
        *route
    } else {
        // Without equal values there is one order, whoever sorts.
        let order = replay();
        if !has_ties(values, &order) {
            return order;
        }
        // Decided outside the lock: numpy is called with the
        // interpreter's own lock, which another thread waiting here may
        // hold.
        let decision = decide(level, with_nan, numpy);
        match *route.get_or_init(|| decision) {
            SortRoute::Replay(replayed) if replayed == level => return order,
            other => other,
        }
    };
    match decided {
        SortRoute::Replay(replayed) => argsort_at(values, replayed),
        SortRoute::Numpy => by_numpy(values, numpy).unwrap_or_else(replay),
    }
}

#[cfg(test)]
#[expect(
    clippy::unnecessary_wraps,
    reason = "the numpys of these tests have the form of `NumpyArgsort`, failing or not"
)]
mod tests {
    use super::super::digests::{GOLDEN, digest};
    use super::*;

    /// The probe arrays' digests of numpy 2.5.4 without AVX2, with AVX2
    /// and with AVX-512 (`argsort_digests.py --probe`), for arrays without
    /// NaN. numpy 1.26.4 and 2.2.6 give the same, except numpy 1.26.4
    /// with AVX2, which gives what it gives without.
    const NUMPYS_PROBES: [u64; 3] = [
        0x8cc3_bca8_6fe6_ac99,
        0x2b2b_5b5c_f45b_065d,
        0x2b2b_5b5c_f45b_065d,
    ];

    /// The same for arrays with NaN, which x86-simd-sort leaves to C++'s
    /// `std::sort`: MSVC's in numpy's Windows wheel, which has no copy
    /// for AVX-512 either (`None`), and GCC's elsewhere.
    const NUMPYS_PROBES_WITH_NAN: [Option<u64>; 3] = if cfg!(windows) {
        [
            Some(0x3b6b_f2f6_6fc9_a95e),
            Some(0xefba_646f_021a_d803),
            None,
        ]
    } else {
        [
            Some(0x3b6b_f2f6_6fc9_a95e),
            Some(0xf97f_c952_03a4_8b6c),
            Some(0xf97f_c952_03a4_8b6c),
        ]
    };

    /// A stable sort, which is none of numpy's unstable ones.
    fn stable(values: &[f64]) -> Vec<usize> {
        let mut order: Vec<usize> = (0..values.len()).collect();
        order.sort_by(|&a, &b| values[a].total_cmp(&values[b]));
        order
    }

    fn numpy_sorting_stably(values: &[f64]) -> Option<Vec<usize>> {
        Some(stable(values))
    }

    fn numpy_with_avx512(values: &[f64]) -> Option<Vec<usize>> {
        Some(argsort_at(values, CpuLevel::V4))
    }

    fn numpy_without_simd_sort(values: &[f64]) -> Option<Vec<usize>> {
        Some(argsort_at(values, CpuLevel::Baseline))
    }

    fn failing(_: &[f64]) -> Option<Vec<usize>> {
        None
    }

    fn too_short(values: &[f64]) -> Option<Vec<usize>> {
        Some((1..values.len()).collect())
    }

    fn out_of_range(values: &[f64]) -> Option<Vec<usize>> {
        Some((1..=values.len()).collect())
    }

    #[test]
    fn the_port_sorts_the_probes_as_numpy_does() {
        let plain = NUMPYS_PROBES.map(Some);
        for (with_nan, numpys) in [(false, plain), (true, NUMPYS_PROBES_WITH_NAN)] {
            for (level, numpy) in CpuLevel::ALL.into_iter().zip(numpys) {
                let Some(numpy) = numpy else { continue };
                let replayed = probes(with_nan).fold(0, |total: u64, values| {
                    let order = argsort_at(&values, level);
                    total.wrapping_mul(GOLDEN).wrapping_add(digest(&order))
                });
                assert_eq!(replayed, numpy, "with NaN: {with_nan}, at {level}");
            }
        }
    }

    #[test]
    fn a_numpy_sorting_as_replayed_is_replayed() {
        for with_nan in [false, true] {
            // The networks of either width are one up to 64 keys.
            for level in [CpuLevel::V3, CpuLevel::V4] {
                assert_eq!(
                    decide(level, with_nan, numpy_with_avx512),
                    SortRoute::Replay(level)
                );
            }
            // numpy 1 with AVX2 has no SIMD sort; without AVX2 none has.
            for level in CpuLevel::ALL {
                assert_eq!(
                    decide(level, with_nan, numpy_without_simd_sort),
                    SortRoute::Replay(CpuLevel::Baseline)
                );
            }
        }
        assert_eq!(SortRoute::Replay(CpuLevel::V3).name(), "v3");
    }

    #[test]
    fn a_numpy_sorting_as_none_replayed_sorts_itself() {
        for with_nan in [false, true] {
            for level in CpuLevel::ALL {
                assert_eq!(
                    decide(level, with_nan, numpy_sorting_stably),
                    SortRoute::Numpy
                );
            }
        }
        assert_eq!(
            decide(CpuLevel::Baseline, false, numpy_with_avx512),
            SortRoute::Numpy
        );
        assert_eq!(SortRoute::Numpy.name(), "numpy");
    }

    #[test]
    fn a_numpy_that_cannot_be_asked_is_taken_for_the_one_replayed() {
        for numpy in [failing, too_short, out_of_range] {
            assert_eq!(
                decide(CpuLevel::V4, false, numpy),
                SortRoute::Replay(CpuLevel::V4)
            );
            assert_eq!(by_numpy(&[2.0, 1.0], numpy), None);
            // A long array it fails to sort is replayed.
            let long = ties(3, 300, 5);
            assert_eq!(
                sort(&long, CpuLevel::V4, numpy),
                argsort_at(&long, CpuLevel::V4)
            );
        }
    }

    #[test]
    fn long_arrays_are_numpys_and_short_ones_without_equal_values_ask_nothing() {
        // A stable order tells numpy's sort from the replay.
        let long = ties(1, LONGEST_REPLAYED + 1, 5);
        assert_eq!(
            sort(&long, CpuLevel::V4, numpy_sorting_stably),
            stable(&long)
        );
        assert_ne!(argsort_at(&long, CpuLevel::V4), stable(&long));
        let with_nan = with_nans(ties(2, 500, 4));
        assert_eq!(
            sort(&with_nan, CpuLevel::V3, numpy_sorting_stably),
            stable(&with_nan)
        );
        // `failing` would be taken for the one replayed if asked.
        assert_eq!(
            sort(&[3.0, 1.0, 2.0, -1.0], CpuLevel::V4, failing),
            [3, 1, 2, 0]
        );
        assert_eq!(sort(&[], CpuLevel::V4, failing), Vec::<usize>::new());
    }

    #[test]
    fn only_equal_values_can_be_ordered_otherwise() {
        let nan = f64::NAN;
        assert!(!has_ties(&[3.0, 1.0, 2.0], &[1, 2, 0]));
        assert!(!has_ties(&[nan, 1.0], &[1, 0]));
        assert!(has_ties(&[1.0, 2.0, 1.0], &[0, 2, 1]));
        assert!(has_ties(&[0.0, -0.0], &[0, 1]));
        assert!(has_ties(&[nan, 1.0, nan], &[1, 0, 2]));
        assert!(!has_ties(&[], &[]));
    }

    #[test]
    fn the_probes_tell_the_sorts_apart() {
        let sorted = |level, with_nan| -> Vec<Vec<usize>> {
            probes(with_nan)
                .map(|values| argsort_at(&values, level))
                .collect()
        };
        for with_nan in [false, true] {
            assert_eq!(
                sorted(CpuLevel::V3, with_nan),
                sorted(CpuLevel::V4, with_nan)
            );
            assert_ne!(
                sorted(CpuLevel::Baseline, with_nan),
                sorted(CpuLevel::V4, with_nan)
            );
            assert!(probes(with_nan).all(|values| values.len() <= LONGEST_REPLAYED));
        }
    }
}
