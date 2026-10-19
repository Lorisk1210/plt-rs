//! libstdc++'s `std::sort`, which x86-simd-sort falls back to for arrays
//! with NaN and when its partitions stop making progress: introsort with
//! a median-of-three pivot, heapsort past the depth limit and a final
//! insertion sort.

/// Ranges this short are left to the final insertion sort (`_S_threshold`).
const THRESHOLD: usize = 16;

/// Sorts `items` by the strict weak order `less`, exactly as `std::sort`.
pub(super) fn sort(items: &mut [usize], less: impl Fn(usize, usize) -> bool) {
    if items.is_empty() {
        return;
    }
    introsort_loop(items, 2 * items.len().ilog2() as usize, &less);
    final_insertion_sort(items, &less);
}

/// `__introsort_loop`: partitions down to short ranges, recursing into
/// the upper part and looping on the lower one.
fn introsort_loop(mut items: &mut [usize], mut depth: usize, less: &impl Fn(usize, usize) -> bool) {
    while items.len() > THRESHOLD {
        if depth == 0 {
            heap_sort(items, less);
            return;
        }
        depth -= 1;
        let cut = partition_pivot(items, less);
        let (low, high) = std::mem::take(&mut items).split_at_mut(cut);
        introsort_loop(high, depth, less);
        items = low;
    }
}

/// `__unguarded_partition_pivot`: moves the median of the second, middle
/// and last items to the front and partitions the rest around it.
fn partition_pivot(items: &mut [usize], less: &impl Fn(usize, usize) -> bool) -> usize {
    let (a, b, c) = (1, items.len() / 2, items.len() - 1);
    let lt = |x: usize, y: usize| less(items[x], items[y]);
    let median = if lt(a, b) {
        if lt(b, c) {
            b
        } else if lt(a, c) {
            c
        } else {
            a
        }
    } else if lt(a, c) {
        a
    } else if lt(b, c) {
        c
    } else {
        b
    };
    items.swap(0, median);
    let pivot = items[0];
    let (mut first, mut last) = (1, items.len());
    loop {
        while less(items[first], pivot) {
            first += 1;
        }
        last -= 1;
        while less(pivot, items[last]) {
            last -= 1;
        }
        if first >= last {
            return first;
        }
        items.swap(first, last);
        first += 1;
    }
}

/// `__partial_sort` of the whole range: `__make_heap`, then
/// `__sort_heap`.
fn heap_sort(items: &mut [usize], less: &impl Fn(usize, usize) -> bool) {
    let len = items.len();
    for parent in (0..len / 2).rev() {
        adjust_heap(items, parent, len, items[parent], less);
    }
    for last in (1..len).rev() {
        let value = items[last];
        items[last] = items[0];
        adjust_heap(items, 0, last, value, less);
    }
}

/// `__adjust_heap`: sinks the hole at `hole` to a leaf of the heap of the
/// first `len` items, then pushes `value` up from there.
fn adjust_heap(
    items: &mut [usize],
    mut hole: usize,
    len: usize,
    value: usize,
    less: &impl Fn(usize, usize) -> bool,
) {
    let top = hole;
    let mut child = hole;
    while child < (len - 1) / 2 {
        child = 2 * (child + 1);
        if less(items[child], items[child - 1]) {
            child -= 1;
        }
        items[hole] = items[child];
        hole = child;
    }
    if len.is_multiple_of(2) && child == (len - 2) / 2 {
        child = 2 * (child + 1);
        items[hole] = items[child - 1];
        hole = child - 1;
    }
    while hole > top && less(items[(hole - 1) / 2], value) {
        let parent = (hole - 1) / 2;
        items[hole] = items[parent];
        hole = parent;
    }
    items[hole] = value;
}

/// `__final_insertion_sort`: the first [`THRESHOLD`] items with a guard,
/// the rest without (the partitions put a smaller item before each).
fn final_insertion_sort(items: &mut [usize], less: &impl Fn(usize, usize) -> bool) {
    let guarded = items.len().min(THRESHOLD);
    for i in 1..guarded {
        if less(items[i], items[0]) {
            items[..=i].rotate_right(1);
        } else {
            linear_insert(items, i, less);
        }
    }
    for i in guarded..items.len() {
        linear_insert(items, i, less);
    }
}

/// `__unguarded_linear_insert`: moves the item at `at` left past every
/// item it is less than.
fn linear_insert(items: &mut [usize], mut at: usize, less: &impl Fn(usize, usize) -> bool) {
    let value = items[at];
    while less(value, items[at - 1]) {
        items[at] = items[at - 1];
        at -= 1;
    }
    items[at] = value;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heapsort_and_insertion_sort_order_everything() {
        let keys: Vec<u32> = (0..100u32).map(|i| (i * 37 + 11) % 23).collect();
        let less = |a: usize, b: usize| keys[a] < keys[b];
        let mut heap: Vec<usize> = (0..keys.len()).collect();
        heap_sort(&mut heap, &less);
        assert!(heap.windows(2).all(|w| keys[w[0]] <= keys[w[1]]));
        let mut all: Vec<usize> = (0..keys.len()).collect();
        sort(&mut all, less);
        assert!(all.windows(2).all(|w| keys[w[0]] <= keys[w[1]]));
    }
}
