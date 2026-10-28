//! Chapter 5: styling, from one line's colour to the defaults of every
//! figure.
//!
//! Styling works at three levels. You style one artist through its
//! properties. You change the defaults new artists take through the
//! runtime settings, matplotlib's *rcParams*, either one at a time or by
//! applying a *style sheet*. And the *look* decides which defaults a
//! program starts from: plt-rs' modern look or matplotlib's.
//!
//! # Colours
//!
//! Every colour argument is `impl Into<`[`Color`]`>`, and strings
//! convert, so a colour is usually a string literal:
//!
//! * a name: `"red"`, `"steelblue"`, a Tableau colour such as
//!   `"tab:orange"`, an xkcd survey colour such as `"xkcd:sky blue"`;
//! * a hex code: `"#ff8800"`, or `"#ff880080"` with an alpha;
//! * a grey level from `"0"` (black) to `"1"` (white): `"0.6"`;
//! * a colour of the cycle: `"C0"` to `"C9"`, the colours lines take in
//!   turn;
//! * `"none"`, for no colour at all (an unfilled shape).
//!
//! Tuples and arrays of floats from 0 to 1 convert too: `(0.1, 0.4, 0.8)`
//! and `[0.1, 0.4, 0.8, 0.5]` with an alpha. A string is checked when the
//! figure is drawn, and a bad one makes drawing fail; [`Color::parse`]
//! checks it at once, which suits colours read from a file.
//!
//! ```
//! use plt::colors::Color;
//! use plt::prelude::*;
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
//! A line has a width in points (1/72 inch, so it does not change with
//! the figure's resolution), a [`LineStyle`] (solid, dashed, dotted,
//! dash-dot, none, or a dash pattern of your own) and, optionally, a
//! [`Marker`] at each point. Markers have a size in points, a face colour
//! and an edge colour; both follow the line's colour unless set.
//!
//! ```
//! use plt::prelude::*;
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
//! // The same, in matplotlib's format-string shorthand, then adjusted.
//! let quick = ax.plot_fmt([1.0, 2.0, 3.0, 4.0], [0.0, 2.0, 1.0, 3.0], "D--")?;
//! quick.set_linewidth(2.5).set_markersize(9.0);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Shapes (bars, histograms, filled areas, polygons) have a face colour,
//! an edge colour and width, and an optional hatch pattern, set through
//! a [`PatchProps`] or [`CollectionProps`](crate::collections::CollectionProps).
//! Gallery: `markers`, `line_styles` and `hatches` in `reference.rs` show
//! every marker, dash style and hatch.
//!
//! # The modern look and matplotlib's
//!
//! plt-rs draws in its own modern look unless told otherwise: Inter
//! type, a light grid behind the data, fewer lines around it, a colour
//! cycle checked for colour-vision deficiencies, and constrained layout
//! (chapter [`layout`](super::layout)); [`look`](crate::look) lists what
//! it changes. The other look is matplotlib's own defaults. In
//! matplotlib's look plt-rs draws the same pixels as matplotlib, which
//! matters when you compare a figure with one made in Python or follow
//! matplotlib's documentation picture by picture. Behaviour is the same
//! in both; only the defaults differ.
//!
//! The environment chooses the look of a whole program without a
//! rebuild:
//!
//! ```text
//! PLT_LOOK=matplotlib cargo run              # matplotlib's figures and windows
//! PLT_FIGURE_LOOK=modern-dark cargo run      # the modern look on a dark background
//! ```
//!
//! and code can choose with [`look::set_figure_look`](crate::look::set_figure_look)
//! at the start of `main`. Each look is a set of rcParams on top of
//! matplotlib's defaults, which you can inspect:
//!
//! ```
//! use plt::look::{self, FigureLook};
//! use plt::prelude::*;
//! use plt::ticker::AxisName;
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
//! Every default comes from the current thread's [`RcParams`]: one typed
//! field per setting, named after matplotlib's dotted keys
//! (`lines.linewidth` is `lines_linewidth`). Most artists read them when
//! they are made, so change the settings before plotting.
//! A setting naming one of a few keywords holds the enum of its concept,
//! the type the matching option takes: `axes.titlelocation` is a
//! [`TitleLocation`], `markers.fillstyle` a [`FillStyle`].
//! [`rc::set`](crate::rc::set) takes a key and a value as text, checked
//! as matplotlib checks it; [`rc::update`](crate::rc::update) changes the
//! typed fields. [`rc::context`](crate::rc::context) runs a closure and
//! then restores the settings as they were, which keeps a change local.
//! It returns what the closure returns.
//!
//! ```
//! use std::error::Error;
//!
//! use plt::axes::TitleLocation;
//! use plt::prelude::*;
//! use plt::rc;
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
//! The closure names its return type, `Result<_, Box<dyn Error>>`, for
//! the sake of its `?`s: they convert each error into the closure's error
//! type, which Rust cannot otherwise tell when the result goes through
//! `?` again. A context that ends the function returning its result
//! needs none.
//!
//! # Style sheets
//!
//! A style sheet is a named set of settings, applied over the current
//! ones. plt-rs bundles matplotlib's sheets (`ggplot`, `bmh`,
//! `dark_background`, `fivethirtyeight`, the `seaborn-v0_8` family, ...)
//! and its own looks (`plt-modern`, `plt-modern-dark`).
//! [`style::use_style`](crate::style::use_style) applies one for the
//! rest of the thread, [`style::context`](crate::style::context) for one
//! closure.
//!
//! ```
//! use std::error::Error;
//!
//! use plt::prelude::*;
//! use plt::style;
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
//! A sheet that does not exist ends the call with an error of the
//! closure's error type, without running it, so one `?` covers both.
//! Gallery: `styles.rs` draws one figure in each bundled sheet.
//!
//! # Your configuration file
//!
//! Settings you want in every program go in a file named `pltrc`, in
//! matplotlib's `matplotlibrc` syntax: one `key: value` per line, `#`
//! for comments.
//!
//! ```text
//! # ~/.config/plt-rs/pltrc
//! lines.linewidth: 2
//! axes.titlesize: large
//! savefig.dpi: 200
//! ```
//!
//! plt-rs reads it once per process and applies it over the look, so it
//! changes the defaults of every thread. It looks for `./pltrc` first,
//! then the file (or directory) that `PLTRC` names, then `pltrc` in the
//! configuration directory: `$PLTCONFIGDIR`, else `plt-rs` in
//! `$XDG_CONFIG_HOME` (by default `~/.config`). Style sheets of your own
//! go in the `stylelib` directory there, as `name.mplstyle` files, and
//! [`style::use_style`](crate::style::use_style) finds them by name.
//! [`config`](crate::config) tells where each file is looked for, and
//! reports the lines of `pltrc` it could not use:
//!
//! ```
//! use plt::config;
//!
//! if let Some(file) = config::user_rc_file()? {
//!     for warning in &file.warnings {
//!         eprintln!("pltrc: {warning}");
//!     }
//! }
//! assert!(config::config_dir()?.is_dir());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`rc::restore_startup`](crate::rc::restore_startup) returns the
//! thread to the look and `pltrc`, and [`rc::reset`](crate::rc::reset) to
//! matplotlib's defaults. The [`porting::colors`](crate::porting::colors)
//! page covers colormaps, norms and colour cycles, and
//! [`porting::configuration`](crate::porting::configuration) the
//! configuration files.
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
