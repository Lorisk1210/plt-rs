//! Matplotlib, reimagined in Rust: figures, axes, artists and a pyplot-style API.
//!
//! pltrs follows matplotlib's architecture and semantics closely so that
//! anything you can draw with matplotlib can be drawn the same way here.
//! Figures take pltrs' modern [look] unless told otherwise; in
//! matplotlib's look (`PLTRS_LOOK=matplotlib`) its PNG output is
//! pixel-identical to matplotlib's.
//!
//! # Quick start
//!
//! The object-oriented interface: make a [`figure::Figure`], add axes and
//! plot on them, then save the figure (PNG, JPEG, TIFF, GIF, WebP, SVG,
//! PDF, PostScript or PGF, chosen from the file name).
//!
//! ```
//! use pltrs::prelude::*;
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
//! (`cargo run -p pltrs --example gallery`) shows many more figures.
//!
//! New to matplotlib? The [`tutorial`] teaches pltrs from the ground up,
//! in Rust terms, chapter by chapter: figures and axes, plot types, data
//! and options, styling, layout, text, scales, saving, interaction,
//! extending and testing.
//!
//! Coming from matplotlib? The [`porting`] guide maps its calls to pltrs
//! topic by topic, with tested side-by-side examples.
//!
//! # Modules
//!
//! The crate is organised like matplotlib's own modules:
//!
//! * [`figure`] / [`axes`] / [`gridspec`] — figures, subfigures and
//!   mosaics, and the canvas a figure keeps between draws for blitting;
//!   axes with their plotting methods, axis, ticks and spines; grids of
//!   subplots. [`prelude`] imports the everyday names.
//! * [`artist`] — the artist model: the handles every artist is reached
//!   through, their shared properties, and artists of your own.
//! * [`lines`] / [`markers`] / [`collections`] / [`text`] / [`fonts`] —
//!   the artists most plots are made of; text includes mathtext and, with
//!   TeX installed, text typeset by LaTeX (`text.usetex`).
//! * [`spines`] / [`orientation`] — the lines framing the data area; the
//!   direction of bars, histograms, colorbars and sliders.
//! * [`transforms`] — coordinate systems and the transforms between them.
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
//! * [`units`] — the conversion of plotting values (categories, dates and
//!   values of your own types) to axis numbers, on 2-D and 3-D axes.
//! * [`image`] — images (`imshow`, `figimage`, `pcolorfast`) and their
//!   resampling.
//! * [`polar`] — polar axes: angle and radius instead of x and y.
//! * [`geo`] — geographic axes: longitude and latitude through the Aitoff,
//!   Hammer, Lambert and Mollweide map projections.
//! * [`projections`] — projections defined by the program, registered by
//!   name and drawn like the built-in ones.
//! * [`patches`] / [`path`] / [`coords`] / [`patheffects`] / [`table`] —
//!   shapes, paths, fancy boxes and arrows, annotation coordinate systems,
//!   path effects and tables.
//! * [`colorbar`] — colorbars for images, collections and contour sets.
//! * [`sankey`] — Sankey flow diagrams.
//! * [`pyplot`] — the stateful `matplotlib.pyplot`-style interface.
//! * [`widgets`] — buttons, sliders, check/radio buttons, text boxes,
//!   cursors and selectors.
//! * [`events`] — figure events, callbacks (`mpl_connect`) and timers;
//!   [`animation`] plays frame functions on them, redrawing or blitting,
//!   and saves animated PNG, GIF and WebP files, HTML players and
//!   (through ffmpeg) videos.
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
//!   contours and more, projected through a camera, with units on each of
//!   their three axes.
//! * [`legend`] / [`offsetbox`] / [`container`] — legends, the box
//!   packers they are laid out with (also anchored and annotation boxes
//!   of texts, images and artists), and the artist groups of bar, error
//!   bar and stem plots they list.
//! * [`rc`] / [`style`] / [`look`] — runtime configuration, the bundled
//!   style sheets and the xkcd sketch style; the modern and matplotlib
//!   looks of figures and windows.
//! * [`config`] — the user's configuration: the configuration directory
//!   (`PLTRSCONFIGDIR`, else `~/.config/pltrs`), the rc file `pltrsrc` that
//!   threads start from (found through `PLTRSRC`, in the working directory
//!   or in the configuration directory) and the user style library in its
//!   `stylelib`.
//! * [`testing`] — testing your own figures as matplotlib's tests do:
//!   their configuration, image comparison by RMS and figure comparison.
//! * [`warnings`] — the warnings pltrs issues while it works, where
//!   matplotlib warns, and where they go.
#![forbid(unsafe_code)]
#![expect(
    clippy::needless_pass_by_value,
    reason = "option structs are passed by value by convention (see `porting::conventions`), \
              whether the function keeps them or only reads them"
)]

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
pub mod config;
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
pub mod path;
pub mod patheffects;
pub mod polar;
pub mod porting;
pub mod prelude;
pub mod projections;
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
pub mod testing;
pub mod text;
pub mod ticker;
pub mod transforms;
pub mod tri;
pub mod tutorial;
pub mod units;
pub mod warnings;
pub mod web;
pub mod widgets;

/// The harness of the fuzz tests (`tests/fuzz`), also driving the unit
/// tests that fuzz what the crate keeps private: the web server's
/// parsers, the configuration lookup and the dialogs of figure windows.
#[cfg(test)]
#[allow(
    dead_code,
    reason = "the fuzz tests use every generator, these unit tests only some"
)]
#[path = "../tests/fuzz/harness.rs"]
mod fuzz;
mod process;

/// The README, whose Rust examples are checked as doctests.
#[cfg(doctest)]
#[doc = include_str!("../../../README.md")]
struct Readme;
