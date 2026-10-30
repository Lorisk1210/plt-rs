//! Axes placed by dividers (matplotlib's `mpl_toolkits.axes_grid1`).
//!
//! A divider splits a rectangle of the figure into columns and rows whose
//! widths and heights are [`Size`]s: absolute sizes in inches, or relative
//! sizes such as the data extent of an axes. The split is recomputed at
//! every layout pass, so paddings stay fixed in inches and image axes keep
//! their aspect.
//!
//! * [`Divider`] splits a rectangle of the figure (or the cells of a
//!   subplot) into the cells axes are placed in.
//! * [`make_axes_locatable`] divides the area of an axes, so that
//!   colorbars or marginal axes can be appended beside it, and
//!   [`make_axes_area_auto_adjustable`] shrinks an axes until its
//!   decorations fit inside its original position.
//! * [`ImageGrid`] (also [`AxesGrid`]) lays out a grid of image axes with
//!   fixed paddings, each, some or one colorbar axes, and tick labels only
//!   on the outer axes.
//! * [`anchored_artists`] holds ready-made anchored boxes: a scale bar,
//!   direction arrows, and anchored drawing areas, aux-transform boxes and
//!   ellipses.
//! * [`host_subplot`] and [`host_axes`] make [`HostAxes`], which carry
//!   [`ParasiteAxes`]: twins drawn by their host, sharing one of its axes
//!   or mapping their coordinates onto its own, and listed in its legend.
//! * [`SimpleAxisArtist`] acts on one side of an axes at once: spine,
//!   ticks, tick labels and label (matplotlib's `ax.axis["right"]`).
//! * [`BboxPatch`], [`BboxConnector`] and [`BboxConnectorPatch`] outline
//!   and join boxes worked out at every draw, such as axes boxes or the
//!   area an inset shows ([`mark_inset`]).

pub mod anchored_artists;
mod axes_divider;
mod bbox_patches;
mod cbar_layout;
mod divider;
mod figure_divider;
mod image_grid;
mod inset_locator;
mod inset_marks;
mod labels;
mod mpl_axes;
mod parasite_axes;
mod rgb_axes;
mod size;

pub use axes_divider::{AxesDivider, make_axes_area_auto_adjustable, make_axes_locatable};
pub use bbox_patches::{
    BboxConnector, BboxConnectorPatch, BboxPatch, BboxSource, Corner, connect_bbox,
};
pub(crate) use divider::{DividerCell, DividerState};
pub use figure_divider::{Divider, DividerError};
pub use image_grid::{
    AxesGrid, AxesGridError, CbarAxes, CbarMode, GridDirection, GridRect, ImageGrid,
    ImageGridOptions,
};
pub(crate) use inset_locator::{InsetLocator, InsetLocatorId, locate_on_bounds};
pub use inset_locator::{
    InsetOptions, inset_axes, place_sized_inset, place_zoomed_inset, zoomed_inset_axes,
};
pub use inset_marks::{InsetMarks, mark_inset};
pub use labels::LabelMode;
pub use mpl_axes::SimpleAxisArtist;
pub use parasite_axes::{
    HostAxes, ParasiteAxes, ParasiteError, ParasiteLink, host_axes, host_subplot,
};
pub(crate) use parasite_axes::{Parasite, is_parasite, parasites_of};
pub use rgb_axes::{RgbAxes, hide_channel_ticklabels, make_rgb_axes, whiten_frame};
pub use size::{Size, SizeAspect, SizeSpec};
