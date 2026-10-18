//! Adding artists to an axes and accounting for them in the data limits.

use std::rc::Rc;

use plt_core::Point;

use super::{Axes, AxesState, DataLimits};
use crate::artist::{Artist, ArtistHandle, LimitPoints, RawHandle};

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

    /// The corners and smallest positive coordinates of `limits`,
    /// extending both limits and requesting autoscaling (matplotlib's
    /// `add_collection`).
    pub(crate) fn limits(limits: DataLimits) -> Self {
        Self::data(vec![limits.bbox.p0, limits.bbox.p1, limits.minpos])
    }

    /// What an artist adds to the data limits, without autoscaling the
    /// view (matplotlib's `add_patch`).
    pub(crate) fn extending(limits: Option<LimitPoints>) -> Self {
        let LimitPoints { points, directions } = limits.unwrap_or_default();
        Self {
            points,
            update: directions,
            autoscale: (false, false),
        }
    }

    /// What an artist adds to the data limits, autoscaling the view
    /// (matplotlib's `add_line` and `add_collection`).
    pub(crate) fn autoscaled(limits: Option<LimitPoints>) -> Self {
        Self {
            autoscale: (true, true),
            ..Self::extending(limits)
        }
    }
}

impl Axes {
    /// Handles to the children of type `T`, in the order they were added
    /// (matplotlib's `ax.lines`, `ax.images`, ...).
    pub(crate) fn children_of<T: Artist>(&self) -> Vec<RawHandle> {
        let figure = self.raw().figure_state();
        let state = figure.borrow();
        let children = state
            .store
            .get_as::<AxesState>(self.id())
            .map(|axes| axes.children.clone())
            .unwrap_or_default();
        children
            .into_iter()
            .filter(|&id| state.store.get_as::<T>(id).is_some())
            .map(|id| RawHandle::new(Rc::clone(figure), id))
            .collect()
    }

    /// Adds `artist` as a child clipped to the axes and applies `update`
    /// (matplotlib's `add_line`/`add_patch`/`add_collection` bookkeeping).
    pub(crate) fn adopt(&self, mut artist: impl Artist, update: &DataUpdate) -> RawHandle {
        let axes_id = self.id();
        artist.base_mut().clip_box = Some(axes_id);
        let figure = self.raw().figure_state();
        let mut state = figure.borrow_mut();
        let id = state.adopt(axes_id, Box::new(artist));
        drop(state);
        self.update_data(update);
        RawHandle::new(Rc::clone(figure), id)
    }

    /// Recomputes the data limits from the lines, patches, images and
    /// collections of the axes, leaving hidden ones out when
    /// `visible_only` (matplotlib's `relim`). The view follows at the next
    /// [`Axes::autoscale_view`].
    ///
    /// ```
    /// use plt::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let line = ax.plot([0.0, 1.0], [0.0, 1.0])?;
    /// line.set_data(&[0.0, 10.0], &[0.0, 5.0]);
    /// assert_eq!(ax.data_lim().bbox.p1.x, 1.0, "set_data leaves the limits alone");
    /// ax.relim(false);
    /// assert_eq!(ax.data_lim().bbox.p1.x, 10.0);
    /// # Ok::<(), plt::axes::PlotError>(())
    /// ```
    pub fn relim(&self, visible_only: bool) -> &Self {
        let axes = self.id();
        let figure = self.raw().figure_state();
        let mut state = figure.borrow_mut();
        let children = state
            .store
            .get_as::<AxesState>(axes)
            .map(|a| a.children.clone())
            .unwrap_or_default();
        let mut limits = DataLimits::NULL;
        for artist in children.into_iter().filter_map(|id| state.store.get(id)) {
            if visible_only && !artist.base().visible {
                continue;
            }
            if let Some(LimitPoints {
                points,
                directions: (x, y),
            }) = artist.limit_points(axes)
            {
                limits.update_from_points(&points, x, y);
            }
        }
        if let Some(axes) = state.store.get_as_mut::<AxesState>(axes) {
            axes.data_lim = limits;
        }
        self
    }

    /// Extends the data limits to include `points` in x when `updatex` and
    /// in y when `updatey` (matplotlib's `update_datalim`).
    pub fn update_datalim(&self, points: &[Point], updatex: bool, updatey: bool) -> &Self {
        self.update_data(&DataUpdate {
            points: points.to_vec(),
            update: (updatex, updatey),
            autoscale: (false, false),
        });
        self
    }

    /// Applies `update` to the data limits and autoscaling state
    /// (matplotlib's `update_datalim` plus `_request_autoscale_view`).
    pub(crate) fn update_data(&self, update: &DataUpdate) {
        self.raw().with_mut::<AxesState, _>(|axes| {
            let (x, y) = update.update;
            axes.data_lim.update_from_points(&update.points, x, y);
            let (x, y) = update.autoscale;
            axes.request_autoscale(x, y);
        });
    }
}
