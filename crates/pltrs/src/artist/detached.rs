//! The artists removed from a figure, kept so that their handles stay
//! usable.

use std::collections::BTreeMap;

use pltrs_core::archive::{Archive, ArchiveError, Decoder, Encoder};

use super::kept::KeptArtist;
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
    artists: BTreeMap<ArtistId, KeptArtist>,
    /// The removed artists whose trees stand in the figure's store while
    /// they are measured (see `figure::standing_in`), innermost last.
    standing: Vec<ArtistId>,
}

/// Archived artist by artist; each artist read back takes the id it is
/// kept under.
impl Archive for DetachedArtists {
    fn encode(&self, encoder: &mut Encoder<'_>) -> Result<(), ArchiveError> {
        self.artists.encode(encoder)
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, ArchiveError> {
        let mut artists: BTreeMap<ArtistId, KeptArtist> = Archive::decode(decoder)?;
        for (&id, artist) in &mut artists {
            artist.get_mut().base_mut().assign_id(id);
        }
        Ok(Self {
            artists,
            standing: Vec::new(),
        })
    }
}

impl DetachedArtists {
    /// Whether artist `id` was removed, itself or with an artist owning it.
    pub(crate) fn contains(&self, id: ArtistId) -> bool {
        self.artists.contains_key(&id)
    }

    /// The removed artist `id`.
    pub(crate) fn get(&self, id: ArtistId) -> Option<&dyn Artist> {
        self.artists.get(&id).map(KeptArtist::get)
    }

    /// The removed artist `id`, shared (see
    /// [`ArtistStore::shared`](super::ArtistStore::shared)).
    pub(crate) fn shared(&self, id: ArtistId) -> Option<std::rc::Rc<dyn Artist>> {
        self.artists.get(&id).map(KeptArtist::share)
    }

    /// The removed artist `id`, mutably.
    pub(crate) fn get_mut(&mut self, id: ArtistId) -> Option<&mut (dyn Artist + 'static)> {
        self.artists.get_mut(&id).map(KeptArtist::get_mut)
    }

    /// Keeps `artist`, removed from the figure, under `id`.
    pub(crate) fn insert(&mut self, id: ArtistId, artist: KeptArtist) {
        self.artists.insert(id, artist);
    }

    /// Takes the removed artist `id` out.
    pub(crate) fn take(&mut self, id: ArtistId) -> Option<KeptArtist> {
        self.artists.remove(&id)
    }

    /// Takes the removed artist `id` and, recursively, every artist it
    /// owns out, the artist first.
    pub(crate) fn take_tree(&mut self, id: ArtistId) -> Vec<(ArtistId, KeptArtist)> {
        take_tree(id, |id| self.take(id))
    }

    /// The ids of the removed artists, in creation order.
    pub(crate) fn ids(&self) -> impl Iterator<Item = ArtistId> + '_ {
        self.artists.keys().copied()
    }

    /// Notes that the tree of the removed artist `root` stands in the
    /// figure's store from now on.
    pub(crate) fn stand_in(&mut self, root: ArtistId) {
        self.standing.push(root);
    }

    /// Forgets that the tree of `root` stands in the figure's store;
    /// whether it did.
    pub(crate) fn stand_out(&mut self, root: ArtistId) -> bool {
        let at = self.standing.iter().rposition(|&standing| standing == root);
        at.map(|at| self.standing.remove(at)).is_some()
    }

    /// Whether the tree of the removed artist `root` stands in the
    /// figure's store.
    pub(crate) fn stands_in(&self, root: ArtistId) -> bool {
        self.standing.contains(&root)
    }

    /// Whether some removed artist has an id `live` (the figure's store)
    /// also holds (checked on artists read back from an archive).
    pub(crate) fn overlaps(&self, live: &super::ArtistStore) -> bool {
        self.artists.keys().any(|&id| live.contains(id))
    }
}
