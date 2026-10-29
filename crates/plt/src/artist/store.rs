//! Arena owning every artist of a figure.

use std::any::Any;
use std::fmt;

use super::Artist;

/// Identifies an artist within its figure's [`ArtistStore`].
///
/// Ids are handed out in the order the artists are created, and compare
/// in that order. An id is never given to another artist, not even once
/// its artist was removed from the figure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArtistId {
    /// The artist's slot in the store, which is also how many artists
    /// were created before it.
    index: u32,
}

impl ArtistId {
    /// The id as a number: how many artists the figure made before this
    /// one. Bindings that keep one object per artist, such as pltrs's
    /// Python classes, look their objects up by it.
    pub fn index(self) -> u32 {
        self.index
    }
}

impl fmt::Display for ArtistId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.index)
    }
}

/// Owner of the artists in a figure, addressed by [`ArtistId`].
///
/// Each artist has a slot of its own. An artist can be taken out of its
/// slot and put back; meanwhile the store behaves as if it were absent.
#[derive(Debug, Default)]
pub struct ArtistStore {
    slots: Vec<Option<Box<dyn Artist>>>,
}

impl ArtistStore {
    /// Takes ownership of `artist` and returns its id.
    pub fn insert(&mut self, mut artist: Box<dyn Artist>) -> ArtistId {
        let index = u32::try_from(self.slots.len()).expect("fewer than 2^32 artists");
        let id = ArtistId { index };
        artist.base_mut().assign_id(id);
        self.slots.push(Some(artist));
        id
    }

    /// Whether `id` refers to an artist in the store.
    pub fn contains(&self, id: ArtistId) -> bool {
        self.get(id).is_some()
    }

    /// The artist with `id`, if it is in the store.
    pub fn get(&self, id: ArtistId) -> Option<&dyn Artist> {
        self.slots.get(id.index as usize)?.as_deref()
    }

    /// Mutable access to the artist with `id`, if it is in the store.
    pub fn get_mut(&mut self, id: ArtistId) -> Option<&mut (dyn Artist + 'static)> {
        self.slots.get_mut(id.index as usize)?.as_deref_mut()
    }

    /// The artist with `id` as concrete type `T`.
    pub fn get_as<T: Artist>(&self, id: ArtistId) -> Option<&T> {
        downcast_ref(self.get(id)?)
    }

    /// Mutable access to the artist with `id` as concrete type `T`.
    pub fn get_as_mut<T: Artist>(&mut self, id: ArtistId) -> Option<&mut T> {
        downcast_mut(self.get_mut(id)?)
    }

    /// Takes the artist out of the store: until it is
    /// [put back](Self::put_back), `id` behaves as if the artist were
    /// absent.
    pub(crate) fn take(&mut self, id: ArtistId) -> Option<Box<dyn Artist>> {
        self.slots.get_mut(id.index as usize)?.take()
    }

    /// Puts back an artist taken out with [`take`](Self::take) or
    /// [`take_tree`](Self::take_tree).
    pub(crate) fn put_back(&mut self, id: ArtistId, artist: Box<dyn Artist>) {
        let slot = self
            .slots
            .get_mut(id.index as usize)
            .expect("artists are only put back under an id the store handed out");
        debug_assert!(slot.is_none(), "slot reoccupied while taken out");
        *slot = Some(artist);
    }

    /// Takes the artist and, recursively, every artist it owns out of the
    /// store (see [`take`](Self::take)), the artist first.
    pub(crate) fn take_tree(&mut self, id: ArtistId) -> Vec<(ArtistId, Box<dyn Artist>)> {
        take_tree(id, |id| self.take(id))
    }

    /// Runs `f` with mutable access to artist `id` (as `T`) and to the rest
    /// of the store. Returns `None` if the artist is missing or not a `T`.
    pub fn with_others<T: Artist, R>(
        &mut self,
        id: ArtistId,
        f: impl FnOnce(&mut T, &mut Self) -> R,
    ) -> Option<R> {
        let mut artist = self.take(id)?;
        let result = downcast_mut::<T>(artist.as_mut()).map(|concrete| f(concrete, self));
        self.put_back(id, artist);
        result
    }

