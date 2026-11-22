//! matplotlib rebuilt in Rust: figures, axes, artists and a pyplot-style API.
//!
//! pltrs follows the architecture and the behavior of matplotlib. You can
//! draw each matplotlib figure in the same way with pltrs. Figures use the
//! modern [look] by default. In the matplotlib look
//! (`PLTRS_LOOK=matplotlib`), the PNG output of pltrs is identical to the
//! matplotlib output, pixel for pixel.
//!
//! # Quick start
//!
//! In the object-oriented interface, you make a [`figure::Figure`], add
//! axes, plot on them and save the figure. The file name selects the
//! format: PNG, JPEG, TIFF, GIF, WebP, AVIF, SVG, PDF, PostScript or PGF.
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
//! [`pyplot`] gives the stateful `plt.plot(...)` and `plt.show()` style.
//! [`gui`] shows figures in windows. The gallery example
//! (`cargo run -p pltrs --example gallery`) shows many more figures.
//!
//! If you do not know matplotlib, read the [`tutorial`]. It teaches pltrs
//! in Rust terms, chapter by chapter.
//!
//! If you know matplotlib, read the [`porting`] guide. It gives the pltrs
//! call for each matplotlib call by topic, with tested examples.
//!
//! # Modules
//!
//! The modules follow the matplotlib modules:
//!
//! * [`figure`], [`axes`], [`gridspec`]: figures, subfigures and mosaics,
//!   axes with their plotting methods, axis, ticks and spines, and grids
//!   of subplots. A figure keeps its canvas between draws for blitting.
//! * [`prelude`]: the common names.
//! * [`artist`]: the artist model. It has the handles to all artists,
//!   their shared properties and your own artists.
//! * [`lines`], [`markers`], [`collections`], [`text`], [`fonts`]: the
//!   artists of most plots. Text includes mathtext and, if TeX is
//!   installed, text that LaTeX typesets (`text.usetex`).
//! * [`spines`], [`orientation`]: the lines at the edges of the data area,
//!   and the direction of bars, histograms, colorbars and sliders.
//! * [`transforms`]: coordinate systems and the transforms between them.
//! * [`numeric`]: numpy-style ranges and the 2-D array [`numeric::Grid`].
//! * [`colors`]: color specifications, colormaps and normalization.
//! * [`ticker`], [`scale`], [`dates`]: tick positions, tick labels and axis
//!   scales.
//! * [`stats`], [`mlab`]: statistics and spectral analysis for `boxplot`,
//!   `violinplot`, `hist`, `psd`, `specgram` and others.
//! * [`contour`], [`tri`]: contours and unstructured triangular grids.
//! * [`quiver`], [`streamplot`]: vector fields as arrows, wind barbs and
//!   streamlines.
//! * [`units`]: the conversion of categories, dates and your own types to
//!   axis numbers, on 2-D and 3-D axes.
//! * [`image`]: images (`imshow`, `figimage`, `pcolorfast`) and their
//!   resampling.
//! * [`polar`]: polar axes, with angle and radius instead of x and y.
//! * [`geo`]: geographic axes with the Aitoff, Hammer, Lambert and
//!   Mollweide map projections.
//! * [`projections`]: your own projections, registered by name.
//! * [`patches`], [`path`], [`coords`], [`patheffects`], [`table`]: shapes,
//!   paths, fancy boxes and arrows, annotation coordinate systems, path
//!   effects and tables.
//! * [`colorbar`]: colorbars for images, collections and contour sets.
//! * [`sankey`]: Sankey flow diagrams.
//! * [`pyplot`]: the stateful interface of `matplotlib.pyplot`.
//! * [`widgets`]: buttons, sliders, check buttons, radio buttons, text
//!   boxes, cursors and selectors.
//! * [`events`]: figure events, callbacks (`mpl_connect`) and timers.
//! * [`animation`]: animations that draw again or blit at each frame. It
//!   saves animated PNG, GIF and WebP files, HTML players and, through
//!   ffmpeg, videos.
//! * [`navigation`]: pan, zoom and view history, without a window system.
//! * [`gui`]: figures in windows with a toolbar ([`gui::show`]).
//! * [`web`]: figures in a web browser, like the matplotlib backend
//!   `WebAgg` ([`web::serve`]).
//! * [`layout`]: the layout engines (constrained and tight layout), which
//!   place the axes so that their labels fit, and aligned labels.
//! * [`inset`]: the rectangles that mark the area that an inset axes
//!   enlarges.
//! * [`axes_grid1`], [`axisartist`]: axes placed by dividers, anchored
//!   scale bars and direction arrows, and axes drawn by axis artists
//!   (`mpl_toolkits`).
//! * [`mplot3d`]: 3-D axes with surfaces, wireframes, scatter plots, bars,
//!   contours and more, projected through a camera, with units on each
//!   axis.
//! * [`legend`], [`offsetbox`], [`container`]: legends and the box packers
//!   that lay them out. The packers also make anchored boxes and
//!   annotation boxes. The containers are the artist groups of bar, error
//!   bar and stem plots.
//! * [`rc`], [`style`], [`look`]: runtime settings, the included style
//!   sheets, the xkcd sketch style, and the modern look and the matplotlib
//!   look of figures and windows.
//! * [`config`]: the configuration directory (`PLTRSCONFIGDIR`, or else
//!   `~/.config/pltrs`), the rc file `pltrsrc` that threads start from,
//!   and the style sheets in `stylelib`. pltrs finds `pltrsrc` through `PLTRSRC`, in the working
//!   directory or in the configuration directory.
//! * [`testing`]: tests of your figures with the matplotlib test
//!   configuration, image comparison by RMS and figure comparison.
//! * [`warnings`]: the warnings that pltrs gives where matplotlib warns,
//!   and where they go.
//! * [`clock`]: the performance counter for slow work, in place of the
//!   matplotlib `time.perf_counter`.
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
pub mod clock;
pub mod collections;
pub mod colorbar;
mod colorizer;
pub mod colors;
pub mod config;
pub mod container;
pub mod contour;
pub mod coords;
pub mod dates;
pub mod diagnostics;
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

/// The harness of the fuzz tests (`pltrs_core::fuzz`), also driving the unit
/// tests that fuzz what the crate keeps private: the web server's
/// parsers, the configuration lookup and the dialogs of figure windows.
#[cfg(test)]
use pltrs_core::fuzz;
/// `pltrs-font`'s test fixtures, for the unit tests that read them.
#[cfg(test)]
#[path = "../tests/support/font_fixtures.rs"]
mod font_fixtures;
mod process;
/// Scratch directories for the unit tests that write files.
#[cfg(test)]
#[path = "../tests/support/scratch.rs"]
mod scratch;
