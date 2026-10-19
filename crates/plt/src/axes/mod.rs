//! Axes: data coordinate systems and the plotting API (matplotlib.axes).

mod add_artist;
mod annotate;
mod aspect;
pub(crate) mod axis;
pub(crate) mod axis_draw;
mod axis_handle;
pub(crate) mod axis_label;
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
mod current_image;
mod data_coords;
mod ecdf;
mod error;
mod errorbar;
mod eventplot;
mod fill;
mod fill_between;
mod grouped_bar;
mod hexbin;
mod hexbin_marginals;
mod hist;
mod hist2d;
mod image;
mod inset;
mod limits;
mod lines;
mod mesh;
mod paint;
mod patches;
mod pcolor;
mod pcolor_rgba;
mod pcolorfast;
mod pie;
mod pie_label;
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
#[cfg(test)]
mod tick_tests;
mod ticks;
mod tight_bbox;
mod title_position;
mod tripcolor;
mod triplot;
mod twins;
mod view;
mod view_limits;
mod violin;

pub use crate::ticker::AxisName;
pub(crate) use aspect::AxesLocator;
pub use aspect::{Adjustable, AspectError};
pub use axis::{Tick, TickSet};
pub use axis_handle::Axis;
pub(crate) use axis_handle::tick_label_formatter;
pub use axis_label::LabelLoc;
pub(crate) use axis_layout::{PlacedText, place_text};
pub use axis_mode::AxisMode;
pub use bar_label::{BarLabelError, BarLabelFormat, BarLabelOptions, BarLabelType};
pub use bars::{BandAlign, BandRange, BarAlign, BarContainer, BarOptions};
pub use boxplot::BoxplotOptions;
pub use bxp::{BoxArtist, BoxStyle, BoxplotArtists, BxpOptions};
pub(crate) use children::DataUpdate;
pub use current_image::ColorizingArtist;
pub use ecdf::{EcdfError, EcdfOptions};
pub use error::PlotError;
pub use errorbar::{ErrorSize, ErrorbarContainer, ErrorbarOptions};
pub use eventplot::EventplotOptions;
pub use fill_between::FillBetweenOptions;
pub(crate) use fill_between::FillGeometry;
pub use grouped_bar::{GroupedBarData, GroupedBarError, GroupedBarOptions};
pub use hexbin::{HexBins, HexGridSize, HexReduce, HexScale, HexbinOptions};
pub use hist::{HistArtists, HistPlot, HistPlotOptions};
pub use hist2d::{Hist2d, Hist2dOptions};
pub use image::{ImshowOptions, NonUniformOptions};
pub use mesh::{Mesh, MeshCoords, MeshValues, Shading};
pub use pcolor::PcolorOptions;
pub use pcolor_rgba::RgbaMesh;
pub use pcolorfast::{FastValues, PcolorFast, PcolorfastOptions};
pub use pie::{PieContainer, PieDirection, PieError, PieLabelFormat, PieOptions, PieShadow};
pub use pie_label::{PieLabelAlignment, PieLabelOptions, PieLabels};
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
pub(crate) use tight_bbox::{clipped_extent, collapse_x};
pub use tripcolor::{TriShading, TriValues, Tripcolor, TripcolorError, TripcolorOptions};
pub use triplot::TriplotLines;
pub use view_limits::Limits;
pub use violin::{ViolinDrawOptions, ViolinParts, ViolinSide, ViolinplotOptions};

use plt_core::Bbox;

use std::rc::Rc;

use crate::artist::Artist;
use crate::artist::{ArtistHandle, RawHandle};
use crate::colors::Color;
use crate::rc::ColorOrAuto;
use crate::scale::{Scale, ScaleError, ScaleOptions, scale_factory};
use crate::spines::{Spine, Spines};
use crate::text::{Text, TextArtist};
use crate::transforms::Transform;
use plt_core::Point;
use plt_text::{HAlign, VAlign};

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

impl TitleLocation {
    /// The horizontal alignment of a title here (matplotlib aligns titles
    /// by their location name).
    fn halign(self) -> HAlign {
        match self {
            Self::Left => HAlign::Left,
            Self::Center => HAlign::Center,
            Self::Right => HAlign::Right,
        }
    }
}

