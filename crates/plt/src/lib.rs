//! Matplotlib, reimagined in Rust: figures, axes, artists and a pyplot-style API.
//!
//! plt-rs follows matplotlib's architecture and semantics closely so that
//! anything you can draw with matplotlib can be drawn the same way here.
//! Figures take plt-rs' modern [look] unless told otherwise; in
//! matplotlib's look (`PLT_LOOK=matplotlib`) its PNG output is
//! pixel-identical to matplotlib's.
//!
//! # Quick start
//!
//! The object-oriented interface: make a [`figure::Figure`], add axes and
//! plot on them, then save the figure (PNG, JPEG, TIFF, GIF, WebP, SVG,
//! PDF or PostScript, chosen from the file name).
//!
//! ```
//! use plt::prelude::*;
//! use plt::legend::LegendOptions;
//!
//! let fig = Figure::with_size(6.4, 4.0, 100.0);
//! let ax = fig.subplot();
//! let x = linspace(0.0, 10.0, 200);
//! let y: Vec<f64> = x.iter().map(|x| x.sin()).collect();
//! ax.plot_with(&x, y, LineProps::default().label("sin"))?;
//! ax.set_title("A sine wave");
//! ax.set_xlabel("x").set_ylabel("sin x");
//! ax.legend(LegendOptions::default())?;
//! let png = fig.to_png()?;
//! assert!(png.starts_with(b"\x89PNG"));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`pyplot`] offers the stateful `plt.plot(...)`/`plt.show()` style, with
//! figures shown in windows ([`gui`]). The gallery example
//! (`cargo run -p plt --example gallery`) shows many more figures.
//!
//! # Modules
//!
//! The crate is organised like matplotlib's own modules:
//!
//! * [`figure`] / [`axes`] / [`gridspec`] — figures, subfigures and
//!   mosaics, and the canvas a figure keeps between draws for blitting;
//!   axes with their plotting methods, axis, ticks and spines; grids of
//!   subplots. [`prelude`] imports the everyday names.
//! * [`lines`] / [`markers`] / [`collections`] / [`text`] / [`fonts`] —
//!   the artists most plots are made of.
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
//!   [`animation`] plays frame functions on them, redrawing or blitting,
//!   and saves animated PNG and GIF files, HTML players and (through
//!   ffmpeg) videos.
//! * [`navigation`] — interactive pan, zoom and view history, independent
//!   of any window system; [`gui`] shows figures in windows with a
//!   toolbar ([`gui::show`]), and [`web`] in a web browser, like
//!   matplotlib's `WebAgg` backend ([`web::serve`]).
//! * [`layout`] — layout engines (constrained and tight layout) placing
//!   the axes so that their decorations fit, and aligned labels.
//! * [`inset`] — rectangles marking the area an inset axes enlarges.
//! * [`axes_grid1`] / [`axisartist`] — axes placed by dividers with fixed
//!   paddings, anchored scale bars and direction arrows, and axes drawn
//!   by axis artists (`mpl_toolkits`).
//! * [`mplot3d`] — 3-D axes: surfaces, wireframes, 3-D scatter, bars,
//!   contours and more, projected through a camera.
//! * [`legend`] / [`offsetbox`] / [`container`] — legends, the box
//!   packers they are laid out with (also anchored and annotation boxes
//!   of texts, images and artists), and the artist groups of bar, error
//!   bar and stem plots they list.
//! * [`rc`] / [`style`] / [`look`] — runtime configuration, the bundled
//!   style sheets and the xkcd sketch style; the modern and matplotlib
//!   looks of figures and windows.
#![forbid(unsafe_code)]

pub mod animation;
#[macro_use]
pub mod artist;
pub mod axes;
pub mod axes_grid1;
pub mod axisartist;
pub mod collections;
pub mod colorbar;
mod colorizer;
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
pub mod look;
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
pub mod web;
pub mod widgets;
