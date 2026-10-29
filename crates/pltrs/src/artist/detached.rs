//! The artists removed from a figure, kept so that their handles stay
//! usable.

use std::collections::BTreeMap;

use super::store::take_tree;
use super::{Artist, ArtistId};

/// The artists removed from a figure with
/// [`ArtistHandle::remove`](super::ArtistHandle::remove), together with
/// the artists they own, under the ids they had in the figure.
///
/// matplotlib keeps a removed artist as long as Python references it;
/// pltrs cannot tell when the last handle goes, so a removed artist lives
/// as long as its figure. The figure never looks here: what it draws, lays
/// out, searches and autoscales over is its live
/// [`ArtistStore`](super::ArtistStore). Handles look in both.
#[derive(Debug, Default)]
pub(crate) struct DetachedArtists {
    artists: BTreeMap<ArtistId, Box<dyn Artist>>,
}

impl DetachedArtists {
    /// Whether artist `id` was removed, itself or with an artist owning it.
    pub(crate) fn contains(&self, id: ArtistId) -> bool {
        self.artists.contains_key(&id)
    }

    /// The removed artist `id`.
    pub(crate) fn get(&self, id: ArtistId) -> Option<&dyn Artist> {
        self.artists.get(&id).map(AsRef::as_ref)
    }

    /// The removed artist `id`, mutably.
    pub(crate) fn get_mut(&mut self, id: ArtistId) -> Option<&mut (dyn Artist + 'static)> {
        self.artists.get_mut(&id).map(AsMut::as_mut)
    }

    /// Keeps `artist`, removed from the figure, under `id`.
    pub(crate) fn insert(&mut self, id: ArtistId, artist: Box<dyn Artist>) {
        self.artists.insert(id, artist);
    }

    /// Takes the removed artist `id` out.
    pub(crate) fn take(&mut self, id: ArtistId) -> Option<Box<dyn Artist>> {
        self.artists.remove(&id)
    }

    /// Takes the removed artist `id` and, recursively, every artist it
    /// owns out, the artist first.
    pub(crate) fn take_tree(&mut self, id: ArtistId) -> Vec<(ArtistId, Box<dyn Artist>)> {
        take_tree(id, |id| self.take(id))
    }
}
