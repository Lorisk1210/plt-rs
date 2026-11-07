//! The comparison sort x86-simd-sort falls back to (C++'s `std::sort` as
//! GCC's standard library implements it), so that ties come out in the
//! same order: an introsort.
//!
//! * Quicksort on median-of-three pivots splits the slice until every
//!   range holds at most [`RUN`] items; a range still longer after
//!   `2 floor(log2 n)` levels is heap-sorted instead.
//! * One insertion sort over the whole slice then orders the short ranges
//!   the quicksort left.
//!
//! The module was written from a description of these steps, not from
//! any standard library's source. Its comparisons and moves happen in the
//! order the description gives, which is what decides where equal items
//! end up; the bounds checks it adds never change the result of a strict
//! weak order.

/// The longest range the quicksort leaves to the final insertion sort.
const RUN: usize = 16;

/// Orders `items` by the strict weak order `less` on them, unstably,
/// leaving equal items where `std::sort` leaves them.
pub(super) fn sort(items: &mut [usize], less: impl Fn(usize, usize) -> bool) {
    if items.is_empty() {
        return;
    }
    let depth = 2 * items.len().ilog2() as usize;
    split(items, &less, depth);
    insertion_sort(items, &less);
}

/// Splits `range` into runs of at most [`RUN`] items, each holding no item
/// greater than any item of the runs after it; `depth` more splits are
/// allowed before a range is heap-sorted.
fn split(mut range: &mut [usize], less: &impl Fn(usize, usize) -> bool, mut depth: usize) {
    while range.len() > RUN {
        if depth == 0 {
            heap_sort(range, less);
            return;
        }
        depth -= 1;
        let cut = partition(range, less);
        let (front, back) = std::mem::take(&mut range).split_at_mut(cut);
        split(back, less, depth);
        range = front;
    }
}

/// Moves the median of three samples of `range` to its front and splits
/// the rest around it: the returned cut has no item greater than the pivot
/// before it and none smaller from it on.
fn partition(range: &mut [usize], less: &impl Fn(usize, usize) -> bool) -> usize {
    let len = range.len();
    let median = median_of_three(range, [1, len / 2, len - 1], less);
    range.swap(0, median);
    let pivot = range[0];
    let (mut low, mut high) = (1, len);
    loop {
        while low < len && less(range[low], pivot) {
            low += 1;
        }
        high -= 1;
        while high > 0 && less(pivot, range[high]) {
            high -= 1;
        }
        if low >= high {
            return low;
        }
        range.swap(low, high);
        low += 1;
    }
}

/// The position, among `[a, b, c]`, of the median of the items there.
fn median_of_three(
    range: &[usize],
    [a, b, c]: [usize; 3],
    less: &impl Fn(usize, usize) -> bool,
) -> usize {
    let before = |x: usize, y: usize| less(range[x], range[y]);
    if before(a, b) {
        if before(b, c) {
            b
        } else if before(a, c) {
            c
        } else {
            a
        }
    } else if before(a, c) {
        a
    } else if before(b, c) {
        c
    } else {
        b
    }
}

/// Sorts `range` through a max-heap of its items.
fn heap_sort(range: &mut [usize], less: &impl Fn(usize, usize) -> bool) {
    let len = range.len();
    for top in (0..len / 2).rev() {
        let item = range[top];
        sift(range, top, item, less);
    }
    for end in (1..len).rev() {
        let item = range[end];
        range[end] = range[0];
        sift(&mut range[..end], 0, item, less);
    }
}

/// Places `item` into `heap` at or below `top`, where the heap has a hole:
/// the hole first sinks to a leaf, always taking the greater child's item
/// (the right one when neither is greater), and `item` then rises from that
/// leaf back towards `top` past the items smaller than it.
fn sift(heap: &mut [usize], top: usize, item: usize, less: &impl Fn(usize, usize) -> bool) {
    let len = heap.len();
    let mut hole = top;
    // Sink while the hole has two children.
    while 2 * hole + 2 < len {
        let right = 2 * hole + 2;
        let child = if less(heap[right], heap[right - 1]) {
            right - 1
        } else {
            right
        };
        heap[hole] = heap[child];
        hole = child;
    }
    // A last parent with only a left child.
    if len.is_multiple_of(2) && 2 * hole + 2 == len {
        heap[hole] = heap[len - 1];
        hole = len - 1;
    }
    while hole > top {
        let parent = (hole - 1) / 2;
        if !less(heap[parent], item) {
            break;
        }
        heap[hole] = heap[parent];
        hole = parent;
    }
    heap[hole] = item;
}

/// Inserts each item of `items` in turn into the sorted ones before it.
/// An item of the first [`RUN`] that is smaller than the first one goes
/// straight to the front.
fn insertion_sort(items: &mut [usize], less: &impl Fn(usize, usize) -> bool) {
    let head = items.len().min(RUN);
    for next in 1..head {
        if less(items[next], items[0]) {
            items[..=next].rotate_right(1);
        } else {
            insert(items, next, less);
        }
    }
    for next in head..items.len() {
        insert(items, next, less);
    }
}

