//! Moving the artists of laid-out offset-box trees into place, and the
//! figure's layout pass over the anchored, annotation and placed boxes.

use pltrs_core::Bbox;

use super::{
    AnchoredBoxArtist, AnnotationBboxArtist, ArtistRef, OffsetBox, PlacedBoxArtist, Placement,
};
use crate::artist::{Artist, ArtistId, ArtistStore, ClipPathSpec, DrawError, LayoutContext};
use crate::figure::{FigureState, draws_before};
use crate::image::BboxImageArtist;
use crate::patches::{FancyBboxPatch, PatchArtist};
use crate::text::TextArtist;
use crate::transforms::Transform;

/// Moves every artist `placements` name to where they put it: texts to
/// their anchors, area artists onto their transforms (and clips), images
/// and frames onto their boxes.
pub(crate) fn apply_placements<A: ArtistRef>(
    store: &mut ArtistStore,
    placements: Vec<Placement<A>>,
) {
    for placement in placements {
        match placement {
            Placement::Text { text, anchor } => {
                if let Some(text) = store.get_as_mut::<TextArtist>(text.artist_id()) {
                    text.position = anchor;
                    text.base_mut().transform = Some(Transform::Display);
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

/// The tree of the anchored, annotation or placed box `id`, if it is one.
fn tree_mut(store: &mut ArtistStore, id: ArtistId) -> Option<&mut OffsetBox<ArtistId>> {
    if store.get_as::<AnchoredBoxArtist>(id).is_some() {
        return store
            .get_as_mut::<AnchoredBoxArtist>(id)
            .map(|anchored| &mut anchored.child);
    }
    if store.get_as::<PlacedBoxArtist>(id).is_some() {
        return store
            .get_as_mut::<PlacedBoxArtist>(id)
            .map(|placed| &mut placed.child);
    }
    store
        .get_as_mut::<AnnotationBboxArtist>(id)
        .map(|annotation| &mut annotation.child)
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

/// Where the parts of the anchored, annotation or placed box `id` go.
fn placements(
    state: &FigureState,
    id: ArtistId,
) -> Option<Result<Vec<Placement<ArtistId>>, String>> {
    let drawn = drawn_before(state, id);
    let ctx = LayoutContext::new(state).with_drawn(&drawn);
    let store = &state.store;
    let laid_out = if let Some(anchored) = store.get_as::<AnchoredBoxArtist>(id) {
        anchored.layout(ctx).map(|layout| layout.placements)
    } else if let Some(placed) = store.get_as::<PlacedBoxArtist>(id) {
        placed.placements(ctx)
    } else {
        store.get_as::<AnnotationBboxArtist>(id)?.placements(ctx)
    };
    Some(laid_out.map_err(|err| err.to_string()))
}

/// Lays out every anchored, annotation and placed box of the figure and
/// moves its parts into place; boxes that cannot be laid out are reported
/// with the next draw.
pub(crate) fn lay_out_boxes(state: &mut FigureState) {
    let boxes: Vec<ArtistId> = state
        .store
        .ids()
        .filter(|&id| {
            state.store.get_as::<AnchoredBoxArtist>(id).is_some()
                || state.store.get_as::<AnnotationBboxArtist>(id).is_some()
                || state.store.get_as::<PlacedBoxArtist>(id).is_some()
        })
        .collect();
    for id in boxes {
        lay_out_box(state, id);
    }
}

/// Lays out the anchored, annotation or placed box `id` in the figure's
/// state of now and moves its parts into place; a box that cannot be laid
/// out is reported with the next draw.
pub(crate) fn lay_out_box(state: &mut FigureState, id: ArtistId) {
    let Some(mut tree) = tree_mut(&mut state.store, id).map(|tree| tree.clone()) else {
        return;
    };
    measure_aux_boxes(state, &mut tree);
    if let Some(stored) = tree_mut(&mut state.store, id) {
        *stored = tree;
    }
    match placements(state, id) {
        Some(Ok(placements)) => apply_placements(&mut state.store, placements),
        Some(Err(message)) => state.pending_errors.push(DrawError::new(id, message)),
        None => {}
    }
}
