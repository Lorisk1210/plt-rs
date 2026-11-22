//! The axes, their x and y axes, and the plotting methods (`matplotlib.axes`).
//!
//! An [`Axes`] is one plotting area of a figure. Its page lists all plot
//! types and all settings, grouped by task. Start there. This module also
//! holds the types that the methods of `Axes` take and return.
//!
//! # The axes and its parts
//!
//! * [`Axes`]: a plotting area with an x axis and a y axis, and all plotting
//!   methods.
//! * [`Axis`]: one axis of an axes, with its ticks, tick labels, axis label,
//!   locators, formatters and units.
//! * [`AxisName`]: the name of the x axis or the y axis, for methods that
//!   act on either.
//! * [`TickSet`] and [`Tick`]: the ticks that an axis draws.
//! * [`TickLine`] and [`GridLine`]: one tick mark and one gridline.
//!
//! # Option structs
//!
//! Each plotting method takes the matplotlib keywords as one option struct.
//!
//! | Group | Types |
//! |-------|----------------|
//! | Lines and markers | [`ScatterOptions`], [`MappedColors`], [`StemOptions`], [`ErrorbarOptions`], [`ErrorSize`], [`EventplotOptions`] |
//! | Bars and histograms | [`BarOptions`], [`GroupedBarOptions`], [`BarLabelOptions`], [`HistPlotOptions`], [`Hist2dOptions`], [`StairsOptions`], [`EcdfOptions`] |
//! | Statistics | [`BoxplotOptions`], [`BxpOptions`], [`ViolinplotOptions`], [`ViolinDrawOptions`] |
//! | Areas | [`FillBetweenOptions`], [`StackplotOptions`] |
//! | Images and meshes | [`ImshowOptions`], [`NonUniformOptions`], [`Mesh`], [`RgbaMesh`], [`PcolorOptions`], [`PcolorfastOptions`], [`HexbinOptions`], [`SpyOptions`] |
//! | Triangulations | [`TriValues`], [`TripcolorOptions`] |
//! | Spectral plots | [`SpectralPlotOptions`], [`SpectrumPlotOptions`], [`SpecgramOptions`] |
//! | Pies | [`PieOptions`], [`PieLabelOptions`] |
//! | Ticks and grid | [`TickParams`], [`LocatorParams`], [`TicklabelFormat`], [`GridOptions`] |
//! | Limits, aspect and position | [`Limits`], [`Aspect`], [`Adjustable`], [`AxisMode`], [`SecondaryLocation`], [`TitleLocation`] |
//!
//! The options of contours, vector fields, tables, legends and texts are in
//! their own modules, for example [`ContourOptions`] and [`LegendOptions`].
//!
//! # Results
//!
//! A plotting method returns a handle, such as a [`Line2D`] or a
//! [`Collection`], or a struct with named fields:
//!
//! * [`BarContainer`], [`ErrorbarContainer`], [`StemContainer`] and
//!   [`PieContainer`], which a legend shows as one entry.
//! * [`HistPlot`], [`Hist2d`], [`BoxplotArtists`], [`ViolinParts`] and
//!   [`SpecgramPlot`], with the values that the method calculated.
//! * [`PcolorFast`], [`Spy`], [`Tripcolor`], [`TriplotLines`] and
//!   [`CorrelationArtists`].
//!
//! # Errors
//!
//! Most methods return a [`PlotError`]. Some methods have an error of their
//! own, such as [`AspectError`], [`BarLabelError`], [`EcdfError`],
//! [`GroupedBarError`], [`PieError`], [`SpecgramError`] and
//! [`TripcolorError`].
//!
//! [`ContourOptions`]: crate::contour::ContourOptions
//! [`LegendOptions`]: crate::legend::LegendOptions
//! [`Line2D`]: crate::lines::Line2D
//! [`Collection`]: crate::collections::Collection

mod add_artist;
mod annotate;
mod aspect;
pub(crate) mod axis;
pub(crate) mod axis_artists;
mod axis_clear;
pub(crate) mod axis_draw;
mod axis_effects;
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
mod failed_calls;
mod figure_reset;
mod fill;
mod fill_between;
mod grid_props;
mod grid_stroke;
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
mod program_axis;
pub(crate) mod projection;
mod projection_sites;
mod prop_cycles;
mod quiver;
mod reference;
mod scatter;
mod secondary;
mod segments;
mod share;
mod share_across;
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
mod tick_sites;
#[cfg(test)]
mod tick_tests;
mod tick_updates;
mod ticks;
mod tight_bbox;
mod title_position;
mod titles;
mod transform_sites;
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
pub use aspect::{Adjustable, AspectError, AxesLocatorFn};
pub(crate) use aspect::{AspectInputs, AxesLocator};
pub use axis::TickHelperKind;
pub(crate) use axis::{TickerDefaults, Tickers};
pub use axis_artists::{AxisGroup, TickPart};
pub use axis_handle::Axis;
pub(crate) use axis_layout::{PlacedText, place_text};
pub use axis_mode::AxisMode;
pub(crate) use axis_ticks::tick_label_formatter;
pub use bar_label::{BarLabelError, BarLabelOptions, BarLabelType};
pub use bars::{BandAlign, BandRange, BarAlign, BarContainer, BarOptions};
pub use box_labels::BoxLabel;
pub use boxplot::BoxplotOptions;
pub use bxp::{BoxArtist, BoxPart, BoxProps, BoxplotArtists, BxpOptions};
pub(crate) use children::DataUpdate;
pub use current_image::ColorizingArtist;
pub use dataset_labels::DatasetLabels;
pub use ecdf::{EcdfError, EcdfOptions};
pub use error::PlotError;
pub use error_size::ErrorSize;
pub(crate) use errorbar::without_marker_style;
pub use errorbar::{ErrorbarContainer, ErrorbarFormat, ErrorbarOptions};
pub use eventplot::EventplotOptions;
pub use failed_calls::CallMark;
pub use fill_between::FillBetweenOptions;
pub(crate) use fill_between::FillGeometry;
pub use grid_props::{GridClip, GridLineProps, GridLinePropsOptions};
pub use grid_stroke::{GridStroke, GridStrokeOptions};
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
pub use program_axis::ProgramAxis;
pub(crate) use program_axis::ProgramAxisArtist;
pub use projection::ProjectionKind;
pub use prop_cycles::PropCycle;
pub use scatter::{MappedColors, ScatterColors, ScatterOptions};
pub use secondary::SecondaryLocation;
pub(crate) use share::{Grouper, ShareGroups};
pub(crate) use share_across::SharesAcross;
pub(crate) use sharing::share_free;
pub use specgram::{SpecgramError, SpecgramOptions, SpecgramPlot};
pub use spectral::{CorrelationArtists, SpectralPlotOptions, SpectrumPlotOptions, SpectrumScale};
pub use spy::{Spy, SpyOptions};
pub use stackplot::{Baseline, StackplotOptions};
pub use state::TitlePlacement;
pub(crate) use state::{AxesState, PatchOutline};
pub(crate) use stem::solid_format;
pub use stem::{StemContainer, StemOptions};
pub use steps::StairsOptions;
pub(crate) use tick_compute::axis_length_points;
pub use tick_config::{LocatorParams, TickLabelProps, TickParams, TicklabelFormat};
pub use tick_lines::{GridLine, TickLine, TickLineProps, TickMarkProps};
pub use tick_set::{Labelling, Tick, TickSet};
pub use tick_updates::{
    SpineAxis, TickLevels, TickUpdater, UpdateSite, UpdatedLevel, UpdatedTicks,
};
pub use ticks::{GridOptions, GridStyle, Sides, TickDirection, TickPosition, TickStyle};
pub(crate) use ticks::{grid_level_on, tick_sides};
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

