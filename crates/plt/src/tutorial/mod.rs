//! A tutorial: plotting with plt-rs, for Rust programmers who have never
//! used matplotlib.
//!
//! plt-rs is a Rust reimplementation of matplotlib, the most widely used
//! plotting library for Python. It keeps matplotlib's model of a figure
//! and most of its names, so what you learn here also explains
//! matplotlib's documentation and the many answers written for it. This
//! tutorial teaches that model from the start, in Rust terms. It assumes
//! you know Rust (ownership, closures, traits, `Result` and `?`) and
//! nothing about plotting libraries. If you already know matplotlib, the
//! [`porting`](crate::porting) guide maps its calls to plt-rs faster.
//!
//! Each chapter is a short page of explanations and examples. Every
//! example is a doctest that `cargo test -p plt --doc` compiles and runs,
//! so it works with the version of plt-rs you are reading about. Read the
//! chapters in order the first time; later, each stands on its own.
//!
//! # Chapters
//!
//! 1. [`first_plot`]: a figure, axes, a line and its labels; saving a
//!    PNG file and showing a window.
//! 2. [`anatomy`]: what a figure is made of (axes, axis, ticks, spines,
//!    legends, artists) and how plt-rs hands you parts of it as handles.
//! 3. [`plot_types`]: lines and markers, scatter plots, bars,
//!    histograms, filled areas, images and contours, and when to use
//!    which.
//! 4. [`data`]: the data plotting methods accept, option structs, and
//!    the errors they return.
//! 5. [`styling`]: colours, line styles and markers; plt-rs' modern look
//!    and matplotlib's; style sheets, runtime settings and your own
//!    configuration file.
//! 6. [`layout`]: several axes in one figure (grids, mosaics, gridspecs,
//!    subfigures), constrained layout, colorbars, shared axes and insets.
//! 7. [`text`]: titles, labels, free text, annotations with arrows, and
//!    mathematical notation.
//! 8. [`scales`]: logarithmic and other scales, choosing where ticks go
//!    and how they read, and dates on an axis.
//! 9. [`saving`]: the output formats and the options of saving:
//!    resolution, cropping, transparency and metadata.
//! 10. [`interactive`]: windows, the toolbar and keyboard, events and
//!     callbacks, widgets, animation and the browser backend.
//! 11. [`extending`]: artists, projections, unit converters, legend
//!     handlers and toolbar tools of your own.
//! 12. [`testing`]: testing the figures your program draws.
//!
//! # Words you will meet
//!
//! matplotlib's vocabulary is plt-rs' vocabulary. The words that mean
//! something specific:
//!
//! | word         | meaning                                                         | chapter |
//! |--------------|-----------------------------------------------------------------|---------|
//! | figure       | the whole image: a size, a background, and everything drawn on it | [`first_plot`] |
//! | axes         | one plotting area of a figure, with its x and y axis (one axes, two axes) | [`first_plot`] |
//! | axis         | the x or the y axis of an axes: its limits, ticks and label     | [`anatomy`] |
//! | tick         | a mark on an axis, with its label                               | [`anatomy`], [`scales`] |
//! | spine        | a line bounding the data area, carrying the ticks               | [`anatomy`] |
//! | artist       | anything drawn: lines, patches (shapes), texts, images, collections | [`anatomy`] |
//! | handle       | the Rust value through which you change an artist               | [`anatomy`] |
//! | colormap     | a function from numbers to colours                              | [`plot_types`] |
//! | rcParams     | the runtime settings every new artist reads its defaults from   | [`styling`] |
//! | style sheet  | a named set of rcParams                                         | [`styling`] |
//! | look         | the defaults figures start from: plt-rs' modern one or matplotlib's | [`styling`] |
//! | layout engine | what places the axes so that their labels fit                  | [`layout`] |
//! | backend      | what shows figures: windows, a web page, or nothing             | [`interactive`] |
//!
//! # How the examples are written
//!
//! * They start with `use plt::prelude::*;`, which imports the names
//!   nearly every program uses ([`Figure`], [`Axes`], [`LineProps`],
//!   [`linspace`], ...). Other items are imported by their path.
//! * Fallible calls end in `?`. Each example runs in a function returning
//!   `Result<(), Box<dyn std::error::Error>>`, hidden from the page; in
//!   your program, `main` or the function drawing the figure can return
//!   the same type. Chapter [`data`] explains the errors.
//! * Most examples check what they did with `assert!`, which shows what a
//!   call returns. Your code rarely needs to.
//! * Figures take plt-rs' modern look unless told otherwise, and the
//!   examples hold in either look. Chapter [`styling`] explains how to
//!   choose.
//!
//! # Seeing the figures
//!
//! The pages describe the figures their examples draw. To see one, paste
//! the example into a `main` returning `Result<(), Box<dyn
//! std::error::Error>>` and end it with `fig.savefig("out.png")?` or
//! `plt::gui::show(&[fig])?`. Or run the gallery: over ninety figures,
//! each drawn by a short, commented function in
//! `crates/plt/examples/gallery/`, grouped by topic.
//!
//! ```text
//! cargo run -p plt --release --example gallery -- target/gallery
//! ```
//!
//! writes each figure as a PNG file and an `index.html` showing them
//! section by section. The chapters name the gallery entries that go
//! further than their examples by function and file: "`line_plot` in
//! `lines.rs`" is the function in `crates/plt/examples/gallery/lines.rs`
//! that draws `target/gallery/line_plot.png`.
//!
//! # After the tutorial
//!
//! * The [crate overview](crate) lists every module.
//! * The [`porting`](crate::porting) guide has tables of the methods of
//!   each topic and longer examples, written for matplotlib users but
//!   useful to anyone looking for a method.
//! * matplotlib's own documentation and tutorials apply too: its names
//!   are plt-rs' names, and [`porting::conventions`](crate::porting::conventions)
//!   explains how its Python calls become Rust calls.
//!
//! [`Figure`]: crate::figure::Figure
//! [`Axes`]: crate::axes::Axes
//! [`LineProps`]: crate::lines::LineProps
//! [`linspace`]: crate::numeric::linspace

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
