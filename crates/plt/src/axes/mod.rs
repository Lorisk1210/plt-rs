//! Axes: data coordinate systems and the plotting API (matplotlib.axes).

mod add_artist;
mod annotate;
mod aspect;
pub(crate) mod axis;
pub(crate) mod axis_draw;
mod axis_handle;
mod axis_layout;
mod axis_mode;
mod bar_label;
mod barbs;
mod bars;
mod boxplot;
mod broadcast;
mod bxp;
mod children;
mod clear;
mod contour;
mod error;
mod errorbar;
mod eventplot;
mod fill;
mod fill_between;
mod grouped_bar;
mod hexbin;
mod hist;
mod hist2d;
mod image;
mod inset;
mod limits;
mod lines;
mod patches;
mod pcolor;
mod pcolorfast;
mod pie;
pub(crate) mod projection;
mod quiver;
mod reference;
mod scatter;
mod secondary;
mod segments;
mod share;
mod specgram;
mod spectral;
mod spy;
mod stackplot;
mod state;
mod stem;
mod steps;
mod streamplot;
mod table;
#[cfg(test)]
mod tests;
mod tick_config;
mod ticks;
mod tight_bbox;
mod title_position;
mod tripcolor;
mod triplot;
mod twins;
mod view;
mod violin;

pub use crate::ticker::AxisName;
pub(crate) use aspect::AxesLocator;
pub use aspect::{Adjustable, AspectError};
pub use axis::{LabelLoc, Tick, TickSet};
pub use axis_handle::Axis;
pub(crate) use axis_layout::{PlacedText, place_text};
pub use axis_mode::AxisMode;
pub use bar_label::{BarLabelError, BarLabelFormat, BarLabelOptions, BarLabelType};
pub use bars::{BandAlign, BarAlign, BarContainer, BarOptions};
pub use boxplot::BoxplotOptions;
pub use bxp::{BoxArtist, BoxStyle, BoxplotArtists, BxpOptions};
pub(crate) use children::DataUpdate;
pub use error::PlotError;
pub use errorbar::{ErrorSize, ErrorbarContainer, ErrorbarOptions};
pub use eventplot::EventplotOptions;
pub use fill_between::FillBetweenOptions;
pub use grouped_bar::{GroupedBarData, GroupedBarError, GroupedBarOptions};
pub use hexbin::{HexBins, HexGridSize, HexReduce, HexScale, HexbinOptions};
pub use hist::{HistArtists, HistPlot, HistPlotOptions};
pub use hist2d::{Hist2d, Hist2dOptions};
pub use image::ImshowOptions;
pub use pcolor::{Mesh, MeshCoords, PcolorOptions, Shading};
pub use pcolorfast::{PcolorFast, PcolorfastOptions};
pub use pie::{PieContainer, PieDirection, PieError, PieLabelFormat, PieOptions, PieShadow};
pub use plt_core::path::DataLimits;
pub use projection::ProjectionKind;
pub use scatter::{MappedColors, ScatterColors, ScatterOptions};
pub use secondary::SecondaryLocation;
pub(crate) use share::{Grouper, ShareGroups};
pub use specgram::{SpecgramError, SpecgramOptions, SpecgramPlot};
pub use spectral::{CorrelationArtists, SpectralPlotOptions, SpectrumPlotOptions, SpectrumScale};
pub use spy::{Spy, SpyOptions};
pub use stackplot::{Baseline, StackplotOptions};
pub(crate) use state::AxesState;
pub use stem::{StemContainer, StemOptions};
pub use steps::StairsOptions;
pub use tick_config::{LocatorParams, TickLabelProps, TickParams, TicklabelFormat};
pub use ticks::{GridOptions, GridStyle, Sides, TickDirection, TickLevel, TickPosition, TickStyle};
pub use tripcolor::{TriShading, TriValues, Tripcolor, TripcolorError, TripcolorOptions};
pub use triplot::TriplotLines;
pub use violin::{ViolinDrawOptions, ViolinParts, ViolinSide, ViolinplotOptions};

