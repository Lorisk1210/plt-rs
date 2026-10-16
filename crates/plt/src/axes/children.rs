//! Adding artists to an axes and accounting for them in the data limits.

use std::rc::Rc;

use plt_core::Point;

use super::{Axes, AxesState};
use crate::artist::{Artist, ArtistHandle, RawHandle};

/// How a new child changes the data limits and autoscaling.
#[derive(Clone, Debug, Default)]
pub(crate) struct DataUpdate {
    /// Points to include, in data coordinates for the updated directions.
    pub(crate) points: Vec<Point>,
    /// Whether x and y data limits take the points into account.
    pub(crate) update: (bool, bool),
    /// Whether x and y view limits are re-autoscaled.
    pub(crate) autoscale: (bool, bool),
}

impl DataUpdate {
    /// Points in data coordinates that extend both limits and request
    /// autoscaling in both directions.
    pub(crate) fn data(points: Vec<Point>) -> Self {
        Self {
            points,
            update: (true, true),
            autoscale: (true, true),
        }
    }
}

impl Axes {
    /// Adds `artist` as a child clipped to the axes and applies `update`
    /// (matplotlib's `add_line`/`add_patch`/`add_collection` bookkeeping).
    pub(crate) fn adopt(&self, mut artist: impl Artist, update: &DataUpdate) -> RawHandle {
        let axes_id = self.id();
        artist.base_mut().clip_box = Some(axes_id);
        let figure = self.raw().figure_state();
        let mut state = figure.borrow_mut();
        let id = state.adopt(axes_id, Box::new(artist));
        if let Some(axes) = state.store.get_as_mut::<AxesState>(axes_id) {
            let (x, y) = update.update;
            axes.data_lim.update(&update.points, x, y);
            let (x, y) = update.autoscale;
            axes.request_autoscale(x, y);
        }
        RawHandle::new(Rc::clone(figure), id)
    }
}
