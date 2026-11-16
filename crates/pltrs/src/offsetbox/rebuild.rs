//! The boxes of an added box as the program has them now: matplotlib
//! reads an added box's boxes again at every draw, so a box measures and
//! places its children with the settings and the children they have then.
//! [`rebuild_tree`] hands pltrs a tree changed that way, and
//! [`node_extent`] and [`node_origin`] answer for any box inside it
//! (matplotlib's `get_bbox` and `get_offset` of a box inside another).

use std::rc::Rc;

use pltrs_core::{Bbox, Point};

use super::apply::{lay_out_box, refresh_tree, tree_layout};
use super::owners::{owner, owner_mut};
use super::pending::Parts;
use super::{BoxItem, OffsetBox, OffsetBoxError};
use crate::artist::{ArtistId, AttachError, LayoutContext, RawHandle};
use crate::figure::{FigureCell, FigureState};

/// An artist of a tree handed to [`rebuild_tree`].
enum Joining {
    /// An artist the box held already.
    Kept(ArtistId),
    /// A loose artist moving in.
    New(BoxItem),
}

/// The axes or (sub)figure artist `id` is, or is in: where the anchored
/// boxes of a tree it owns are anchored.
pub(super) fn container_of(state: &FigureState, id: ArtistId) -> ArtistId {
    let ctx = LayoutContext::new(state);
    let mut current = id;
    loop {
        if ctx.axes_extent(current).is_some() || ctx.frame_extent(current).is_some() {
            return current;
        }
        match state
            .store
            .get(current)
            .and_then(|artist| artist.base().parent)
        {
            Some(parent) => current = parent,
            None => return id,
        }
    }
}

/// Whether `owner_handle` is an added anchored, annotation or placed box
/// still in its figure, whose tree [`rebuild_tree`] replaces: not one
/// removed, alone or with the axes or box holding it.
///
/// ```
/// use pltrs::artist::ArtistHandle;
/// use pltrs::offsetbox::{AnchorLoc, AnchoredBox, AnchoredOptions, TextArea, holds_tree};
/// use pltrs::prelude::*;
/// use pltrs::text::TextProps;
/// let fig = Figure::new();
/// let axes = fig.subplot();
/// let text = TextArea::from_text("held", TextProps::default());
/// let anchored = axes.add_anchored_box(AnchoredBox::new(text, AnchoredOptions::new(AnchorLoc::Center)));
/// assert!(holds_tree(anchored.raw()));
/// axes.remove();
/// assert!(!holds_tree(anchored.raw()));
/// ```
#[must_use]
pub fn holds_tree(owner_handle: &RawHandle) -> bool {
    owner(
        &owner_handle.figure_state().borrow().store,
        owner_handle.id(),
    )
    .is_some()
}

/// The artists of the tree of `owner_handle`, an added anchored,
/// annotation or placed box, those of hidden boxes too, in the order of
/// [`OffsetBox::all_artists`] (that of [`rebuild_tree`]'s handles); `None`
/// when `owner_handle` is no such box. The handles' own `artists` list
/// only what the box draws.
///
/// ```
/// use pltrs::offsetbox::{
///     AnchorLoc, AnchoredBox, AnchoredOptions, OffsetBox, PackAlign, Packer, TextArea,
///     tree_artists,
/// };
/// use pltrs::prelude::*;
/// use pltrs::text::TextProps;
/// let fig = Figure::new();
/// let mut hidden = TextArea::from_text("hidden", TextProps::default());
/// hidden.visible = false;
/// let tree = OffsetBox::VPacker(Packer::new(0.0, 2.0, PackAlign::Left, vec![
///     TextArea::from_text("shown", TextProps::default()).into(),
///     hidden.into(),
/// ]));
/// let anchored = fig.subplot().add_anchored_box(AnchoredBox::new(tree, AnchoredOptions::new(AnchorLoc::Center)));
/// assert_eq!(anchored.artists().len(), 1, "drawn");
/// assert_eq!(tree_artists(anchored.raw()).map(|artists| artists.len()), Some(2));
/// ```
#[must_use]
pub fn tree_artists(owner_handle: &RawHandle) -> Option<Vec<RawHandle>> {
    let figure = owner_handle.figure_state();
    let ids: Vec<ArtistId> = owner(&figure.borrow().store, owner_handle.id())?
        .tree()
        .all_artists()
        .into_iter()
        .copied()
        .collect();
    Some(
        ids.into_iter()
            .map(|id| RawHandle::new(Rc::clone(figure), id))
            .collect(),
    )
}

