//! Axes drawn by axis artists (matplotlib's `mpl_toolkits.axisartist`).
//!
//! In an [`Axes`] of this module the spines and axes of the underlying
//! axes are hidden; each axis line is an [`AxisArtist`] instead, drawing
//! its line, ticks, tick labels, offset text and label. Besides the four
//! edges, floating axis lines can run through the data
//! ([`Axes::new_floating_axis`], or `xzero` and `yzero` of
//! [`Axes::zero`]), and lines can end in arrow heads
//! ([`AxislineStyle`]).
//!
//! Tick labels and labels are placed at a distance from their reference
//! point in the direction away from the line, and turned by that
//! direction, following matplotlib's `axis_direction` conventions.

mod artist;
mod axes;
mod helper;
mod labels;
mod line;

pub use axes::{Axes, AxisArtist, Toggle};
pub use line::{AxislineStyle, AxislineStyleError};
