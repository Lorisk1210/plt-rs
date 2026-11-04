//! Offset boxes drawn with their origin at a point of the display
//! (matplotlib's `add_artist` of a plain `OffsetBox`, such as a
//! `PaddedBox`, which draws its children from its offset: `(0, 0)`, the
//! lower-left corner of the figure, unless set).

use std::rc::Rc;

use pltrs_core::{Bbox, Point};

use super::anchored_box::part;
use super::owners::BoxOwner;
use super::pending::{Parts, adopt_box};
use super::{BoxItem, OffsetBox, OffsetBoxError, TreeLayout};
use crate::artist::{
    Artist, ArtistBase, ArtistHandle, ArtistId, DrawContext, LayoutContext, RawHandle, TypedHandle,
};
use crate::axes::Axes;
use crate::figure::Figure;

/// State of a box placed at a display point: its child tree is made of
/// artists of the figure owned by the box; the layout pass places them.
#[derive(Clone, Debug)]
pub(crate) struct PlacedBoxArtist {
    base: ArtistBase,
    /// The tree (matplotlib's box itself, with its children).
    pub(crate) child: OffsetBox<ArtistId>,
    /// Where the tree's origin goes, in display pixels (matplotlib's
    /// `set_offset`).
    offset: Point,
    /// Every part, for removal with the box.
    parts: Vec<ArtistId>,
}

pltrs_core::archive_struct!(PlacedBoxArtist {
    base,
    child,
    offset,
    parts
});

impl BoxOwner for PlacedBoxArtist {
    fn tree(&self) -> &OffsetBox<ArtistId> {
        &self.child
    }

    fn tree_mut(&mut self) -> &mut OffsetBox<ArtistId> {
        &mut self.child
    }

    fn parts_mut(&mut self) -> &mut Vec<ArtistId> {
        &mut self.parts
    }

    /// The tree with its origin at the box's offset (matplotlib's
    /// `OffsetBox.draw`).
    fn lay_out_tree(&self, ctx: LayoutContext<'_>) -> Result<TreeLayout<ArtistId>, OffsetBoxError> {
        self.child.lay_out(ctx, self.offset)
    }
}

impl Artist for PlacedBoxArtist {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    /// Draws the tree's artists in box order, frames first.
    fn draw(&self, ctx: &mut DrawContext<'_>) {
        for &id in self.child.artists() {
            ctx.draw_child(id);
        }
    }

    fn children(&self) -> Vec<ArtistId> {
        self.parts.clone()
    }

    fn remove_child(&mut self, child: ArtistId) {
        self.parts.retain(|&id| id != child);
    }

    fn window_extent(&self, ctx: &LayoutContext<'_>) -> Option<Bbox> {
        self.child.window_extent(*ctx, self.offset).ok()
    }

    /// matplotlib's `OffsetBox.contains`: on an artist of the tree.
    fn contains(&self, ctx: &LayoutContext<'_>, point: Point) -> bool {
        self.child
            .artists()
            .into_iter()
            .any(|&id| ctx.store().get(id).is_some_and(|a| a.contains(ctx, point)))
    }
}

/// A handle to a box drawn at a display point (matplotlib's plain
/// `OffsetBox` added to an axes or a figure).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacedOffsetBox {
    raw: RawHandle,
}

impl TypedHandle for PlacedOffsetBox {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        raw.is::<PlacedBoxArtist>()
            .then(|| Self { raw: raw.clone() })
    }
}

impl ArtistHandle for PlacedOffsetBox {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl PlacedOffsetBox {
    /// Where the box's origin is, in display pixels (matplotlib's
    /// `get_offset`).
    pub fn offset(&self) -> Point {
        self.raw.with::<PlacedBoxArtist, _>(|placed| placed.offset)
    }

    /// Moves the box's origin to `offset`, in display pixels (matplotlib's
    /// `set_offset`), and lays the box out there in the figure as it is
    /// now (matplotlib's parent sets a child's offset right before it draws
    /// the child).
    pub fn set_offset(&self, offset: impl Into<Point>) -> &Self {
        let offset = offset.into();
        self.raw
            .with_mut::<PlacedBoxArtist, _>(|placed| placed.offset = offset);
        super::lay_out_box(&mut self.raw.figure_state().borrow_mut(), self.raw.id());
        self
    }

    /// The artists the box draws in drawing order, frames before what
    /// they frame.
    pub fn artists(&self) -> Vec<RawHandle> {
        let ids: Vec<ArtistId> = self.raw.with::<PlacedBoxArtist, _>(|placed| {
            placed.child.artists().into_iter().copied().collect()
        });
        ids.into_iter().map(|id| part(&self.raw, id)).collect()
    }

