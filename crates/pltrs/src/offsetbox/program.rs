//! Offset boxes of the program's: matplotlib's `OffsetBox` subclasses
//! that size and draw themselves (their `get_bbox(renderer)` and
//! `draw(renderer)`), as leaves of the tree of a box pltrs lays out.
//!
//! Such a leaf ([`ProgramBox`]) stands for an artist of its own
//! ([`ProgramBoxArtist`]) whose [painter](crate::artist::ArtistPainter)
//! does the program's work. The box holding the leaf lays it out with the
//! extent the painter gives ([`ArtistPainter::box_extent`], asked once the
//! figure is laid out, when the box's artists are where they are drawn,
//! after which the boxes holding such leaves are laid out again), places
//! the leaf's origin ([`ProgramOffsetBox::offset`], matplotlib's parent
//! calling the child's `set_offset`), and draws the leaf where its tree
//! draws it, through the painter (matplotlib's parent calling the child's
//! `draw`).
//!
//! [`ArtistPainter::box_extent`]: crate::artist::ArtistPainter::box_extent

use pltrs_core::{Bbox, Point};

use super::{ArtistRef, BoxItem, OffsetBox, lay_out_box};
use crate::artist::{
    Artist, ArtistBase, ArtistHandle, ArtistId, ArtistStore, AttachError, DrawContext,
    LayoutContext, RawHandle, TypedHandle,
};
use crate::figure::FigureState;

/// A leaf of an offset-box tree that the program sizes and draws
/// (matplotlib's `OffsetBox` subclass of the program's in a packer, a
/// padded box, an anchored box or an annotation box).
#[derive(Clone, Debug, PartialEq)]
pub struct ProgramBox<A> {
    /// The box's artist (a [`ProgramBoxArtist`] once the tree joined a
    /// figure), drawn by its painter.
    pub artist: A,
    /// The extent relative to the box's origin, in display pixels, as the
    /// box's artist was last measured.
    pub(super) extent: Bbox,
    /// Whether the box takes part in its parent's layout: the artist's
    /// visibility as the layout pass last read it.
    pub visible: bool,
}

pltrs_core::archive_struct!(ProgramBox<A> { artist, extent, visible });

impl<A> ProgramBox<A> {
    /// The same leaf referring to `f` of its artist.
    pub(super) fn map<B>(self, f: &mut impl FnMut(A) -> B) -> ProgramBox<B> {
        ProgramBox {
            artist: f(self.artist),
            extent: self.extent,
            visible: self.visible,
        }
    }

    /// The extent relative to the box's origin (matplotlib's `get_bbox`).
    pub(super) fn bbox(&self) -> Bbox {
        self.extent
    }
}

impl ProgramBox<BoxItem> {
    /// The leaf of the loose artist `artist` (made outside any figure, see
    /// [`RawHandle::is_loose`]), which becomes a [`ProgramBoxArtist`] with
    /// its properties, of no extent until the program measures it.
    ///
    /// ```
    /// use pltrs::artist::{ArtistHandle, Custom};
    /// use pltrs::offsetbox::{ProgramBox, ProgramBoxArtist};
    /// let artist = Custom::loose(ProgramBoxArtist::default());
    /// let leaf = ProgramBox::from_loose(artist.raw())?;
    /// assert!(leaf.visible);
    /// assert!(ProgramBox::from_loose(artist.raw()).is_err(), "moved once");
    /// # Ok::<(), pltrs::artist::AttachError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`AttachError::OtherFigure`] when the artist is not loose.
    pub fn from_loose(artist: &RawHandle) -> Result<Self, AttachError> {
        let item = BoxItem::from_loose(artist)?;
        let base = item.artist.base().clone();
        let visible = base.visible;
        Ok(Self {
            artist: BoxItem {
                artist: Box::new(ProgramBoxArtist {
                    base,
                    ..ProgramBoxArtist::default()
                }),
            },
            extent: Bbox::from_bounds(0.0, 0.0, 0.0, 0.0),
            visible,
        })
    }
}

/// The artist of an offset box of the program's: it draws nothing of its
/// own, its painter draws it.
#[derive(Clone, Debug)]
pub struct ProgramBoxArtist {
    base: ArtistBase,
    /// The extent relative to the box's origin, in display pixels.
    extent: Bbox,
    /// Where the box holding it placed the box's origin, in display
    /// pixels.
    offset: Point,
}

