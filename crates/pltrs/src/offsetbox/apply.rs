//! Moving the artists of laid-out offset-box trees into place, and the
//! figure's layout pass over the anchored, annotation and placed boxes.

use pltrs_core::{Affine2D, Bbox};

use super::owners::{owner, owner_mut};
use super::{AnnotationBboxArtist, ArtistRef, OffsetBox, OffsetBoxError, Placement, TreeLayout};
use crate::artist::{
    Artist, ArtistId, ArtistStore, ClipPathSpec, DrawError, LayoutContext, RawHandle,
};
use crate::figure::{FigureState, draws_before};
use crate::image::BboxImageArtist;
use crate::patches::{FancyBboxPatch, PatchArtist};
use crate::text::TextArtist;
use crate::transforms::Transform;

/// Moves every artist `placements` name to where they put it: texts and
/// area artists onto their transforms (and clips), images and frames onto
/// their boxes.
pub(crate) fn apply_placements<A: ArtistRef>(
    store: &mut ArtistStore,
    placements: Vec<Placement<A>>,
) {
    for placement in placements {
        match placement {
            Placement::Text {
                text,
                offset,
                baseline_shift,
            } => {
                if let Some(text) = store.get_as_mut::<TextArtist>(text.artist_id()) {
                    let placed =
                        Affine2D::translation(offset.x, offset.y).translate(0.0, baseline_shift);
                    text.base_mut().transform = Some(Transform::Affine(placed));
                }
            }
            Placement::Artists {
                artists,
                transform,
                clip,
            } => {
                for artist in artists {
                    artist.attach(store, transform.clone());
                    if let (Some(path), Some(clipped)) = (&clip, store.get_mut(artist.artist_id()))
                    {
                        clipped.base_mut().clip_path = Some(ClipPathSpec::Path {
                            path: path.clone(),
                            transform: transform.clone(),
                        });
                    }
                }
            }
            Placement::Image { image, bbox } => {
                if let Some(image) = store.get_as_mut::<BboxImageArtist>(image.artist_id()) {
                    image.bbox = Some(bbox);
                }
            }
            Placement::Frame {
                frame,
                bbox,
                mutation_scale,
            } => {
                if let Some(patch) =
                    store.get_as_mut::<PatchArtist<FancyBboxPatch>>(frame.artist_id())
                {
                    patch.shape.xy = bbox.p0;
                    patch.shape.width = bbox.width();
                    patch.shape.height = bbox.height();
                    if let Some(scale) = mutation_scale {
                        patch.shape.mutation_scale = scale;
                    }
                }
            }
            Placement::Program { artist, offset } => {
                super::program::place(store, artist.artist_id(), offset);
            }
        }
    }
}

/// Measures the artists of every aux-transform box of `tree` under their
/// aux transform alone (matplotlib's `AuxTransformBox.get_bbox`, which
/// clears the box's offset first).
fn measure_aux_boxes(state: &mut FigureState, tree: &mut OffsetBox<ArtistId>) {
    tree.for_each_aux_mut(&mut |aux| {
        for &id in &aux.artists {
            if let Some(artist) = state.store.get_mut(id) {
                artist.base_mut().transform = Some(aux.aux_transform.clone());
            }
        }
        let ctx = LayoutContext::new(state);
        let extent = aux
            .artists
            .iter()
            .filter_map(|&id| state.store.get(id)?.window_extent(&ctx))
            .reduce(|a, b| a.union(&b));
        aux.set_extent(extent.unwrap_or(Bbox::from_bounds(0.0, 0.0, 0.0, 0.0)));
    });
}

