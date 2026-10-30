//! Dragging offset boxes with the mouse: annotation boxes move while the
//! annotated point stays (matplotlib's `AnnotationBbox.draggable`), and
//! anchored boxes are pinned where they are dropped (matplotlib's
//! `DraggableOffsetBox`).

use pltrs_core::Point;

use super::anchored_box::AnchoredBoxArtist;
use super::annotation_bbox::AnnotationBboxArtist;
use super::{AnchoredOffsetbox, AnnotationBbox};
use crate::artist::{ArtistHandle, LayoutContext, RawHandle};
use crate::events::{DragTarget, Dragger};
use crate::figure::Figure;

/// Where the dragged box's anchor was when the drag started, in display
/// pixels.
#[derive(Debug, Default)]
struct BoxDrag {
    origin: Option<Point>,
}

impl DragTarget for BoxDrag {
    fn save_offset(&mut self, artist: &RawHandle) {
        let figure = artist.figure_state();
        let mut state = figure.borrow_mut();
        state.prepare();
        let layout = LayoutContext::new(&state);
        self.origin = layout
            .store()
            .get_as::<AnnotationBboxArtist>(artist.id())
            .and_then(|annotation| annotation.box_display(layout));
    }

    /// The box anchor goes to its starting display position moved by the
    /// pointer, given back in `boxcoords`. (matplotlib moves `xybox` by
    /// the pointer as if it were in pixels whatever `boxcoords` are.)
    fn update_offset(&mut self, artist: &RawHandle, dx: f64, dy: f64) {
        let Some(origin) = self.origin else {
            return;
        };
        let at = Point::new(origin.x + dx, origin.y + dy);
        let figure = artist.figure_state();
        let mut state = figure.borrow_mut();
        state.prepare();
        let xybox = {
            let layout = LayoutContext::new(&state);
            layout
                .store()
                .get_as::<AnnotationBboxArtist>(artist.id())
                .and_then(|annotation| annotation.xybox_at(layout, at))
        };
        if let (Some(xybox), Some(annotation)) = (
            xybox,
            state.store.get_as_mut::<AnnotationBboxArtist>(artist.id()),
        ) {
            annotation.set_xybox(xybox);
        }
        state.stale = true;
    }
}

impl AnnotationBbox {
    /// Lets the mouse drag the box, or stops it (matplotlib's
    /// `draggable(state)`): a press on the box's contents picks it up and
    /// the box follows the pointer while the annotated point stays.
    pub fn set_draggable(&self, draggable: bool) -> &Self {
        let dragger = self
            .raw()
            .with::<AnnotationBboxArtist, _>(|annotation| annotation.dragger);
        let set = |dragger| {
            self.raw()
                .with_mut::<AnnotationBboxArtist, _>(|annotation| annotation.dragger = dragger);
        };
        match (draggable, dragger) {
            (true, None) => set(Some(Dragger::connect(self.raw(), BoxDrag::default()))),
            (false, Some(dragger)) => {
                dragger.disconnect(&Figure::root_of(self.raw().figure_state()));
                set(None);
            }
            _ => {}
        }
        self
    }

    /// Whether the mouse can drag the box.
    pub fn draggable(&self) -> bool {
        self.raw()
            .with::<AnnotationBboxArtist, _>(|annotation| annotation.dragger.is_some())
    }
}

/// Where the dragged anchored box's origin was when the drag started.
#[derive(Debug, Default)]
struct PinDrag {
    origin: Option<Point>,
}

impl DragTarget for PinDrag {
    fn save_offset(&mut self, artist: &RawHandle) {
        let figure = artist.figure_state();
        let mut state = figure.borrow_mut();
        state.prepare();
        let layout = LayoutContext::new(&state);
        self.origin = layout
            .store()
            .get_as::<AnchoredBoxArtist>(artist.id())
            .and_then(|anchored| anchored.layout(layout).ok())
            .map(|laid_out| laid_out.offset);
    }

    /// The box is pinned at its starting offset moved by the pointer.
    fn update_offset(&mut self, artist: &RawHandle, dx: f64, dy: f64) {
        let Some(origin) = self.origin else {
            return;
        };
        let pinned = Point::new(origin.x + dx, origin.y + dy);
        artist.with_mut::<AnchoredBoxArtist, _>(|anchored| anchored.pinned = Some(pinned));
    }
}

impl AnchoredOffsetbox {
    /// Lets the mouse drag the box, or stops it (matplotlib's
    /// `DraggableOffsetBox(box, box)`): a press on the box's contents picks
    /// it up, and once moved the box stays at that place in the figure
    /// instead of following its anchor.
    pub fn set_draggable(&self, draggable: bool) -> &Self {
        let dragger = self
            .raw()
            .with::<AnchoredBoxArtist, _>(|anchored| anchored.dragger);
        let set = |dragger| {
            self.raw()
                .with_mut::<AnchoredBoxArtist, _>(|anchored| anchored.dragger = dragger);
        };
        match (draggable, dragger) {
            (true, None) => set(Some(Dragger::connect(self.raw(), PinDrag::default()))),
            (false, Some(dragger)) => {
                dragger.disconnect(&Figure::root_of(self.raw().figure_state()));
                set(None);
            }
            _ => {}
        }
        self
    }

    /// Whether the mouse can drag the box.
    pub fn draggable(&self) -> bool {
        self.raw()
            .with::<AnchoredBoxArtist, _>(|anchored| anchored.dragger.is_some())
    }
}

/// Connects the drag of `artist`, if it is an annotation box or an anchored box read back draggable from
/// an archive, to the callbacks of its figure: an archive keeps that the
/// artist is draggable, not the callbacks dragging it.
pub(crate) fn reconnect_drag(artist: &RawHandle) {
    if artist.is::<AnnotationBboxArtist>()
        && artist.with::<AnnotationBboxArtist, _>(|annotation| annotation.dragger.is_some())
    {
        let dragger = Dragger::connect(artist, BoxDrag::default());
        artist.with_mut::<AnnotationBboxArtist, _>(|annotation| annotation.dragger = Some(dragger));
    } else if artist.is::<AnchoredBoxArtist>()
        && artist.with::<AnchoredBoxArtist, _>(|anchored| anchored.dragger.is_some())
    {
        let dragger = Dragger::connect(artist, PinDrag::default());
        artist.with_mut::<AnchoredBoxArtist, _>(|anchored| anchored.dragger = Some(dragger));
    }
}
