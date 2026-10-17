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
//! * [`geo`] — geographic axes: longitude and latitude through the Aitoff,
//!   Hammer, Lambert and Mollweide map projections.
//! * [`patches`] / [`coords`] / [`patheffects`] / [`table`] — shapes,
//!   fancy boxes and arrows, annotation coordinate systems, path effects
//!   and tables.
//! * [`colorbar`] — colorbars for images, collections and contour sets.
//! * [`sankey`] — Sankey flow diagrams.
//! * [`pyplot`] — the stateful `matplotlib.pyplot`-style interface.
//! * [`widgets`] — buttons, sliders, check/radio buttons and text boxes.
//! * [`events`] — figure events, callbacks (`mpl_connect`) and timers;
//!   [`animation`] plays frame functions on them and saves animated PNGs.
//! * [`navigation`] — interactive pan, zoom and view history, independent
//!   of any window system; [`gui`] shows figures in windows with a
//!   toolbar ([`gui::show`]).
//! * [`layout`] — layout engines (constrained and tight layout) placing
//!   the axes so that their decorations fit, and aligned labels.
//! * [`inset`] — rectangles marking the area an inset axes enlarges.
//! * [`axes_grid1`] / [`axisartist`] — axes placed by dividers with fixed
//!   paddings, and axes drawn by axis artists (`mpl_toolkits`).
//! * [`mplot3d`] — 3-D axes: surfaces, wireframes, 3-D scatter, bars,
//!   contours and more, projected through a camera.
//! * [`legend`] / [`offsetbox`] / [`container`] — legends, the box
//!   packers they are laid out with, and the artist groups of bar, error
//!   bar and stem plots they list.
//! * [`rc`] / [`style`] — runtime configuration and the bundled style
//!   sheets.
#![forbid(unsafe_code)]

pub mod animation;
#[macro_use]
pub mod artist;
pub mod axes;
pub mod axes_grid1;
pub mod axisartist;
pub mod collections;
pub mod colorbar;
pub mod colors;
pub mod container;
pub mod contour;
pub mod coords;
pub mod dates;
pub mod events;
pub mod figure;
pub mod fonts;
pub mod geo;
pub mod gridspec;
pub mod gui;
pub mod image;
pub mod inset;
pub mod layout;
pub mod legend;
pub mod lines;
pub mod markers;
pub mod mlab;
pub mod mplot3d;
pub mod navigation;
pub mod numeric;
pub mod offsetbox;
pub mod orientation;
pub mod patches;
pub mod patheffects;
pub mod polar;
pub mod prelude;
pub mod pyplot;
pub mod quiver;
pub mod rc;
pub mod sankey;
pub mod scale;
pub mod spines;
pub mod stats;
pub mod streamplot;
pub mod style;
pub mod table;
#[cfg(test)]
mod testing;
pub mod text;
pub mod ticker;
pub mod transforms;
pub mod tri;
pub mod units;
pub mod widgets;
