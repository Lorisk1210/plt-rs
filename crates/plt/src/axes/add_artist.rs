//! Adding ready-made artists and collections to an axes (`add_artist`,
//! `add_collection`).

use super::Axes;
use super::children::DataUpdate;
use crate::artist::{Artist, ArtistHandle};
use crate::collections::{Collection, CollectionSpec, TriMesh, TriMeshSpec};
use crate::patches::{Patch, PatchArtist, PatchShape};
use plt_core::path::DataLimits;

/// The corners and smallest positive point of `limits`, in the directions
/// that are data values, requesting autoscaling there.
fn limits_update(limits: DataLimits, (x, y): (bool, bool)) -> DataUpdate {
    let points = if limits.bbox.is_null() {
        Vec::new()
    } else {
        vec![limits.bbox.p0, limits.bbox.p1, limits.minpos]
    };
    DataUpdate {
        points,
        update: (x, y),
        autoscale: (true, true),
    }
}

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
        let (limits, directions) = artist.data_limits(self.id());
        Collection::from_raw(self.adopt(artist, &limits_update(limits, directions)))
    }

    /// Adds a triangle mesh in data coordinates (matplotlib's
    /// `add_collection` with a `TriMesh`).
    pub fn add_trimesh(&self, spec: TriMeshSpec) -> TriMesh {
        let mut artist = spec.artist;
        artist.base_mut().transform = Some(self.trans_data());
        let limits = artist.data_limits();
        TriMesh::from_raw(self.adopt(artist, &limits_update(limits, (true, true))))
    }
}