/// Replaces the tree of `owner`, an added anchored, annotation or placed
/// box, with `tree`, whose artists are those the box holds or loose ones
/// (matplotlib's box drawn with the settings and the children it has
/// now): the artists the box held stay where they are in the figure, the
/// loose ones move in, and those the new tree leaves out leave the figure
/// (they stay usable apart from it, as removed artists do). The box
/// becomes stale and is laid out again. Returns the handles of the new
/// tree's artists in the order of [`OffsetBox::all_artists`]; `None` when
/// `owner` is no such box.
///
/// ```
/// use pltrs::artist::ArtistHandle;
/// use pltrs::offsetbox::{
///     AnchorLoc, AnchoredBox, AnchoredOptions, OffsetBox, PackAlign, Packer, TextArea,
///     rebuild_tree,
/// };
/// use pltrs::prelude::*;
/// use pltrs::text::{Text, TextProps};
/// let fig = Figure::new();
/// let text = TextArea::from_text("kept", TextProps::default());
/// let anchored = fig.subplot().add_anchored_box(AnchoredBox::new(text, AnchoredOptions::new(AnchorLoc::Center)));
/// let kept = anchored.texts()[0].raw().clone();
/// let added = Text::loose(0.0, 0.0, "added").raw().clone();
/// let tree = OffsetBox::VPacker(Packer::new(0.0, 4.0, PackAlign::Left, vec![
///     TextArea::new(kept.clone(), false).into(),
///     TextArea::new(added, false).into(),
/// ]));
/// let artists = rebuild_tree(anchored.raw(), tree).expect("an added box")?;
/// assert_eq!(artists[0], kept, "the text the box held stays");
/// assert_eq!(anchored.texts().len(), 2);
/// # Ok::<(), pltrs::artist::AttachError>(())
/// ```
///
/// # Errors
///
/// [`AttachError::OtherFigure`] when an artist of `tree` is neither loose
/// nor one the box holds; the box is left as it was.
pub fn rebuild_tree(
    owner_handle: &RawHandle,
    tree: OffsetBox<RawHandle>,
) -> Option<Result<Vec<RawHandle>, AttachError>> {
    // A removed box (or one only given its figure) is rebuilt in place,
    // its new parts going apart with it.
    let _standing = owner_handle.standing_in();
    let figure = owner_handle.figure_state();
    let held: Vec<ArtistId> = owner(&figure.borrow().store, owner_handle.id())?
        .tree()
        .all_artists()
        .into_iter()
        .copied()
        .collect();
    Some(rebuild(figure, owner_handle.id(), &held, tree))
}

/// [`rebuild_tree`] of the box `id` of `figure`, which holds `held`.
fn rebuild(
    figure: &Rc<FigureCell>,
    id: ArtistId,
    held: &[ArtistId],
    tree: OffsetBox<RawHandle>,
) -> Result<Vec<RawHandle>, AttachError> {
    let is_held = |artist: &RawHandle| {
        Rc::ptr_eq(artist.figure_state(), figure) && held.contains(&artist.id())
    };
    if tree
        .all_artists()
        .into_iter()
        .any(|artist| !is_held(artist) && !artist.is_loose())
    {
        return Err(AttachError::OtherFigure);
    }
    let joining = tree.try_map(&mut |artist| {
        if is_held(&artist) {
            Ok(Joining::Kept(artist.id()))
        } else {
            BoxItem::from_loose(&artist).map(Joining::New)
        }
    })?;
    let mut state = figure.borrow_mut();
    let container = container_of(&state, id);
    let mut parts = Parts::new(&mut state.store);
    let mut tree = joining.map(&mut |artist| match artist {
        Joining::Kept(id) => id,
        Joining::New(item) => parts.insert(item),
    });
    let added = parts.into_ids();
    tree.join_owner(container);
    super::program::hold(&mut state.store, &mut tree);
    for &part in &added {
        if let Some(artist) = state.store.get_mut(part) {
            artist.base_mut().parent = Some(id);
        }
    }
    let artists: Vec<ArtistId> = tree.all_artists().into_iter().copied().collect();
    if let Some(found) = owner_mut(&mut state.store, id) {
        *found.tree_mut() = tree;
        found.parts_mut().extend(&added);
    }
    for &dropped in held.iter().filter(|held| !artists.contains(held)) {
        state.remove_artist(dropped);
    }
    state.mark_stale(id);
    lay_out_box(&mut state, id);
    Ok(artists
        .into_iter()
        .map(|artist| RawHandle::new(Rc::clone(figure), artist))
        .collect())
}

