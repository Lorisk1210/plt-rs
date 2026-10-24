//! The [`Projection`] trait a program implements to give axes a
//! coordinate system of its own, and the shapes and placements its hooks
//! return.

use std::any::Any;
use std::error::Error;
use std::fmt;

use plt_core::{Path, Point};
use plt_text::{HAlign, VAlign};

use super::ProjectionView;
use crate::axes::{Axes, Sides};
use crate::spines::SpineSide;
use crate::ticker::AxisName;
use crate::transforms::ResolvedTransform;

/// A coordinate system for axes defined by the program (matplotlib's
/// `Axes` subclasses made for `register_projection`, such as the gallery's
/// Hammer and SkewT-logP axes).
///
/// Only [`name`](Self::name) and [`data_to_axes`](Self::data_to_axes) are
/// required; every other hook defaults to what rectilinear axes do, so a
/// projection overrides what matplotlib's subclasses override:
///
/// | matplotlib                                   | hook                                   |
/// |----------------------------------------------|----------------------------------------|
/// | `name`                                       | [`name`](Self::name)                   |
/// | `clear`                                      | [`set_up`](Self::set_up)               |
/// | `_set_lim_and_transforms` (`transData`)      | [`data_to_axes`](Self::data_to_axes)   |
/// | `get_xaxis_transform`, `get_yaxis_transform` | [`axis_transform`](Self::axis_transform) |
/// | `get_xaxis_text1_transform` and friends      | [`tick_label`](Self::tick_label)       |
/// | `Axis.get_view_interval`                     | [`tick_interval`](Self::tick_interval) |
/// | `Tick.draw` hiding parts of a tick           | [`tick_sides`](Self::tick_sides)       |
/// | `_gen_axes_patch`, `_gen_axes_spines`        | [`outline`](Self::outline), [`spine_order`](Self::spine_order) |
/// | `Spine._adjust_location`                     | [`spine_interval`](Self::spine_interval) |
/// | `set_xlim` raising                           | [`fixed_limits`](Self::fixed_limits)   |
/// | `set_xscale` raising                         | [`linear_only`](Self::linear_only)     |
/// | `get_data_ratio`                             | [`data_ratio`](Self::data_ratio)       |
/// | `can_zoom`, `can_pan`                        | [`can_zoom`](Self::can_zoom), [`can_pan`](Self::can_pan) |
/// | `format_coord`                               | [`format_coord`](Self::format_coord)   |
///
/// The hooks run while the figure is being laid out or drawn: they must
/// not use the axes or any other handle to its figure (only
/// [`set_up`](Self::set_up) is given the axes to work on). Settings of
/// one axes live in the value itself, reached with
/// [`Axes::with_projection`] and [`Axes::with_projection_mut`].
///
/// See the [module documentation](super) for a complete projection.
pub trait Projection: Any + fmt::Debug {
    /// The name axes made with this projection report and the registry
    /// knows it by (matplotlib's `name` class attribute).
    fn name(&self) -> &'static str;

