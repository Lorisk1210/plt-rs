//! numpy's own index sort (`aquicksort_` and `aheapsort_` of its
//! `npysort`), which sorts the indices of `float64` arrays wherever numpy
//! has no SIMD sort for them: on every processor but x86.
//!
//! An introsort: quicksort on median-of-three pivots, the larger part
//! put aside each time, down to ranges of at most [`SMALL`] + 1 indices,
//! which an insertion sort orders; a range still that long after
//! `2 floor(log2 n)` splits is heap-sorted. The comparisons and swaps
//! happen in numpy's order, which is what decides where equal values end
//! up.

use super::nan_last_less;

/// The longest range (less one) left to the insertion sort (numpy's
/// `SMALL_QUICKSORT`).
const SMALL: usize = 15;

/// Sorts `order`, indices into `values`, by their values ascending, NaN
/// last, leaving equal values where numpy leaves them.
pub(super) fn sort(values: &[f64], order: &mut [usize]) {
    let Some(mut right) = order.len().checked_sub(1) else {
        return;
    };
    let mut left = 0;
    // The ranges put aside, each with the splits it may still take when
    // its turn comes: `None` once numpy's depth counter is negative.
    let mut pending: Vec<(usize, usize, Option<u32>)> = Vec::new();
    let mut splits_left = Some(2 * order.len().ilog2());
    loop {
        if splits_left.is_some() {
            while right > left + SMALL {
                let pivot_at = partition(values, order, left, right);
                splits_left = splits_left.and_then(|splits| splits.checked_sub(1));
                // The larger part waits; the smaller one is split on.
                if pivot_at - left < right - pivot_at {
                    pending.push((pivot_at + 1, right, splits_left));
                    right = pivot_at - 1;
                } else {
                    pending.push((left, pivot_at - 1, splits_left));
                    left = pivot_at + 1;
                }
            }
            insertion_sort(values, order, left, right);
        } else {
            heap_sort(values, &mut order[left..=right]);
        }
        let Some(next) = pending.pop() else {
            return;
        };
        (left, right, splits_left) = next;
    }
}

/// Splits `order[left..=right]` (more than [`SMALL`] + 1 indices) around
/// the median of its first, middle and last values, and returns where the
/// pivot ends up: nothing greater before it, nothing smaller after it.
fn partition(values: &[f64], order: &mut [usize], left: usize, right: usize) -> usize {
    let less = |a: usize, b: usize| nan_last_less(values[a], values[b]);
    let middle = left + ((right - left) >> 1);
    if less(order[middle], order[left]) {
        order.swap(middle, left);
    }
    if less(order[right], order[middle]) {
        order.swap(right, middle);
    }
    if less(order[middle], order[left]) {
        order.swap(middle, left);
    }
    let pivot = values[order[middle]];
    let (mut low, mut high) = (left, right - 1);
    order.swap(middle, high);
    loop {
        // The first value is no greater than the pivot and the pivot sits
        // before the last, so both scans stop inside the range.
        low += 1;
        while nan_last_less(values[order[low]], pivot) {
            low += 1;
        }
        high -= 1;
        while nan_last_less(pivot, values[order[high]]) {
            high -= 1;
        }
        if low >= high {
            break;
        }
        order.swap(low, high);
    }
    order.swap(low, right - 1);
    low
}

/// Orders `order[left..=right]` by insertion.
fn insertion_sort(values: &[f64], order: &mut [usize], left: usize, right: usize) {
    for at in left + 1..=right {
        let index = order[at];
        let value = values[index];
        let mut hole = at;
        while hole > left && nan_last_less(value, values[order[hole - 1]]) {
            order[hole] = order[hole - 1];
            hole -= 1;
        }
        order[hole] = index;
    }
}

