//! Chapter 1: a first plot.
//!
//! A plot is a [`Figure`]: the whole image, with a size in inches, a
//! resolution in dots (pixels) per inch and a background. Data goes into
//! an [`Axes`], a rectangle inside the figure with an x axis and a y
//! axis. "Axes" is singular here: one axes is one plotting area, and a
//! figure can hold several (chapter [`layout`](super::layout)). You add
//! axes to a figure, call plotting methods on the axes, and save the
//! figure to a file or show it in a window.
//!
//! # Setting up
//!
//! plt-rs is not published on crates.io. Depend on its `plt` crate by
//! path, or through git from its repository:
//!
//! ```toml
//! [dependencies]
//! plt = { path = "../plt-rs/crates/plt" }
//! ```
//!
//! It has no dependencies of its own and needs no system libraries, so
//! `cargo build` is all it takes. Rendering is numeric code that runs
//! slowly unoptimised; to keep debug builds quick, optimise your
//! dependencies in the dev profile:
//!
//! ```toml
//! [profile.dev.package."*"]
//! opt-level = 2
//! ```
//!
//! # A line
//!
//! ```
//! use plt::prelude::*;
//! # let dir = std::env::temp_dir().join(format!("plt-tutorial-first-{}", std::process::id()));
//! # std::fs::create_dir_all(&dir)?;
//! # std::env::set_current_dir(&dir)?;
//!
//! let fig = Figure::with_size(6.0, 4.0, 100.0); // 6 × 4 inches at 100 dpi
//! let ax = fig.subplot();
//!
//! let x = linspace(0.0, 10.0, 200);
//! let y: Vec<f64> = x.iter().map(|x| x.sin()).collect();
//! ax.plot(&x, &y)?;
//!
//! ax.set_title("A sine wave");
//! ax.set_xlabel("time (s)").set_ylabel("amplitude");
//! fig.savefig("sine.png")?;
//! # assert!(std::fs::read("sine.png")?.starts_with(b"\x89PNG"));
//! # std::fs::remove_dir_all(&dir)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! The program writes `sine.png`, 600 by 400 pixels: about one and a
//! half periods of a sine curve, a title above the axes and a label under
//! each axis. Step by step:
//!
//! * [`Figure::with_size`] takes the width and height in inches and the
//!   resolution. [`Figure::new`] takes the default size instead.
//! * [`Figure::subplot`] adds one axes filling the figure inside margins
//!   for the tick and axis labels, and returns it.
//! * [`linspace`] makes `n` evenly spaced numbers from `start` to `stop`,
//!   both included.
//! * [`Axes::plot`] draws a line through the points `(x[i], y[i])`. It
//!   borrows or takes the data (slices, arrays, `Vec`s) and returns a
//!   handle to the new line. It returns a `Result` because data can be
//!   wrong: an x and a y of different lengths are an error, not a panic.
//! * The axes chose its limits from the data and placed the ticks and
//!   their labels by itself. Chapter [`scales`](super::scales) shows how
//!   to choose them.
//! * [`Axes::set_title`] returns the new title, a text you could style
//!   further. The label setters return the axes, so they chain.
//! * [`Figure::savefig`] writes the file, in the format its extension
//!   names: `.svg` or `.pdf` would write a vector document instead
//!   (chapter [`saving`](super::saving)).
//!
//! The gallery's first figure, `line_plot` in `lines.rs`, is a similar
//! plot of three damped waves, with a grid and a legend.
//!
//! # Several lines and a legend
//!
//! Each call to `plot` adds another line to the same axes, in the next
//! colour of a fixed cycle. Give lines labels and ask the axes for a
//! legend, which lists them.
//!
//! ```
//! use plt::legend::LegendOptions;
//! use plt::prelude::*;
//!
//! let fig = Figure::with_size(6.0, 4.0, 100.0);
//! let ax = fig.subplot();
//! let x = linspace(0.0, 10.0, 200);
//! for (k, label) in [(1.0, "sin x"), (2.0, "sin 2x")] {
//!     let y: Vec<f64> = x.iter().map(|x| (k * x).sin()).collect();
//!     ax.plot_with(&x, y, LineProps::default().label(label))?;
//! }
//! let legend = ax.legend(LegendOptions::default())?;
//!
//! assert_eq!(legend.labels(), ["sin x", "sin 2x"]);
//! let lines = ax.lines();
//! assert_ne!(lines[0].rgba()?, lines[1].rgba()?, "two colours of the cycle");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`Axes::plot_with`] is `plot` with a [`LineProps`]: the properties of
//! the line, here only its label. Chapter [`data`](super::data) explains
//! these option values and chapter [`styling`](super::styling) the
//! properties.
//!
//! # Bytes instead of a file
//!
//! [`Figure::to_png`] returns the PNG file's bytes, for a web response, a
//! database or a test. Every format has such a method (chapter
//! [`saving`](super::saving)).
//!
//! ```
//! use plt::image::imread_bytes;
//! use plt::prelude::*;
//!
//! let fig = Figure::with_size(6.0, 4.0, 100.0);
//! fig.subplot().plot([1.0, 2.0, 3.0], [2.0, 4.0, 3.0])?;
//! let png: Vec<u8> = fig.to_png()?;
//! assert_eq!(imread_bytes(&png)?.shape(), (400, 600), "rows, then columns");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # A window
//!
//! [`gui::show`](crate::gui::show) opens a window for each figure and
//! returns when they are all closed. In the window you can pan and zoom
//! with the mouse, and save the figure from the toolbar (chapter
//! [`interactive`](super::interactive)).
//!
//! ```no_run
//! use plt::prelude::*;
//!
//! let fig = Figure::new();
//! fig.subplot().plot([1.0, 2.0, 3.0], [2.0, 4.0, 3.0])?;
//! plt::gui::show(&[fig])?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Windows need X11. [`pyplot::show`] adapts to the system instead: it
//! opens windows where there is X11, serves the figures to a web browser
//! where there is none (macOS, Windows), and does nothing on a machine
//! without a display. It shows pyplot's figures, so hand it yours with
//! [`pyplot::figure_activate`] first.
//!
//! # The pyplot shortcut
//!
//! [`pyplot`] is a second way to drive plt-rs, copied from matplotlib's
//! `pyplot` module. It keeps a current figure and a current axes per
//! thread, and its free functions act on them, making both when needed.
//! It suits short scripts:
//!
//! ```
//! use plt::pyplot;
//!
//! pyplot::plot([1.0, 2.0, 3.0], [1.0, 4.0, 9.0])?;
//! pyplot::title("Squares");
//! pyplot::xlabel("n");
//! // `pyplot::savefig("squares.png")?` or `pyplot::show()?` would end a script.
//! let png = pyplot::gcf().to_png()?;
//! assert!(png.starts_with(b"\x89PNG"));
//! pyplot::close_all();
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! This tutorial uses the figure and axes handles instead (matplotlib
//! calls this the object-oriented interface): a program with more than
//! one axes reads better when each call names the axes it draws on. The
//! two mix freely, since pyplot hands out the same handles:
//! [`pyplot::gca`] returns the current [`Axes`].
//!
//! Next: [the anatomy of a figure](super::anatomy).
//!
//! [`Figure`]: crate::figure::Figure
//! [`Figure::with_size`]: crate::figure::Figure::with_size
//! [`Figure::new`]: crate::figure::Figure::new
//! [`Figure::subplot`]: crate::figure::Figure::subplot
//! [`Figure::savefig`]: crate::figure::Figure::savefig
//! [`Figure::to_png`]: crate::figure::Figure::to_png
//! [`Axes`]: crate::axes::Axes
//! [`Axes::plot`]: crate::axes::Axes::plot
//! [`Axes::plot_with`]: crate::axes::Axes::plot_with
//! [`Axes::set_title`]: crate::axes::Axes::set_title
//! [`LineProps`]: crate::lines::LineProps
//! [`linspace`]: crate::numeric::linspace
//! [`pyplot`]: crate::pyplot
//! [`pyplot::show`]: crate::pyplot::show
//! [`pyplot::figure_activate`]: crate::pyplot::figure_activate
//! [`pyplot::gca`]: crate::pyplot::gca