/// Moves the item at `at` left, past every item before it greater than it.
fn insert(items: &mut [usize], at: usize, less: &impl Fn(usize, usize) -> bool) {
    let item = items[at];
    let mut hole = at;
    while hole > 0 && less(item, items[hole - 1]) {
        items[hole] = items[hole - 1];
        hole -= 1;
    }
    items[hole] = item;
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use pltrs_core::rng::SplitMix64;

    use super::super::nan_last_less;
    use super::*;

    /// The indices of `keys` in the order `sort` gives them under `less`.
    fn order<T: Copy>(keys: &[T], less: impl Fn(T, T) -> bool) -> Vec<usize> {
        let mut items: Vec<usize> = (0..keys.len()).collect();
        sort(&mut items, |a, b| less(keys[a], keys[b]));
        items
    }

    /// Keys that make every quicksort split as uneven as it can be (the
    /// adversary of "A Killer Adversary for Quicksort", 1999): each
    /// comparison of two keys not settled yet settles the one that looks
    /// like the pivot as the next smallest key.
    fn adversary(n: usize) -> Vec<usize> {
        let unsettled = n;
        let keys = RefCell::new(vec![unsettled; n]);
        let settled = Cell::new(0);
        let pivot = Cell::new(0);
        let mut items: Vec<usize> = (0..n).collect();
        sort(&mut items, |a, b| {
            let mut keys = keys.borrow_mut();
            if keys[a] == unsettled && keys[b] == unsettled {
                let next = if a == pivot.get() { a } else { b };
                keys[next] = settled.replace(settled.get() + 1);
            }
            if keys[a] == unsettled {
                pivot.set(a);
            } else if keys[b] == unsettled {
                pivot.set(b);
            }
            keys[a] < keys[b]
        });
        keys.into_inner()
    }

    #[test]
    fn short_slices_sort_stably() {
        assert_eq!(order::<u8>(&[], |a, b| a < b), Vec::<usize>::new());
        assert_eq!(order(&[5], |a: u8, b| a < b), [0]);
        assert_eq!(order(&[2, 1, 2, 0, 1], |a: u8, b| a < b), [3, 1, 4, 0, 2]);
    }

    #[test]
    fn ties_fall_where_std_sort_leaves_them() {
        let keys: [u8; 40] = [
            1, 2, 2, 0, 0, 0, 0, 1, 2, 2, 1, 1, 2, 1, 2, 1, 1, 0, 0, 1, 2, 1, 2, 1, 2, 2, 0, 2, 0,
            1, 1, 0, 2, 2, 2, 2, 2, 2, 0, 1,
        ];
        assert_eq!(
            order(&keys, |a, b| a < b),
            [
                17, 38, 3, 4, 5, 6, 28, 18, 26, 31, 30, 19, 0, 21, 23, 15, 29, 16, 13, 11, 10, 7,
                39, 20, 34, 35, 33, 32, 36, 37, 27, 25, 24, 22, 14, 12, 9, 8, 2, 1
            ]
        );
    }

    #[test]
    fn nan_goes_last_in_std_sort_order() {
        let nan = f64::NAN;
        let keys = [
            1.0, 2.0, 0.0, 1.0, 2.0, nan, 1.0, 1.0, nan, nan, 1.0, 0.0, nan, 2.0, nan, nan, 1.0,
            0.0, 1.0, nan, 1.0, 2.0, 1.0, nan, nan, nan, 0.0, 2.0, nan, nan, 1.0, nan, 1.0, 1.0,
            0.0, 0.0,
        ];
        assert_eq!(
            order(&keys, nan_last_less),
            [
                35, 2, 34, 26, 11, 17, 18, 33, 32, 30, 22, 20, 0, 16, 3, 10, 6, 7, 1, 13, 4, 21,
                27, 25, 5, 31, 29, 28, 14, 24, 23, 8, 9, 19, 12, 15
            ]
        );
    }

    #[test]
    fn exhausted_depth_heap_sorts_ties_like_std_sort() {
        // The keys the adversary settles until the depth runs out (the odd
        // ones at odd positions before 20, the even ones from 24 on), and
        // 28 equal keys left unsettled, whose order the heap sort decides.
        let keys: Vec<u8> = (0..48)
            .map(|i| match i {
                0..20 if i % 2 == 1 => i,
                24..34 => 2 * (i - 24),
                _ => 48,
            })
            .collect();
        assert_eq!(
            order(&keys, |a, b| a < b),
            [
                24, 1, 25, 3, 26, 5, 27, 7, 28, 9, 29, 11, 30, 13, 31, 15, 32, 17, 33, 19, 38, 42,
                12, 36, 46, 8, 40, 0, 44, 16, 35, 47, 6, 37, 23, 39, 10, 41, 21, 43, 14, 45, 2, 18,
                20, 22, 4, 34
            ]
        );
    }

    #[test]
    fn adversarial_keys_cost_n_log_n_comparisons() {
        // Quicksort alone would compare about n²/2 times.
        let n = 2000;
        let keys = adversary(n);
        let comparisons = Cell::new(0);
        let sorted = order(&keys, |a, b| {
            comparisons.set(comparisons.get() + 1);
            a < b
        });
        assert!(sorted.windows(2).all(|w| keys[w[0]] <= keys[w[1]]));
        assert!(comparisons.get() < 4 * n * n.ilog2() as usize);
    }

    #[test]
    fn inconsistent_orders_stay_in_bounds() {
        // `<` with NaN, or a `less` that always or never holds, is no
        // strict weak order: the sort must still return a permutation.
        let mut random = SplitMix64::new(3);
        for n in [17, 100, 1000] {
            let keys: Vec<f64> = (0..n)
                .map(|_| match random.below(4) {
                    0 => f64::NAN,
                    k => k as f64,
                })
                .collect();
            let orders: [&dyn Fn(f64, f64) -> bool; 3] =
                [&|a, b| a < b, &|_, _| true, &|_, _| false];
            for less in orders {
                let mut items = order(&keys, less);
                items.sort_unstable();
                assert!(items.into_iter().eq(0..n));
            }
        }
    }
}