/// numpy's `aheapsort_` of `order`: a max-heap built by sifting the first
/// half down, then emptied from the top.
fn heap_sort(values: &[f64], order: &mut [usize]) {
    let less = |a: usize, b: usize| nan_last_less(values[a], values[b]);
    // numpy indexes the heap from one.
    let sift = |heap: &mut [usize], moved: usize, from: usize, len: usize| {
        let (mut at, mut child) = (from, from << 1);
        while child <= len {
            if child < len && less(heap[child - 1], heap[child]) {
                child += 1;
            }
            if !less(moved, heap[child - 1]) {
                break;
            }
            heap[at - 1] = heap[child - 1];
            at = child;
            child += child;
        }
        heap[at - 1] = moved;
    };
    let mut len = order.len();
    for start in (1..=len >> 1).rev() {
        let moved = order[start - 1];
        sift(order, moved, start, len);
    }
    while len > 1 {
        let moved = order[len - 1];
        order[len - 1] = order[0];
        len -= 1;
        sift(order, moved, 1, len);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sorted(values: &[f64]) -> Vec<usize> {
        let mut order: Vec<usize> = (0..values.len()).collect();
        sort(values, &mut order);
        order
    }

    /// `np.argsort` of numpy 2.5.4 on arm64 macOS.
    #[test]
    fn ties_come_out_in_numpys_order() {
        assert_eq!(
            sorted(&[1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0]),
            [1, 3, 5, 7, 0, 2, 4, 6, 8]
        );
        let cycle: Vec<f64> = (0..40).map(|i| f64::from((i * 7) % 3)).collect();
        assert_eq!(
            sorted(&cycle),
            [
                0, 36, 33, 30, 27, 24, 21, 18, 15, 12, 39, 9, 3, 6, 25, 37, 1, 34, 31, 28, 4, 10,
                19, 16, 13, 7, 22, 14, 35, 8, 2, 32, 23, 29, 38, 26, 20, 5, 17, 11
            ]
        );
        let squares: Vec<f64> = (0..70).map(|i| f64::from((i * i) % 2)).collect();
        assert_eq!(
            sorted(&squares),
            [
                0, 28, 30, 32, 68, 36, 38, 40, 42, 44, 26, 46, 50, 52, 54, 56, 58, 60, 62, 64, 66,
                48, 24, 34, 22, 8, 12, 14, 6, 4, 18, 16, 10, 2, 20, 51, 67, 1, 53, 7, 55, 63, 57,
                5, 9, 59, 61, 3, 65, 11, 47, 25, 21, 27, 29, 19, 31, 33, 17, 35, 37, 15, 39, 41,
                13, 43, 45, 23, 49, 69
            ]
        );
    }

    /// `np.argsort` of numpy 2.5.4 on arm64 macOS: NaN goes last.
    #[test]
    fn nan_sorts_last() {
        let values: Vec<f64> = (0..50)
            .map(|i| {
                if i % 9 == 0 {
                    f64::NAN
                } else {
                    f64::from((i * 5) % 4)
                }
            })
            .collect();
        assert_eq!(
            sorted(&values),
            [
                24, 48, 12, 28, 20, 32, 8, 4, 40, 44, 16, 37, 29, 25, 41, 21, 33, 49, 1, 5, 13, 17,
                42, 2, 38, 34, 30, 46, 6, 26, 22, 14, 10, 47, 43, 7, 11, 23, 35, 39, 3, 15, 19, 31,
                45, 18, 9, 27, 36, 0
            ]
        );
    }

    #[test]
    fn degenerate_inputs() {
        assert_eq!(sorted(&[]), Vec::<usize>::new());
        assert_eq!(sorted(&[2.0]), [0]);
        assert_eq!(sorted(&[f64::NAN, 1.0]), [1, 0]);
    }

    #[test]
    fn the_heap_sort_orders_what_it_is_given() {
        let values: Vec<f64> = (0..37).map(|i| f64::from((i * 11) % 13)).collect();
        let mut order: Vec<usize> = (0..values.len()).collect();
        heap_sort(&values, &mut order);
        assert!(order.windows(2).all(|w| values[w[0]] <= values[w[1]]));
        let mut seen = order.clone();
        seen.sort_unstable();
        assert_eq!(seen, (0..values.len()).collect::<Vec<_>>());
    }
}
