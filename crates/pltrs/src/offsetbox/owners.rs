//! The artists that own an offset-box tree in a figure: anchored boxes,
//! annotation boxes and boxes placed at a point, which lay their tree out
//! and own its artists as their parts.

use super::TreeLayout;
use super::{AnchoredBoxArtist, AnnotationBboxArtist, OffsetBox, OffsetBoxError, PlacedBoxArtist};
use crate::artist::{ArtistId, ArtistStore, LayoutContext};

/// An artist owning an offset-box tree (see the module documentation).
pub(crate) trait BoxOwner {
    /// The tree.
    fn tree(&self) -> &OffsetBox<ArtistId>;

    /// The tree, mutably.
    fn tree_mut(&mut self) -> &mut OffsetBox<ArtistId>;

    /// The artists the box owns: those of its tree and its own frame.
    fn parts_mut(&mut self) -> &mut Vec<ArtistId>;

    /// Lays the tree out where the box puts it in the figure now: where
    /// its parts go (the box's frame first) and where its boxes' origins
    /// went.
    fn lay_out_tree(&self, ctx: LayoutContext<'_>) -> Result<TreeLayout<ArtistId>, OffsetBoxError>;
}

/// The owner of a tree that artist `id` is, if it is one.
pub(crate) fn owner(store: &ArtistStore, id: ArtistId) -> Option<&dyn BoxOwner> {
    if let Some(anchored) = store.get_as::<AnchoredBoxArtist>(id) {
        return Some(anchored);
    }
    if let Some(placed) = store.get_as::<PlacedBoxArtist>(id) {
        return Some(placed);
    }
    store
        .get_as::<AnnotationBboxArtist>(id)
        .map(|annotation| annotation as &dyn BoxOwner)
}

/// The owner of a tree that artist `id` is, mutably.
pub(crate) fn owner_mut(store: &mut ArtistStore, id: ArtistId) -> Option<&mut dyn BoxOwner> {
    if store.get_as::<AnchoredBoxArtist>(id).is_some() {
        return store
            .get_as_mut::<AnchoredBoxArtist>(id)
            .map(|anchored| anchored as &mut dyn BoxOwner);
    }
    if store.get_as::<PlacedBoxArtist>(id).is_some() {
        return store
            .get_as_mut::<PlacedBoxArtist>(id)
            .map(|placed| placed as &mut dyn BoxOwner);
    }
    store
        .get_as_mut::<AnnotationBboxArtist>(id)
        .map(|annotation| annotation as &mut dyn BoxOwner)
}
