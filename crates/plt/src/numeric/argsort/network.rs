//! x86-simd-sort's bitonic sorting networks for up to 256 keys
//! (`argsort_n`), with each AVX-512 register a run of [`LANES`] lanes.
//!
//! Every step of every network compares a lane with a partner and leaves
//! the smaller key in the lower one, moving the index along; equal keys
//! never move. That rule is what decides the order of equal values.

use super::LANES;

/// The most keys the networks sort.
pub(super) const MAX_KEYS: usize = 256;

/// One register lane: a key and its index (`None` for the padding, an
/// infinite key, that fills the last registers up).
#[derive(Clone, Copy)]
struct Lane {
    key: f64,
    index: Option<usize>,
}

/// One step of an in-register network: each lane whose bit is clear in
/// the mask takes the minimum of itself and the lane at its position XOR
/// the distance.
type Step = (usize, u8);

/// `sort_reg_8lanes`: sorts one register.
const SORT_REGISTER: [Step; 6] = [
    (1, 0xAA),
    (3, 0xCC),
    (1, 0xAA),
    (7, 0xF0),
    (2, 0xCC),
    (1, 0xAA),
];

/// `bitonic_merge_reg_8lanes`: sorts one bitonic register.
const MERGE_REGISTER: [Step; 3] = [(4, 0xF0), (2, 0xCC), (1, 0xAA)];

/// Sorts `indices` (at most [`MAX_KEYS`]) by their `values`.
pub(super) fn sort(values: &[f64], indices: &mut [usize]) {
    let n = indices.len();
    let mut registers = MAX_KEYS / LANES;
    while registers > 1 && n * 2 <= registers * LANES {
        registers /= 2;
    }
    let padding = Lane {
        key: f64::INFINITY,
        index: None,
    };
    let mut lanes: Vec<Lane> = indices
        .iter()
        .map(|&index| Lane {
            key: values[index],
            index: Some(index),
        })
        .chain(std::iter::repeat(padding))
        .take(registers * LANES)
        .collect();
    for register in lanes.as_chunks_mut::<LANES>().0 {
        run(register, &SORT_REGISTER);
    }
    // `bitonic_fullmerge_n_vec`: merge pairs of sorted runs, then pairs
    // of those, until one run is left.
    let mut run_registers = 2;
    while run_registers <= registers {
        for group in lanes.chunks_exact_mut(run_registers * LANES) {
            merge(group);
        }
        run_registers *= 2;
    }
    // The padding sorts last, so this stores the first `n` lanes.
    for (slot, index) in indices.iter_mut().zip(lanes.iter().filter_map(|l| l.index)) {
        *slot = index;
    }
}

/// `bitonic_merge_n_vec`: merges the two sorted halves of `lanes`.
fn merge(lanes: &mut [Lane]) {
    let registers = lanes.len() / LANES;
    // Each register of the lower half against its mirror image.
    for low in 0..registers / 2 {
        let high = registers - 1 - low;
        for lane in 0..LANES {
            compare_exchange(lanes, low * LANES + lane, high * LANES + LANES - 1 - lane);
        }
    }
    // `bitonic_clean_n_vec`: half cleaners over whole registers.
    let mut span = registers / 2;
    while span >= 2 {
        for start in (0..registers).step_by(span) {
            for low in start..start + span / 2 {
                for lane in 0..LANES {
                    let at = low * LANES + lane;
                    compare_exchange(lanes, at, at + span / 2 * LANES);
                }
            }
        }
        span /= 2;
    }
    for register in lanes.as_chunks_mut::<LANES>().0 {
        run(register, &MERGE_REGISTER);
    }
}

/// Runs the in-register network `steps` on one `register`.
fn run(register: &mut [Lane; LANES], steps: &[Step]) {
    for &(distance, mask) in steps {
        for low in (0..LANES).filter(|lane| mask & (1 << lane) == 0) {
            compare_exchange(register, low, low ^ distance);
        }
    }
}

/// Leaves the smaller key of lanes `low` and `high` in `low`; equal keys
/// stay where they are.
fn compare_exchange(lanes: &mut [Lane], low: usize, high: usize) {
    if lanes[high].key < lanes[low].key {
        lanes.swap(low, high);
    }
}
