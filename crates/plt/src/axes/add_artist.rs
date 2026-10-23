//! Adding ready-made artists and collections to an axes (`add_artist`,
//! `add_collection`) and filtered groups of artists.

use super::Axes;
use super::children::DataUpdate;
use crate::artist::{
    AggFilter, Artist, ArtistHandle, Custom, FilteredGroup, FilteredGroupArtist, RawHandle,
};
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

    /// Adds an artist of your own (matplotlib's `add_artist` of an
    /// `Artist` subclass; see [`Custom`] for an example): in data
    /// coordinates unless its base has a transform, clipped to the axes
    /// unless it has a clip path, drawn with the other children in
    /// z-order, and leaving the data limits alone (extend them with
    /// [`Axes::update_datalim`]).
    pub fn add_custom_artist<A: Artist>(&self, mut artist: A) -> Custom<A> {
        let base = artist.base_mut();
        if base.transform.is_none() {
            base.transform = Some(self.trans_data());
        }
        Custom::from_raw(self.adopt(artist, &DataUpdate::default()))
    }

    /// Draws `members` a second time, together, through `filter` as one
    /// rasterized layer, at the z-order of the returned group (0 until
    /// changed): the agg filter example's `FilteredArtistList`, which
    /// draws glows behind texts and shadows behind groups of patches. The
    /// members are drawn on their own as well.
    ///
    /// ```
    /// use plt::artist::{AggFilter, ArtistHandle, Filtered};
    /// use plt::prelude::*;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// let line = ax.plot([0.0, 1.0], [0.0, 1.0])?;
    /// let shifted = AggFilter::new(|image, _dpi| Filtered { image: image.clone(), offset: (3.0, -3.0) });
    /// ax.add_filtered_group(&[line.raw().clone()], shifted).set_zorder(line.zorder() - 0.1);
    /// fig.to_png()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn add_filtered_group(&self, members: &[RawHandle], filter: AggFilter) -> FilteredGroup {
        let ids = members.iter().map(RawHandle::id).collect();
        let group = FilteredGroupArtist::new(ids, filter);
        FilteredGroup::from_raw(self.adopt(group, &DataUpdate::default()))
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
        let handle = if artist.in_autoscale {
            let update = DataUpdate::autoscaled(artist.limit_points(self.id()));
            self.adopt_collection(artist, &update)
        } else {
            self.adopt(artist, &DataUpdate::default())
        };
        Collection::from_raw(handle)
    }

    /// Adds a triangle mesh in data coordinates (matplotlib's
    /// `add_collection` with a `TriMesh`).
    pub fn add_trimesh(&self, spec: TriMeshSpec) -> TriMesh {
        let mut artist = spec.artist;
        artist.base_mut().transform = Some(self.trans_data());
        let update = DataUpdate::autoscaled(artist.limit_points(self.id()));
        TriMesh::from_raw(self.adopt_collection(artist, &update))
    }
}
