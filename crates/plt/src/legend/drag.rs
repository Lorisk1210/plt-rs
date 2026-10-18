//! Dragging legends with the mouse (matplotlib's `Legend.set_draggable`
//! and `DraggableLegend` with `update="loc"`).

use plt_core::{Bbox, Point};

use super::Legend;
use super::artist::LegendArtist;
use super::settings::Location;
use crate::artist::{ArtistHandle, LayoutContext, RawHandle};
use crate::events::{DragTarget, Dragger};
use crate::figure::Figure;

/// Where the dragged legend's lower-left corner started.
#[derive(Debug, Default)]
struct LegendDrag {
    corner: Point,
}

impl DragTarget for LegendDrag {
    fn save_offset(&mut self, legend: &RawHandle) {
        if let Some(extent) = legend.window_extent() {
            self.corner = extent.p0;
        }
    }

    /// The lower-left corner follows the pointer and becomes the legend's
    /// `loc`, in fractions of its anchor box; an anchor box without area
    /// gives way to the default one (matplotlib's `_update_loc`, applied
    /// as the legend moves).
    fn update_offset(&mut self, legend: &RawHandle, dx: f64, dy: f64) {
        let corner = Point::new(self.corner.x + dx, self.corner.y + dy);
        let figure = legend.figure_state();
        let mut state = figure.borrow_mut();
        state.prepare();
        let anchor = |state: &crate::figure::FigureState| {
            let layout = LayoutContext::new(state);
            layout
                .store()
                .get_as::<LegendArtist>(legend.id())
                .and_then(|artist| artist.anchor_box(layout))
        };
        let degenerate = |bbox: &Bbox| bbox.width() == 0.0 || bbox.height() == 0.0;
        let mut bbox = anchor(&state);
        if bbox.as_ref().is_some_and(degenerate) {
            if let Some(artist) = state.store.get_as_mut::<LegendArtist>(legend.id()) {
                artist.anchor = None;
            }
            bbox = anchor(&state);
        }
        let Some(bbox) = bbox else {
            return;
        };
        if let Some(artist) = state.store.get_as_mut::<LegendArtist>(legend.id()) {
            artist.location = Location::Point(
                (corner.x - bbox.x0()) / bbox.width(),
                (corner.y - bbox.y0()) / bbox.height(),
            );
        }
        state.stale = true;
    }
}

impl Legend {
    /// Lets the mouse drag the legend, or stops it (matplotlib's
    /// `set_draggable(state)` with `update="loc"`): a press on the legend
    /// picks it up, and its `loc` becomes the position of its lower-left
    /// corner in its anchor box as it moves.
    pub fn set_draggable(&self, draggable: bool) -> &Self {
        let dragger = self.read(|legend| legend.dragger);
        match (draggable, dragger) {
            (true, None) => {
                let dragger = Dragger::connect(&self.raw, LegendDrag::default());
                self.raw
                    .with_mut::<LegendArtist, _>(|legend| legend.dragger = Some(dragger));
            }
            (false, Some(dragger)) => {
                dragger.disconnect(&Figure::root_of(self.raw.figure_state()));
                self.raw
                    .with_mut::<LegendArtist, _>(|legend| legend.dragger = None);
            }
            _ => {}
        }
        self
    }

    /// Whether the mouse can drag the legend (matplotlib's
    /// `get_draggable`).
    pub fn draggable(&self) -> bool {
        self.read(|legend| legend.dragger.is_some())
    }
}
