//! Axes placed by dividers (matplotlib's `mpl_toolkits.axes_grid1`).
//!
//! A divider splits a rectangle of the figure into columns and rows whose
//! widths and heights are [`Size`]s: absolute sizes in inches, or relative
//! sizes such as the data extent of an axes. The split is recomputed at
//! every layout pass, so paddings stay fixed in inches and image axes keep
//! their aspect.
//!
//! * [`make_axes_locatable`] divides the area of an axes, so that
//!   colorbars or marginal axes can be appended beside it.
//! * [`ImageGrid`] (also [`AxesGrid`]) lays out a grid of image axes with
//!   fixed paddings, each, some or one colorbar axes, and tick labels only
//!   on the outer axes.
//! * [`anchored_artists`] holds ready-made anchored boxes: a scale bar,
//!   direction arrows, and anchored drawing areas, aux-transform boxes and
//!   ellipses.

pub mod anchored_artists;
mod axes_divider;
mod cbar_layout;
mod divider;
mod image_grid;
mod inset_locator;
mod inset_marks;
mod labels;
mod rgb_axes;
mod size;

pub use axes_divider::{AxesDivider, make_axes_locatable};
pub(crate) use divider::{Divider, DividerCell};
pub use image_grid::{
    AxesGrid, AxesGridError, CbarAxes, CbarMode, GridDirection, GridRect, ImageGrid,
    ImageGridOptions,
};
pub(crate) use inset_locator::{InsetLocator, InsetLocatorId};
pub use inset_locator::{InsetOptions, inset_axes, zoomed_inset_axes};
pub use inset_marks::{Corner, InsetArea, InsetConnector, InsetMarks, mark_inset};
pub use labels::LabelMode;
pub use rgb_axes::RgbAxes;
pub use size::{Size, SizeAspect, SizeSpec};
