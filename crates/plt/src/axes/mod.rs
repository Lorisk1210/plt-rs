//! Axes: data coordinate systems and the plotting API (matplotlib.axes).

mod limits;
mod lines;
mod patches;
mod state;
#[cfg(test)]
mod tests;

pub use limits::DataLimits;
pub use lines::PlotError;
pub(crate) use state::AxesState;
pub use state::AxisName;

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
        self.raw.with_mut::<AxesState, _>(|a| {
            a.position = Bbox::from_bounds(left, bottom, width, height);
        });
        self
    }

    /// The extent of all data plotted so far (`ax.dataLim`).
    pub fn data_lim(&self) -> DataLimits {
        self.raw.with::<AxesState, _>(|a| a.data_lim)
    }

    /// The x view limits `(left, right)`, autoscaled first if needed.
    pub fn xlim(&self) -> (f64, f64) {
        self.view_lim().interval_x()
    }

    /// The y view limits `(bottom, top)`, autoscaled first if needed.
    pub fn ylim(&self) -> (f64, f64) {
        self.view_lim().interval_y()
    }

    /// Sets the x view limits and turns x autoscaling off;
    /// `left > right` inverts the axis.
    pub fn set_xlim(&self, left: f64, right: f64) -> &Self {
        self.raw
            .with_mut::<AxesState, _>(|a| a.set_limits(AxisName::X, left, right));
        self
    }

    /// Sets the y view limits and turns y autoscaling off;
    /// `bottom > top` inverts the axis.
    pub fn set_ylim(&self, bottom: f64, top: f64) -> &Self {
        self.raw
            .with_mut::<AxesState, _>(|a| a.set_limits(AxisName::Y, bottom, top));
        self
    }

    /// Sets the padding added around the data when autoscaling, as a
    /// fraction of the data span (matplotlib's `margins`).
    pub fn set_margins(&self, x: f64, y: f64) -> &Self {
        self.raw.with_mut::<AxesState, _>(|a| {
            a.autoscale_x.margin = x;
            a.autoscale_y.margin = y;
            a.autoscale_x.stale = true;
            a.autoscale_y.stale = true;
        });
        self
    }

    /// Turns autoscaling of both axes on or off.
    pub fn set_autoscale_on(&self, on: bool) -> &Self {
        self.raw.with_mut::<AxesState, _>(|a| {
            a.autoscale_x.enabled = on;
            a.autoscale_y.enabled = on;
            a.autoscale_x.stale = on;
            a.autoscale_y.stale = on;
        });
        self
    }

    fn view_lim(&self) -> Bbox {
        self.raw.figure_state().borrow_mut().unstale_axes(self.id());
        self.raw.with::<AxesState, _>(|a| a.view_lim)
    }
}
