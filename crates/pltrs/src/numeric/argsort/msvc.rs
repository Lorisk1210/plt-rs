//! C++'s `std::sort` as Microsoft's STL implements it, the comparison sort
//! x86-simd-sort falls back to in numpy's Windows wheels (built with
//! MSVC), so that ties come out in the same order there. Written from the
//! STL's source (`_Sort_unchecked` of `<algorithm>`, Apache-2.0 with LLVM
//! exception), an introsort:
//!
//! * a range of at most [`INSERTION_MAX`] items is insertion-sorted;
//! * a longer one is split three ways around a guessed median (Tukey's
//!   ninther past 40 items, else a median of three): the items before the
//!   pivots, those equal to them and those after, gathered in one pass
//!   from the pivot outwards;
//! * the shorter side is sorted first, the longer one in the loop, which
//!   allows `1.5 log2 n` splits before a range is heap-sorted.

/// The longest range sorted by insertion (`_ISORT_MAX`).
const INSERTION_MAX: usize = 32;

/// Orders `items` by the strict weak order `less` on them, unstably,
/// leaving equal items where MSVC's `std::sort` leaves them.
pub(super) fn sort(items: &mut [usize], less: impl Fn(usize, usize) -> bool) {
    let ideal = items.len();
    sort_range(items, ideal, &less);
}

/// `_Sort_unchecked`: sorts `range` with `ideal` (shrinking by a quarter
/// at each split) left before it is heap-sorted.
fn sort_range(mut range: &mut [usize], mut ideal: usize, less: &impl Fn(usize, usize) -> bool) {
    loop {
        if range.len() <= INSERTION_MAX {
            insertion_sort(range, less);
            return;
        }
        if ideal == 0 {
            make_heap(range, less);
            sort_heap(range, less);
            return;
        }
        let (equal_start, equal_end) = partition(range, less);
        ideal = (ideal >> 1) + (ideal >> 2);
        let whole = std::mem::take(&mut range);
        if equal_start < whole.len() - equal_end {
            let (front, rest) = whole.split_at_mut(equal_start);
            sort_range(front, ideal, less);
            range = &mut rest[equal_end - equal_start..];
        } else {
            let (front, back) = whole.split_at_mut(equal_end);
            sort_range(back, ideal, less);
            range = &mut front[..equal_start];
        }
    }
}

/// `_Insertion_sort_unchecked`: an item smaller than the first goes to the
/// front at once, any other moves down past the greater ones before it.
fn insertion_sort(range: &mut [usize], less: &impl Fn(usize, usize) -> bool) {
    for next in 1..range.len() {
        let value = range[next];
        if less(value, range[0]) {
            range.copy_within(0..next, 1);
            range[0] = value;
        } else {
            let mut hole = next;
            while less(value, range[hole - 1]) {
                range[hole] = range[hole - 1];
                hole -= 1;
            }
            range[hole] = value;
        }
    }
}

/// `_Med3_unchecked`: orders the items at `first`, `mid` and `last` so
/// that their median is at `mid`.
fn median_of_three(
    range: &mut [usize],
    [first, mid, last]: [usize; 3],
    less: &impl Fn(usize, usize) -> bool,
) {
    if less(range[mid], range[first]) {
        range.swap(mid, first);
    }
    if less(range[last], range[mid]) {
        range.swap(last, mid);
        if less(range[mid], range[first]) {
            range.swap(mid, first);
        }
    }
}

/// `_Guess_median_unchecked`: puts a guess of the median of `range`'s
/// items at `mid`, sampling from `first` to `last` (both included).
fn guess_median(
    range: &mut [usize],
    first: usize,
    mid: usize,
    last: usize,
    less: &impl Fn(usize, usize) -> bool,
) {
    let count = last - first;
    if count > 40 {
        let step = (count + 1) >> 3;
        let two_steps = step << 1;
        median_of_three(range, [first, first + step, first + two_steps], less);
        median_of_three(range, [mid - step, mid, mid + step], less);
        median_of_three(range, [last - two_steps, last - step, last], less);
        median_of_three(range, [first + step, mid, last - step], less);
    } else {
        median_of_three(range, [first, mid, last], less);
    }
}

