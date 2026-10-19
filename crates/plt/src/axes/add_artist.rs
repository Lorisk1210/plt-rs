//! Adding ready-made artists and collections to an axes (`add_artist`,
//! `add_collection`).

use super::Axes;
use super::children::DataUpdate;
use crate::artist::{Artist, ArtistHandle};
use crate::collections::{Collection, CollectionSpec, TriMesh, TriMeshSpec};
use crate::patches::{Patch, PatchArtist, PatchProps, PatchShape};

impl Axes {
    /// Adds a patch in data coordinates (unless its shape chose others),
    /// clipped to the axes unless the shape turns clipping off, without
    /// touching the data limits (matplotlib's `add_artist`).
    pub fn add_artist<S: PatchShape>(&self, shape: S) -> Patch<S> {
        self.add_artist_with(shape, PatchProps::default())
    }

    /// [`add_artist`](Self::add_artist) for a patch styled by `props`
    /// before it is added, as matplotlib adds a patch made with its
    /// keyword arguments: a `clip_path` replaces the axes clip box.
    pub fn add_artist_with<S: PatchShape>(&self, shape: S, props: PatchProps) -> Patch<S> {
        let mut patch = PatchArtist::new(shape, crate::rc::params());
        props.apply(&mut patch);
        let base = patch.base_mut();
        if base.transform.is_none() {
            base.transform = Some(self.trans_data());
        }
        Patch::from_raw(self.adopt(patch, &DataUpdate::default()))
    }

    /// Adds a collection, in data coordinates unless it is drawn in display
    /// space at offsets, clipped to the axes, and extends the data limits
    /// and autoscales to cover it (matplotlib's `add_collection`) unless it
    /// was made [`without_autolim`](CollectionSpec::without_autolim).
    pub fn add_collection(&self, spec: CollectionSpec) -> Collection {
        let mut artist = spec.artist;
        let base = artist.base_mut();
        if base.transform.is_none() {
            base.transform = Some(self.trans_data());
        }
        let update = if artist.in_autoscale {
            DataUpdate::autoscaled(artist.limit_points(self.id()))
        } else {
            DataUpdate::default()
        };
        Collection::from_raw(self.adopt(artist, &update))
    }

    /// Adds a triangle mesh in data coordinates (matplotlib's
    /// `add_collection` with a `TriMesh`).
    pub fn add_trimesh(&self, spec: TriMeshSpec) -> TriMesh {
        let mut artist = spec.artist;
        artist.base_mut().transform = Some(self.trans_data());
        let update = DataUpdate::autoscaled(artist.limit_points(self.id()));
        TriMesh::from_raw(self.adopt(artist, &update))
    }
}