pltrs_core::archive_struct!(ProgramBoxArtist {
    base,
    extent,
    offset
});

impl Default for ProgramBoxArtist {
    /// A box of no extent at the display origin.
    fn default() -> Self {
        Self {
            base: ArtistBase::default(),
            extent: Bbox::from_bounds(0.0, 0.0, 0.0, 0.0),
            offset: Point::ORIGIN,
        }
    }
}

impl Artist for ProgramBoxArtist {
    fn base(&self) -> &ArtistBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ArtistBase {
        &mut self.base
    }

    /// Draws nothing: the artist's painter draws the box.
    fn draw(&self, _ctx: &mut DrawContext<'_>) {}

    /// The extent at the box's origin (matplotlib's `get_window_extent`
    /// of an `OffsetBox`).
    fn window_extent(&self, _ctx: &LayoutContext<'_>) -> Option<Bbox> {
        Some(self.extent.translated(self.offset.x, self.offset.y))
    }
}

/// A handle to an offset box of the program's (see the module
/// documentation).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgramOffsetBox {
    raw: RawHandle,
}

impl TypedHandle for ProgramOffsetBox {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        raw.is::<ProgramBoxArtist>()
            .then(|| Self { raw: raw.clone() })
    }
}

impl ArtistHandle for ProgramOffsetBox {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl ProgramOffsetBox {
    /// Where the box holding it last placed the box's origin, in display
    /// pixels (what matplotlib's parent gives the child's `set_offset`).
    pub fn offset(&self) -> Point {
        self.raw.with::<ProgramBoxArtist, _>(|artist| artist.offset)
    }

    /// The extent relative to the box's origin, in display pixels.
    pub fn extent(&self) -> Bbox {
        self.raw.with::<ProgramBoxArtist, _>(|artist| artist.extent)
    }

