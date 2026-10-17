//! Matplotlib, reimagined in Rust: figures, axes, artists and a pyplot-style API.
//!
//! plt-rs follows matplotlib's architecture and semantics closely so that
//! anything you can draw with matplotlib can be drawn the same way here.
//! The crate is organised like matplotlib's own modules:
//!
//! * [`numeric`] — numpy-style ranges and the [`numeric::Grid`] 2-D array.
//! * [`colors`] — colour specifications, colormaps and normalisation.
//! * [`ticker`] / [`scale`] / [`dates`] — tick placement, formatting and
//!   axis scales.
//! * [`stats`] / [`mlab`] — statistics and spectral analysis used by
//!   `boxplot`, `violinplot`, `hist`, `psd`, `specgram`, ...
//! * [`contour`] / [`tri`] — contour generation and unstructured triangular
//!   grids.
//! * [`quiver`] / [`streamplot`] — vector fields as arrows, wind barbs and
//!   streamlines.
//! * [`units`] — categorical axes and the conversion of plotting values.
//! * [`image`] — images (`imshow`, `figimage`, `pcolorfast`) and their
//!   resampling.
//! * [`polar`] — polar axes: angle and radius instead of x and y.
//! * [`patches`] / [`coords`] / [`patheffects`] / [`table`] — shapes,
//!   fancy boxes and arrows, annotation coordinate systems, path effects
//!   and tables.
//! * [`colorbar`] — colorbars for images, collections and contour sets.
//! * [`navigation`] — interactive pan, zoom and view history, independent
//!   of any window system.
//! * [`layout`] — layout engines (constrained layout) placing the axes so
//!   that their decorations fit.
#![forbid(unsafe_code)]

#[macro_use]
pub mod artist;
pub mod axes;
pub mod collections;
pub mod colorbar;
pub mod colors;
pub mod contour;
pub mod coords;
pub mod dates;
pub mod figure;
pub mod fonts;
pub mod gridspec;
pub mod image;
pub mod layout;
pub mod lines;
pub mod markers;
pub mod mlab;
pub mod navigation;
pub mod numeric;
pub mod orientation;
pub mod patches;
pub mod patheffects;
pub mod polar;
pub mod prelude;
pub mod quiver;
pub mod rc;
pub mod scale;
pub mod spines;
pub mod stats;
pub mod streamplot;
pub mod table;
#[cfg(test)]
mod testing;
pub mod text;
pub mod ticker;
pub mod transforms;
pub mod tri;
pub mod units;
