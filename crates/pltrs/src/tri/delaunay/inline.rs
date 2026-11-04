//! Short sets kept inline: the vertices and neighbours of a simplicial
//! facet (three each) and the vertices of a ridge (two). qhull creates and
//! copies such sets for every facet; holding them inline spares the
//! allocator. Longer sets (those of merged facets) move to the heap.

use std::ops::{Deref, DerefMut};

/// Items held inline before a set moves to the heap.
const INLINE: usize = 3;

/// A sequence of `Copy` items, inline up to three of them.
#[derive(Clone, Debug)]
pub(super) enum InlineVec<T> {
    /// The first `len` items of `items` (the rest are fillers).
    Inline { len: u8, items: [T; INLINE] },
    /// More than fit inline (once moved, it stays on the heap).
    #[expect(
        clippy::box_collection,
        reason = "a pointer rather than a vector halves the many inline sets"
    )]
    Heap(Box<Vec<T>>),
}

impl<T: Copy + Default> InlineVec<T> {
    /// An empty sequence.
    pub(super) fn new() -> Self {
        Self::Inline {
            len: 0,
            items: [T::default(); INLINE],
        }
    }

    /// The items on the heap, moving them there first, with room for one
    /// more.
    fn heap(&mut self) -> &mut Vec<T> {
        if let Self::Inline { len, items } = self {
            let mut heap = Vec::with_capacity(INLINE + 1);
            heap.extend_from_slice(&items[..usize::from(*len)]);
            *self = Self::Heap(Box::new(heap));
        }
        match self {
            Self::Heap(heap) => heap,
            Self::Inline { .. } => unreachable!("moved to the heap above"),
        }
    }

    /// Appends `item`.
    pub(super) fn push(&mut self, item: T) {
        match self {
            Self::Inline { len, items } if usize::from(*len) < INLINE => {
                items[usize::from(*len)] = item;
                *len += 1;
            }
            _ => self.heap().push(item),
        }
    }

    /// Inserts `item` at `index`, shifting the later items up.
    ///
    /// # Panics
    ///
    /// When `index` is past the end, as `Vec::insert` does.
    pub(super) fn insert(&mut self, index: usize, item: T) {
        match self {
            Self::Inline { len, items } if usize::from(*len) < INLINE => {
                let end = usize::from(*len);
                assert!(index <= end, "insertion index {index} past {end} items");
                items.copy_within(index..end, index + 1);
                items[index] = item;
                *len += 1;
            }
            _ => self.heap().insert(index, item),
        }
    }

    /// Removes and returns the item at `index`, shifting the later items
    /// down.
    pub(super) fn remove(&mut self, index: usize) -> T {
        match self {
            Self::Inline { len, items } => {
                let item = items[..usize::from(*len)][index];
                items.copy_within(index + 1..usize::from(*len), index);
                *len -= 1;
                item
            }
            Self::Heap(heap) => heap.remove(index),
        }
    }

    /// Removes and returns the item at `index`, moving the last item into
    /// its place.
    pub(super) fn swap_remove(&mut self, index: usize) -> T {
        match self {
            Self::Inline { len, items } => {
                let last = usize::from(*len) - 1;
                let item = items[..=last][index];
                items[index] = items[last];
                *len -= 1;
                item
            }
            Self::Heap(heap) => heap.swap_remove(index),
        }
    }

    /// Removes every item.
    pub(super) fn clear(&mut self) {
        *self = Self::new();
    }

    /// Keeps the first `len` items (all of them when there are fewer).
    pub(super) fn truncate(&mut self, len: usize) {
        match self {
            Self::Inline { len: current, .. } => {
                *current = (*current).min(u8::try_from(len).unwrap_or(u8::MAX));
            }
            Self::Heap(heap) => heap.truncate(len),
        }
    }

    /// Truncates to `len` items, or appends copies of `value` up to `len`.
    pub(super) fn resize(&mut self, len: usize, value: T) {
        self.truncate(len);
        for _ in self.len()..len {
            self.push(value);
        }
    }
}

impl<T: Copy + Default> Default for InlineVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Deref for InlineVec<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        match self {
            Self::Inline { len, items } => &items[..usize::from(*len)],
            Self::Heap(heap) => heap,
        }
    }
}

impl<T> DerefMut for InlineVec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        match self {
            Self::Inline { len, items } => &mut items[..usize::from(*len)],
            Self::Heap(heap) => heap,
        }
    }
}