use plt_core::Bbox;

use std::rc::Rc;

use crate::artist::Artist;
use crate::artist::{ArtistHandle, RawHandle};
use crate::colors::Color;
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

    /// The figure this axes belongs to (matplotlib's
    /// `get_figure(root=True)`).
    pub fn figure(&self) -> crate::figure::Figure {
        crate::figure::Figure::root_of(self.raw.figure_state())
    }

    /// Data coordinates of this axes (`ax.transData`).
    pub fn trans_data(&self) -> Transform {
        Transform::Data(self.id())
    }

    /// Axes-fraction coordinates (`ax.transAxes`).
    pub fn trans_axes(&self) -> Transform {
        Transform::Axes(self.id())
    }

    /// x in data, y in axes-fraction coordinates
    /// (`ax.get_xaxis_transform()`).
    pub fn trans_xaxis(&self) -> Transform {
        Transform::blended(self.trans_data(), self.trans_axes())
    }

    /// x in axes-fraction, y in data coordinates
    /// (`ax.get_yaxis_transform()`).
    pub fn trans_yaxis(&self) -> Transform {
        Transform::blended(self.trans_axes(), self.trans_data())
    }

    /// The grid cell the axes occupies, if it was placed on a grid.
    pub fn subplot_spec(&self) -> Option<crate::gridspec::SubplotSpec> {
        self.raw.with::<AxesState, _>(|a| a.subplot_spec.clone())
    }

    /// Placement as a fraction of the figure (or subfigure), after the
    /// aspect adjustment (matplotlib's `get_position`).
    pub fn position(&self) -> Bbox {
        self.apply_aspect();
        self.raw
            .with::<AxesState, _>(|a| a.aspect.active_position(&a.position))
    }

    /// Moves the axes to `[left, bottom, width, height]` in figure fractions.
    pub fn set_position(&self, rect: [f64; 4]) -> &Self {
        let [left, bottom, width, height] = rect;
        self.raw.with_mut::<AxesState, _>(|a| {
            a.position = Bbox::from_bounds(left, bottom, width, height);
        });
        self
    }

    /// Sets the title at the `axes.titlelocation` position (centre by
    /// default; matplotlib's `set_title`) and returns it.
    pub fn set_title(&self, text: impl Into<String>) -> Text {
        let location = match crate::rc::params().axes_titlelocation.as_str() {
            "left" => TitleLocation::Left,
            "right" => TitleLocation::Right,
            _ => TitleLocation::Center,
        };
        self.set_title_at(location, text)
    }

    /// Sets the title at `location` and returns it. Like matplotlib's
    /// `set_title`, this resets the gap of all titles to `axes.titlepad`;
    /// call [`Axes::set_title_pad`] afterwards to change it.
    pub fn set_title_at(&self, location: TitleLocation, text: impl Into<String>) -> Text {
        self.set_title_pad(crate::rc::params().axes_titlepad);
        let title = self.title_handle(location);
        title.set_text(text);
        title
    }

    /// Places the title at `location` at height `y` in axes fractions and
    /// stops lifting this axes' titles above decorations on top
    /// (matplotlib's `set_title(..., y=...)`).
    pub fn set_title_y(&self, location: TitleLocation, y: f64) -> &Self {
        self.title_handle(location).set_y(y);
        self.raw
            .with_mut::<AxesState, _>(|axes| axes.title_placement = state::TitlePlacement::Fixed);
        self
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
        self.set_limits(AxisName::X, left, right)
    }

    /// Sets the y view limits and turns y autoscaling off;
    /// `bottom > top` inverts the axis.
    pub fn set_ylim(&self, bottom: f64, top: f64) -> &Self {
        self.set_limits(AxisName::Y, bottom, top)
    }

    /// Sets the view limits of one axis, and of the axes sharing it, and
    /// turns its autoscaling off (matplotlib's `set_xlim(..., auto=False)`).
    /// Singular limits are expanded and limits a log axis cannot show are
    /// ignored, as in matplotlib.
    pub fn set_limits(&self, axis: AxisName, low: f64, high: f64) -> &Self {
        self.raw.figure_state().borrow_mut().set_axis_limits(
            self.id(),
            axis,
            (low, high),
            Some(false),
            true,
        );
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

    /// Autoscales one axis (or both) from now on (matplotlib's
    /// `autoscale(enable=True, axis=..., tight=...)`); `tight` drops the
    /// margins and skips rounding the view to ticks.
    pub fn autoscale(&self, axis: Option<AxisName>, tight: Option<bool>) -> &Self {
        self.raw.with_mut::<AxesState, _>(|a| {
            for name in [AxisName::X, AxisName::Y] {
                if axis.is_some_and(|only| only != name) {
                    continue;
                }
                let autoscale = &mut a.axis_mut(name).autoscale;
                autoscale.enabled = true;
                autoscale.stale = true;
                if tight == Some(true) {
                    autoscale.margin = 0.0;
                }
            }
            if let Some(tight) = tight {
                a.tight = tight;
            }
        });
        self
    }

    /// Hides the patch, spines and both axes, leaving titles and plotted
    /// artists (matplotlib's `set_axis_off`, `axis("off")`).
    pub fn set_axis_off(&self) -> &Self {
        self.raw
            .with_mut::<AxesState, _>(|a| a.decorations.axis_on = false);
        self
    }

    /// Shows the patch, spines and axes again (matplotlib's `set_axis_on`).
    pub fn set_axis_on(&self) -> &Self {
        self.raw
            .with_mut::<AxesState, _>(|a| a.decorations.axis_on = true);
        self
    }

    /// Whether the patch, spines and axes are drawn.
    pub fn axis_on(&self) -> bool {
        self.raw.with::<AxesState, _>(|a| a.decorations.axis_on)
    }

    /// Shows or hides the background patch and the spines (matplotlib's
    /// `set_frame_on`).
    pub fn set_frame_on(&self, on: bool) -> &Self {
        self.raw
            .with_mut::<AxesState, _>(|a| a.decorations.frame_on = on);
        self
    }

    /// Sets the background colour (matplotlib's `set_facecolor`).
    pub fn set_facecolor(&self, color: impl Into<Color>) -> &Self {
        let color = color.into();
        self.raw
            .with_mut::<AxesState, _>(|a| a.patch.facecolor = color);
        self
    }

    /// The background colour.
    pub fn facecolor(&self) -> Color {
        self.raw.with::<AxesState, _>(|a| a.patch.facecolor.clone())
    }

    /// Sets the gap between the axes top and its titles in points
    /// (matplotlib's `set_title(..., pad=...)`).
    pub fn set_title_pad(&self, pad: f64) -> &Self {
        for location in [
            TitleLocation::Left,
            TitleLocation::Center,
            TitleLocation::Right,
        ] {
            self.title_handle(location)
                .set_transform(Transform::Axes(self.id()).offset_points(0.0, pad));
        }
        self
    }

    fn view_lim(&self) -> Bbox {
        self.unstale_view();
        self.raw.with::<AxesState, _>(|a| a.view_lim)
    }

    /// Applies pending autoscaling to the view limits.
    pub(crate) fn unstale_view(&self) {
        self.raw.figure_state().borrow_mut().unstale_axes(self.id());
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

    /// Installs a scale object on one axis and on the axes sharing it,
    /// with default tick helpers, and re-autoscales that axis (matplotlib's
    /// `_set_axes_scale`).
    pub fn set_scale(&self, axis: AxisName, scale: Box<dyn Scale>) -> &Self {
        self.raw
            .figure_state()
            .borrow_mut()
            .set_shared_scale(self.id(), axis, &Rc::from(scale));
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
