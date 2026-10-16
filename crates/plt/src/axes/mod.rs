//! Axes: data coordinate systems and the plotting API (matplotlib.axes).

mod state;

pub(crate) use state::AxesState;

use plt_core::Bbox;

use crate::artist::{ArtistHandle, RawHandle};
use crate::transforms::Transform;

/// A handle to one axes of a figure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Axes {
    raw: RawHandle,
}

impl ArtistHandle for Axes {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }
}

impl Axes {
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self { raw }
    }

    /// Data coordinates of this axes (`ax.transData`).
    pub fn trans_data(&self) -> Transform {
        Transform::Data(self.id())
    }

    /// Axes-fraction coordinates (`ax.transAxes`).
    pub fn trans_axes(&self) -> Transform {
        Transform::Axes(self.id())
    }

    /// Placement as a fraction of the figure (or subfigure).
    pub fn position(&self) -> Bbox {
        self.raw.with::<AxesState, _>(|a| a.position)
    }

    /// Moves the axes to `[left, bottom, width, height]` in figure fractions.
    pub fn set_position(&self, rect: [f64; 4]) -> &Self {
        let [left, bottom, width, height] = rect;
        self.raw
            .with_mut::<AxesState, _>(|a| a.position = Bbox::from_bounds(left, bottom, width, height));
        self
    }

    /// The extent of all data plotted so far (`ax.dataLim`).
    pub fn data_lim(&self) -> Bbox {
        self.raw.with::<AxesState, _>(|a| a.data_lim)
    }

    /// The x view limits `(left, right)`.
    pub fn xlim(&self) -> (f64, f64) {
        self.raw.with::<AxesState, _>(|a| a.view_lim.interval_x())
    }

    /// The y view limits `(bottom, top)`.
    pub fn ylim(&self) -> (f64, f64) {
        self.raw.with::<AxesState, _>(|a| a.view_lim.interval_y())
    }

    /// Sets the x view limits; `left > right` inverts the axis.
    pub fn set_xlim(&self, left: f64, right: f64) -> &Self {
        self.raw.with_mut::<AxesState, _>(|a| {
            a.view_lim = Bbox::from_extents(left, a.view_lim.y0(), right, a.view_lim.y1());
        });
        self
    }

    /// Sets the y view limits; `bottom > top` inverts the axis.
    pub fn set_ylim(&self, bottom: f64, top: f64) -> &Self {
        self.raw.with_mut::<AxesState, _>(|a| {
            a.view_lim = Bbox::from_extents(a.view_lim.x0(), bottom, a.view_lim.x1(), top);
        });
        self
    }
}