/// The extent of the box `index` (in tree order, see [`OffsetBox::node`])
/// of the tree of `owner`, an added anchored, annotation or placed box,
/// relative to the box's origin, in display pixels (matplotlib's
/// `get_bbox` of a box inside another), in the figure as it is now.
/// `None` when `owner` is no such box or its tree has no box `index`.
///
/// ```
/// use pltrs::artist::ArtistHandle;
/// use pltrs::offsetbox::{AnchorLoc, AnchoredBox, AnchoredOptions, DrawingArea, node_extent};
/// use pltrs::prelude::*;
/// let fig = Figure::with_size(4.0, 3.0, 72.0);
/// let area = DrawingArea::new(10.0, 20.0, 2.0, 0.0);
/// let anchored = fig.subplot().add_anchored_box(AnchoredBox::new(area, AnchoredOptions::new(AnchorLoc::Center)));
/// let extent = node_extent(anchored.raw(), 0).expect("a box")?;
/// assert_eq!((extent.x0(), extent.width(), extent.height()), (-2.0, 10.0, 20.0));
/// assert!(node_extent(anchored.raw(), 1).is_none());
/// # Ok::<(), pltrs::offsetbox::OffsetBoxError>(())
/// ```
///
/// # Errors
///
/// When the box cannot be measured (a text of malformed math).
pub fn node_extent(owner_handle: &RawHandle, index: usize) -> Option<Result<Bbox, OffsetBoxError>> {
    let _standing = owner_handle.standing_in();
    let mut state = owner_handle.figure_state().borrow_mut();
    refresh_tree(&mut state, owner_handle.id());
    let ctx = LayoutContext::new(&state);
    let node = owner(&state.store, owner_handle.id())?.tree().node(index)?;
    Some(node.bbox(ctx))
}