impl<T: PartialEq> PartialEq for InlineVec<T> {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl<T: Eq> Eq for InlineVec<T> {}

impl<T: Copy + Default> FromIterator<T> for InlineVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut set = Self::new();
        for item in iter {
            set.push(item);
        }
        set
    }
}

impl<T: Copy + Default> Extend<T> for InlineVec<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for item in iter {
            self.push(item);
        }
    }
}

impl<T: Copy> IntoIterator for InlineVec<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> IntoIter<T> {
        IntoIter { set: self, next: 0 }
    }
}

/// The items of an [`InlineVec`], by value.
#[derive(Debug)]
pub(super) struct IntoIter<T> {
    set: InlineVec<T>,
    next: usize,
}

impl<T: Copy> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        let item = self.set.get(self.next).copied()?;
        self.next += 1;
        Some(item)
    }
}

impl<'a, T> IntoIterator for &'a InlineVec<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// A sequence that takes the room of one pointer and no allocation while
/// empty: the coplanar points of a facet, which few facets ever have.
#[derive(Clone, Debug, Default)]
pub(super) struct LazyVec<T>(
    #[expect(
        clippy::box_collection,
        reason = "a pointer rather than a vector in every facet"
    )]
    Option<Box<Vec<T>>>,
);

impl<T> LazyVec<T> {
    /// The items, allocating room for them when there are none.
    fn items(&mut self) -> &mut Vec<T> {
        self.0.get_or_insert_with(Box::default)
    }

    /// Appends `item`.
    pub(super) fn push(&mut self, item: T) {
        self.items().push(item);
    }

    /// Inserts `item` at `index`, shifting the later items up.
    ///
    /// # Panics
    ///
    /// When `index` is past the end, as `Vec::insert` does.
    pub(super) fn insert(&mut self, index: usize, item: T) {
        self.items().insert(index, item);
    }
}

impl<T> Deref for LazyVec<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        self.0.as_deref().map_or(&[], Vec::as_slice)
    }
}

impl<'a, T> IntoIterator for &'a LazyVec<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn behaves_like_a_vec_inline_and_on_the_heap() {
        let mut set = InlineVec::new();
        let mut reference = Vec::new();
        for item in [5, 1, 4] {
            set.push(item);
            reference.push(item);
        }
        assert!(matches!(set, InlineVec::Inline { len: 3, .. }));
        set.insert(1, 9);
        reference.insert(1, 9);
        assert!(matches!(set, InlineVec::Heap(_)));
        assert_eq!(*set, *reference);
        assert_eq!(set.swap_remove(0), reference.swap_remove(0));
        assert_eq!(set.remove(1), reference.remove(1));
        assert_eq!(*set, *reference);
        set.clear();
        assert!(set.is_empty());
    }

    #[test]
    fn inline_edits_follow_vec() {
        let mut set: InlineVec<u32> = [7, 8].into_iter().collect();
        set.insert(0, 6);
        assert_eq!(*set, [6, 7, 8]);
        assert_eq!(set.remove(1), 7);
        assert_eq!(*set, [6, 8]);
        set.push(3);
        assert_eq!(set.swap_remove(0), 6);
        assert_eq!(*set, [3, 8]);
        assert_eq!(set, [3, 8].into_iter().collect());
        assert_ne!(set, [3, 8, 1, 2].into_iter().collect());
    }

    #[test]
    fn truncates_resizes_and_iterates() {
        let mut set: InlineVec<u32> = [1, 2, 3, 4].into_iter().collect();
        set.truncate(1);
        set.resize(3, 0);
        assert_eq!(set.clone().into_iter().collect::<Vec<_>>(), [1, 0, 0]);
        let mut inline: InlineVec<u32> = [1, 2].into_iter().collect();
        inline.truncate(5);
        inline.resize(3, 9);
        assert_eq!(*inline, [1, 2, 9]);
        inline.truncate(0);
        assert!(inline.into_iter().next().is_none());
    }

    #[test]
    fn lazy_sets_allocate_once_they_hold_something() {
        let mut set = LazyVec::default();
        assert!(set.is_empty() && set.0.is_none());
        set.push(4);
        set.insert(0, 2);
        assert_eq!(*set, [2, 4]);
        assert_eq!((&set).into_iter().sum::<i32>(), 6);
        assert_eq!(std::mem::size_of::<LazyVec<usize>>(), 8);
    }

    #[test]
    #[should_panic(expected = "past")]
    fn inserting_past_the_end_panics() {
        let mut set: InlineVec<u32> = std::iter::once(1).collect();
        set.insert(2, 0);
    }
}
