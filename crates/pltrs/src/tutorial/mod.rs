//! A tutorial about plotting with pltrs, for Rust programmers who never
//! used matplotlib.
//!
//! pltrs is a Rust reimplementation of matplotlib, the Python plotting
//! library. It keeps the matplotlib model of a figure and most of its
//! names, so the matplotlib documentation also applies. This tutorial
//! teaches that model in Rust terms. It assumes that you know Rust, but no
//! plotting library. If you know matplotlib, read the
//! [`porting`](crate::porting) guide instead.
//!
//! Each example is a doctest that `cargo test -p pltrs --doc` runs. Read
//! the chapters in order the first time.
//!
//! # Chapters
//!
//! 1. [`first_plot`]: a figure, axes, a line and labels, saved to a file
//!    and shown in a window.
//! 2. [`anatomy`]: the parts of a figure and the handles to them.
//! 3. [`plot_types`]: lines, scatter plots, bars, histograms, filled
//!    areas, images and contours.
//! 4. [`data`]: data types, option structs and errors.
//! 5. [`styling`]: colors, line styles, markers, looks, style sheets,
//!    runtime settings and the configuration file.
//! 6. [`layout`]: several axes in one figure (grids, mosaics, gridspecs,
//!    subfigures), constrained layout, colorbars, shared axes and insets.
//! 7. [`text`]: titles, labels, free text, annotations with arrows,
//!    mathematical notation and text that LaTeX typesets.
//! 8. [`scales`]: limits, scales, ticks and dates.
//! 9. [`saving`]: output formats and save options.
//! 10. [`interactive`]: windows, the toolbar and keyboard, events and
//!     callbacks, widgets, animation and the browser backend.
//! 11. [`extending`]: your own artists, projections, unit converters,
//!     legend handlers and toolbar tools.
//! 12. [`testing`]: tests for the figures that your program draws.
//!
//! # Words you will meet
//!
//! pltrs uses the matplotlib vocabulary. These words have a specific
//! meaning:
//!
//! | word         | meaning                                                         | chapter |
//! |--------------|-----------------------------------------------------------------|---------|
//! | figure       | the whole image                                                 | [`first_plot`] |
//! | axes         | one plotting area, with an x axis and a y axis (one axes, two axes) | [`first_plot`] |
//! | axis         | the x axis or the y axis of an axes, with limits, ticks and a label | [`anatomy`] |
//! | tick         | a mark on an axis, with a label                                 | [`anatomy`], [`scales`] |
//! | spine        | a line at the edge of the data area                             | [`anatomy`] |
//! | artist       | anything drawn: lines, patches (shapes), texts, images, collections | [`anatomy`] |
//! | handle       | the Rust value that changes an artist                           | [`anatomy`] |
//! | colormap     | a function from numbers to colors                               | [`plot_types`] |
//! | rcParams     | the runtime settings that give the defaults                     | [`styling`] |
//! | style sheet  | a named set of rcParams                                         | [`styling`] |
//! | look         | the defaults at the start: modern or matplotlib                 | [`styling`] |
//! | layout engine | the part that places the axes                                  | [`layout`] |
//! | backend      | the part that shows figures                                     | [`interactive`] |
//!
//! # How the examples are written
//!
//! * `use pltrs::prelude::*;` imports the common names ([`Figure`],
//!   [`Axes`], [`LineProps`], [`LegendOptions`], [`linspace`], [`Grid`],
//!   ...).
//! * Each example runs in a hidden function that returns
//!   `Result<(), Box<dyn std::error::Error>>`, so calls that can fail end
//!   in `?`. Chapter [`data`] explains the errors.
//! * The `assert!` lines show what a call returns.
//! * The examples give the same results in the modern look and the
//!   matplotlib look (chapter [`styling`]).
//!
//! # Seeing the figures
//!
//! To see a figure, paste the example into a `main` that returns
//! `Result<(), Box<dyn std::error::Error>>`. End it with
//! `fig.savefig("out.png")?` or `pltrs::gui::show(&[fig])?`.
//!
//! The gallery has more than ninety figures. Each is a short function in
//! `crates/pltrs/examples/gallery/`. This command writes them as PNG
//! files, with an `index.html` that shows them by topic:
//!
//! ```text
//! cargo run -p pltrs --release --example gallery -- target/gallery
//! ```
//!
//! The chapters name gallery entries. For example, "`line_plot` in
//! `lines.rs`" is a function in `crates/pltrs/examples/gallery/lines.rs`
//! that writes `target/gallery/line_plot.png`.
//!
//! # After the tutorial
//!
//! * The [crate overview](crate) lists every module.
//! * The [`porting`](crate::porting) guide has tables of methods by topic
//!   and longer examples.
//! * The matplotlib documentation also applies.
//!   [`porting::conventions`](crate::porting::conventions) explains how a
//!   Python call becomes a Rust call.
//!
//! [`Figure`]: crate::figure::Figure
//! [`Axes`]: crate::axes::Axes
//! [`LineProps`]: crate::lines::LineProps
//! [`LegendOptions`]: crate::legend::LegendOptions
//! [`linspace`]: crate::numeric::linspace
//! [`Grid`]: crate::numeric::Grid

pub mod anatomy;
pub mod data;
pub mod extending;
pub mod first_plot;
pub mod interactive;
pub mod layout;
pub mod plot_types;
pub mod saving;
pub mod scales;
pub mod styling;
pub mod testing;
pub mod text;