/// Where the box `index` (in tree order) of the tree of `owner`, an added
/// anchored, annotation or placed box, has its origin, in display pixels
/// (matplotlib's `get_offset` of a box inside another, which its parent
/// set), its owner laid out in the figure as it is now: `None` inside when
/// its parent does not place it (a hidden box). `None` when `owner` is no
/// such box or its tree has no box `index`.
///
/// ```
/// use pltrs::artist::ArtistHandle;
/// use pltrs::offsetbox::{OffsetBox, TextArea, node_origin};
/// use pltrs::prelude::*;
/// use pltrs::transforms::Point;
/// use pltrs::text::TextProps;
/// let ax = Figure::new().subplot();
/// let placed = ax.add_offset_box(TextArea::from_text("t", TextProps::default()), (5.0, 6.0));
/// let origin = node_origin(placed.raw(), 0).expect("a box")?;
/// assert_eq!(origin, Some(Point::new(5.0, 6.0)));
/// # Ok::<(), pltrs::offsetbox::OffsetBoxError>(())
/// ```
///
/// # Errors
///
/// When the box cannot be laid out (its anchor is gone, a text of
/// malformed math).
pub fn node_origin(
    owner_handle: &RawHandle,
    index: usize,
) -> Option<Result<Option<Point>, OffsetBoxError>> {
    let _standing = owner_handle.standing_in();
    let mut state = owner_handle.figure_state().borrow_mut();
    refresh_tree(&mut state, owner_handle.id());
    if index >= owner(&state.store, owner_handle.id())?.tree().box_count() {
        return None;
    }
    let layout = tree_layout(&state, owner_handle.id())?;
    Some(layout.map(|layout| layout.origins.get(index).copied().flatten()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;
    use crate::offsetbox::{AnchorLoc, AnchoredBox, AnchoredOptions, PackAlign, Packer, TextArea};
    use crate::text::{Text, TextProps};

    /// A figure's anchored box of a vertical packer of `n` texts.
    fn packed(n: usize) -> (Figure, crate::offsetbox::AnchoredOffsetbox) {
        let fig = Figure::with_size(4.0, 3.0, 100.0);
        let texts = (0..n)
            .map(|i| TextArea::from_text(format!("text {i}"), TextProps::default()).into())
            .collect();
        let packer = OffsetBox::VPacker(Packer::new(0.0, 2.0, PackAlign::Left, texts));
        let options = AnchoredOptions::new(AnchorLoc::UpperRight);
        let anchored = fig
            .subplot()
            .add_anchored_box(AnchoredBox::new(packer, options));
        (fig, anchored)
    }

    /// The tree of `anchored` referring to its artists' handles.
    fn handles(anchored: &crate::offsetbox::AnchoredOffsetbox) -> OffsetBox<RawHandle> {
        let raw = anchored.raw().clone();
        let state = raw.figure_state().borrow();
        owner(&state.store, raw.id())
            .expect("an owner")
            .tree()
            .clone()
            .map(&mut |id| RawHandle::new(Rc::clone(raw.figure_state()), id))
    }

    #[test]
    fn dropped_artists_leave_and_settings_change() {
        let (fig, anchored) = packed(2);
        let before = anchored.window_extent().expect("laid out");
        let OffsetBox::VPacker(mut packer) = handles(&anchored) else {
            panic!("a vertical packer");
        };
        let dropped = packer.children.pop().expect("two texts");
        packer.sep = 30.0;
        packer
            .children
            .push(TextArea::new(Text::loose(0.0, 0.0, "new").raw().clone(), false).into());
        let artists = rebuild_tree(anchored.raw(), OffsetBox::VPacker(packer))
            .expect("an owner")
            .expect("valid");
        assert_eq!(artists.len(), 2);
        let OffsetBox::TextArea(TextArea { text: dropped, .. }) = dropped else {
            panic!("a text area");
        };
        assert!(!dropped.is_attached(), "the dropped text left the figure");
        assert_eq!(anchored.texts()[1].text(), "new");
        let after = anchored.window_extent().expect("laid out");
        assert!(
            after.height() > before.height() + 20.0,
            "{before:?} {after:?}"
        );
        assert!(fig.is_stale());
    }

    #[test]
    fn artists_of_other_figures_are_refused() {
        let (_fig, anchored) = packed(1);
        let (_other, elsewhere) = packed(1);
        let before = anchored.texts();
        let stray = handles(&elsewhere);
        assert_eq!(
            rebuild_tree(anchored.raw(), stray).expect("an owner").err(),
            Some(AttachError::OtherFigure)
        );
        assert_eq!(anchored.texts(), before, "left as it was");
        assert!(rebuild_tree(&anchored.texts()[0].raw().clone(), handles(&anchored)).is_none());
    }

    #[test]
    fn boxes_inside_report_their_extent_and_origin() {
        let (fig, anchored) = packed(2);
        fig.layout();
        let first = node_extent(anchored.raw(), 1)
            .expect("a box")
            .expect("measured");
        assert!(first.width() > 0.0 && first.y0() < 0.0, "{first:?}");
        let top = node_origin(anchored.raw(), 1)
            .expect("a box")
            .expect("laid out");
        let bottom = node_origin(anchored.raw(), 2)
            .expect("a box")
            .expect("laid out");
        let (top, bottom) = (top.expect("placed"), bottom.expect("placed"));
        assert!(top.y > bottom.y, "{top:?} {bottom:?}");
        assert!(node_origin(anchored.raw(), 3).is_none());
    }
}
