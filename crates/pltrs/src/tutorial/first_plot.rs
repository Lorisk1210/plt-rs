//! Chapter 1: a first plot.
//!
//! A plot is a [`Figure`], the whole image. It has a size in inches, a
//! resolution in dots (pixels) per inch and a background. Data goes into an
//! [`Axes`], a rectangle in the figure with an x axis and a y axis. The
//! word "axes" is singular. One axes is one plotting area, and a figure can
//! hold several (chapter [`layout`](super::layout)). You add axes to a figure,
//! call plotting methods on them, and save or show the figure.
//!
//! # Setting up
//!
//! Run `cargo add pltrs`, or depend on the crate by path:
//!
//! ```toml
//! [dependencies]
//! pltrs = { path = "../plt-rs/crates/pltrs" }
//! ```
//!
//! pltrs needs no other crates or system libraries. Rendering is slow
//! without optimization, so optimize dependencies in the dev profile:
//!
//! ```toml
//! [profile.dev.package."*"]
//! opt-level = 2
//! ```
//!
//! # A line
//!
//! ```
//! use pltrs::prelude::*;
//! # let dir = std::env::temp_dir().join(format!("pltrs-tutorial-first-{}", std::process::id()));
//! # std::fs::create_dir_all(&dir)?;
//! # std::env::set_current_dir(&dir)?;
//!
//! let fig = Figure::with_size(6.0, 4.0, 100.0); // 6 × 4 inches at 100 dpi
//! let ax = fig.subplot();
//!
//! let x = linspace(0.0, 10.0, 200);
//! ax.plot(&x, x.iter().map(|x| x.sin()))?;
//!
//! ax.set_title("A sine wave");
//! ax.set_xlabel("time (s)").set_ylabel("amplitude");
//! fig.savefig("sine.png")?;
//! # assert!(std::fs::read("sine.png")?.starts_with(b"\x89PNG"));
//! # std::env::set_current_dir(std::env::temp_dir())?; // Windows cannot remove the directory while it is current
//! # std::fs::remove_dir_all(&dir)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! The program writes `sine.png`, 600 by 400 pixels, with a sine curve, a
//! title and two axis labels. Step by step:
//!
//! * [`Figure::with_size`] takes the width and height in inches and the
//!   resolution. [`Figure::new`] uses the default size.
//! * [`Figure::subplot`] adds one axes that fills the figure, with margins
//!   for the labels, and returns it.
//! * [`linspace`] makes `n` evenly spaced numbers from `start` to `stop`,
//!   both included.
//! * [`Axes::plot`] draws a line through the points `(x[i], y[i])`. It
//!   accepts arrays, slices, `Vec`s and iterators of any number type,
//!   borrowed or owned. It returns a handle to the line in a `Result`. If x
//!   and y have different lengths, it returns an error.
//! * The axes selects its limits and ticks from the data (chapter
//!   [`scales`](super::scales)).
//! * [`Axes::set_title`] returns the title text. The label setters return
//!   the axes, so you can chain them.
//! * [`Figure::savefig`] uses the extension to select the format. `.svg`
//!   and `.pdf` give a vector document (chapter [`saving`](super::saving)).
//!
//! The gallery entry `line_plot` in `lines.rs` plots three damped waves
//! with a grid and a legend.
//!
//! # Several lines and a legend
//!
//! Each call to `plot` adds a line in the next color of a fixed cycle. Give
//! the lines labels, and ask the axes for a legend.
//!
//! ```
//! use pltrs::prelude::*;
//!
//! let fig = Figure::with_size(6.0, 4.0, 100.0);
//! let ax = fig.subplot();
//! let x = linspace(0.0, 10.0, 200);
//! for (k, label) in [(1.0, "sin x"), (2.0, "sin 2x")] {
//!     let y = x.iter().map(|x| (k * x).sin());
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
//! [`Axes::plot_with`] is `plot` with a [`LineProps`], the properties of
//! the line. Chapter [`data`](super::data) explains option values, and
//! chapter [`styling`](super::styling) explains the properties.
//!
//! # Bytes instead of a file
//!
//! [`Figure::to_png`] returns the PNG bytes, for example for a web
//! response or a test. Each format has such a method (chapter
//! [`saving`](super::saving)).
//!
//! ```
//! use pltrs::image::imread_bytes;
//! use pltrs::prelude::*;
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
//! returns when you close all of them. In the window, you can pan, zoom and
//! save (chapter [`interactive`](super::interactive)).
//!
//! ```no_run
//! use pltrs::prelude::*;
//!
//! let fig = Figure::new();
//! fig.subplot().plot([1.0, 2.0, 3.0], [2.0, 4.0, 3.0])?;
//! pltrs::gui::show(&[fig])?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Like [`pyplot::show`], `gui::show` uses Wayland or X11 on Linux,
//! `AppKit` on macOS and Win32 on Windows. On Linux without a display, it
//! returns immediately.
//!
//! # The pyplot shortcut
//!
//! [`pyplot`] copies the matplotlib module `pyplot`. Its free functions
//! act on a current figure and axes for each thread, and make them when
//! necessary. Use it for short scripts:
//!
//! ```
//! use pltrs::pyplot;
//!
//! pyplot::plot([1.0, 2.0, 3.0], [1.0, 4.0, 9.0])?;
//! pyplot::title("Squares");
//! pyplot::xlabel("n");
//! // A script ends with `pyplot::savefig("squares.png")?` or `pyplot::show()?`.
//! let png = pyplot::gcf().to_png()?;
//! assert!(png.starts_with(b"\x89PNG"));
//! pyplot::close_all();
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! This tutorial uses the figure and axes handles instead (the
//! object-oriented interface), so each call names its axes. You can mix
//! the two: [`pyplot::gca`] returns the current [`Axes`].
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
//! [`pyplot::gca`]: crate::pyplot::gca