    /// Maps data coordinates onto the unit square of the axes
    /// (matplotlib's `transData` without its final `transAxes`).
    ///
    /// Non-affine steps go first: a [`Step::Plane`] map that bends
    /// straight lines should override [`PointMap::transform_path`] to
    /// subdivide paths before it maps them, as matplotlib's
    /// `transform_path_non_affine` does. [`ProjectionView::rectilinear`]
    /// is the transform of rectilinear axes, to build upon.
    ///
    /// [`Step::Plane`]: crate::transforms::Step::Plane
    /// [`PointMap::transform_path`]: crate::transforms::PointMap::transform_path
    fn data_to_axes(&self, view: &ProjectionView<'_>) -> ResolvedTransform;

    /// Prepares a fresh axes (matplotlib's `clear`): its tick locators and
    /// formatters, tick positions, grid, aspect and limits. It runs when
    /// the axes is made and whenever it is cleared, before the projection
    /// takes over the axes: limits set here are kept even when
    /// [`fixed_limits`](Self::fixed_limits) refuses later changes.
    ///
    /// # Errors
    ///
    /// Whatever the set-up runs into; making the axes then fails with
    /// [`ProjectionError::SetUp`](super::ProjectionError::SetUp).
    fn set_up(&mut self, axes: &Axes) -> Result<(), Box<dyn Error>> {
        let _ = axes;
        Ok(())
    }

    /// Maps `(data, fraction)` onto the unit square of the axes for the
    /// gridlines, tick marks and spines of `axis`: a data value along the
    /// axis and a fraction of the way across it (matplotlib's
    /// `get_xaxis_transform` and `get_yaxis_transform`). `None`, the
    /// default, takes the data coordinate through
    /// [`data_to_axes`](Self::data_to_axes) and the fraction as it is.
    fn axis_transform(
        &self,
        view: &ProjectionView<'_>,
        axis: AxisName,
    ) -> Option<ResolvedTransform> {
        let _ = (view, axis);
        None
    }

    /// Where the label of the tick at `loc` goes on `side` (bottom or top
    /// for the x axis, left or right for the y axis; matplotlib's
    /// `get_xaxis_text1_transform` and friends). `None`, the default, puts
    /// it beyond the tick mark by its padding, as on rectilinear axes.
    fn tick_label(
        &self,
        view: &ProjectionView<'_>,
        side: SpineSide,
        loc: f64,
    ) -> Option<TickLabelPlacement> {
        let _ = (view, side, loc);
        None
    }

    /// The interval of `axis` the locators place ticks in and ticks are
    /// drawn for (matplotlib's `Axis.get_view_interval`); the view
    /// interval by default.
    fn tick_interval(&self, view: &ProjectionView<'_>, axis: AxisName) -> (f64, f64) {
        view.interval(axis)
    }

    /// The sides on which the tick at `loc` of `axis` draws its marks and
    /// labels (a matplotlib `Tick.draw` hiding some of them); both by
    /// default. Its gridline is drawn either way, and layouts count its
    /// labels like matplotlib's, which hides them only while drawing.
    fn tick_sides(&self, view: &ProjectionView<'_>, axis: AxisName, loc: f64) -> Sides {
        let _ = (view, axis, loc);
        Sides::BOTH
    }

    /// The shape of the axes background, which clips what is drawn in the
    /// axes, and of its frame (matplotlib's `_gen_axes_patch` and
    /// `_gen_axes_spines`).
    fn outline(&self) -> AxesOutline {
        AxesOutline::Rectangle
    }

    /// The order the side spines of a rectangular outline are drawn in
    /// (the order of the dictionary matplotlib's `_gen_axes_spines`
    /// returns); sides left out are drawn after the others.
    fn spine_order(&self) -> [SpineSide; 4] {
        SpineSide::ALL
    }

    /// The data interval a side spine of a rectangular outline spans
    /// (matplotlib's `Spine._adjust_location`); `None`, the default, spans
    /// the view interval of its axis. Bounds set on the spine take
    /// precedence.
    fn spine_interval(&self, view: &ProjectionView<'_>, side: SpineSide) -> Option<(f64, f64)> {
        let _ = (view, side);
        None
    }

    /// Whether the view limits refuse to change once
    /// [`set_up`](Self::set_up) has set them (matplotlib's geographic
    /// `set_xlim` raising); attempts are reported when the figure is drawn.
    fn fixed_limits(&self) -> bool {
        false
    }

    /// Whether both axes must keep linear scales (matplotlib's geographic
    /// `set_xscale` raising); other scales are refused and reported when
    /// the figure is drawn.
    fn linear_only(&self) -> bool {
        false
    }

    /// The ratio of the y to the x span of the data the aspect works with
    /// (matplotlib's `get_data_ratio`); `None`, the default, computes it
    /// from the view limits. A fixed ratio rules out adjusting the data
    /// limits for the aspect.
    fn data_ratio(&self) -> Option<f64> {
        None
    }

    /// Whether the zoom-to-rectangle tool works on the axes (matplotlib's
    /// `can_zoom`).
    fn can_zoom(&self) -> bool {
        true
    }

    /// Whether the pan/zoom tool works on the axes (matplotlib's
    /// `can_pan`); it moves the view through the inverse of
    /// [`data_to_axes`](Self::data_to_axes).
    fn can_pan(&self) -> bool {
        true
    }

    /// The status-bar text for the data point `(x, y)` (matplotlib's
    /// `format_coord`); `None`, the default, formats both coordinates
    /// with the axis formatters.
    fn format_coord(&self, x: f64, y: f64) -> Option<String> {
        let _ = (x, y);
        None
    }
}

/// The shape of axes drawn through a [`Projection`].
#[derive(Clone, Debug, Default, PartialEq)]
pub enum AxesOutline {
    /// The unit square, framed by the four side spines (matplotlib's
    /// `Rectangle` patch and linear spines).
    #[default]
    Rectangle,
    /// A closed path in axes fractions, such as the circle that becomes
    /// the ellipse of a map. It clips what is drawn in the axes and is
    /// drawn by the top spine alone, the other spines staying hidden
    /// (matplotlib's `Spine.circular_spine` and friends).
    Path(Path),
}

/// Where a [`Projection`] puts a tick label (see
/// [`Projection::tick_label`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TickLabelPlacement {
    /// The anchor in axes fractions.
    pub anchor: Point,
    /// A shift of the anchor in display pixels (matplotlib's translations
    /// after `transAxes`).
    pub offset: Point,
    /// The horizontal alignment at the anchor, unless the tick parameters
    /// set one.
    pub halign: HAlign,
    /// The vertical alignment at the anchor, unless the tick parameters
    /// set one.
    pub valign: VAlign,
}
