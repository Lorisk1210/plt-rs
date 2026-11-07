//! x86-simd-sort's quicksort of indices (`argsort_`): ranges of more than
//! [`network::MAX_KEYS`] keys are partitioned around a median of eight
//! samples, four AVX-512 registers at a time (`argpartition_unrolled`).

use super::{LANES, fallback, network};

/// Registers each partition step takes in (`num_unroll`).
const UNROLL: usize = 4;

/// Keys each partition step takes in.
const BLOCK: usize = UNROLL * LANES;

/// Sorts `indices` by their `values`, with `max_iters` more levels of
/// partitions before `std::sort` ([`fallback`]) takes over.
pub(super) fn sort(values: &[f64], indices: &mut [usize], max_iters: usize) {
    if max_iters == 0 {
        fallback::sort(indices, |a, b| values[a] < values[b]);
        return;
    }
    if indices.len() <= network::MAX_KEYS {
        network::sort(values, indices);
        return;
    }
    let pivot = pivot(values, indices);
    let partition = partition(values, indices, pivot);
    let (below, rest) = indices.split_at_mut(partition.split);
    if partition.smallest < pivot {
        sort(values, below, max_iters - 1);
    }
    if pivot < partition.biggest {
        sort(values, rest, max_iters - 1);
    }
}

/// The fifth smallest of eight evenly spaced samples (`get_pivot_64bit`
/// for a range of more than eight keys).
fn pivot(values: &[f64], indices: &[usize]) -> f64 {
    let step = (indices.len() - 1) / LANES;
    let mut samples: [f64; LANES] = std::array::from_fn(|k| values[indices[(k + 1) * step]]);
    samples.sort_unstable_by(f64::total_cmp);
    samples[LANES / 2]
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

/// Puts the indices of keys below `pivot` first (`argpartition_unrolled`
/// for a range of more than [`network::MAX_KEYS`] keys).
fn partition(values: &[f64], indices: &mut [usize], pivot: f64) -> Partition {
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
    for _ in 0..indices.len() % BLOCK {
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
    let first = block(indices, left);
    let last = block(indices, right - BLOCK);
    store.left = left;
    store.right = right;
    left += BLOCK;
    right -= BLOCK;
    while right != left {
        // Take the next block from the side with less room left.
        let next = if store.right - right < left - store.left {
            right -= BLOCK;
            block(indices, right)
        } else {
            left += BLOCK;
            block(indices, left - BLOCK)
        };
        store.block(indices, &next);
    }
    store.block(indices, &first);
    store.block(indices, &last);
    Partition {
        split: store.left,
        smallest: store.smallest,
        biggest: store.biggest,
    }
}

/// A copy of the `BLOCK` indices from `at` on, as the registers hold them.
fn block(indices: &[usize], at: usize) -> [usize; BLOCK] {
    std::array::from_fn(|k| indices[at + k])
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
    fn block(&mut self, indices: &mut [usize], block: &[usize; BLOCK]) {
        for register in block.as_chunks::<LANES>().0 {
            self.register(indices, register);
        }
    }

    /// `partition_vec_avx512`: compresses the indices of keys below the
    /// pivot to the left store and the others to the right one, each in
    /// lane order.
    fn register(&mut self, indices: &mut [usize], register: &[usize]) {
        let (rest, below): (Vec<usize>, Vec<usize>) = register
            .iter()
            .copied()
            .partition(|&index| self.values[index] >= self.pivot);
        for &index in register {
            self.see(self.values[index]);
        }
        indices[self.left..self.left + below.len()].copy_from_slice(&below);
        indices[self.right - rest.len()..self.right].copy_from_slice(&rest);
        self.left += below.len();
        self.right -= rest.len();
    }
}
