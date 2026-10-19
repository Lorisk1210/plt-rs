//! Arena owning every artist of a figure.

use std::any::Any;
use std::collections::BTreeMap;
use std::fmt;

use super::Artist;

/// Identifies an artist within its figure's [`ArtistStore`].
///
/// Every insertion gets a new serial number, so once an artist is removed,
/// stale ids never alias a newer artist that reuses the slot; ids compare
/// in the order their artists were created.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArtistId {
    /// When the artist was inserted, counting from 0.
    serial: u64,
    /// Its slot in the store.
    index: u32,
}

impl fmt::Display for ArtistId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}v{}", self.index, self.serial)
    }
}

#[derive(Debug)]
struct Slot {
    /// The serial of the artist in the slot, or of the last one.
    serial: u64,
    artist: Option<Box<dyn Artist>>,
}

/// Owner of all artists in a figure, addressed by [`ArtistId`].
#[derive(Debug, Default)]
pub struct ArtistStore {
    slots: Vec<Slot>,
    free: Vec<u32>,
    /// The slots of the live artists by serial: the order the artists were
    /// created in, which slot order stops being once the slots of removed
    /// artists are reused.
    order: BTreeMap<u64, u32>,
    next_serial: u64,
}

impl ArtistStore {
    /// Takes ownership of `artist` and returns its id.
    pub fn insert(&mut self, mut artist: Box<dyn Artist>) -> ArtistId {
        let index = self.free.pop().unwrap_or_else(|| {
            let index = u32::try_from(self.slots.len()).expect("fewer than 2^32 artists");
            self.slots.push(Slot {
                serial: 0,
                artist: None,
            });
            index
        });
        let id = ArtistId {
            serial: self.next_serial,
            index,
        };
        self.next_serial += 1;
        artist.base_mut().assign_id(id);
        self.order.insert(id.serial, index);
        self.slots[index as usize] = Slot {
            serial: id.serial,
            artist: Some(artist),
        };
        id
    }

    /// Whether `id` refers to a live artist.
    pub fn contains(&self, id: ArtistId) -> bool {
        self.slot(id).is_some_and(|s| s.artist.is_some())
    }

    /// The artist with `id`, if it is alive.
    pub fn get(&self, id: ArtistId) -> Option<&dyn Artist> {
        self.slot(id)?.artist.as_deref()
    }

    /// Mutable access to the artist with `id`, if it is alive.
    pub fn get_mut(&mut self, id: ArtistId) -> Option<&mut (dyn Artist + 'static)> {
        self.slot_mut(id)?.artist.as_deref_mut()
    }

    /// The artist with `id` as concrete type `T`.
    pub fn get_as<T: Artist>(&self, id: ArtistId) -> Option<&T> {
        let artist: &dyn Any = self.get(id)?;
        artist.downcast_ref()
    }

    /// Mutable access to the artist with `id` as concrete type `T`.
    pub fn get_as_mut<T: Artist>(&mut self, id: ArtistId) -> Option<&mut T> {
        let artist: &mut dyn Any = self.get_mut(id)?;
        artist.downcast_mut()
    }

    /// Removes the artist and, recursively, every artist it owns.
    pub fn remove(&mut self, id: ArtistId) -> Option<Box<dyn Artist>> {
        let artist = self.slot_mut(id)?.artist.take()?;
        self.order.remove(&id.serial);
        self.free.push(id.index);
        for child in artist.children() {
            self.remove(child);
        }
        Some(artist)
    }

    /// Temporarily takes the artist out of the store so it can be mutated
    /// while the rest of the store is read or written.
    ///
    /// The artist must be returned with [`restore`](Self::restore); until
    /// then `id` behaves as if the artist were absent.
    pub fn detach(&mut self, id: ArtistId) -> Option<Box<dyn Artist>> {
        self.slot_mut(id)?.artist.take()
    }

    /// Puts back an artist taken out with [`detach`](Self::detach).
    pub fn restore(&mut self, id: ArtistId, artist: Box<dyn Artist>) {
        let slot = self
            .slot_mut(id)
            .expect("restore is only called with the id used to detach");
        debug_assert!(slot.artist.is_none(), "slot reoccupied while detached");
        slot.artist = Some(artist);
    }

