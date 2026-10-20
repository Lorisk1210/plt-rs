//! Axes drawn by axis artists (matplotlib's `mpl_toolkits.axisartist`).
//!
//! In an [`Axes`] of this module the spines and axes of the underlying
//! axes are hidden; each axis line is an [`AxisArtist`] instead, drawing
//! its line, ticks, tick labels, offset text and label. Every part is
//! styled on its own ([`AxisArtist::major_ticks`],
//! [`AxisArtist::major_ticklabels`], [`AxisArtist::label`],
//! [`AxisArtist::line`], ...), and [`AxisArtist::set_axis_direction`]
//! turns the texts to matplotlib's conventions for an axis on another
//! side. Besides the four edges, floating axis lines can run through the
//! data ([`Axes::new_floating_axis`], or `xzero` and `yzero` of
//! [`Axes::zero`]), lines can end in arrow heads ([`AxislineStyle`]), and
//! [`Axes::grid`] draws gridlines through the grid helper. Outside such
//! axes, [`AxisArtist::add_fixed`] puts an axis artist on an edge of any
//! axes, and [`TickMarks`] are tick marks drawn on their own.
//!
//! A grid helper can make the grid curvilinear: with a
//! [`grid_helper_curvelinear::GridHelperCurveLinear`] the grid lines,
//! ticks and floating axes follow the lines of constant coordinate of an
//! [`AuxTransform`], located and labelled by the [`grid_finder`] and
//! [`angle_helper`] locators and formatters. [`floating_axes`] shows a
//! fixed range of such coordinates in axes bounded by its edges, and
//! [`Axes::get_aux_axes`] adds axes to plot in curvilinear coordinates.
//! [`Axes::twinx`], [`Axes::twiny`] and [`Axes::add_parasite_axes`] add
//! parasite axes whose axis lines are axis artists of their own.
//!
//! Tick labels and labels are placed at a distance from their reference
//! point in the direction away from the line, and turned by that
//! direction, following matplotlib's `axis_direction` conventions.

pub mod angle_helper;
mod artist;
mod aux_transform;
mod axes;
pub mod floating_axes;
pub mod grid_finder;
pub mod grid_helper_curvelinear;
mod gridlines;
mod handles;
mod helper;
mod labels;
mod line;
mod parasites;
mod tick_marks;
mod ticks;

pub use aux_transform::AuxTransform;
pub use axes::{Axes, GridHelper};
pub use handles::{
    AxisArtist, AxisLabel, AxisLine, LabelBase, LabelDirection, TickLabels, Ticks, Toggle,
};
pub use line::{AxislineStyle, AxislineStyleError};
pub use tick_marks::{TickMarks, TickMarksArtist};