    /// Clips the artists of the drawing area at `area` among those of the
    /// box to its bounds, or stops clipping them, as
    /// [`AnchoredOffsetbox::set_clip_children`] does; returns whether the
    /// box has such an area.
    ///
    /// [`AnchoredOffsetbox::set_clip_children`]: super::AnchoredOffsetbox::set_clip_children
    pub fn set_clip_children(&self, area: usize, clip: bool) -> bool {
        self.raw.with_mut::<PlacedBoxArtist, _>(|placed| {
            placed
                .child
                .drawing_area_mut(area)
                .map(|found| found.clip = clip)
                .is_some()
        })
    }
}

impl Axes {
    /// Adds the box `child` with its origin at `offset`, in display pixels
    /// (matplotlib's `add_artist` of a plain `OffsetBox` such as a
    /// `PaddedBox`). Anchored boxes inside it are drawn at their anchors
    /// in the axes.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::offsetbox::{PaddedBox, TextArea};
    /// use pltrs::patches::PatchProps;
    /// use pltrs::text::TextProps;
    /// let ax = Figure::new().subplot();
    /// let text = TextArea::from_text("corner", TextProps::default());
    /// let framed = PaddedBox::new(text, 2.0, true, PatchProps::default());
    /// let placed = ax.add_offset_box(framed, (10.0, 10.0));
    /// assert_eq!(placed.artists().len(), 2);
    /// assert!(placed.window_extent().is_some_and(|extent| extent.x0() < 10.0));
    /// ```
    pub fn add_offset_box(
        &self,
        child: impl Into<OffsetBox<BoxItem>>,
        offset: impl Into<Point>,
    ) -> PlacedOffsetBox {
        add_placed(self.raw(), child.into(), offset.into())
    }
}

impl Figure {
    /// Adds the box `child` to the figure with its origin at `offset`, in
    /// display pixels (matplotlib's `fig.add_artist` of a plain
    /// `OffsetBox`), as [`Axes::add_offset_box`] does.
    pub fn add_offset_box(
        &self,
        child: impl Into<OffsetBox<BoxItem>>,
        offset: impl Into<Point>,
    ) -> PlacedOffsetBox {
        add_placed(self.raw(), child.into(), offset.into())
    }
}

/// Adds the box `child` at `offset` to `owner`: an axes or a (sub)figure,
/// or a box of the program's, which then owns it (see
/// [`ProgramOffsetBox::add_box`](super::ProgramOffsetBox::add_box)). Its
/// anchored boxes are anchored in the axes or figure it is in.
pub(super) fn add_placed(
    owner: &RawHandle,
    child: OffsetBox<BoxItem>,
    offset: Point,
) -> PlacedOffsetBox {
    let figure = owner.figure_state();
    let mut state = figure.borrow_mut();
    let container = super::rebuild::container_of(&state, owner.id());
    let mut parts = Parts::new(&mut state.store);
    let child = parts.insert_tree(child, container);
    let parts = parts.into_ids();
    let artist = PlacedBoxArtist {
        base: ArtistBase::default(),
        child,
        offset,
        parts: parts.clone(),
    };
    let id = adopt_box(&mut state, owner.id(), Box::new(artist), &parts);
    drop(state);
    PlacedOffsetBox {
        raw: RawHandle::new(Rc::clone(figure), id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::offsetbox::{AnchorLoc, AnchoredBox, AnchoredOptions, PaddedBox};
    use crate::patches::{FancyBboxPatch, Patch, PatchProps};
    use crate::testing::{RecordingRenderer, close, data_axes};
    use crate::text::TextProps;

    #[test]
    fn a_padded_box_frames_an_anchored_box_at_its_offset() {
        let (fig, ax) = data_axes();
        // The paddings follow the text, 10 points by default.
        let options = AnchoredOptions::new(AnchorLoc::UpperLeft);
        let anchored =
            AnchoredBox::text("foo", TextProps::default(), options).expect("no alignment");
        let red = PatchProps::default().facecolor("r");
        let placed = ax.add_offset_box(PaddedBox::new(anchored, 0.0, true, red), (0.0, 0.0));
        let mut renderer = RecordingRenderer::new(640.0, 480.0, 100.0);
        fig.draw(&mut renderer).expect("drawable");
        let artists = placed.artists();
        assert_eq!(artists.len(), 3, "two frames, then the text");
        let frame = |index: usize| {
            Patch::<FancyBboxPatch>::from_raw(artists[index].clone())
                .window_extent()
                .expect("an extent")
        };
        let (outer, inner) = (frame(0), frame(1));
        // The padded box sits at the origin around the anchored box's
        // extent; the anchored box itself is drawn at its anchor.
        assert!(outer.x0() < 0.0 && outer.x1() > 0.0, "{outer:?}");
        assert!(close(inner.width(), outer.width()));
        let pad = 0.5 * 10.0 * 100.0 / 72.0;
        assert!(close(inner.x0(), 80.0 + pad), "{inner:?}");
        assert!(close(inner.y1(), 52.8 + 369.6 - pad));
        assert_eq!(placed.window_extent(), Some(outer));
        placed.set_offset((100.0, 50.0));
        assert_eq!(placed.offset(), Point::new(100.0, 50.0));
    }
}