    /// Sizes the box (matplotlib's `get_bbox`) and lays the box holding
    /// it out again, in the figure as it is now.
    pub fn set_extent(&self, extent: Bbox) -> &Self {
        let mut state = self.raw.figure_state().borrow_mut();
        let id = self.raw.id();
        if let Some(owner) = set_extent(&mut state, id, extent) {
            lay_out_box(&mut state, owner);
        }
        self
    }
}

/// Records the extent of the program's box `id`; the box holding it.
fn set_extent(state: &mut FigureState, id: ArtistId, extent: Bbox) -> Option<ArtistId> {
    let artist = state.store.get_as_mut::<ProgramBoxArtist>(id)?;
    artist.extent = extent;
    artist.base().parent
}

/// Has the painters of the program's boxes of the figure `root` measure
/// them, once the figure is laid out and with no borrow of it held, then
/// lays the boxes holding them out again with their extents (see the
/// module documentation).
pub(crate) fn measure_program_boxes(root: &RawHandle) {
    let painted: Vec<_> = {
        let state = root.figure_state().borrow();
        state
            .store
            .ids()
            .filter(|&id| state.store.get_as::<ProgramBoxArtist>(id).is_some())
            .filter_map(|id| Some((id, state.store.get(id)?.base().painter.clone()?)))
            .collect()
    };
    let measured: Vec<_> = painted
        .into_iter()
        .filter_map(|(id, painter)| Some((id, painter.get().box_extent(&root.sibling(id))?)))
        .collect();
    let mut state = root.figure_state().borrow_mut();
    let mut owners: Vec<ArtistId> = measured
        .into_iter()
        .filter_map(|(id, extent)| set_extent(&mut state, id, extent))
        .collect();
    owners.sort_unstable();
    owners.dedup();
    for owner in owners {
        lay_out_box(&mut state, owner);
    }
}

/// Brings the program's boxes of `tree` up to date with their artists in
/// `state`: their extents and visibility.
pub(super) fn sync_program_boxes(state: &FigureState, tree: &mut OffsetBox<ArtistId>) {
    tree.for_each_program_mut(&mut |leaf| {
        if let Some(artist) = state
            .store
            .get_as::<ProgramBoxArtist>(leaf.artist.artist_id())
        {
            leaf.extent = artist.extent;
            leaf.visible = artist.base().visible;
        }
    });
}

/// Records where the box holding the program's box `artist` placed its
/// origin.
pub(super) fn place(store: &mut ArtistStore, artist: ArtistId, offset: Point) {
    if let Some(artist) = store.get_as_mut::<ProgramBoxArtist>(artist) {
        artist.offset = offset;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::{ArtistPainter, Custom, Painter};
    use crate::axes::Axes;
    use crate::figure::Figure;
    use crate::offsetbox::{
        AnchorLoc, AnchoredBox, AnchoredOffsetbox, AnchoredOptions, PackAlign, Packer, TextArea,
    };
    use crate::text::TextProps;

    /// Sizes its box as a fixed rectangle.
    #[derive(Debug)]
    struct Sized;

    /// The extent [`Sized`] gives.
    fn sized() -> Bbox {
        Bbox::from_bounds(-5.0, -2.0, 40.0, 20.0)
    }

    impl ArtistPainter for Sized {
        fn draw(&self, _ctx: &mut DrawContext<'_>, _artist: ArtistId) {}

        fn box_extent(&self, _artist: &RawHandle) -> Option<Bbox> {
            Some(sized())
        }
    }

    /// A leaf of the program's sized by [`Sized`].
    fn leaf() -> OffsetBox<BoxItem> {
        let loose = Custom::loose(ProgramBoxArtist::default());
        loose.raw().set_painter(Some(Painter::new(Sized)));
        OffsetBox::Program(ProgramBox::from_loose(loose.raw()).expect("a loose artist"))
    }

    /// `child` anchored without padding nor frame in the upper right
    /// corner of an axes of a 100-dpi figure.
    fn anchored(child: OffsetBox<BoxItem>) -> (Figure, Axes, AnchoredOffsetbox) {
        let fig = Figure::with_size(6.4, 4.8, 100.0);
        let ax = fig.subplot();
        let options = AnchoredOptions {
            pad: 0.0,
            borderpad: (0.0, 0.0),
            frameon: false,
            ..AnchoredOptions::new(AnchorLoc::UpperRight)
        };
        let anchored = ax.add_anchored_box(AnchoredBox::new(child, options));
        (fig, ax, anchored)
    }

    /// The program's box among the artists `anchored` draws.
    fn program_box(anchored: &AnchoredOffsetbox) -> ProgramOffsetBox {
        anchored
            .artists()
            .iter()
            .find_map(RawHandle::downcast)
            .expect("a program box")
    }

    #[test]
    fn parents_lay_out_program_boxes_at_their_measured_extent() {
        let (fig, ax, anchored) = anchored(leaf());
        fig.layout();
        let program = program_box(&anchored);
        assert_eq!(program.extent(), sized());
        let axes = ax.window_extent().expect("an axes");
        let offset = program.offset();
        assert!(
            (offset.x - (axes.x1() - 35.0)).abs() < 1e-9,
            "{offset:?} {axes:?}"
        );
        assert!(
            (offset.y - (axes.y1() - 18.0)).abs() < 1e-9,
            "{offset:?} {axes:?}"
        );
        let extent = program.window_extent().expect("placed");
        assert!((extent.x1() - axes.x1()).abs() < 1e-9 && (extent.y1() - axes.y1()).abs() < 1e-9);
    }

    #[test]
    fn hidden_program_boxes_leave_the_layout() {
        let text = TextArea::from_text("a", TextProps::default()).into();
        let packer = Packer::new(0.0, 0.0, PackAlign::Left, vec![text, leaf()]);
        let (fig, _ax, anchored) = anchored(OffsetBox::VPacker(packer));
        fig.layout();
        let shown = anchored.window_extent().expect("laid out");
        program_box(&anchored).set_visible(false);
        fig.layout();
        let hidden = anchored.window_extent().expect("laid out");
        assert!(hidden.height() < shown.height(), "{hidden:?} {shown:?}");
        assert_eq!(anchored.artists().len(), 1, "only the text is drawn");
    }

    #[test]
    fn set_extent_lays_the_holding_box_out_again() {
        let (_fig, _ax, anchored) = anchored(leaf());
        program_box(&anchored).set_extent(Bbox::from_bounds(0.0, 0.0, 100.0, 50.0));
        let extent = anchored.window_extent().expect("laid out");
        assert_eq!((extent.width(), extent.height()), (100.0, 50.0));
    }
}