/// One plotting area of a figure, with an x axis and a y axis.
///
/// An axes maps data coordinates to a rectangle in its figure. It holds the
/// x axis and the y axis ([`Axis`]), the plotted artists, the titles, the
/// spines and the legend. `Axes` is a handle. A clone is cheap and refers to
/// the same axes. A [`Figure`] makes axes. [`Figure::subplot`] gives one axes
/// that fills the figure, [`Figure::subplots`] gives a grid, and
/// [`Figure::add_axes`] gives an axes at a rectangle that you select.
///
/// ```
/// use pltrs::prelude::*;
///
/// let fig = Figure::new();
/// let ax = fig.subplot();
/// ax.plot([0.0, 1.0, 2.0, 3.0], [0.0, 1.0, 4.0, 9.0])?;
/// ax.set_title("Squares");
/// ax.set_xlabel("n").set_ylabel("n²");
/// ax.set_xlim(0.0, 3.0)?.set_ylim(0.0, 10.0)?;
/// assert_eq!(ax.ylim(), (0.0, 10.0));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Method names
///
/// The names follow the rules in [`porting::conventions`]:
///
/// * A plotting method has the matplotlib name and returns the artists that
///   it made. If the method can fail, it returns a `Result` with an error
///   enum such as [`PlotError`].
/// * The matplotlib keywords are the public fields of an option struct. An
///   unset field takes its rcParam. Fill the struct with
///   `..Default::default()`.
/// * Most `_with` methods take an option struct that the plain method does
///   not take: [`plot_with`] takes a [`LineProps`], and [`scatter_with`] takes
///   a [`ScatterOptions`].
/// * Getters drop `get_`: [`xlim`] is `get_xlim`. Setters keep `set_` and
///   return `&Self`, so calls chain. A setter that can refuse its value
///   returns `Result<&Self, _>`: `ax.set_xlim(0.0, 1.0)?.set_ylim(0.0, 2.0)?`.
/// * A method for either axis takes an [`AxisName`]: `margin(AxisName::X)` is
///   `get_xmargin`. The common pairs keep their x and y names, such as
///   [`set_xlim`] and [`set_ylim`].
/// * Data is [`AxisValues`]: numbers of any type, dates, category names or
///   values with units. One value is an [`AxisValue`].
///
/// # Plot types
///
/// Each table lists the plotting methods of one group. The last column names
/// the option struct or the special argument of the method.
///
/// ## Lines and markers
///
/// | Method | What it draws | Options |
/// |--------|---------------|---------|
/// | [`plot`] | A line through points, styled by the property cycle | |
/// | [`plot_y`] | A line through `y` against the indices 0, 1, 2, ... | |
/// | [`plot_fmt`] | A line styled by a format string such as `"ro--"` | |
/// | [`plot_with`] | A line with the properties that you set | [`LineProps`] |
/// | [`plot_scaled`] | A line that autoscales only the axes that you select | [`LineProps`] |
/// | [`plot_columns`] | One line for each column of a table | [`LineProps`] |
/// | [`loglog`], [`semilogx`], [`semilogy`] | A line on logarithmic axes | |
/// | [`scatter`] | Markers at points | |
/// | [`scatter_with`] | Markers with a size and a color for each point | [`ScatterOptions`] |
/// | [`step`] | A line drawn as steps | [`StepWhere`], [`LineProps`] |
/// | [`stem`] | A line from a baseline to each value, with a marker | [`StemOptions`] |
/// | [`errorbar`] | Points with error bars | [`ErrorbarOptions`] |
/// | [`eventplot`] | A row of short lines at the events of each dataset | [`EventplotOptions`] |
/// | [`hlines`], [`vlines`] | Horizontal or vertical segments | |
/// | [`hlines_in`], [`vlines_in`] | The same segments in other coordinates | [`Transform`] |
///
/// ## Reference lines and spans
///
/// | Method | What it draws | Options |
/// |--------|---------------|---------|
/// | [`axhline`], [`axvline`] | A horizontal or vertical line across the axes | [`LineProps`] |
/// | [`axhline_between`], [`axvline_between`] | The same line between two axes fractions | [`LineProps`] |
/// | [`axline`] | An infinite line through two points, or through one point with a slope | [`AxLineThrough`] |
/// | [`axhspan`], [`axvspan`] | A horizontal or vertical band across the axes | |
/// | [`axhspan_between`], [`axvspan_between`] | The same band between two axes fractions | |
///
/// ## Bars and histograms
///
/// | Method | What it draws | Options |
/// |--------|---------------|---------|
/// | [`bar`], [`barh`] | Vertical or horizontal bars | [`BarOptions`] |
/// | [`grouped_bar`] | One group of bars for each category, one bar for each dataset | [`GroupedBarOptions`] |
/// | [`bar_label`] | A label at each bar of a [`BarContainer`] | [`BarLabelOptions`] |
/// | [`broken_barh`] | A horizontal band of rectangles | [`BandRange`], [`BandAlign`] |
/// | [`hist`] | A histogram of one dataset, or of several datasets | [`HistPlotOptions`] |
/// | [`hist2d`] | A 2-D histogram as a color mesh | [`Hist2dOptions`] |
/// | [`stairs`] | A stepwise constant function as one patch | [`StairsOptions`] |
/// | [`ecdf`] | The empirical cumulative distribution as a step line | [`EcdfOptions`] |
///
/// ## Statistics
///
/// | Method | What it draws | Options |
/// |--------|---------------|---------|
/// | [`boxplot`] | One box for each dataset: quartiles, whiskers, median and outliers | [`BoxplotOptions`] |
/// | [`bxp`] | Boxes from statistics that you calculated ([`BoxStats`]) | [`BxpOptions`] |
/// | [`violinplot`] | One violin for each dataset, from a kernel density estimate | [`ViolinplotOptions`] |
/// | [`violin`] | Violins from statistics that you calculated ([`ViolinStats`]) | [`ViolinDrawOptions`] |
///
/// ## Areas
///
/// | Method | What it draws | Options |
/// |--------|---------------|---------|
/// | [`fill`] | A filled polygon | [`PatchProps`] |
/// | [`fill_with_format_color`] | A filled polygon in the color of a format string | [`PatchProps`] |
/// | [`fill_between`] | The area between two curves along x | [`FillBetweenOptions`] |
/// | [`fill_betweenx`] | The area between two curves along y | [`FillBetweenOptions`] |
/// | [`stackplot`] | Layers stacked on top of each other | [`StackplotOptions`] |
///
/// ## Images and meshes
///
/// | Method | What it draws | Options |
/// |--------|---------------|---------|
/// | [`imshow`] | An image on a regular grid, color-mapped or in RGB(A) | [`ImshowOptions`] |
/// | [`matshow`] | A matrix with row 0 at the top and square cells | [`ImshowOptions`] |
/// | [`nonuniform_image`] | An image with pixel centers at uneven coordinates | [`NonUniformOptions`] |
/// | [`pcolormesh`] | A color mesh of quadrilaterals | [`Mesh`], [`PcolorOptions`] |
/// | [`pcolor`] | A color mesh with one polygon for each cell | [`Mesh`], [`PcolorOptions`] |
/// | [`pcolormesh_rgba`], [`pcolor_rgba`] | The same meshes in colors that you give | [`RgbaMesh`], [`PcolorOptions`] |
/// | [`pcolorfast`] | A color mesh drawn as an image where the grid permits | [`PcolorfastOptions`] |
/// | [`hexbin`] | Points binned into hexagons | [`HexbinOptions`] |
/// | [`spy`] | The non-zero entries of a matrix | [`SpyOptions`] |
/// | [`add_bbox_image`] | An image stretched over a box | [`BboxImageOptions`] |
/// | [`add_bbox_image_over`] | An image stretched over the box of another artist | [`BboxImageOptions`] |
///
/// ## Contours and triangulations
///
/// | Method | What it draws | Options |
/// |--------|---------------|---------|
/// | [`contour`], [`contourf`] | Contour lines or filled contours of a [`QuadField`] | [`ContourOptions`] |
/// | [`contour_segments`] | Contours from segments that you calculated | [`ContourOptions`] |
/// | [`tricontour`], [`tricontourf`] | Contour lines or filled contours on a [`Triangulation`] | [`ContourOptions`] |
/// | [`tricontour_bools`], [`tricontourf_bools`] | The same contours of one boolean for each point | [`ContourOptions`] |
/// | [`tripcolor`] | Triangles colored by values | [`TriValues`], [`TripcolorOptions`] |
/// | [`triplot`] | The edges and points of a triangulation | format string, [`LineProps`] |
///
/// To label contour lines, call [`ContourSet::clabel`] with a
/// [`ClabelOptions`].
///
/// ## Vector fields
///
/// | Method | What it draws | Options |
/// |--------|---------------|---------|
/// | [`quiver`] | An arrow for each vector of a [`VectorField`] | [`QuiverOptions`] |
/// | [`quiverkey`] | A key arrow with a label for a quiver plot | [`QuiverKeyOptions`] |
/// | [`barbs`] | A wind barb for each vector | [`BarbsOptions`] |
/// | [`streamplot`] | Streamlines of a [`StreamField`] with arrows | [`StreamplotOptions`] |
///
/// [`removed_quiver`], [`removed_barbs`] and [`removed_quiverkey`] make the
/// same artists outside the figure. Add them later with [`adopt_artist`].
///
/// ## Spectral plots
///
/// | Method | What it draws | Options |
/// |--------|---------------|---------|
/// | [`psd`] | The power spectral density | [`SpectralPlotOptions`] |
/// | [`csd`] | The cross spectral density of two signals | [`SpectralPlotOptions`] |
/// | [`cohere`] | The coherence of two signals | [`SpectralPlotOptions`] |
/// | [`specgram`] | A spectrogram as an image | [`SpecgramOptions`] |
/// | [`magnitude_spectrum`] | The magnitude spectrum | [`SpectrumScale`], [`SpectrumPlotOptions`] |
/// | [`angle_spectrum`] | The wrapped phase spectrum | [`SpectrumPlotOptions`] |
/// | [`phase_spectrum`] | The unwrapped phase spectrum | [`SpectrumPlotOptions`] |
/// | [`acorr`] | The autocorrelation of a signal | [`XcorrOptions`], [`LineProps`] |
/// | [`xcorr`] | The cross-correlation of two signals | [`XcorrOptions`], [`LineProps`] |
///
/// ## Pies, tables and diagrams
///
/// | Method | What it draws | Options |
/// |--------|---------------|---------|
/// | [`pie`] | A pie chart | [`PieOptions`] |
/// | [`pie_label`] | A label at each wedge of a [`PieContainer`] | [`PieLabelOptions`] |
/// | [`table`] | A table of texts | [`TableOptions`] |
///
/// A Sankey diagram is not a method of the axes. [`Sankey::new`] draws one
/// on an axes.
///
/// # Text
///
/// [`text`] puts a text at a data position. [`annotate`] puts a text with an
/// optional arrow to a point. [`Axis`] has the settings of the axis labels,
/// such as [`Axis::set_label_loc`].
///
/// | Task | Methods |
/// |------|---------|
/// | Titles | [`set_title`], [`set_title_at`], [`title`], [`title_text`], [`shown_title`] |
/// | Title position | [`set_title_pad`], [`title_pad`], [`set_title_y`], [`set_title_placement`], [`title_placement`], [`update_title_position`], [`lift_titles`] |
/// | Axis labels | [`set_xlabel`], [`set_ylabel`], [`label`] |
/// | Texts and annotations | [`text`], [`annotate`] with [`AnnotateOptions`] |
/// | Arrows | [`arrow`] with a [`FancyArrow`], or [`annotate`] with an arrow |
/// | Boxes of texts and other artists | [`add_anchored_box`], [`add_annotation_bbox`], [`add_offset_box`] |
///
/// # Legend and colorbar
///
/// [`legend`] makes the legend of the axes from the labeled artists. A
/// colorbar is a separate axes. [`Figure::colorbar`] makes it from an image
/// or another color-mapped artist.
///
/// | Task | Methods |
/// |------|---------|
/// | Make the legend | [`legend`] with [`LegendOptions`] |
/// | Read the legend | [`current_legend`], [`legend_handles_labels`], [`legend_candidates`] |
/// | Show several legends | [`keep_legend`], [`set_current_legend`] |
/// | The artist that a colorbar uses | [`current_image`], [`set_current_image`] |
///
/// # Limits, margins and autoscaling
///
/// The axes fits its view limits to the data, with a margin. A limit that you
/// set turns autoscaling off for that axis. To invert an axis, give the
/// larger limit first or call [`Axis::set_inverted`].
///
/// | Task | Methods |
/// |------|---------|
/// | Set and read the limits | [`set_xlim`], [`set_ylim`], [`xlim`], [`ylim`] |
/// | Set one limit, or set limits quietly | [`set_limits_with`] with [`Limits`], [`set_limits_quietly`], [`set_limits`] |
/// | Bounds in increasing order | [`set_bound`], [`bound`] |
/// | Margins | [`set_margins`], [`set_margins_with`], [`set_margin`], [`margins`], [`margin`] |
/// | Autoscaling | [`autoscale`], [`autoscale_with`], [`set_autoscale_on`], [`set_axis_autoscale_on`], [`autoscale_on`] |
/// | Autoscale now | [`autoscale_view`], [`autoscale_view_along`], [`autoscales_tight`] |
/// | Round the view to ticks | [`set_autolimit_mode`], [`autolimit_mode`] |
/// | Sticky edges | [`set_use_sticky_edges`], [`use_sticky_edges`] |
/// | Data limits | [`data_lim`], [`relim`], [`update_datalim`], [`has_data`], [`set_ignores_data_limits`], [`ignores_data_limits`] |
/// | React to new limits | [`on_limits_changed`], [`disconnect_limits_changed`] |
///
/// # Scales
///
/// A scale maps the values of an axis to positions. The names are
/// [`ScaleName::Linear`], [`ScaleName::Log`], [`ScaleName::Symlog`],
/// [`ScaleName::Logit`], [`ScaleName::Asinh`] and the function scales
/// [`ScaleName::Function`] and [`ScaleName::FunctionLog`]. A function scale
/// takes its two functions in the field `functions` of [`ScaleOptions`].
///
/// | Task | Methods |
/// |------|---------|
/// | Set a scale by name | [`set_xscale`], [`set_yscale`] |
/// | Set a scale with options, such as a log base | [`set_scale_with`] with [`ScaleOptions`] |
/// | Set a scale object, such as your own [`Scale`] | [`set_scale`] |
/// | Read the scale | [`xscale`], [`yscale`] |
/// | Plot and set log scales | [`loglog`], [`semilogx`], [`semilogy`] |
///
/// # Ticks
///
/// The axes selects the ticks with a locator and labels them with a
/// formatter. Each [`Axis`] holds its own, for example
/// [`Axis::set_major_locator`] and [`Axis::set_major_formatter`]. Get the
/// axis with [`xaxis`] or [`yaxis`]. The module
/// [`ticker`] has the locators and formatters.
///
/// | Task | Methods |
/// |------|---------|
/// | The [`Axis`] of x or y | [`xaxis`], [`yaxis`], [`axis_of`] |
/// | Ticks at fixed positions | [`set_xticks`], [`set_yticks`], [`Axis::set_minor_ticks`] |
/// | Fixed tick labels | [`set_xticklabels`], [`set_yticklabels`] |
/// | Ticks as drawn now | [`ticks`], [`Axis::ticklocs`], [`Axis::ticklabels`] |
/// | Minor ticks | [`minorticks_on`], [`minorticks_off`] |
/// | Size, direction, color and labels of the ticks | [`tick_params`] with [`TickParams`] |
/// | Number of ticks | [`locator_params`] with [`LocatorParams`] |
/// | Scientific notation and offsets | [`ticklabel_format`] with [`TicklabelFormat`] |
/// | Ticks on the top or right side | [`Axis::tick_to`], [`Axis::set_ticks_position`] |
///
/// # Dates, categories and units
///
/// The plotting methods take dates, category names and values with units
/// directly. The axis then gets a converter and matching ticks. The module
/// [`dates`] has the date types, locators and formatters.
///
/// | Task | Methods |
/// |------|---------|
/// | Date ticks | [`xaxis_date`], [`yaxis_date`], [`Axis::axis_date`] |
/// | Categories | [`Axis::categories`], [`Axis::set_categories`] |
/// | Units and converters | [`Axis::set_units`], [`Axis::set_converter`], [`Axis::update_units`] |
/// | Convert a data point to axis numbers | [`convert_point`] |
///
/// # Grid, spines and the axes frame
///
/// The frame of an axes is its background patch and its four spines.
///
/// | Task | Methods |
/// |------|---------|
/// | Gridlines | [`grid`], [`grid_with`] with [`GridOptions`], [`has_grid`] |
/// | Ticks and gridlines above or below the artists | [`set_axisbelow`], [`axisbelow`] |
/// | Spines | [`spines`], then [`Spine::set_position`] or [`Spine::set_bounds`] on one [`Spine`] |
/// | Spines that you make | [`new_spine`], [`add_spine`], [`replace_spine`] |
/// | Hide or show the frame | [`set_frame_on`], [`frame_on`] |
/// | Hide or show the frame and both axes | [`set_axis_off`], [`set_axis_on`], [`axis_on`] |
/// | Presets such as `"equal"` or `"off"` | [`axis_mode`] with [`AxisMode`] |
/// | Background color | [`set_facecolor`], [`facecolor`] |
/// | Background patch | [`set_patch_visible`], [`set_patch_edgecolor`], [`set_patch_linewidth`], [`set_patch_alpha`], [`set_patch_style`], [`set_patch_zorder`], [`set_patch_gid`] |
/// | Read the background patch | [`patch_visible`], [`patch_edgecolor`], [`patch_linewidth`], [`patch_alpha`], [`patch_style`], [`patch_zorder`], [`patch_gid`] |
/// | Background in another shape | [`set_patch_outline`], [`patch_outline`], [`clips_to_patch_shape`] |
///
/// # Aspect and position
///
/// The aspect is the ratio of y to x data units on the screen. The position
/// is the rectangle of the axes in figure fractions.
///
/// | Task | Methods |
/// |------|---------|
/// | Aspect | [`set_aspect`] with [`Aspect`], [`aspect`], [`set_shared_aspect`], [`data_ratio`] |
/// | What changes for the aspect | [`set_adjustable`] with [`Adjustable`], [`adjustable`] |
/// | Fixed box shape | [`set_box_aspect`], [`box_aspect`] |
/// | Position of a smaller box | [`set_anchor`], [`anchor`] |
/// | Apply the aspect now | [`apply_aspect`], [`apply_aspect_at`] |
/// | Position | [`position`], [`try_position`], [`set_position`], [`set_position_which`], [`set_position_box`] |
/// | Position before the aspect | [`original_position`], [`reset_position`], [`located_position`] |
/// | Grid cell | [`set_subplot_spec`], [`subplot_spec`] |
/// | Position from a function | [`set_axes_locator`] with [`AxesLocatorFn`], [`axes_locator`] |
/// | Labels of a subplot grid | [`label_outer`], [`label_outer_with`] |
/// | Extent in pixels | [`bbox`], [`tight_bbox`], [`decorations_tight_bbox`] |
///
/// # Shared, twin and secondary axes
///
/// Axes that share an axis show the same range on it. A twin axes lies on top
/// of this axes and shares one axis with it. A secondary axis shows the range
/// of this axes through a pair of functions.
///
/// | Task | Methods |
/// |------|---------|
/// | Share an axis | [`share_x`], [`share_y`], [`share_with`], [`leave_sharing`] |
/// | Read the shares | [`shared_axes`], [`sharing`], [`shared_from`] |
/// | Share across figures | [`join_across`], [`shared_across`], [`shared_from_across`] |
/// | Share limits only | [`join_sharing`] |
/// | Twin axes | [`twinx`], [`twiny`], [`twin_with`], [`twin_with_projection`], [`twins`] |
/// | Secondary axes | [`secondary_xaxis`], [`secondary_yaxis`] with [`SecondaryLocation`] |
/// | Change a secondary axis | [`set_secondary_functions`], [`set_secondary_location`] |
///
/// # Insets and child axes
///
/// An inset is an axes inside this axes. It moves with this axes.
///
/// | Task | Methods |
/// |------|---------|
/// | Make an inset | [`inset_axes`], [`inset_axes_in`], [`inset_axes_with_projection`] |
/// | Make a polar or a geographic inset | [`polar_inset_axes`], [`geo_inset_axes`] |
/// | Mark the area of a zoomed inset | [`indicate_inset_zoom`], [`indicate_inset`] |
/// | Read and add child axes | [`child_axes`], [`add_child_axes`], [`place_inset`] |
///
/// # Add your own artists
///
/// The plotting methods make artists for you. These methods add artists that
/// you make, for example a [`Patch`] shape or a collection.
///
/// | Task | Methods |
/// |------|---------|
/// | Patches | [`add_patch`], [`add_patch_with`], [`add_artist`], [`add_artist_with`] |
/// | Lines | [`add_line`] |
/// | Collections | [`add_collection`] with [`CollectionSpec`], [`add_trimesh`] with [`TriMeshSpec`] |
/// | Artists that implement [`Artist`] | [`add_custom_artist`] |
/// | Containers for the legend | [`add_container`] |
/// | Removed artists, or artists made outside the figure | [`attach`], [`adopt_artist`] with [`AddAs`] |
/// | Rasterize the lower artists in vector output | [`set_rasterization_zorder`], [`rasterization_zorder`] |
///
/// # Colors and the property cycle
///
/// The property cycle gives each new line or patch its next color and style.
///
/// | Task | Methods |
/// |------|---------|
/// | Replace the cycle | [`set_prop_cycle`] with a [`Cycler`], [`set_prop_cycle_of`] |
/// | Position in the cycle | [`prop_cycle_position`], [`set_prop_cycle_position`] |
/// | Next color | [`next_line_color`], [`next_patch_color`] |
/// | Use the cycle of another axes | [`share_prop_cycle`] |
///
/// # Find, remove and clear artists
///
/// The lists give the artists in the order that you added them. To remove an
/// artist, call [`ArtistHandle::remove`] on its handle.
/// [`ArtistHandle::findobj`] searches the artists of the axes.
///
/// | Task | Methods |
/// |------|---------|
/// | Artists by kind | [`lines`], [`patches`], [`collections`], [`texts`], [`images`], [`data_images`], [`tables`], [`artists`], [`containers`] |
/// | All artists | [`added_children`] |
/// | Clear the axes | [`clear`], [`cla`] |
///
/// # Coordinates and transforms
///
/// A [`Transform`] names a coordinate system. Artists, texts and insets take
/// one to place themselves.
///
/// | matplotlib | pltrs | Coordinates |
/// |------------|-------|-------------|
/// | `ax.transData` | [`trans_data`] | Data |
/// | `ax.transAxes` | [`trans_axes`] | Axes fractions, (0, 0) at the lower left |
/// | `ax.get_xaxis_transform()` | [`trans_xaxis`] | x in data, y in axes fractions |
/// | `ax.get_yaxis_transform()` | [`trans_yaxis`] | x in axes fractions, y in data |
/// | `ax.transData.transform(p)` | [`data_to_display`] | Display pixels |
///
/// Annotations take their coordinate systems as a [`Coords`].
///
/// # Interaction
///
/// In a figure window, the toolbar pans and zooms the axes and shows the data
/// position under the pointer. These methods control that behavior. Picking
/// starts with [`ArtistHandle::set_picker`] on an artist.
///
/// | Task | Methods |
/// |------|---------|
/// | Let the toolbar act on the axes | [`set_navigate`], [`navigate`], [`can_pan`], [`can_zoom`] |
/// | Pan and zoom from your code | [`start_pan`], [`zoom_view`], [`zoomed_bounds`] |
/// | Pan and zoom the axes below | [`set_forward_navigation_events`], [`forward_navigation_events`] |
/// | Coordinate readout | [`format_coord`], [`format_data`], [`set_fmt_data`], [`cursor_text`], [`pixel_resolution`] |
/// | Hit test | [`contains_point`] |
/// | Draw quickly for blitting | [`draw_artist`], [`redraw_in_frame`] |
///
/// # Projections
///
/// An axes draws through a projection. [`Figure::polar_subplot`] gives a
/// [`PolarAxes`], and [`Figure::geo_subplot`] gives a [`GeoAxes`]. Both
/// dereference to `Axes`. 3-D plots use [`Axes3D`].
///
/// | Task | Methods |
/// |------|---------|
/// | Kind of projection | [`projection`] |
/// | Polar or geographic settings | [`as_polar`], [`as_geo`] |
/// | Projections that you define | [`install_projection`], [`with_projection`], [`with_projection_mut`] |
///
/// # Methods for programs that wrap pltrs
///
/// pltrs (Python) uses these methods to give matplotlib objects to Python
/// code. Most Rust programs do not need them.
///
/// | Purpose | Methods |
/// |---------|---------|
/// | An image filter on a group of artists | [`add_filtered_group`] |
/// | Parts of an `axisartist` axis | [`add_axis_texts`], [`add_tick_marks`] |
/// | Axes that your program draws | [`add_program_axis`], [`program_axes`] |
/// | Undo a call that failed | [`call_mark`], [`undo_since`] |
///
/// [`Figure`]: crate::figure::Figure
/// [`Figure::subplot`]: crate::figure::Figure::subplot
/// [`Figure::subplots`]: crate::figure::Figure::subplots
/// [`Figure::add_axes`]: crate::figure::Figure::add_axes
/// [`porting::conventions`]: crate::porting::conventions
/// [`plot_with`]: Self::plot_with
/// [`LineProps`]: crate::lines::LineProps
/// [`scatter_with`]: Self::scatter_with
/// [`xlim`]: Self::xlim
/// [`set_xlim`]: Self::set_xlim
/// [`set_ylim`]: Self::set_ylim
/// [`AxisValues`]: crate::units::AxisValues
/// [`plot`]: Self::plot
/// [`plot_y`]: Self::plot_y
/// [`plot_fmt`]: Self::plot_fmt
/// [`plot_scaled`]: Self::plot_scaled
/// [`plot_columns`]: Self::plot_columns
/// [`loglog`]: Self::loglog
/// [`semilogx`]: Self::semilogx
/// [`semilogy`]: Self::semilogy
/// [`scatter`]: Self::scatter
/// [`step`]: Self::step
/// [`StepWhere`]: crate::lines::StepWhere
/// [`stem`]: Self::stem
/// [`errorbar`]: Self::errorbar
/// [`eventplot`]: Self::eventplot
/// [`hlines`]: Self::hlines
/// [`vlines`]: Self::vlines
/// [`hlines_in`]: Self::hlines_in
/// [`vlines_in`]: Self::vlines_in
/// [`axhline`]: Self::axhline
/// [`axvline`]: Self::axvline
/// [`axhline_between`]: Self::axhline_between
/// [`axvline_between`]: Self::axvline_between
/// [`axline`]: Self::axline
/// [`AxLineThrough`]: crate::lines::AxLineThrough
/// [`axhspan`]: Self::axhspan
/// [`axvspan`]: Self::axvspan
/// [`axhspan_between`]: Self::axhspan_between
/// [`axvspan_between`]: Self::axvspan_between
/// [`bar`]: Self::bar
/// [`barh`]: Self::barh
/// [`grouped_bar`]: Self::grouped_bar
/// [`bar_label`]: Self::bar_label
/// [`broken_barh`]: Self::broken_barh
/// [`hist`]: Self::hist
/// [`hist2d`]: Self::hist2d
/// [`stairs`]: Self::stairs
/// [`ecdf`]: Self::ecdf
/// [`boxplot`]: Self::boxplot
/// [`bxp`]: Self::bxp
/// [`BoxStats`]: crate::stats::BoxStats
/// [`violinplot`]: Self::violinplot
/// [`violin`]: Self::violin
/// [`ViolinStats`]: crate::stats::ViolinStats
/// [`fill`]: Self::fill
/// [`PatchProps`]: crate::patches::PatchProps
/// [`fill_with_format_color`]: Self::fill_with_format_color
/// [`fill_between`]: Self::fill_between
/// [`fill_betweenx`]: Self::fill_betweenx
/// [`stackplot`]: Self::stackplot
/// [`imshow`]: Self::imshow
/// [`matshow`]: Self::matshow
/// [`nonuniform_image`]: Self::nonuniform_image
/// [`pcolormesh`]: Self::pcolormesh
/// [`pcolor`]: Self::pcolor
/// [`pcolormesh_rgba`]: Self::pcolormesh_rgba
/// [`pcolor_rgba`]: Self::pcolor_rgba
/// [`pcolorfast`]: Self::pcolorfast
/// [`hexbin`]: Self::hexbin
/// [`spy`]: Self::spy
/// [`add_bbox_image`]: Self::add_bbox_image
/// [`BboxImageOptions`]: crate::image::BboxImageOptions
/// [`add_bbox_image_over`]: Self::add_bbox_image_over
/// [`contour`]: Self::contour
/// [`contourf`]: Self::contourf
/// [`QuadField`]: crate::contour::QuadField
/// [`ContourOptions`]: crate::contour::ContourOptions
/// [`contour_segments`]: Self::contour_segments
/// [`tricontour`]: Self::tricontour
/// [`tricontourf`]: Self::tricontourf
/// [`Triangulation`]: crate::tri::Triangulation
/// [`tricontour_bools`]: Self::tricontour_bools
/// [`tricontourf_bools`]: Self::tricontourf_bools
/// [`tripcolor`]: Self::tripcolor
/// [`triplot`]: Self::triplot
/// [`ContourSet::clabel`]: crate::contour::ContourSet::clabel
/// [`ClabelOptions`]: crate::contour::ClabelOptions
/// [`quiver`]: Self::quiver
/// [`VectorField`]: crate::quiver::VectorField
/// [`QuiverOptions`]: crate::quiver::QuiverOptions
/// [`quiverkey`]: Self::quiverkey
/// [`QuiverKeyOptions`]: crate::quiver::QuiverKeyOptions
/// [`barbs`]: Self::barbs
/// [`BarbsOptions`]: crate::quiver::BarbsOptions
/// [`streamplot`]: Self::streamplot
/// [`StreamField`]: crate::streamplot::StreamField
/// [`StreamplotOptions`]: crate::streamplot::StreamplotOptions
/// [`removed_quiver`]: Self::removed_quiver
/// [`removed_barbs`]: Self::removed_barbs
/// [`removed_quiverkey`]: Self::removed_quiverkey
/// [`adopt_artist`]: Self::adopt_artist
/// [`psd`]: Self::psd
/// [`csd`]: Self::csd
/// [`cohere`]: Self::cohere
/// [`specgram`]: Self::specgram
/// [`magnitude_spectrum`]: Self::magnitude_spectrum
/// [`angle_spectrum`]: Self::angle_spectrum
/// [`phase_spectrum`]: Self::phase_spectrum
/// [`acorr`]: Self::acorr
/// [`XcorrOptions`]: crate::mlab::XcorrOptions
/// [`xcorr`]: Self::xcorr
/// [`pie`]: Self::pie
/// [`pie_label`]: Self::pie_label
/// [`table`]: Self::table
/// [`TableOptions`]: crate::table::TableOptions
/// [`Sankey::new`]: crate::sankey::Sankey::new
/// [`text`]: Self::text
/// [`annotate`]: Self::annotate
/// [`set_title`]: Self::set_title
/// [`set_title_at`]: Self::set_title_at
/// [`title`]: Self::title
/// [`title_text`]: Self::title_text
/// [`shown_title`]: Self::shown_title
/// [`set_title_pad`]: Self::set_title_pad
/// [`title_pad`]: Self::title_pad
/// [`set_title_y`]: Self::set_title_y
/// [`set_title_placement`]: Self::set_title_placement
/// [`title_placement`]: Self::title_placement
/// [`update_title_position`]: Self::update_title_position
/// [`lift_titles`]: Self::lift_titles
/// [`set_xlabel`]: Self::set_xlabel
/// [`set_ylabel`]: Self::set_ylabel
/// [`label`]: Self::label
/// [`AnnotateOptions`]: crate::text::AnnotateOptions
/// [`arrow`]: Self::arrow
/// [`FancyArrow`]: crate::patches::FancyArrow
/// [`add_anchored_box`]: Self::add_anchored_box
/// [`add_annotation_bbox`]: Self::add_annotation_bbox
/// [`add_offset_box`]: Self::add_offset_box
/// [`legend`]: Self::legend
/// [`Figure::colorbar`]: crate::figure::Figure::colorbar
/// [`LegendOptions`]: crate::legend::LegendOptions
/// [`current_legend`]: Self::current_legend
/// [`legend_handles_labels`]: Self::legend_handles_labels
/// [`legend_candidates`]: Self::legend_candidates
/// [`keep_legend`]: Self::keep_legend
/// [`set_current_legend`]: Self::set_current_legend
/// [`current_image`]: Self::current_image
/// [`set_current_image`]: Self::set_current_image
/// [`ylim`]: Self::ylim
/// [`set_limits_with`]: Self::set_limits_with
/// [`set_limits_quietly`]: Self::set_limits_quietly
/// [`set_limits`]: Self::set_limits
/// [`set_bound`]: Self::set_bound
/// [`bound`]: Self::bound
/// [`set_margins`]: Self::set_margins
/// [`set_margins_with`]: Self::set_margins_with
/// [`set_margin`]: Self::set_margin
/// [`margins`]: Self::margins
/// [`margin`]: Self::margin
/// [`autoscale`]: Self::autoscale
/// [`autoscale_with`]: Self::autoscale_with
/// [`set_autoscale_on`]: Self::set_autoscale_on
/// [`set_axis_autoscale_on`]: Self::set_axis_autoscale_on
/// [`autoscale_on`]: Self::autoscale_on
/// [`autoscale_view`]: Self::autoscale_view
/// [`autoscale_view_along`]: Self::autoscale_view_along
/// [`autoscales_tight`]: Self::autoscales_tight
/// [`set_autolimit_mode`]: Self::set_autolimit_mode
/// [`autolimit_mode`]: Self::autolimit_mode
/// [`set_use_sticky_edges`]: Self::set_use_sticky_edges
/// [`use_sticky_edges`]: Self::use_sticky_edges
/// [`data_lim`]: Self::data_lim
/// [`relim`]: Self::relim
/// [`update_datalim`]: Self::update_datalim
/// [`has_data`]: Self::has_data
/// [`set_ignores_data_limits`]: Self::set_ignores_data_limits
/// [`ignores_data_limits`]: Self::ignores_data_limits
/// [`on_limits_changed`]: Self::on_limits_changed
/// [`disconnect_limits_changed`]: Self::disconnect_limits_changed
/// [`set_xscale`]: Self::set_xscale
/// [`set_yscale`]: Self::set_yscale
/// [`set_scale_with`]: Self::set_scale_with
/// [`set_scale`]: Self::set_scale
/// [`xscale`]: Self::xscale
/// [`yscale`]: Self::yscale
/// [`xaxis`]: Self::xaxis
/// [`yaxis`]: Self::yaxis
/// [`ticker`]: crate::ticker
/// [`axis_of`]: Self::axis_of
/// [`set_xticks`]: Self::set_xticks
/// [`set_yticks`]: Self::set_yticks
/// [`set_xticklabels`]: Self::set_xticklabels
/// [`set_yticklabels`]: Self::set_yticklabels
/// [`ticks`]: Self::ticks
/// [`minorticks_on`]: Self::minorticks_on
/// [`minorticks_off`]: Self::minorticks_off
/// [`tick_params`]: Self::tick_params
/// [`locator_params`]: Self::locator_params
/// [`ticklabel_format`]: Self::ticklabel_format
/// [`dates`]: crate::dates
/// [`xaxis_date`]: Self::xaxis_date
/// [`yaxis_date`]: Self::yaxis_date
/// [`convert_point`]: Self::convert_point
/// [`grid`]: Self::grid
/// [`grid_with`]: Self::grid_with
/// [`has_grid`]: Self::has_grid
/// [`set_axisbelow`]: Self::set_axisbelow
/// [`axisbelow`]: Self::axisbelow
/// [`spines`]: Self::spines
/// [`Spine::set_position`]: crate::spines::Spine::set_position
/// [`Spine::set_bounds`]: crate::spines::Spine::set_bounds
/// [`new_spine`]: Self::new_spine
/// [`add_spine`]: Self::add_spine
/// [`replace_spine`]: Self::replace_spine
/// [`set_frame_on`]: Self::set_frame_on
/// [`frame_on`]: Self::frame_on
/// [`set_axis_off`]: Self::set_axis_off
/// [`set_axis_on`]: Self::set_axis_on
/// [`axis_on`]: Self::axis_on
/// [`axis_mode`]: Self::axis_mode
/// [`set_facecolor`]: Self::set_facecolor
/// [`facecolor`]: Self::facecolor
/// [`set_patch_visible`]: Self::set_patch_visible
/// [`set_patch_edgecolor`]: Self::set_patch_edgecolor
/// [`set_patch_linewidth`]: Self::set_patch_linewidth
/// [`set_patch_alpha`]: Self::set_patch_alpha
/// [`set_patch_style`]: Self::set_patch_style
/// [`set_patch_zorder`]: Self::set_patch_zorder
/// [`set_patch_gid`]: Self::set_patch_gid
/// [`patch_visible`]: Self::patch_visible
/// [`patch_edgecolor`]: Self::patch_edgecolor
/// [`patch_linewidth`]: Self::patch_linewidth
/// [`patch_alpha`]: Self::patch_alpha
/// [`patch_style`]: Self::patch_style
/// [`patch_zorder`]: Self::patch_zorder
/// [`patch_gid`]: Self::patch_gid
/// [`set_patch_outline`]: Self::set_patch_outline
/// [`patch_outline`]: Self::patch_outline
/// [`clips_to_patch_shape`]: Self::clips_to_patch_shape
/// [`set_aspect`]: Self::set_aspect
/// [`aspect`]: Self::aspect
/// [`set_shared_aspect`]: Self::set_shared_aspect
/// [`data_ratio`]: Self::data_ratio
/// [`set_adjustable`]: Self::set_adjustable
/// [`adjustable`]: Self::adjustable
/// [`set_box_aspect`]: Self::set_box_aspect
/// [`box_aspect`]: Self::box_aspect
/// [`set_anchor`]: Self::set_anchor
/// [`anchor`]: Self::anchor
/// [`apply_aspect`]: Self::apply_aspect
/// [`apply_aspect_at`]: Self::apply_aspect_at
/// [`position`]: Self::position
/// [`try_position`]: Self::try_position
/// [`set_position`]: Self::set_position
/// [`set_position_which`]: Self::set_position_which
/// [`set_position_box`]: Self::set_position_box
/// [`original_position`]: Self::original_position
/// [`reset_position`]: Self::reset_position
/// [`located_position`]: Self::located_position
/// [`set_subplot_spec`]: Self::set_subplot_spec
/// [`subplot_spec`]: Self::subplot_spec
/// [`set_axes_locator`]: Self::set_axes_locator
/// [`axes_locator`]: Self::axes_locator
/// [`label_outer`]: Self::label_outer
/// [`label_outer_with`]: Self::label_outer_with
/// [`bbox`]: Self::bbox
/// [`tight_bbox`]: Self::tight_bbox
/// [`decorations_tight_bbox`]: Self::decorations_tight_bbox
/// [`share_x`]: Self::share_x
/// [`share_y`]: Self::share_y
/// [`share_with`]: Self::share_with
/// [`leave_sharing`]: Self::leave_sharing
/// [`shared_axes`]: Self::shared_axes
/// [`sharing`]: Self::sharing
/// [`shared_from`]: Self::shared_from
/// [`join_across`]: Self::join_across
/// [`shared_across`]: Self::shared_across
/// [`shared_from_across`]: Self::shared_from_across
/// [`join_sharing`]: Self::join_sharing
/// [`twinx`]: Self::twinx
/// [`twiny`]: Self::twiny
/// [`twin_with`]: Self::twin_with
/// [`twin_with_projection`]: Self::twin_with_projection
/// [`twins`]: Self::twins
/// [`secondary_xaxis`]: Self::secondary_xaxis
/// [`secondary_yaxis`]: Self::secondary_yaxis
/// [`set_secondary_functions`]: Self::set_secondary_functions
/// [`set_secondary_location`]: Self::set_secondary_location
/// [`inset_axes`]: Self::inset_axes
/// [`inset_axes_in`]: Self::inset_axes_in
/// [`inset_axes_with_projection`]: Self::inset_axes_with_projection
/// [`polar_inset_axes`]: Self::polar_inset_axes
/// [`geo_inset_axes`]: Self::geo_inset_axes
/// [`indicate_inset_zoom`]: Self::indicate_inset_zoom
/// [`indicate_inset`]: Self::indicate_inset
/// [`child_axes`]: Self::child_axes
/// [`add_child_axes`]: Self::add_child_axes
/// [`place_inset`]: Self::place_inset
/// [`Patch`]: crate::patches::Patch
/// [`add_patch`]: Self::add_patch
/// [`add_patch_with`]: Self::add_patch_with
/// [`add_artist`]: Self::add_artist
/// [`add_artist_with`]: Self::add_artist_with
/// [`add_line`]: Self::add_line
/// [`add_collection`]: Self::add_collection
/// [`CollectionSpec`]: crate::collections::CollectionSpec
/// [`add_trimesh`]: Self::add_trimesh
/// [`TriMeshSpec`]: crate::collections::TriMeshSpec
/// [`add_custom_artist`]: Self::add_custom_artist
/// [`add_container`]: Self::add_container
/// [`attach`]: Self::attach
/// [`add_filtered_group`]: Self::add_filtered_group
/// [`add_axis_texts`]: Self::add_axis_texts
/// [`add_tick_marks`]: Self::add_tick_marks
/// [`add_program_axis`]: Self::add_program_axis
/// [`program_axes`]: Self::program_axes
/// [`set_rasterization_zorder`]: Self::set_rasterization_zorder
/// [`rasterization_zorder`]: Self::rasterization_zorder
/// [`set_prop_cycle`]: Self::set_prop_cycle
/// [`Cycler`]: crate::rc::Cycler
/// [`set_prop_cycle_of`]: Self::set_prop_cycle_of
/// [`prop_cycle_position`]: Self::prop_cycle_position
/// [`set_prop_cycle_position`]: Self::set_prop_cycle_position
/// [`next_line_color`]: Self::next_line_color
/// [`next_patch_color`]: Self::next_patch_color
/// [`share_prop_cycle`]: Self::share_prop_cycle
/// [`ArtistHandle::remove`]: crate::artist::ArtistHandle::remove
/// [`ArtistHandle::findobj`]: crate::artist::ArtistHandle::findobj
/// [`lines`]: Self::lines
/// [`patches`]: Self::patches
/// [`collections`]: Self::collections
/// [`texts`]: Self::texts
/// [`images`]: Self::images
/// [`data_images`]: Self::data_images
/// [`tables`]: Self::tables
/// [`artists`]: Self::artists
/// [`containers`]: Self::containers
/// [`added_children`]: Self::added_children
/// [`clear`]: Self::clear
/// [`cla`]: Self::cla
/// [`call_mark`]: Self::call_mark
/// [`undo_since`]: Self::undo_since
/// [`trans_data`]: Self::trans_data
/// [`trans_axes`]: Self::trans_axes
/// [`trans_xaxis`]: Self::trans_xaxis
/// [`trans_yaxis`]: Self::trans_yaxis
/// [`data_to_display`]: Self::data_to_display
/// [`Coords`]: crate::coords::Coords
/// [`ArtistHandle::set_picker`]: crate::artist::ArtistHandle::set_picker
/// [`set_navigate`]: Self::set_navigate
/// [`navigate`]: Self::navigate
/// [`can_pan`]: Self::can_pan
/// [`can_zoom`]: Self::can_zoom
/// [`start_pan`]: Self::start_pan
/// [`zoom_view`]: Self::zoom_view
/// [`zoomed_bounds`]: Self::zoomed_bounds
/// [`set_forward_navigation_events`]: Self::set_forward_navigation_events
/// [`forward_navigation_events`]: Self::forward_navigation_events
/// [`format_coord`]: Self::format_coord
/// [`format_data`]: Self::format_data
/// [`set_fmt_data`]: Self::set_fmt_data
/// [`cursor_text`]: Self::cursor_text
/// [`pixel_resolution`]: Self::pixel_resolution
/// [`contains_point`]: Self::contains_point
/// [`draw_artist`]: Self::draw_artist
/// [`redraw_in_frame`]: Self::redraw_in_frame
/// [`Figure::polar_subplot`]: crate::figure::Figure::polar_subplot
/// [`PolarAxes`]: crate::polar::PolarAxes
/// [`Figure::geo_subplot`]: crate::figure::Figure::geo_subplot
/// [`GeoAxes`]: crate::geo::GeoAxes
/// [`Axes3D`]: crate::mplot3d::Axes3D
/// [`projection`]: Self::projection
/// [`as_polar`]: Self::as_polar
/// [`as_geo`]: Self::as_geo
/// [`install_projection`]: Self::install_projection
/// [`with_projection`]: Self::with_projection
/// [`with_projection_mut`]: Self::with_projection_mut
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
    /// The handle of the axes behind `raw`.
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

    /// Which axes, `(x, y)`, wait to be rescaled at the next layout (or
    /// the next reading of the view), which then no longer wait: what
    /// matplotlib's `_unstale_viewLim` takes to call `autoscale_view`
    /// along them, which an Axes class of the program's overrides
    /// (cartopy's `GeoAxes` keeps the view within its map).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// assert_eq!(ax.take_stale_autoscale(), (false, false));
    /// ax.plot([0.0, 10.0], [0.0, 5.0])?;
    /// assert_eq!(ax.take_stale_autoscale(), (true, true));
    /// assert_eq!(ax.take_stale_autoscale(), (false, false));
    /// assert_eq!(ax.xlim(), (0.0, 1.0), "left as it was");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn take_stale_autoscale(&self) -> (bool, bool) {
        let [x, y] = self
            .raw
            .figure_state()
            .borrow_mut()
            .take_stale_axes(self.id());
        (x, y)
    }

    /// Which axes, `(x, y)`, wait to be rescaled, as
    /// [`take_stale_autoscale`](Self::take_stale_autoscale) tells, which
    /// still wait: whether matplotlib's next draw computes the view limits
    /// along them with the locators (`autoscale_view`).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.plot([0.0, 10.0], [0.0, 5.0])?;
    /// assert_eq!(ax.stale_autoscale(), (true, true));
    /// assert_eq!(ax.stale_autoscale(), (true, true));
    /// let (low, high) = ax.xlim();
    /// assert!(low <= 0.0 && 10.0 <= high, "rescaled as it is read");
    /// assert_eq!(ax.stale_autoscale(), (false, false));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn stale_autoscale(&self) -> (bool, bool) {
        let [x, y] = self.raw.figure_state().borrow().stale_axes(self.id());
        (x, y)
    }

    /// Brings the axes up to date to be drawn alone (see
    /// `FigureState::prepare_axes_alone`), then runs the limit callbacks its
    /// autoscaling called for.
    pub(crate) fn prepare_to_draw_alone(&self) {
        let figure = self.raw.figure_state();
        figure.work_on(|state| state.prepare_axes_alone(self.id()));
        run_limit_callbacks(figure);
    }

    /// Applies pending autoscaling to the view limits, then runs the limit
    /// callbacks it queued; a view that is up to date is left untouched
    /// (and the figure unborrowed for change).
    pub(crate) fn unstale_view(&self) {
        let figure = self.raw.figure_state();
        if !figure.borrow().view_is_stale(self.id()) {
            return;
        }
        // Worked on outside its cell: autoscaling asks the program's class
        // of an axis answering its view interval whether it is inverted.
        figure.work_on(|state| state.unstale_axes(self.id()));
        run_limit_callbacks(figure);
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
    /// now: a layout engine moves the axes, and its aspect adjusts it,
    /// only when the figure is drawn or laid out (call
    /// [`Figure::layout`](crate::figure::Figure::layout) first for the
    /// ticks of the axes it places), as matplotlib computes ticks between
    /// draws. An axes removed from its figure has its ticks where it was.
    ///
    /// # Errors
    ///
    /// When the locator or the formatter of the axis fails, or the locator
    /// places more than [`MAX_DRAWN_TICKS`](crate::ticker::MAX_DRAWN_TICKS)
    /// ticks.
    pub fn ticks(&self, axis: AxisName) -> Result<TickSet, TickerError> {
        let _standing = self.raw.standing_in();
        self.raw
            .figure_state()
            .work_on(|state| state.update_ticks_as_is(self.id(), axis));
        self.raw
            .with::<AxesState, _>(|a| a.axis(axis).ticks.clone())
    }
}
