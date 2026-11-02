//! Dragging legends with the mouse (matplotlib's `Legend.set_draggable`
//! and `DraggableLegend`).

use pltrs_core::{Bbox, Point};

use super::Legend;
use super::artist::{LegendArtist, LegendParent};
use super::settings::Location;
use crate::artist::{ArtistHandle, LayoutContext, RawHandle};
use crate::events::{DragTarget, Dragger};
use crate::figure::{Figure, FigureState};
use crate::transforms::Transform;

/// What dragging a legend changes (matplotlib's `DraggableLegend`
/// `update`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LegendDragUpdate {
    /// Its `loc`: the position of its lower-left corner in fractions of
    /// its anchor box (`"loc"`).
    #[default]
    Loc,
    /// Its `bbox_to_anchor`, in fractions of its parent, as matplotlib's
    /// `_update_bbox_to_anchor` computes it: the display position of the
    /// lower-left corner through the parent axes' `transAxes` (`"bbox"`;
    /// a figure legend, which has no axes, stays where it is).
    Bbox,
}

pltrs_core::archive_enum!(LegendDragUpdate { Loc, Bbox });

/// Where the dragged legend's lower-left corner started, and what the
/// drag changes.
#[derive(Debug, Default)]
struct LegendDrag {
    corner: Point,
    update: LegendDragUpdate,
}

impl LegendDrag {
    fn new(update: LegendDragUpdate) -> Self {
        Self {
            corner: Point::ORIGIN,
            update,
        }
    }

    /// Makes the corner at `corner` the legend's `loc` in its anchor box;
    /// an anchor box without area gives way to the default one
    /// (matplotlib's `_update_loc`).
    fn update_loc(state: &mut FigureState, legend: &RawHandle, corner: Point) {
        let anchor = |state: &FigureState| {
            let layout = LayoutContext::new(state);
            layout
                .store()
                .get_as::<LegendArtist>(legend.id())
                .and_then(|artist| artist.anchor_box(layout))
        };
        let degenerate = |bbox: &Bbox| bbox.width() == 0.0 || bbox.height() == 0.0;
        let mut bbox = anchor(state);
        if bbox.as_ref().is_some_and(degenerate) {
            if let Some(artist) = state.store.get_as_mut::<LegendArtist>(legend.id()) {
                artist.anchor = None;
            }
            bbox = anchor(state);
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
    }

    /// Anchors the legend at the corner at `corner` mapped through its
    /// parent axes' fractions (matplotlib's `_update_bbox_to_anchor`).
    fn update_bbox(state: &mut FigureState, legend: &RawHandle, corner: Point) {
        let Some(LegendParent::Axes(axes)) = state
            .store
            .get_as::<LegendArtist>(legend.id())
            .map(|artist| artist.parent)
        else {
            return;
        };
        let fractions = Transform::Axes(axes);
        let Some(point) = LayoutContext::new(state)
            .resolve(&fractions)
            .map(|map| map.transform_point(corner))
        else {
            return;
        };
        if let Some(artist) = state.store.get_as_mut::<LegendArtist>(legend.id()) {
            artist.anchor = Some((Bbox::from_bounds(point.x, point.y, 0.0, 0.0), fractions));
        }
    }
}

impl DragTarget for LegendDrag {
    fn save_offset(&mut self, legend: &RawHandle) {
        if let Some(extent) = legend.window_extent() {
            self.corner = extent.p0;
        }
    }

    /// The lower-left corner follows the pointer and becomes what the
    /// drag updates (matplotlib's `finalize_offset`, applied as the
    /// legend moves).
    fn update_offset(&mut self, legend: &RawHandle, dx: f64, dy: f64) {
        let corner = Point::new(self.corner.x + dx, self.corner.y + dy);
        let figure = legend.figure_state();
        let mut state = figure.borrow_mut();
        state.prepare();
        match self.update {
            LegendDragUpdate::Loc => Self::update_loc(&mut state, legend, corner),
            LegendDragUpdate::Bbox => Self::update_bbox(&mut state, legend, corner),
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
        self.set_draggable_by(draggable, LegendDragUpdate::Loc)
    }

    /// Lets the mouse drag the legend, changing what `update` says, or
    /// stops it (matplotlib's `set_draggable(state, update=...)`); a
    /// legend already draggable keeps what its drag changes, as in
    /// matplotlib.
    ///
    /// ```
    /// use pltrs::legend::{LegendDragUpdate, LegendOptions};
    /// use pltrs::lines::LineProps;
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.plot_with([0.0, 1.0], [0.0, 1.0], LineProps::default().label("line"))?;
    /// let legend = ax.legend(LegendOptions::default())?;
    /// legend.set_draggable_by(true, LegendDragUpdate::Bbox);
    /// assert_eq!(legend.drag_update(), Some(LegendDragUpdate::Bbox));
    /// legend.set_draggable(true);
    /// assert_eq!(legend.drag_update(), Some(LegendDragUpdate::Bbox));
    /// legend.set_draggable(false);
    /// assert_eq!(legend.drag_update(), None);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_draggable_by(&self, draggable: bool, update: LegendDragUpdate) -> &Self {
        let dragger = self.read(|legend| legend.dragger);
        match (draggable, dragger) {
            (true, None) => {
                let dragger = Dragger::connect(&self.raw, LegendDrag::new(update));
                self.raw.with_mut::<LegendArtist, _>(|legend| {
                    legend.dragger = Some(dragger);
                    legend.drag_update = update;
                });
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

    /// What dragging the legend changes, while it is draggable.
    pub fn drag_update(&self) -> Option<LegendDragUpdate> {
        self.read(|legend| legend.dragger.map(|_| legend.drag_update))
    }
}

/// Connects the drag of `artist`, if it is a legend read back draggable from
/// an archive, to the callbacks of its figure: an archive keeps that the
/// artist is draggable and what its drag changes, not the callbacks
/// dragging it.
pub(crate) fn reconnect_drag(artist: &RawHandle) {
    if artist.is::<LegendArtist>() && artist.with::<LegendArtist, _>(|l| l.dragger.is_some()) {
        let update = artist.with::<LegendArtist, _>(|legend| legend.drag_update);
        let dragger = Dragger::connect(artist, LegendDrag::new(update));
        artist.with_mut::<LegendArtist, _>(|legend| legend.dragger = Some(dragger));
    }
}