    /// Runs `f` with mutable access to artist `id` (as `T`) and to the rest
    /// of the store. Returns `None` if the artist is missing or not a `T`.
    pub fn with_detached<T: Artist, R>(
        &mut self,
        id: ArtistId,
        f: impl FnOnce(&mut T, &mut Self) -> R,
    ) -> Option<R> {
        let mut artist = self.detach(id)?;
        let any: &mut dyn Any = artist.as_mut();
        let result = any.downcast_mut::<T>().map(|concrete| f(concrete, self));
        self.restore(id, artist);
        result
    }

    /// The ids of every live artist, in the order they were inserted.
    pub fn ids(&self) -> impl Iterator<Item = ArtistId> + '_ {
        self.order.iter().filter_map(|(&serial, &index)| {
            self.slots[index as usize]
                .artist
                .as_ref()
                .map(|_| ArtistId { serial, index })
        })
    }

    /// Number of live artists.
    pub fn len(&self) -> usize {
        self.slots.iter().filter(|s| s.artist.is_some()).count()
    }

    /// Returns `true` when the store holds no artists.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn slot(&self, id: ArtistId) -> Option<&Slot> {
        self.slots
            .get(id.index as usize)
            .filter(|s| s.serial == id.serial)
    }

    fn slot_mut(&mut self, id: ArtistId) -> Option<&mut Slot> {
        self.slots
            .get_mut(id.index as usize)
            .filter(|s| s.serial == id.serial)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::{ArtistBase, DrawContext};

    #[derive(Debug)]
    struct Node {
        base: ArtistBase,
        children: Vec<ArtistId>,
    }

    impl Node {
        fn boxed(children: Vec<ArtistId>) -> Box<dyn Artist> {
            Box::new(Self {
                base: ArtistBase::default(),
                children,
            })
        }
    }

    impl Artist for Node {
        fn base(&self) -> &ArtistBase {
            &self.base
        }
        fn base_mut(&mut self) -> &mut ArtistBase {
            &mut self.base
        }
        fn draw(&self, _ctx: &mut DrawContext<'_>) {}
        fn children(&self) -> Vec<ArtistId> {
            self.children.clone()
        }
    }

    #[test]
    fn stale_ids_do_not_alias_reused_slots() {
        let mut store = ArtistStore::default();
        let first = store.insert(Node::boxed(Vec::new()));
        assert_eq!(store.get(first).map(|a| a.base().id()), Some(first));
        store.remove(first);
        let second = store.insert(Node::boxed(Vec::new()));
        assert_ne!(first, second, "the reused slot gets a new serial");
        assert!(!store.contains(first));
        assert!(store.contains(second));
    }

    #[test]
    fn ids_follow_insertion_order_when_slots_are_reused() {
        let mut store = ArtistStore::default();
        let first = store.insert(Node::boxed(Vec::new()));
        let second = store.insert(Node::boxed(Vec::new()));
        store.remove(first);
        let third = store.insert(Node::boxed(Vec::new()));
        assert_eq!(store.ids().collect::<Vec<_>>(), [second, third]);
        assert!(second < third, "ids compare in creation order");
    }

    #[test]
    fn removal_is_recursive() {
        let mut store = ArtistStore::default();
        let leaf = store.insert(Node::boxed(Vec::new()));
        let middle = store.insert(Node::boxed(vec![leaf]));
        let root = store.insert(Node::boxed(vec![middle]));
        store.remove(middle);
        assert!(!store.contains(leaf));
        assert_eq!(store.len(), 1);
        assert!(store.contains(root));
    }

    #[test]
    fn detached_artists_can_mutate_the_rest_of_the_store() {
        let mut store = ArtistStore::default();
        let a = store.insert(Node::boxed(Vec::new()));
        let b = store.insert(Node::boxed(Vec::new()));
        let result = store.with_detached::<Node, _>(a, |node, rest| {
            assert!(!rest.contains(a), "the detached artist is absent meanwhile");
            rest.get_mut(b).expect("b is alive").base_mut().zorder = 7.0;
            node.base.zorder = 3.0;
        });
        assert!(result.is_some());
        assert_eq!(store.get(a).map(|n| n.base().zorder), Some(3.0));
        assert_eq!(store.get(b).map(|n| n.base().zorder), Some(7.0));
    }
}