    /// The ids of every artist in the store, in the order they were
    /// inserted.
    pub fn ids(&self) -> impl Iterator<Item = ArtistId> + '_ {
        (0..)
            .zip(&self.slots)
            .filter(|(_, slot)| slot.is_some())
            .map(|(index, _)| ArtistId { index })
    }

    /// Number of artists in the store.
    pub fn len(&self) -> usize {
        self.slots.iter().flatten().count()
    }

    /// Returns `true` when the store holds no artists.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Artists looked up by id: those in a figure (its [`ArtistStore`]), or
/// those of a figure, removed ones included (what handles see).
pub(crate) trait ArtistLookup {
    /// The artist with `id`, if there is one here.
    fn artist(&self, id: ArtistId) -> Option<&dyn Artist>;

    /// The artist with `id` as concrete type `T`, if there is one here.
    fn artist_as<T: Artist>(&self, id: ArtistId) -> Option<&T> {
        downcast_ref(self.artist(id)?)
    }
}

impl ArtistLookup for ArtistStore {
    fn artist(&self, id: ArtistId) -> Option<&dyn Artist> {
        self.get(id)
    }
}

/// Takes artist `root` and, recursively, every artist it owns out of a
/// collection of artists with `take`, the root first.
pub(crate) fn take_tree(
    root: ArtistId,
    mut take: impl FnMut(ArtistId) -> Option<Box<dyn Artist>>,
) -> Vec<(ArtistId, Box<dyn Artist>)> {
    let mut taken = Vec::new();
    let mut pending = vec![root];
    while let Some(id) = pending.pop() {
        if let Some(artist) = take(id) {
            pending.extend(artist.children());
            taken.push((id, artist));
        }
    }
    taken
}

/// `artist` as concrete type `T`, if it is one.
pub(crate) fn downcast_ref<T: Artist>(artist: &dyn Artist) -> Option<&T> {
    let artist: &dyn Any = artist;
    artist.downcast_ref()
}

/// `artist` as concrete type `T`, mutably, if it is one.
pub(crate) fn downcast_mut<T: Artist>(artist: &mut dyn Artist) -> Option<&mut T> {
    let artist: &mut dyn Any = artist;
    artist.downcast_mut()
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
    fn ids_are_never_reused() {
        let mut store = ArtistStore::default();
        let first = store.insert(Node::boxed(Vec::new()));
        assert_eq!(store.get(first).map(|a| a.base().id()), Some(first));
        let taken = store.take(first);
        let second = store.insert(Node::boxed(Vec::new()));
        assert_ne!(first, second, "a taken-out artist keeps its slot");
        assert!(!store.contains(first) && store.contains(second));
        store.put_back(first, taken.expect("first was in the store"));
        assert!(store.contains(first));
    }

    #[test]
    fn ids_follow_insertion_order() {
        let mut store = ArtistStore::default();
        let first = store.insert(Node::boxed(Vec::new()));
        let second = store.insert(Node::boxed(Vec::new()));
        let third = store.insert(Node::boxed(Vec::new()));
        store.take(second);
        assert_eq!(store.ids().collect::<Vec<_>>(), [first, third]);
        assert_eq!(store.len(), 2);
        assert!(
            first < second && second < third,
            "ids compare in creation order"
        );
    }

    #[test]
    fn trees_are_taken_out_whole() {
        let mut store = ArtistStore::default();
        let leaf = store.insert(Node::boxed(Vec::new()));
        let middle = store.insert(Node::boxed(vec![leaf]));
        let root = store.insert(Node::boxed(vec![middle]));
        let taken: Vec<ArtistId> = store
            .take_tree(middle)
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert_eq!(taken, [middle, leaf], "the artist first");
        assert!(!store.contains(leaf) && !store.contains(middle));
        assert_eq!(store.len(), 1);
        assert!(store.contains(root));
    }

    #[test]
    fn an_artist_can_mutate_the_rest_of_the_store() {
        let mut store = ArtistStore::default();
        let a = store.insert(Node::boxed(Vec::new()));
        let b = store.insert(Node::boxed(Vec::new()));
        let result = store.with_others::<Node, _>(a, |node, rest| {
            assert!(!rest.contains(a), "the artist is absent meanwhile");
            rest.get_mut(b)
                .expect("b is in the store")
                .base_mut()
                .zorder = 7.0;
            node.base.zorder = 3.0;
        });
        assert!(result.is_some());
        assert_eq!(store.get(a).map(|n| n.base().zorder), Some(3.0));
        assert_eq!(store.get(b).map(|n| n.base().zorder), Some(7.0));
    }
}
