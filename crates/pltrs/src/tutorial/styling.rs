//! Chapter 5: styling, from one line to the defaults of all figures.
//!
//! The properties of an artist style that artist. The runtime settings
//! (*rcParams*) give the defaults of new artists. A *style sheet* changes
//! many settings. The *look* gives the settings at the start.
//!
//! # Colors
//!
//! Each color argument is `impl Into<`[`Color`]`>`, so a color is usually
//! a string literal:
//!
//! * a name: `"red"`, `"steelblue"`, a Tableau color such as
//!   `"tab:orange"` or an xkcd color such as `"xkcd:sky blue"`
//! * a hex code: `"#ff8800"`, or `"#ff880080"` with an alpha
//! * a gray level from `"0"` (black) to `"1"` (white): `"0.6"`
//! * a color of the cycle that lines use in turn: `"C0"` to `"C9"`
//! * `"none"` for no color (an unfilled shape)
//!
//! Floats from 0 to 1 also convert: `(0.1, 0.4, 0.8)`, or
//! `[0.1, 0.4, 0.8, 0.5]` with an alpha. A bad string makes the drawing
//! fail. [`Color::parse`] checks a string immediately.
//!
//! ```
//! use pltrs::colors::Color;
//! use pltrs::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! let line = ax.plot([0.0, 1.0], [0.0, 1.0])?;
//! line.set_color("tab:green");
//! line.set_color((0.1, 0.4, 0.8));
//! line.set_color("#ff8800");
//! assert_eq!(line.rgba()?, Color::from("#ff8800").resolve()?);
//! assert!(Color::parse("not a colour").is_err());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Lines and markers
//!
//! A line has a width in points (1/72 inch), a [`LineStyle`] (solid,
//! dashed, dotted, dash-dot, none or a dash pattern) and an optional
//! [`Marker`]. A marker has a size in points, a face color and an edge
//! color. Both colors are the line color by default.
//!
//! ```
//! use pltrs::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! let props = LineProps::default()
//!     .color("tab:purple")
//!     .linewidth(2.5)
//!     .linestyle(LineStyle::Dashed)
//!     .marker(MarkerSymbol::Diamond)
//!     .markersize(9.0)
//!     .markerfacecolor("white");
//! let line = ax.plot_with([1.0, 2.0, 3.0, 4.0], [1.0, 3.0, 2.0, 4.0], props)?;
//! assert_eq!(line.marker(), Marker::Symbol(MarkerSymbol::Diamond));
//!
//! // The same line from a matplotlib format string, then changed.
//! let quick = ax.plot_fmt([1.0, 2.0, 3.0, 4.0], [0.0, 2.0, 1.0, 3.0], "D--")?;
//! quick.set_linewidth(2.5).set_markersize(9.0);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Shapes have a face color, an edge color, an edge width and an optional
//! hatch pattern, in a [`PatchProps`] or a
//! [`CollectionProps`](crate::collections::CollectionProps). The gallery
//! entries `markers`, `line_styles` and `hatches` in `reference.rs` show
//! all of them.
//!
//! # The modern look and the matplotlib look
//!
//! The modern look is the default. It uses the Inter font, a light grid,
//! fewer lines around the data and constrained layout (chapter
//! [`layout`](super::layout)). Its color cycle is safe for color-vision
//! deficiencies. [`look`](crate::look) lists all changes. In
//! the matplotlib look, pltrs draws the same pixels as matplotlib.
//!
//! Environment variables select the look without a rebuild:
//!
//! ```text
//! PLTRS_LOOK=matplotlib cargo run            # the matplotlib look for figures and windows
//! PLTRS_FIGURE_LOOK=modern-dark cargo run    # the modern look on a dark background
//! ```
//!
//! In code, call [`look::set_figure_look`](crate::look::set_figure_look)
//! at the start of `main`. Each look is a set of rcParams over the
//! matplotlib defaults:
//!
//! ```
//! use pltrs::look::{self, FigureLook};
//! use pltrs::prelude::*;
//! use pltrs::ticker::AxisName;
//!
//! assert!(FigureLook::Modern.params().axes_grid);
//! assert!(!FigureLook::Matplotlib.params().axes_grid);
//!
//! look::set_figure_look(FigureLook::Matplotlib);
//! let ax = Figure::new().subplot();
//! assert!(!ax.has_grid(AxisName::X, true), "matplotlib draws no grid");
//! ```
//!
//! # Runtime settings
//!
//! The defaults come from the [`RcParams`] of the current thread. Each
//! setting is a typed field with the name of the matplotlib key
//! (`lines.linewidth` is `lines_linewidth`). Most artists read the settings
//! when pltrs makes them, so change the settings before you plot. A
//! setting with a fixed set of values is an enum, for example
//! [`TitleLocation`] or [`FillStyle`].
//! [`rc::set`](crate::rc::set) takes a key and a value as text and checks
//! them. [`rc::update`](crate::rc::update) changes the typed fields.
//! [`rc::context`](crate::rc::context) runs a closure, restores the
//! settings and returns the result of the closure.
//!
//! ```
//! use std::error::Error;
//!
//! use pltrs::axes::TitleLocation;
//! use pltrs::prelude::*;
//! use pltrs::rc;
//!
//! let line = rc::context(|| -> Result<_, Box<dyn Error>> {
//!     rc::set("lines.linewidth", "3")?;
//!     rc::update(|params| {
//!         params.lines_markersize = 10.0;
//!         params.axes_titlelocation = TitleLocation::Left;
//!     });
//!     assert_eq!(rc::params().get("axes.titlelocation").as_deref(), Some("left"));
//!     let ax = Figure::new().subplot();
//!     Ok(ax.plot([0.0, 1.0], [0.0, 1.0])?)
//! })?;
//! assert_eq!(line.linewidth(), 3.0);
//! assert_ne!(rc::params().lines_linewidth, 3.0, "restored after the context");
//! assert!(rc::set("lines.linewidth", "thick").is_err());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! The closure states its return type, because Rust cannot infer the error
//! type of its `?` operators. As the last expression of a function, the
//! context needs no type.
//!
//! # Style sheets
//!
//! A style sheet is a named set of settings. pltrs includes the matplotlib
//! sheets (`ggplot`, `bmh`, `dark_background`, `fivethirtyeight`, the
//! `seaborn-v0_8` family, ...) and its looks (`pltrs-modern`,
//! `pltrs-modern-dark`). [`style::use_style`](crate::style::use_style)
//! applies a sheet for the rest of the thread, and
//! [`style::context`](crate::style::context) for one closure.
//!
//! ```
//! use std::error::Error;
//!
//! use pltrs::prelude::*;
//! use pltrs::style;
//!
//! assert!(style::available().contains(&"ggplot".to_owned()));
//! let png = style::context("dark_background", || -> Result<_, Box<dyn Error>> {
//!     let fig = Figure::new();
//!     fig.subplot().plot([0.0, 1.0, 2.0], [1.0, 3.0, 2.0])?;
//!     Ok(fig.to_png()?)
//! })?;
//! assert!(png.starts_with(b"\x89PNG"));
//! assert!(style::use_style("no such style").is_err());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! If the sheet does not exist, `context` returns an error of the closure
//! error type and does not run the closure. The gallery file `styles.rs`
//! draws one figure in each sheet.
//!
//! # Your configuration file
//!
//! Put the settings for all your programs in a file `pltrsrc`, in the
//! `matplotlibrc` syntax: one `key: value` per line and `#` for comments.
//!
//! ```text
//! # ~/.config/pltrs/pltrsrc
//! lines.linewidth: 2
//! axes.titlesize: large
//! savefig.dpi: 200
//! ```
//!
//! pltrs reads the file once per process and applies it over the look, in
//! all threads. It uses the first file that it finds:
//!
//! 1. `./pltrsrc`
//! 2. the file or directory in `PLTRSRC`
//! 3. `pltrsrc` in the configuration directory, which is `$PLTRSCONFIGDIR`,
//!    or else `pltrs` in `$XDG_CONFIG_HOME` (default `~/.config`)
//!
//! Put your style sheets as `name.mplstyle` files in `stylelib` in the
//! configuration directory. [`style::use_style`](crate::style::use_style)
//! finds them by name. [`config`](crate::config) shows where pltrs looks
//! for files and the lines of `pltrsrc` that it could not use:
//!
//! ```
//! use pltrs::config;
//!
//! if let Some(file) = config::user_rc_file()? {
//!     for warning in &file.warnings {
//!         eprintln!("pltrsrc: {warning}");
//!     }
//! }
//! assert!(config::config_dir()?.is_dir());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`rc::restore_startup`](crate::rc::restore_startup) sets the thread
//! back to the look and `pltrsrc`. [`rc::reset`](crate::rc::reset) sets it
//! to the matplotlib defaults. See also
//! [`porting::colors`](crate::porting::colors) for colormaps, norms and
//! color cycles, and
//! [`porting::configuration`](crate::porting::configuration).
//!
//! Next: [layout](super::layout).
//!
//! [`Color`]: crate::colors::Color
//! [`Color::parse`]: crate::colors::Color::parse
//! [`LineStyle`]: crate::lines::LineStyle
//! [`Marker`]: crate::markers::Marker
//! [`PatchProps`]: crate::patches::PatchProps
//! [`RcParams`]: crate::rc::RcParams
//! [`TitleLocation`]: crate::axes::TitleLocation
//! [`FillStyle`]: crate::markers::FillStyle