/// Which position of an axes [`Axes::set_position_which`] sets
/// (matplotlib's `which`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WhichPosition {
    /// The original and the active position.
    #[default]
    Both,
    /// The position before any aspect adjustment.
    Original,
    /// The position drawn, until the aspect is next applied.
    Active,
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

    /// Sets the drawing order; among twinned axes, the bottom-most one
    /// draws the background.
    fn set_zorder(&self, zorder: f64) -> &Self {
        self.raw.with_base_mut(|b| b.zorder = zorder);
        self.update_twinned_patch_visibility();
        self
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
    /// aspect adjustment (matplotlib's `get_position`). Insets and
    /// secondary axes, which are placed in their host axes, are reported
    /// in the host's figure fractions too.
    pub fn position(&self) -> Bbox {
        let host = self.host().map(|host| host.position());
        self.apply_aspect();
        let active = self
            .raw
            .with::<AxesState, _>(|a| a.aspect.active_position(&a.position));
        host.map_or(active, |host| {
            active.transformed(&crate::transforms::unit_to_bbox(&host))
        })
    }

    /// The axes this one is placed in (insets, secondary axes).
    fn host(&self) -> Option<Self> {
        let figure = self.raw.figure_state();
        let parent = self.raw.with::<AxesState, _>(|a| a.base().parent)?;
        let is_axes = figure.borrow().store.get_as::<AxesState>(parent).is_some();
        is_axes.then(|| Self::from_raw(RawHandle::new(Rc::clone(figure), parent)))
    }

    /// Moves the axes and its twins to `[left, bottom, width, height]` in
    /// figure fractions, out of the layout engine's control (matplotlib's
    /// `set_position`).
    pub fn set_position(&self, rect: [f64; 4]) -> &Self {
        self.set_position_which(rect, WhichPosition::Both)
    }

    /// Moves the axes and its twins to `[left, bottom, width, height]` in
    /// figure fractions: the position before any aspect adjustment, the
    /// one drawn until the aspect is next applied, or both (matplotlib's
    /// `set_position(pos, which=...)`). The layout engine leaves this axes
    /// alone from now on.
    pub fn set_position_which(&self, rect: [f64; 4], which: WhichPosition) -> &Self {
        self.set_in_layout(false);
        let [left, bottom, width, height] = rect;
        let position = Bbox::from_bounds(left, bottom, width, height);
        for twin in self.twins() {
            twin.raw.with_mut::<AxesState, _>(|a| {
                if which != WhichPosition::Active {
                    a.position = position;
                }
                if which != WhichPosition::Original {
                    a.aspect.set_active(a.position, position);
                }
            });
        }
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
    /// `set_title`, this resets the gap of all titles to `axes.titlepad`,
    /// the title's height to `axes.titley` (which, when set, also stops
    /// lifting the titles above decorations on top) and its size, weight,
    /// colour and alignment to the `axes.title*` defaults; change them on
    /// the returned title, or with [`Axes::set_title_pad`] and
    /// [`Axes::set_title_y`], afterwards.
    pub fn set_title_at(&self, location: TitleLocation, text: impl Into<String>) -> Text {
        let rc = crate::rc::params();
        self.set_title_pad(rc.axes_titlepad);
        let title = self.title_handle(location);
        match rc.axes_titley {
            Some(y) => {
                self.set_title_y(location, y);
            }
            None => {
                title.set_y(1.0);
            }
        }
        title
            .set_text(text)
            .set_size(rc.axes_titlesize)
            .set_weight(rc.axes_titleweight)
            .set_valign(VAlign::Baseline)
            .set_halign(location.halign());
        if let ColorOrAuto::Color(color) = &rc.axes_titlecolor {
            title.set_color(color.clone());
        }
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

    /// Sets the z-order of the background patch (matplotlib's
    /// `ax.patch.set_zorder`). It matters only where a parasite axes
    /// shares the patch and draws it among its children; as the
    /// background the patch is always drawn first.
    pub fn set_patch_zorder(&self, zorder: f64) -> &Self {
        self.raw
            .with_mut::<AxesState, _>(|a| a.patch.zorder = zorder);
        self
    }

    /// The z-order of the background patch (1 by default).
    pub fn patch_zorder(&self) -> f64 {
        self.raw.with::<AxesState, _>(|a| a.patch.zorder)
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
