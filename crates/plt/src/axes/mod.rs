//! Axes: data coordinate systems and the plotting API (matplotlib.axes).

mod axis;
mod axis_draw;
mod limits;
mod lines;
mod patches;
mod state;
#[cfg(test)]
mod tests;
mod ticks;

pub use crate::ticker::AxisName;
pub use axis::{Tick, TickSet};
pub use limits::DataLimits;
pub use lines::PlotError;
pub(crate) use state::AxesState;
pub use ticks::{GridStyle, Sides, TickDirection, TickStyle};

use plt_core::Bbox;

use std::rc::Rc;

use crate::artist::Artist;
use crate::artist::{ArtistHandle, RawHandle};
use crate::scale::{Scale, ScaleError, ScaleOptions, scale_factory};
use crate::spines::{Spine, Spines};
use crate::text::{Text, TextArtist};
use crate::transforms::Transform;
use plt_core::Point;

/// Where an axes title sits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TitleLocation {
    /// Flush left.
    Left,
    /// Centred.
    #[default]
    Center,
    /// Flush right.
    Right,
}

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

    /// Sets the centre title (matplotlib's `set_title`) and returns it.
    pub fn set_title(&self, text: impl Into<String>) -> Text {
        self.set_title_at(TitleLocation::Center, text)
    }

    /// Sets the title at `location` and returns it.
    pub fn set_title_at(&self, location: TitleLocation, text: impl Into<String>) -> Text {
        let title = self.title_handle(location);
        title.set_text(text);
        title
    }

    /// The title at `location` (empty when unset).
    pub fn title(&self, location: TitleLocation) -> String {
        self.title_handle(location).text()
    }

    fn title_handle(&self, location: TitleLocation) -> Text {
        let index = match location {
            TitleLocation::Left => 0,
            TitleLocation::Center => 1,
            TitleLocation::Right => 2,
        };
        let id = self.raw.with::<AxesState, _>(|a| a.titles[index]);
        Text::from_raw(RawHandle::new(Rc::clone(self.raw.figure_state()), id))
    }

    /// Adds text at `(x, y)` in data coordinates (matplotlib's `ax.text`).
    pub fn text(&self, x: f64, y: f64, text: impl Into<String>) -> Text {
        let mut artist = TextArtist::new(Point::new(x, y), text, crate::rc::params());
        let base = artist.base_mut();
        base.transform = Some(Transform::Data(self.id()));
        base.clip_box = Some(self.id());
        base.clip_on = false;
        let id = self
            .raw
            .figure_state()
            .borrow_mut()
            .adopt(self.id(), Box::new(artist));
        Text::from_raw(RawHandle::new(Rc::clone(self.raw.figure_state()), id))
    }

    /// Sets the x-axis label.
    pub fn set_xlabel(&self, text: impl Into<String>) -> &Self {
        let text = text.into();
        self.raw
            .with_mut::<AxesState, _>(|a| a.xaxis.label.text = text);
        self
    }

    /// Sets the y-axis label.
    pub fn set_ylabel(&self, text: impl Into<String>) -> &Self {
        let text = text.into();
        self.raw
            .with_mut::<AxesState, _>(|a| a.yaxis.label.text = text);
        self
    }

    /// The label of one axis.
    pub fn label(&self, axis: AxisName) -> String {
        self.raw
            .with::<AxesState, _>(|a| a.axis(axis).label.text.clone())
    }

    /// The four spines framing the axes.
    pub fn spines(&self) -> Spines {
        let ids = self.raw.with::<AxesState, _>(|a| a.spines.clone());
        let spine =
            |i: usize| Spine::from_raw(RawHandle::new(Rc::clone(self.raw.figure_state()), ids[i]));
        Spines {
            left: spine(0),
            right: spine(1),
            bottom: spine(2),
            top: spine(3),
        }
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
            a.xaxis.autoscale.margin = x;
            a.yaxis.autoscale.margin = y;
            a.xaxis.autoscale.stale = true;
            a.yaxis.autoscale.stale = true;
        });
        self
    }

    /// Turns autoscaling of both axes on or off.
    pub fn set_autoscale_on(&self, on: bool) -> &Self {
        self.raw.with_mut::<AxesState, _>(|a| {
            a.xaxis.autoscale.enabled = on;
            a.yaxis.autoscale.enabled = on;
            a.xaxis.autoscale.stale = on;
            a.yaxis.autoscale.stale = on;
        });
        self
    }

    fn view_lim(&self) -> Bbox {
        self.raw.figure_state().borrow_mut().unstale_axes(self.id());
        self.raw.with::<AxesState, _>(|a| a.view_lim)
    }

    /// Sets the x scale by name (`"linear"`, `"log"`, `"symlog"`,
    /// `"logit"`, `"asinh"`, ...) with default options.
    pub fn set_xscale(&self, name: &str) -> Result<&Self, ScaleError> {
        self.set_scale(AxisName::X, scale_factory(name, &ScaleOptions::default())?);
        Ok(self)
    }

    /// Sets the y scale by name.
    pub fn set_yscale(&self, name: &str) -> Result<&Self, ScaleError> {
        self.set_scale(AxisName::Y, scale_factory(name, &ScaleOptions::default())?);
        Ok(self)
    }

    /// Installs a scale object on one axis, with its default tick helpers,
    /// and re-autoscales that axis (matplotlib's `_set_axes_scale`).
    pub fn set_scale(&self, axis: AxisName, scale: Box<dyn Scale>) -> &Self {
        self.raw.with_mut::<AxesState, _>(|a| {
            let axis = a.axis_mut(axis);
            axis.set_scale(scale);
            axis.autoscale.stale = true;
        });
        self
    }

    /// The name of the x scale.
    pub fn xscale(&self) -> &'static str {
        self.raw.with::<AxesState, _>(|a| a.xaxis.scale().name())
    }

    /// The name of the y scale.
    pub fn yscale(&self) -> &'static str {
        self.raw.with::<AxesState, _>(|a| a.yaxis.scale().name())
    }

    /// The ticks one axis draws in the current view (computed by the last
    /// draw or layout pass; call [`Figure::layout`](crate::figure::Figure::layout)
    /// to refresh them without drawing).
    pub fn ticks(&self, axis: AxisName) -> Result<TickSet, String> {
        self.raw.figure_state().borrow_mut().prepare();
        self.raw
            .with::<AxesState, _>(|a| a.axis(axis).ticks.clone())
    }
}
