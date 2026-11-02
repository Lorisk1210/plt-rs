//! Axes: data coordinate systems and the plotting API (matplotlib.axes).

mod add_artist;
mod annotate;
mod aspect;
pub(crate) mod axis;
pub(crate) mod axis_artists;
mod axis_clear;
pub(crate) mod axis_draw;
mod axis_handle;
mod axis_intervals;
pub(crate) mod axis_label;
mod axis_label_layout;
mod axis_layout;
mod axis_mode;
mod axis_pick;
mod axis_ticks;
mod axis_units;
mod axis_user_units;
mod bar_label;
mod barbs;
mod bars;
mod box_labels;
mod boxplot;
mod broadcast;
mod bxp;
mod children;
mod clear;
mod contour;
mod current_image;
mod data_coords;
mod dataset_labels;
mod ecdf;
mod error;
mod error_size;
mod errorbar;
mod eventplot;
#[cfg(test)]
mod extent_tests;
mod fill;
mod fill_between;
mod grouped_bar;
mod hexbin;
mod hexbin_lattice;
mod hexbin_marginals;
mod hist;
mod hist2d;
mod image;
mod inset;
mod label_loc;
mod label_outer;
mod limit_callbacks;
mod limits;
mod lines;
mod mesh;
mod paint;
mod patch;
mod patches;
mod pcolor;
mod pcolor_rgba;
mod pcolorfast;
mod pie;
mod pie_label;
mod position;
pub(crate) mod projection;
mod quiver;
mod reference;
mod scatter;
mod secondary;
mod segments;
mod share;
mod sharing;
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
mod tick_compute;
mod tick_config;
mod tick_lines;
mod tick_set;
#[cfg(test)]
mod tick_tests;
mod ticks;
mod tight_bbox;
mod title_position;
mod titles;
mod tripcolor;
mod triplot;
mod twins;
#[cfg(test)]
mod unit_change_tests;
mod view;
mod view_limits;
mod violin;
mod which;

// The settings `set_aspect` and `set_axisbelow` take, which the
// `image.aspect` and `axes.axisbelow` rcParams hold too.
pub use crate::rc::{Aspect, AxisBelow};
pub use crate::ticker::AxisName;
pub use add_artist::AddAs;
pub(crate) use aspect::AxesLocator;
pub use aspect::{Adjustable, AspectError, AxesLocatorFn};
pub(crate) use axis::TickerDefaults;
pub use axis_artists::{AxisGroup, TickPart};
pub use axis_handle::Axis;
pub(crate) use axis_layout::{PlacedText, place_text};
pub use axis_mode::AxisMode;
pub(crate) use axis_ticks::tick_label_formatter;
pub use bar_label::{BarLabelError, BarLabelOptions, BarLabelType};
pub use bars::{BandAlign, BandRange, BarAlign, BarContainer, BarOptions};
pub use box_labels::BoxLabel;
pub use boxplot::BoxplotOptions;
pub use bxp::{BoxArtist, BoxProps, BoxplotArtists, BxpOptions};
pub(crate) use children::DataUpdate;
pub use current_image::ColorizingArtist;
pub use dataset_labels::DatasetLabels;
pub use ecdf::{EcdfError, EcdfOptions};
pub use error::PlotError;
pub use error_size::ErrorSize;
pub(crate) use errorbar::without_marker_style;
pub use errorbar::{ErrorbarContainer, ErrorbarFormat, ErrorbarOptions};
pub use eventplot::EventplotOptions;
pub use fill_between::FillBetweenOptions;
pub(crate) use fill_between::FillGeometry;
pub use grouped_bar::{GroupedBarData, GroupedBarError, GroupedBarOptions};
pub use hexbin::{
    HexBins, HexGridSize, HexReduce, HexReduceFn, HexScale, HexbinOptions, MAX_HEXAGONS,
};
pub use hist::{HistArtists, HistPlot, HistPlotOptions, MAX_HIST_BARS};
pub use hist2d::{Hist2d, Hist2dOptions, MAX_HIST2D_CELLS};
pub use image::{ImshowOptions, NonUniformOptions};
pub use label_loc::{LabelLoc, XLabelLoc, YLabelLoc};
pub use limit_callbacks::LimitsCallbackId;
pub(crate) use limit_callbacks::{LimitCallbacks, run_limit_callbacks};
pub use mesh::{Mesh, MeshCoords, MeshValues, Shading};
pub use pcolor::PcolorOptions;
pub use pcolor_rgba::RgbaMesh;
pub use pcolorfast::{FastValues, PcolorFast, PcolorfastOptions};
pub use pie::{PieContainer, PieDirection, PieError, PieOptions, PieShadow};
pub use pie_label::{PieLabelAlignment, PieLabelOptions, PieLabels};
pub use pltrs_core::path::DataLimits;
pub use position::WhichPosition;
pub use projection::ProjectionKind;
pub use scatter::{MappedColors, ScatterColors, ScatterOptions};
pub use secondary::SecondaryLocation;
pub(crate) use share::{Grouper, ShareGroups};
pub(crate) use sharing::share_free;
pub use specgram::{SpecgramError, SpecgramOptions, SpecgramPlot};
pub use spectral::{CorrelationArtists, SpectralPlotOptions, SpectrumPlotOptions, SpectrumScale};
pub use spy::{Spy, SpyOptions};
pub use stackplot::{Baseline, StackplotOptions};
pub(crate) use state::AxesState;
pub(crate) use stem::solid_format;
pub use stem::{StemContainer, StemOptions};
pub use steps::StairsOptions;
pub use tick_config::{LocatorParams, TickLabelProps, TickParams, TicklabelFormat};
pub use tick_lines::{GridLine, TickLine, TickLineProps};
pub use tick_set::{Tick, TickSet};
pub(crate) use ticks::grid_level_on;
pub use ticks::{GridOptions, GridStyle, Sides, TickDirection, TickPosition, TickStyle};
pub(crate) use tight_bbox::fully_clipped;
pub(crate) use tight_bbox::spine_extent_with_ticks;
pub(crate) use tight_bbox::{clipped_extent, collapse_x};
pub use titles::TitleLocation;
pub use tripcolor::{TriShading, TriValues, Tripcolor, TripcolorError, TripcolorOptions};
pub use triplot::TriplotLines;
pub use view_limits::Limits;
pub use violin::{ViolinDrawOptions, ViolinParts, ViolinSide, ViolinplotOptions};
pub use which::{TickLevel, WhichAxis};