/// Changes the tree of `owner`, an added anchored, annotation or placed
/// box, with `f` (matplotlib's changes to the boxes of an added box, such
/// as the zoom of an offset image or the size of a drawing area); the box
/// becomes stale and is laid out again with the figure. `None` when
/// `owner` is no such box.
///
/// ```
/// use pltrs::artist::ArtistHandle;
/// use pltrs::offsetbox::{AnchorLoc, AnchoredBox, AnchoredOptions, DrawingArea, edit_tree};
/// use pltrs::prelude::*;
/// let fig = Figure::new();
/// let area = DrawingArea::new(10.0, 10.0, 0.0, 0.0);
/// let anchored = fig.subplot().add_anchored_box(AnchoredBox::new(area, AnchoredOptions::new(AnchorLoc::Center)));
/// let widened = edit_tree(anchored.raw(), |tree| tree.drawing_area_mut(0).map(|area| area.width = 40.0));
/// assert_eq!(widened, Some(Some(())));
/// assert!(anchored.raw().is_stale());
/// assert_eq!(edit_tree(fig.subplot().raw(), |_| ()), None, "an axes has no tree");
/// ```
pub fn edit_tree<R>(owner: &RawHandle, f: impl FnOnce(&mut OffsetBox<ArtistId>) -> R) -> Option<R> {
    let mut state = owner.figure_state().borrow_mut();
    let result = owner_mut(&mut state.store, owner.id()).map(|found| f(found.tree_mut()))?;
    state.mark_stale(owner.id());
    Some(result)
}

/// The artists the annotation box `id` is placed against that the figure
/// draws before it: matplotlib places the box as it draws, when their
/// arrows have just been drawn at the canvas resolution.
fn drawn_before(state: &FigureState, id: ArtistId) -> Vec<ArtistId> {
    let store = &state.store;
    store
        .get_as::<AnnotationBboxArtist>(id)
        .map(AnnotationBboxArtist::placing_artists)
        .unwrap_or_default()
        .into_iter()
        .filter(|&artist| {
            store.get(artist).is_some_and(|a| a.base().visible) && draws_before(store, artist, id)
        })
        .collect()
}

/// The layout of the tree of the anchored, annotation or placed box `id`
/// in the figure's state of now.
pub(super) fn tree_layout(
    state: &FigureState,
    id: ArtistId,
) -> Option<Result<TreeLayout<ArtistId>, OffsetBoxError>> {
    let drawn = drawn_before(state, id);
    let ctx = LayoutContext::new(state).with_drawn(&drawn);
    Some(owner(&state.store, id)?.lay_out_tree(ctx))
}

/// Lays out every anchored, annotation and placed box of the figure and
/// moves its parts into place; boxes that cannot be laid out are reported
/// with the next draw.
pub(crate) fn lay_out_boxes(state: &mut FigureState) {
    let boxes: Vec<ArtistId> = state
        .store
        .ids()
        .filter(|&id| owner(&state.store, id).is_some())
        .collect();
    for id in boxes {
        lay_out_box(state, id);
    }
}

/// Brings what the tree of the anchored, annotation or placed box `id`
/// measures of its artists up to date: the extents of its aux-transform
/// boxes and of its boxes of the program's.
pub(super) fn refresh_tree(state: &mut FigureState, id: ArtistId) {
    let Some(mut tree) = owner(&state.store, id).map(|found| found.tree().clone()) else {
        return;
    };
    measure_aux_boxes(state, &mut tree);
    super::program::sync_program_boxes(state, &mut tree);
    if let Some(found) = owner_mut(&mut state.store, id) {
        *found.tree_mut() = tree;
    }
}

/// Lays out the anchored, annotation or placed box `id` in the figure's
/// state of now and moves its parts into place; a box that cannot be laid
/// out is reported with the next draw.
pub(crate) fn lay_out_box(state: &mut FigureState, id: ArtistId) {
    refresh_tree(state, id);
    match tree_layout(state, id) {
        Some(Ok(layout)) => apply_placements(&mut state.store, layout.placements),
        Some(Err(err)) => state
            .pending_errors
            .push(DrawError::new(id, err.to_string())),
        None => {}
    }
}