/// `_Partition_by_median_guess_unchecked`: splits `range` around a guessed
/// median and returns where the items equal to it start and end: none
/// greater before the start, none smaller from the end on.
fn partition(range: &mut [usize], less: &impl Fn(usize, usize) -> bool) -> (usize, usize) {
    let len = range.len();
    let mid = len >> 1;
    guess_median(range, 0, mid, len - 1, less);
    let equivalent = |range: &[usize], a: usize, b: usize| {
        !less(range[a], range[b]) && !less(range[b], range[a])
    };
    let (mut pivot_first, mut pivot_last) = (mid, mid + 1);
    while pivot_first > 0 && equivalent(range, pivot_first - 1, pivot_first) {
        pivot_first -= 1;
    }
    while pivot_last < len && equivalent(range, pivot_last, pivot_first) {
        pivot_last += 1;
    }
    let (mut greater, mut smaller) = (pivot_last, pivot_first);
    loop {
        while greater < len {
            if less(range[pivot_first], range[greater]) {
                greater += 1;
                continue;
            }
            if less(range[greater], range[pivot_first]) {
                break;
            }
            if pivot_last != greater {
                range.swap(pivot_last, greater);
            }
            pivot_last += 1;
            greater += 1;
        }
        while smaller > 0 {
            if less(range[smaller - 1], range[pivot_first]) {
                smaller -= 1;
                continue;
            }
            if less(range[pivot_first], range[smaller - 1]) {
                break;
            }
            pivot_first -= 1;
            if pivot_first != smaller - 1 {
                range.swap(pivot_first, smaller - 1);
            }
            smaller -= 1;
        }
        match (smaller == 0, greater == len) {
            (true, true) => return (pivot_first, pivot_last),
            // No room at the bottom: the pivots move up one place.
            (true, false) => {
                if pivot_last != greater {
                    range.swap(pivot_first, pivot_last);
                }
                pivot_last += 1;
                range.swap(pivot_first, greater);
                pivot_first += 1;
                greater += 1;
            }
            // No room at the top: the pivots move down one place.
            (false, true) => {
                smaller -= 1;
                pivot_first -= 1;
                if smaller != pivot_first {
                    range.swap(smaller, pivot_first);
                }
                pivot_last -= 1;
                range.swap(pivot_first, pivot_last);
            }
            (false, false) => {
                smaller -= 1;
                range.swap(greater, smaller);
                greater += 1;
            }
        }
    }
}

/// `_Push_heap_by_index`: moves `value` up from `hole` towards `top` to
/// where it belongs in the heap.
fn push_heap(
    range: &mut [usize],
    mut hole: usize,
    top: usize,
    value: usize,
    less: &impl Fn(usize, usize) -> bool,
) {
    while top < hole {
        let parent = (hole - 1) >> 1;
        if !less(range[parent], value) {
            break;
        }
        range[hole] = range[parent];
        hole = parent;
    }
    range[hole] = value;
}

/// `_Pop_heap_hole_by_index`: moves the hole at `hole` down along the
/// greater children to the bottom of the heap of `bottom` items, then
/// puts `value` in from there.
fn sift_hole(
    range: &mut [usize],
    mut hole: usize,
    bottom: usize,
    value: usize,
    less: &impl Fn(usize, usize) -> bool,
) {
    let top = hole;
    let mut child = hole;
    let last_parent = (bottom - 1) >> 1;
    while child < last_parent {
        child = 2 * child + 2;
        if less(range[child], range[child - 1]) {
            child -= 1;
        }
        range[hole] = range[child];
        hole = child;
    }
    if child == last_parent && bottom.is_multiple_of(2) {
        range[hole] = range[bottom - 1];
        hole = bottom - 1;
    }
    push_heap(range, hole, top, value, less);
}

/// `_Make_heap_unchecked`: makes `range` a max-heap, from its last parent
/// up.
fn make_heap(range: &mut [usize], less: &impl Fn(usize, usize) -> bool) {
    let bottom = range.len();
    for hole in (0..bottom >> 1).rev() {
        let value = range[hole];
        sift_hole(range, hole, bottom, value, less);
    }
}

/// `_Sort_heap_unchecked`: moves the heap's greatest item to its end, one
/// at a time.
fn sort_heap(range: &mut [usize], less: &impl Fn(usize, usize) -> bool) {
    for last in (1..range.len()).rev() {
        let value = range[last];
        range[last] = range[0];
        sift_hole(range, 0, last, value, less);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `items` sorted by their keys `keys[item]`.
    fn sorted(keys: &[u32]) -> Vec<usize> {
        let mut items: Vec<usize> = (0..keys.len()).collect();
        sort(&mut items, |a, b| keys[a] < keys[b]);
        items
    }

    #[test]
    fn short_ranges_are_insertion_sorted_stably() {
        assert_eq!(sorted(&[]), Vec::<usize>::new());
        assert_eq!(sorted(&[2, 1, 2, 0, 1]), [3, 1, 4, 0, 2]);
    }

    #[test]
    fn every_order_is_sorted_and_a_permutation() {
        let mut state = 0x9e37_79b9_u32;
        for len in [33, 41, 100, 1000, 5000] {
            let keys: Vec<u32> = (0..len)
                .map(|_| {
                    state ^= state << 13;
                    state ^= state >> 17;
                    state ^= state << 5;
                    state % 7
                })
                .collect();
            let items = sorted(&keys);
            assert!(items.windows(2).all(|pair| keys[pair[0]] <= keys[pair[1]]));
            let mut seen = items.clone();
            seen.sort_unstable();
            assert_eq!(seen, (0..len).collect::<Vec<_>>());
        }
    }

    #[test]
    fn heap_sort_takes_over_without_progress() {
        let keys: Vec<u32> = (0..200).map(|i| i % 3).collect();
        let mut items: Vec<usize> = (0..keys.len()).collect();
        sort_range(&mut items, 0, &|a, b| keys[a] < keys[b]);
        assert!(items.windows(2).all(|pair| keys[pair[0]] <= keys[pair[1]]));
    }
}