use pltrs_core::{Bbox, Point};

use std::rc::Rc;

use crate::artist::Artist;
use crate::artist::{ArtistHandle, RawHandle, TypedHandle};
use crate::scale::{Scale, ScaleError, ScaleName, ScaleOptions};
use crate::spines::{Spine, Spines};
use crate::text::{Text, TextArtist};
use crate::ticker::TickerError;
use crate::transforms::{Transform, TransformError};
use crate::units::{AxisValue, DataPoint, UnitHandle, UnitSources};

/// A handle to one axes of a figure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Axes {
    raw: RawHandle,
}

pltrs_core::archive_struct!(Axes { raw });

impl TypedHandle for Axes {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        raw.is::<AxesState>().then(|| Self::from_raw(raw.clone()))
    }
}

impl ArtistHandle for Axes {
    fn raw(&self) -> &RawHandle {
        &self.raw
    }

    /// Sets the drawing order; among twinned axes, the bottom-most one
    /// draws the background.
    fn set_zorder(&self, zorder: f64) -> &Self {
        self.raw.with_base_mut(|b| b.zorder = zorder);
        self.update_twinned_patch_visibility();
        self
    }

    /// Removes the axes from the figure; removing the axes of a colorbar
    /// removes the colorbar (matplotlib's `_remove_cbar_axes`).
    fn remove(&self) {
        crate::colorbar::release(self);
        self.raw
            .figure_state()
            .borrow_mut()
            .remove_artist(self.id());
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

    /// Where the data point `at` lands in display pixels, from the lower
    /// left corner of the figure, with the figure laid out as for drawing
    /// (matplotlib's `ax.transData.transform(at)`): where a window would
    /// report a click on that point.
    ///
    /// ```
    /// use pltrs::events::EventSource;
    /// use pltrs::navigation::PointerButton;
    /// use pltrs::prelude::*;
    /// let fig = Figure::with_size(4.0, 4.0, 100.0);
    /// let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0])?;
    /// ax.set_xlim(0.0, 4.0)?.set_ylim(0.0, 4.0)?;
    /// let at = ax.data_to_display((1.0, 3.0))?;
    /// assert_eq!((at.x, at.y), (100.0, 300.0));
    /// EventSource::new().button_press(&fig, at.x, at.y, PointerButton::Left, false);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the axes was removed from its figure.
    pub fn data_to_display(&self, at: impl Into<Point>) -> Result<Point, TransformError> {
        let to_display = self.figure().resolve_transform(&self.trans_data())?;
        Ok(to_display.transform_point(at.into()))
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

    /// Adds text at `(x, y)` in data coordinates: numbers, dates,
    /// categories or user values (matplotlib's `ax.text`). They convert
    /// as matplotlib's texts convert their position when drawn: without
    /// giving the axes a converter, tick helpers or a unit (which plotting
    /// data does), and again, for user values, when an axis changes
    /// unit. A category name joins the categories of its axis; on an axis
    /// without categories it cannot be placed, and the figure reports it
    /// when it next draws.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// ax.bar(["apple", "banana"], [4.0, 6.0], Default::default())?;
    /// assert_eq!(ax.text("cherry", 5.0, "none yet").position().x, 2.0);
    /// assert!(ax.text(1.0, "tall", "?").position().y.is_nan());
    /// assert!(fig.to_png().is_err(), "the y axis has no categories");
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn text(
        &self,
        x: impl Into<AxisValue>,
        y: impl Into<AxisValue>,
        text: impl Into<String>,
    ) -> Text {
        let at = DataPoint {
            x: x.into(),
            y: y.into(),
        };
        let position = Point::new(
            self.xaxis().convert_when_drawn(&at.x),
            self.yaxis().convert_when_drawn(&at.y),
        );
        let mut artist = TextArtist::new(position, text, crate::rc::params());
        artist.unit_sources = UnitSources::of_point(&at);
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
            .with_mut::<AxesState, _>(|a| a.xaxis.label.set_text(text));
        self
    }

    /// Sets the y-axis label.
    pub fn set_ylabel(&self, text: impl Into<String>) -> &Self {
        let text = text.into();
        self.raw
            .with_mut::<AxesState, _>(|a| a.yaxis.label.set_text(text));
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

    /// Sets the x view limits, numbers, dates or categories, and turns x
    /// autoscaling off; `left > right` inverts the axis (matplotlib's
    /// `set_xlim`; see [`Axes::set_limits`]).
    ///
    /// # Errors
    ///
    /// When a limit is NaN or infinite; then the view is unchanged.
    pub fn set_xlim(
        &self,
        left: impl Into<AxisValue>,
        right: impl Into<AxisValue>,
    ) -> Result<&Self, PlotError> {
        let xaxis = self.xaxis();
        let (left, right) = (xaxis.convert_value(left), xaxis.convert_value(right));
        self.set_limits(AxisName::X, left, right)
    }

    /// Sets the y view limits, numbers, dates or categories, and turns y
    /// autoscaling off; `bottom > top` inverts the axis (matplotlib's
    /// `set_ylim`; see [`Axes::set_limits`]).
    ///
    /// # Errors
    ///
    /// When a limit is NaN or infinite; then the view is unchanged.
    pub fn set_ylim(
        &self,
        bottom: impl Into<AxisValue>,
        top: impl Into<AxisValue>,
    ) -> Result<&Self, PlotError> {
        let yaxis = self.yaxis();
        let (bottom, top) = (yaxis.convert_value(bottom), yaxis.convert_value(top));
        self.set_limits(AxisName::Y, bottom, top)
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
        self.update_twinned_patch_visibility();
        self
    }

    /// Whether the background patch and the spines are drawn (matplotlib's
    /// `get_frame_on`).
    pub fn frame_on(&self) -> bool {
        self.raw.with::<AxesState, _>(|a| a.decorations.frame_on)
    }

    fn view_lim(&self) -> Bbox {
        self.unstale_view();
        self.raw.with::<AxesState, _>(|a| a.view_lim)
    }

    /// Applies pending autoscaling to the view limits, then runs the limit
    /// callbacks it queued.
    pub(crate) fn unstale_view(&self) {
        self.raw.figure_state().borrow_mut().unstale_axes(self.id());
        run_limit_callbacks(self.raw.figure_state());
    }

    /// Sets the x scale by name ([`ScaleName::Log`], or its string
    /// `"log"`, ...) with default options (matplotlib's `set_xscale`).
    ///
    /// # Errors
    ///
    /// When `name` is not a built-in scale, or the scale needs options.
    pub fn set_xscale(&self, name: impl Into<ScaleName>) -> Result<&Self, ScaleError> {
        self.set_scale_with(AxisName::X, name, ScaleOptions::default())
    }

    /// Sets the y scale by name with default options (matplotlib's
    /// `set_yscale`).
    ///
    /// # Errors
    ///
    /// As [`Axes::set_xscale`].
    pub fn set_yscale(&self, name: impl Into<ScaleName>) -> Result<&Self, ScaleError> {
        self.set_scale_with(AxisName::Y, name, ScaleOptions::default())
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

    /// Replaces the scale object of one axis and of the axes sharing it,
    /// keeping their tick helpers and limits (a colorbar following the new
    /// limits of a norm its scale is made from).
    pub(crate) fn adopt_scale(&self, axis: AxisName, scale: Box<dyn Scale>) {
        let scale: Rc<dyn Scale> = Rc::from(scale);
        self.raw
            .figure_state()
            .borrow_mut()
            .update_shared_axis(self.id(), axis, |state| {
                state.adopt_scale(Rc::clone(&scale));
            });
    }

    /// The name of the x scale (matplotlib's `get_xscale`).
    pub fn xscale(&self) -> ScaleName {
        self.raw.with::<AxesState, _>(|a| a.xaxis.scale().name())
    }

    /// The name of the y scale (matplotlib's `get_yscale`).
    pub fn yscale(&self) -> ScaleName {
        self.raw.with::<AxesState, _>(|a| a.yaxis.scale().name())
    }

    /// The ticks one axis draws in the current view, where the axes is
    /// now: a layout engine moves the axes only when the figure is drawn
    /// or laid out (call [`Figure::layout`](crate::figure::Figure::layout)
    /// first for the ticks of the axes it places), as matplotlib computes
    /// ticks between draws.
    ///
    /// # Errors
    ///
    /// When the locator or the formatter of the axis fails, or the locator
    /// places more than [`MAX_DRAWN_TICKS`](crate::ticker::MAX_DRAWN_TICKS)
    /// ticks.
    pub fn ticks(&self, axis: AxisName) -> Result<TickSet, TickerError> {
        self.raw.figure_state().borrow_mut().prepare_as_is();
        self.raw
            .with::<AxesState, _>(|a| a.axis(axis).ticks.clone())
    }
}
