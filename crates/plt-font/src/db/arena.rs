//! An append-only list that grows through shared references. Items never
//! move or go away while the list lives, so references to them last as
//! long as the list: fonts can be added to a database already in use.

use std::sync::{Mutex, OnceLock, PoisonError};

/// Items in the first chunk; every further chunk is twice as large.
const FIRST: usize = 64;
/// Chunks: together more items than memory could hold.
const CHUNKS: usize = 48;

/// The chunk holding item `index` and the item's place in it.
fn locate(index: usize) -> (usize, usize) {
    let chunk = usize::try_from((index / FIRST + 1).ilog2()).expect("a bit count fits in usize");
    (chunk, index - FIRST * ((1 << chunk) - 1))
}

/// An append-only list (see the module documentation).
#[derive(Debug)]
pub(crate) struct Arena<T> {
    chunks: [OnceLock<Box<[OnceLock<T>]>>; CHUNKS],
    len: Mutex<usize>,
}

impl<T> Arena<T> {
    /// An empty list.
    pub(crate) fn new() -> Self {
        Self {
            chunks: [const { OnceLock::new() }; CHUNKS],
            len: Mutex::new(0),
        }
    }

    /// Appends `value` and returns its index.
    pub(crate) fn push(&self, value: T) -> usize {
        let mut len = self.len.lock().unwrap_or_else(PoisonError::into_inner);
        let index = *len;
        let (chunk, slot) = locate(index);
        let slots = self.chunks[chunk]
            .get_or_init(|| (0..FIRST << chunk).map(|_| OnceLock::new()).collect());
        // Indices only grow under the lock, so the slot is still empty.
        let _ = slots[slot].set(value);
        *len += 1;
        index
    }

    /// The item at `index`, if there is one.
    pub(crate) fn get(&self, index: usize) -> Option<&T> {
        let (chunk, slot) = locate(index);
        self.chunks.get(chunk)?.get()?.get(slot)?.get()
    }

    /// The number of items.
    pub(crate) fn len(&self) -> usize {
        *self.len.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The items in order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        (0..self.len()).filter_map(|index| self.get(index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indices_map_to_doubling_chunks() {
        assert_eq!(locate(0), (0, 0));
        assert_eq!(locate(63), (0, 63));
        assert_eq!(locate(64), (1, 0));
        assert_eq!(locate(191), (1, 127));
        assert_eq!(locate(192), (2, 0));
        assert!(locate(usize::MAX).0 >= CHUNKS, "out of range, not a panic");
    }

    #[test]
    fn items_stay_put_while_the_list_grows() {
        let arena = Arena::new();
        assert_eq!(arena.push("first".to_owned()), 0);
        let first: &String = arena.get(0).unwrap();
        for i in 1..500 {
            assert_eq!(arena.push(i.to_string()), i);
        }
        assert_eq!(first, "first");
        assert_eq!(arena.len(), 500);
        assert_eq!(arena.get(499).map(String::as_str), Some("499"));
        assert_eq!(arena.get(500), None);
        assert_eq!(arena.get(usize::MAX), None);
        assert_eq!(arena.iter().count(), 500);
    }
}
