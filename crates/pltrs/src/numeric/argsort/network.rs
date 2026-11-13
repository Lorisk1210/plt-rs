//! x86-simd-sort's bitonic sorting networks for up to 256 keys
//! (`argsort_n`), with each register a run of [`Width::LANES`] lanes.
//!
//! Every step of every network compares a lane with a partner and leaves
//! the smaller key in the lower one, moving the index along; equal keys
//! never move. That rule is what decides the order of equal values.

use super::width::{Step, Width};

/// The most keys the networks sort.
pub(super) const MAX_KEYS: usize = 256;

/// One register lane: a key and its index (`None` for the padding, an
/// infinite key, that fills the last registers up).
#[derive(Clone, Copy)]
struct Lane {
    key: f64,
    index: Option<usize>,
}

/// Sorts `indices` (at most [`MAX_KEYS`]) by their `values`.
pub(super) fn sort<W: Width>(values: &[f64], indices: &mut [usize]) {
    let n = indices.len();
    let mut registers = MAX_KEYS / W::LANES;
    while registers > 1 && n * 2 <= registers * W::LANES {
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
        .take(registers * W::LANES)
        .collect();
    for register in lanes.chunks_exact_mut(W::LANES) {
        run(register, W::SORT_REGISTER);
    }
    // `bitonic_fullmerge_n_vec`: merge pairs of sorted runs, then pairs
    // of those, until one run is left.
    let mut run_registers = 2;
    while run_registers <= registers {
        for group in lanes.chunks_exact_mut(run_registers * W::LANES) {
            merge::<W>(group);
        }
        run_registers *= 2;
    }
    // The padding sorts last, so this stores the first `n` lanes.
    for (slot, index) in indices.iter_mut().zip(lanes.iter().filter_map(|l| l.index)) {
        *slot = index;
    }
}

/// `bitonic_merge_n_vec`: merges the two sorted halves of `lanes`.
fn merge<W: Width>(lanes: &mut [Lane]) {
    let registers = lanes.len() / W::LANES;
    // Each register of the lower half against its mirror image.
    for low in 0..registers / 2 {
        let high = registers - 1 - low;
        for lane in 0..W::LANES {
            compare_exchange(
                lanes,
                low * W::LANES + lane,
                high * W::LANES + W::LANES - 1 - lane,
            );
        }
    }
    // `bitonic_clean_n_vec`: half cleaners over whole registers.
    let mut span = registers / 2;
    while span >= 2 {
        for start in (0..registers).step_by(span) {
            for low in start..start + span / 2 {
                for lane in 0..W::LANES {
                    let at = low * W::LANES + lane;
                    compare_exchange(lanes, at, at + span / 2 * W::LANES);
                }
            }
        }
        span /= 2;
    }
    for register in lanes.chunks_exact_mut(W::LANES) {
        run(register, W::MERGE_REGISTER);
    }
}

/// Runs the in-register network `steps` on one `register`.
fn run(register: &mut [Lane], steps: &[Step]) {
    for &(distance, mask) in steps {
        for low in (0..register.len()).filter(|lane| mask & (1 << lane) == 0) {
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
