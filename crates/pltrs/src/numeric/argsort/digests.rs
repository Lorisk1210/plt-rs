//! numpy's `argsort` of seeded arrays against this module's, at every
//! level: `digests.txt` holds a digest of the indices numpy 2.5.4 returns
//! for each family of arrays and size (written by
//! `crates/pltrs/tools/argsort_digests.py`, which this file mirrors), with
//! numpy run without AVX2, with AVX2 and with AVX-512.

use pltrs_core::numeric::CpuLevel;
use pltrs_core::rng::SplitMix64;

use super::argsort_at;
use super::numpy::{ties, with_nans};

/// numpy's digests: a family, a size and a digest per level on each line.
const DIGESTS: &str = include_str!("digests.txt");

/// The lengths chained into the digest of the size `0-600`.
const CHAINED: std::ops::RangeInclusive<usize> = 0..=600;

/// The 64-bit golden ratio, `SplitMix64`'s increment.
pub(super) const GOLDEN: u64 = 0x9E37_79B9_7F4A_7C15;

/// The families of arrays, in the order that numbers their seeds.
const FAMILIES: [&str; 12] = [
    "ties2", "ties5", "ties40", "ties1000", "uniform", "nan", "infinite", "zeros", "sorted",
    "reversed", "constant", "stalled",
];

/// The array of `n` values of `family`.
fn values(family: &str, seed: u64, n: usize) -> Vec<f64> {
    let mut random = SplitMix64::new(seed);
    let indexed = |value: fn(usize, usize) -> f64| (0..n).map(|i| value(i, n)).collect();
    match family {
        "ties2" => ties(seed, n, 2),
        "ties5" => ties(seed, n, 5),
        "ties40" => ties(seed, n, 40),
        "ties1000" => ties(seed, n, 1000),
        "uniform" => (0..n).map(|_| random.next_f64()).collect(),
        "nan" => with_nans(ties(seed, n, 4)),
        "infinite" => ties(seed, n, 4)
            .into_iter()
            .enumerate()
            .map(|(i, v)| match i {
                _ if i % 7 == 1 => f64::NEG_INFINITY,
                _ if i % 3 == 0 => f64::INFINITY,
                _ => v,
            })
            .collect(),
        "zeros" => (0..n)
            .map(|_| [0.0, -0.0, 1.0, -1.0][random.below(4)])
            .collect(),
        "sorted" => indexed(|i, _| (i / 3) as f64),
        "reversed" => indexed(|i, n| ((n - i) / 3) as f64),
        "constant" => vec![2.5; n],
        "stalled" => indexed(|i, _| if i % 7 == 3 { (i % 13) as f64 } else { 0.0 }),
        other => panic!("unknown family {other}"),
    }
}

/// The tool's digest of `order`: its indices mixed with their positions.
pub(super) fn digest(order: &[usize]) -> u64 {
    order
        .iter()
        .enumerate()
        .map(|(position, &index)| {
            let mut z = index as u64 ^ (position as u64).wrapping_mul(GOLDEN);
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        })
        .fold(0, u64::wrapping_add)
}

/// The digest of this module's `argsort` of a family's array of `size`
/// values, or of every length of [`CHAINED`] in turn.
fn family_digest(family: &str, size: &str, level: CpuLevel) -> u64 {
    let number = FAMILIES
        .iter()
        .position(|name| *name == family)
        .expect("a family of the table") as u64;
    let one = |n: usize| {
        let values = values(family, number * 10_007 + n as u64, n);
        digest(&argsort_at(&values, level))
    };
    if size == "0-600" {
        CHAINED.fold(0, |total: u64, n| {
            total.wrapping_mul(GOLDEN).wrapping_add(one(n))
        })
    } else {
        one(size.parse().expect("a size"))
    }
}

/// The lines of the table: family, size, and numpy's digest per level.
fn table() -> impl Iterator<Item = (&'static str, &'static str, [u64; 3])> {
    DIGESTS
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| {
            let mut fields = line.split(' ');
            let mut field = || fields.next().expect("five fields a line");
            let (family, size) = (field(), field());
            let numpy = [(); 3].map(|()| u64::from_str_radix(field(), 16).expect("a digest"));
            (family, size, numpy)
        })
}

/// Checks every line of the table whose size passes `wanted`.
fn check(wanted: impl Fn(&str) -> bool) {
    let mut checked = 0;
    for (family, size, numpy) in table().filter(|(_, size, _)| wanted(size)) {
        for (level, expected) in CpuLevel::ALL.into_iter().zip(numpy) {
            assert_eq!(
                family_digest(family, size, level),
                expected,
                "{family} {size} at {level}"
            );
        }
        checked += 1;
    }
    assert_eq!(checked % FAMILIES.len(), 0);
    assert!(checked > 0);
}

#[test]
fn every_length_to_600_sorts_as_numpy_does_at_each_level() {
    check(|size| size == "0-600");
}

#[test]
fn longer_arrays_sort_as_numpy_does_at_each_level() {
    check(|size| size.parse().is_ok_and(|n: usize| n <= 100_000));
}

#[test]
fn a_million_values_sort_as_numpy_does_at_each_level() {
    check(|size| size == "1000000");
}

#[test]
fn the_levels_part_only_above_the_networks() {
    // The table itself: AVX2 and AVX-512 agree up to 256 values (their
    // digests of the chained lengths differ only through 257..=600), and
    // numpy's own sort differs from both.
    for family in ["ties5", "infinite", "zeros"] {
        for n in [2, 9, 64, 200, 256] {
            let values = values(family, n as u64, n);
            assert_eq!(
                argsort_at(&values, CpuLevel::V3),
                argsort_at(&values, CpuLevel::V4),
                "{family} {n}"
            );
        }
        let values = values(family, 300, 300);
        let [v2, v3, v4] = CpuLevel::ALL.map(|level| argsort_at(&values, level));
        assert!(v2 != v3 && v3 != v4, "{family}");
    }
}
