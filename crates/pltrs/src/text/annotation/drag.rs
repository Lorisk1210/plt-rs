//! Dragging annotations with the mouse (matplotlib's
//! `Annotation.draggable` and `DraggableAnnotation`): the text moves, the
//! annotated point stays.

use pltrs_core::Point;

use super::Annotation;
use crate::artist::{ArtistHandle, LayoutContext, RawHandle};
use crate::events::{DragTarget, Dragger};
use crate::figure::Figure;
use crate::text::TextArtist;

/// Where the dragged annotation's text was when the drag started, in
/// display pixels.
#[derive(Debug, Default)]
struct AnnotationDrag {
    origin: Option<Point>,
}

impl DragTarget for AnnotationDrag {
    fn save_offset(&mut self, annotation: &RawHandle) {
        let figure = annotation.figure_state();
        let mut state = figure.borrow_mut();
        state.prepare();
        let layout = LayoutContext::new(&state);
        self.origin = layout
            .store()
            .get_as::<TextArtist>(annotation.id())
            .and_then(|text| text.annotation.as_ref()?.text_display(text, layout).ok());
    }

    /// The text goes to its starting display position moved by the
    /// pointer, given back in its own coordinates.
    fn update_offset(&mut self, annotation: &RawHandle, dx: f64, dy: f64) {
        let Some(origin) = self.origin else {
            return;
        };
        let at = Point::new(origin.x + dx, origin.y + dy);
        let figure = annotation.figure_state();
        let mut state = figure.borrow_mut();
        state.prepare();
        let position = {
            let layout = LayoutContext::new(&state);
            layout
                .store()
                .get_as::<TextArtist>(annotation.id())
                .and_then(|text| text.annotation.as_ref()?.text_position_at(text, layout, at))
        };
        if let (Some(position), Some(text)) = (
            position,
            state.store.get_as_mut::<TextArtist>(annotation.id()),
        ) {
            text.position = position;
            text.position_replaced();
        }
        state.stale = true;
    }
}

impl Annotation {
    /// Lets the mouse drag the annotation's text, or stops it (matplotlib's
    /// `draggable(state)`): a press on the annotation picks it up and the
    /// text follows the pointer while the annotated point stays.
    pub fn set_draggable(&self, draggable: bool) -> &Self {
        let dragger = self.state(|a| a.dragger);
        match (draggable, dragger) {
            (true, None) => {
                let dragger = Dragger::connect(self.raw(), AnnotationDrag::default());
                self.update(|a| a.dragger = Some(dragger));
            }
            (false, Some(dragger)) => {
                dragger.disconnect(&Figure::root_of(self.raw().figure_state()));
                self.update(|a| a.dragger = None);
            }
            _ => {}
        }
        self
    }

    /// Whether the mouse can drag the annotation.
    pub fn draggable(&self) -> bool {
        self.state(|a| a.dragger.is_some())
    }
}
