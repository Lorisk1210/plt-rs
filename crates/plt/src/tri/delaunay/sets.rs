//! qhull's set operations (`qset_r.c`) on vectors.
//!
//! Element order in qhull's sets is observable in the output, so the
//! deletions here reproduce qhull's exactly: unsorted deletes move the last
//! element into the hole, sorted deletes shift the tail down.

/// Removes the first `elem` by moving the last element into its place
/// (qhull's `qh_setdel`). Returns whether it was found.
pub(super) fn delete<T: PartialEq + Copy>(set: &mut Vec<T>, elem: T) -> bool {
    match set.iter().position(|&e| e == elem) {
        Some(i) => {
            set.swap_remove(i);
            true
        }
        None => false,
    }
}

/// Removes the first `elem`, keeping the order of the others
/// (qhull's `qh_setdelsorted`).
pub(super) fn delete_sorted<T: PartialEq + Copy>(set: &mut Vec<T>, elem: T) {
    if let Some(i) = set.iter().position(|&e| e == elem) {
        set.remove(i);
    }
}

/// Replaces the first `old` with `new` (qhull's `qh_setreplace`).
pub(super) fn replace<T: PartialEq + Copy>(set: &mut [T], old: T, new: T) {
    if let Some(slot) = set.iter_mut().find(|e| **e == old) {
        *slot = new;
    }
}

/// Appends `elem` unless present (qhull's `qh_setunique`).
pub(super) fn append_unique<T: PartialEq + Copy>(set: &mut Vec<T>, elem: T) {
    if !set.contains(&elem) {
        set.push(elem);
    }
}

/// A copy of `set` without element `nth` (qhull's `qh_setnew_delnthsorted`).
pub(super) fn without_nth<T: Copy>(set: &[T], nth: usize) -> Vec<T> {
    let mut out = Vec::with_capacity(set.len().saturating_sub(1));
    out.extend_from_slice(&set[..nth]);
    out.extend_from_slice(&set[nth + 1..]);
    out
}

/// Whether sorted sets `a` and `b` are equal except for `skip_a` in `a` and
/// `skip_b` in `b` (qhull's `qh_setequal_except` with both skips given).
pub(super) fn equal_except<T: PartialEq + Copy>(a: &[T], skip_a: T, b: &[T], skip_b: T) -> bool {
    if !a.contains(&skip_a) || !b.contains(&skip_b) || a.len() != b.len() {
        return false;
    }
    let left = a.iter().filter(|&&e| e != skip_a);
    let right = b.iter().filter(|&&e| e != skip_b);
    left.eq(right)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsorted_delete_moves_the_last_element() {
        let mut set = vec![1, 2, 3, 4];
        assert!(delete(&mut set, 2));
        assert_eq!(set, [1, 4, 3]);
        assert!(!delete(&mut set, 9));
        delete_sorted(&mut set, 1);
        assert_eq!(set, [4, 3]);
    }

    #[test]
    fn skips_and_replacements() {
        assert!(equal_except(&[5, 4, 2], 5, &[6, 4, 2], 6));
        assert!(!equal_except(&[5, 4, 2], 5, &[6, 4, 2], 4));
        assert_eq!(without_nth(&[5, 4, 3], 1), [5, 3]);
        let mut set = vec![1, 2, 2];
        replace(&mut set, 2, 7);
        assert_eq!(set, [1, 7, 2]);
        append_unique(&mut set, 1);
        append_unique(&mut set, 9);
        assert_eq!(set, [1, 7, 2, 9]);
    }
}
