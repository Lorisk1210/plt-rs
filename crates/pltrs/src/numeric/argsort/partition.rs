//! x86-simd-sort's quicksort of indices (`argsort_`): ranges of more than
//! [`network::MAX_KEYS`] keys are partitioned around the upper median of
//! one register of samples, four registers at a time
//! (`argpartition_unrolled`).

use super::width::{MAX_LANES, Width};
use super::{network, std_sort};

/// Registers each partition step takes in (`num_unroll`).
const UNROLL: usize = 4;

/// The most keys a partition step takes in.
const MAX_BLOCK: usize = UNROLL * MAX_LANES;

/// Sorts `indices` by their `values`, with `max_iters` more levels of
/// partitions before `std::sort` ([`std_sort`]) takes over.
pub(super) fn sort<W: Width>(values: &[f64], indices: &mut [usize], max_iters: usize) {
    if max_iters == 0 {
        std_sort(indices, |a, b| values[a] < values[b]);
        return;
    }
    if indices.len() <= network::MAX_KEYS {
        network::sort::<W>(values, indices);
        return;
    }
    let pivot = pivot::<W>(values, indices);
    let partition = partition::<W>(values, indices, pivot);
    let (below, rest) = indices.split_at_mut(partition.split);
    if partition.smallest < pivot {
        sort::<W>(values, below, max_iters - 1);
    }
    if pivot < partition.biggest {
        sort::<W>(values, rest, max_iters - 1);
    }
}

/// The upper median of a register of evenly spaced samples
/// (`get_pivot_64bit` for a range of more keys than a register holds):
/// the fifth smallest of eight, the third smallest of four.
fn pivot<W: Width>(values: &[f64], indices: &[usize]) -> f64 {
    let step = (indices.len() - 1) / W::LANES;
    let mut samples = [0.0; MAX_LANES];
    let samples = &mut samples[..W::LANES];
    for (k, sample) in samples.iter_mut().enumerate() {
        *sample = values[indices[(k + 1) * step]];
    }
    samples.sort_unstable_by(f64::total_cmp);
    samples[W::LANES / 2]
}

/// A partitioned range.
struct Partition {
    /// Where the keys of at least the pivot start.
    split: usize,
    /// The smallest key in the range.
    smallest: f64,
    /// The largest key in the range.
    biggest: f64,
}

/// The indices a partition step holds in registers, taken out of the
/// range before their places are stored into.
struct Block {
    indices: [usize; MAX_BLOCK],
    len: usize,
}

impl Block {
    /// A copy of the `UNROLL` registers of `indices` from `at` on.
    fn load<W: Width>(indices: &[usize], at: usize) -> Self {
        let len = UNROLL * W::LANES;
        let mut block = [0; MAX_BLOCK];
        block[..len].copy_from_slice(&indices[at..at + len]);
        Self {
            indices: block,
            len,
        }
    }

    /// The indices held.
    fn indices(&self) -> &[usize] {
        &self.indices[..self.len]
    }
}

/// Puts the indices of keys below `pivot` first (`argpartition_unrolled`
/// for a range of more than [`network::MAX_KEYS`] keys).
fn partition<W: Width>(values: &[f64], indices: &mut [usize], pivot: f64) -> Partition {
    let block = UNROLL * W::LANES;
    let mut store = Stores {
        values,
        pivot,
        smallest: f64::INFINITY,
        biggest: f64::NEG_INFINITY,
        left: 0,
        right: 0,
    };
    let (mut left, mut right) = (0, indices.len());
    // One key at a time until the rest is a whole number of blocks.
    for _ in 0..indices.len() % block {
        let key = values[indices[left]];
        store.see(key);
        if key < pivot {
            left += 1;
        } else {
            right -= 1;
            indices.swap(left, right);
        }
    }
    // The first and last block are partitioned last, which frees the
    // room the blocks in between are stored into.
    let first = Block::load::<W>(indices, left);
    let last = Block::load::<W>(indices, right - block);
    store.left = left;
    store.right = right;
    left += block;
    right -= block;
    while right != left {
        // Take the next block from the side with less room left.
        let next = if store.right - right < left - store.left {
            right -= block;
            Block::load::<W>(indices, right)
        } else {
            left += block;
            Block::load::<W>(indices, left - block)
        };
        store.block::<W>(indices, &next);
    }
    store.block::<W>(indices, &first);
    store.block::<W>(indices, &last);
    Partition {
        split: store.left,
        smallest: store.smallest,
        biggest: store.biggest,
    }
}

/// The state of a partition: where the next keys below and at least the
/// pivot go, and the extremes seen so far.
struct Stores<'a> {
    values: &'a [f64],
    pivot: f64,
    smallest: f64,
    biggest: f64,
    /// Where the next index of a key below the pivot goes.
    left: usize,
    /// Where the stored indices of keys of at least the pivot start.
    right: usize,
}

impl Stores<'_> {
    /// Takes `key` into the extremes.
    fn see(&mut self, key: f64) {
        self.smallest = self.smallest.min(key);
        self.biggest = self.biggest.max(key);
    }

    /// Partitions the registers of `block` in turn.
    fn block<W: Width>(&mut self, indices: &mut [usize], block: &Block) {
        for register in block.indices().chunks_exact(W::LANES) {
            self.register::<W>(indices, register);
        }
    }

    /// `partition_vec`: stores the indices of keys below the pivot at the
    /// left store in lane order, and the others before the right one, in
    /// lane order too (`partition_vec_avx512`) or from the last lane down
    /// (`partition_vec_avx2`).
    fn register<W: Width>(&mut self, indices: &mut [usize], register: &[usize]) {
        let mut below = self.left;
        let mut upper = self.right;
        for &index in register {
            let key = self.values[index];
            self.see(key);
            if key < self.pivot {
                indices[below] = index;
                below += 1;
            } else {
                upper -= 1;
            }
        }
        let mut at = upper;
        let mut place = |index: usize| {
            indices[at] = index;
            at += 1;
        };
        let upper_keys = register
            .iter()
            .copied()
            .filter(|&index| self.values[index] >= self.pivot);
        if W::REVERSES_UPPER_KEYS {
            upper_keys.rev().for_each(&mut place);
        } else {
            upper_keys.for_each(&mut place);
        }
        self.left = below;
        self.right = upper;
    }
}
