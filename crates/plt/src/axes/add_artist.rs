//! Adding ready-made artists and collections to an axes (`add_artist`,
//! `add_collection`).

use super::Axes;
use super::children::DataUpdate;
use crate::artist::{Artist, ArtistHandle};
use crate::collections::{Collection, CollectionSpec, TriMesh, TriMeshSpec};
use crate::patches::{Patch, PatchArtist, PatchShape};

impl Axes {
    /// Adds a patch in data coordinates (unless its shape chose others),
    /// clipped to the axes unless the shape turns clipping off, without
    /// touching the data limits (matplotlib's `add_artist`).
    pub fn add_artist<S: PatchShape>(&self, shape: S) -> Patch<S> {
        let mut patch = PatchArtist::new(shape, crate::rc::params());
        let base = patch.base_mut();
        if base.transform.is_none() {
            base.transform = Some(self.trans_data());
        }
        Patch::from_raw(self.adopt(patch, &DataUpdate::default()))
    }

    /// Adds a collection, in data coordinates unless it is drawn in display
    /// space at offsets, clipped to the axes, and extends the data limits
    /// and autoscales to cover it (matplotlib's `add_collection`).
    pub fn add_collection(&self, spec: CollectionSpec) -> Collection {
        let mut artist = spec.artist;
        let base = artist.base_mut();
        if base.transform.is_none() {
            base.transform = Some(self.trans_data());
        }
        let update = DataUpdate::autoscaled(artist.limit_points(self.id()));
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
