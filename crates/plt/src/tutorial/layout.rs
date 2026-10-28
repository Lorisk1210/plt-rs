//! Chapter 6: layout, several axes in one figure.
//!
//! A figure can hold any number of axes. Most figures place them on a
//! grid, and a *layout engine*, when the figure has one, sizes the
//! grid's cells so that tick labels, axis labels and titles fit without
//! overlapping. This chapter goes from regular grids to irregular ones,
//! then to what sits beside or inside an axes: colorbars, twin axes and
//! insets.
//!
//! # A grid of axes
//!
//! [`Figure::subplots`] adds a grid of axes and returns them as a
//! [`Grid`]`<`[`Axes`]`>`, indexed by `(row, column)`;
//! [`Grid::as_slice`] lists them row by row for a loop.
//! [`Figure::add_subplot`] adds one cell of a grid at a time, numbered
//! from 1 in reading order.
//!
//! ```
//! use plt::prelude::*;
//!
//! let fig = Figure::with_size(8.0, 5.0, 100.0);
//! let axs = fig.subplots(2, 3)?;
//! let x = linspace(0.0, 6.0, 100);
//! for (k, ax) in [1.0, 2.0, 3.0, 4.0, 5.0, 6.0].into_iter().zip(axs.as_slice()) {
//!     ax.plot(&x, x.iter().map(|x| (k * x).sin()))?;
//!     ax.set_title(format!("k = {k}"));
//! }
//! axs[(1, 0)].set_xlabel("x");
//! fig.suptitle("sin kx for six values of k");
//! assert_eq!(axs.shape(), (2, 3));
//! assert_eq!(fig.axes().len(), 6);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`Figure::suptitle`], [`Figure::supxlabel`] and
//! [`Figure::supylabel`] write a title and labels for the whole figure.
//!
//! # Shared axes
//!
//! Axes that show comparable data should share their scales: zooming one
//! zooms the others, and the limits cover the data of all of them.
//! [`Figure::subplots_shared`] takes a [`Share`] for x and one for y:
//! shared by all the axes, by each row, by each column, or not at all.
//! [`Axes::label_outer`] then keeps tick labels only on the outer edge of
//! the grid.
//!
//! ```
//! use plt::figure::Share;
//! use plt::prelude::*;
//! use plt::ticker::AxisName;
//!
//! let fig = Figure::new();
//! let axs = fig.subplots_shared(2, 1, Share::All, Share::None)?;
//! axs[(0, 0)].plot([0.0, 10.0], [0.0, 1.0])?;
//! axs[(1, 0)].plot([5.0, 20.0], [3.0, 2.0])?;
//! for ax in axs.as_slice() {
//!     ax.label_outer();
//! }
//! assert_eq!(axs[(0, 0)].xlim(), axs[(1, 0)].xlim(), "one x range for both");
//! assert!(axs[(0, 0)].shared_axes(AxisName::X).contains(&axs[(1, 0)]));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`Axes::twinx`] makes a second axes over the first, sharing its x
//! axis but with a y axis of its own on the right, for two quantities
//! measured in different units along the same x. It returns a `Result`:
//! axes that fix their aspect, as polar axes do, cannot have twins. Each
//! of the two axes has its own legend; for one legend listing the
//! artists of both, give it their handles, and put it on the twin, which
//! is drawn on top.
//!
//! ```
//! use plt::legend::LegendHandle;
//! use plt::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! let months = ["Jan", "Feb", "Mar", "Apr"];
//! let rain = BarOptions { label: Some("rain".into()), ..BarOptions::default() };
//! let bars = ax.bar(months, [64.0, 59.0, 70.0, 88.0], rain)?;
//! ax.set_ylabel("rain (mm)");
//!
//! let right = ax.twinx()?;
//! let warmth = LineProps::default().color("tab:red").label("temperature");
//! let line = right.plot_with(months, [0.5, 1.8, 5.6, 9.4], warmth)?;
//! right.set_ylabel("temperature (°C)");
//!
//! let both = [LegendHandle::from(&bars), LegendHandle::from(&line)];
//! let legend = right.legend(LegendOptions::default().handles(both))?;
//! assert_eq!(legend.labels(), ["rain", "temperature"]);
//! assert_eq!(right.xlim(), ax.xlim(), "one x axis");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Gallery: `twin_axes` and `secondary_axes` in `layout.rs`.
//!
//! # Mosaics
//!
//! When the cells differ in size, draw the grid as text.
//! [`Figure::subplot_mosaic`] takes rows of labels separated by `;` (or
//! a [`Mosaic`] built from cells): a label repeated across cells makes
//! one axes spanning them, and `.` leaves a cell empty. The result maps
//! each label to its axes. Index it with a label, which panics for a
//! label the mosaic does not have, as indexing a map does; `get` returns
//! an `Option` instead.
//!
//! ```
//! use plt::figure::MosaicOptions;
//! use plt::prelude::*;
//!
//! let fig = Figure::new();
//! let axd = fig.subplot_mosaic("AAB;CCB", MosaicOptions::default())?;
//! axd["A"].set_title("A spans two columns");
//! axd["B"].set_title("B spans two rows");
//! assert_eq!(axd.iter().count(), 3);
//! assert!(axd.get("D").is_none());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Gallery: `mosaic` in `layout.rs`.
//!
//! # Gridspecs
//!
//! A [`GridSpec`] is the grid underneath: its numbers of rows and
//! columns, their relative sizes, and the spacing. Take a block of its
//! cells with [`GridSpec::span`], which uses Rust ranges, and add an axes
//! on it. [`GridSpec::nested`] divides one block into a grid of its own.
//!
//! ```
//! use plt::gridspec::{GridSpec, SubplotOverrides};
//! use plt::prelude::*;
//!
//! let fig = Figure::new();
//! let widths = Some(vec![2.0, 1.0]);
//! let gs = GridSpec::with_ratios(2, 2, widths, None, SubplotOverrides::default())?;
//! let main = fig.add_subplot_spec(&gs.span(0..2, 0..1)?)?; // both rows, first column
//! let top = fig.add_subplot_spec(&gs.cell(0, 1)?)?;
//! let bottom = fig.add_subplot_spec(&gs.cell(1, 1)?)?;
//! main.set_title("twice as wide");
//! assert_eq!(fig.axes(), [main, top, bottom]);
//! assert!(gs.cell(2, 0).is_err(), "no third row");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Gallery: `nested_gridspecs` in `layout.rs`.
//!
//! # Layout engines
//!
//! Without help, axes keep fixed positions in the figure and long labels
//! can run into their neighbours or off the edge. A layout engine moves
//! the axes, every time the figure is drawn, so that their decorations
//! fit. *Constrained layout* ([`LayoutEngine::constrained`]) also makes
//! room for colorbars, legends outside the axes and figure titles; the
//! modern look turns it on for every figure. *Tight layout*
//! ([`LayoutEngine::tight`]) is an older, simpler engine. matplotlib's
//! look starts without an engine, as matplotlib does, so set one
//! explicitly when the layout must not depend on the look.
//!
//! ```
//! use plt::layout::LayoutEngine;
//! use plt::prelude::*;
//!
//! let fig = Figure::new();
//! fig.set_layout_engine(LayoutEngine::constrained())?;
//! let axs = fig.subplots(1, 2)?;
//! axs[(0, 0)].set_ylabel("a long label that needs room");
//! assert!(matches!(fig.layout_engine(), Some(LayoutEngine::Constrained(_))));
//!
//! fig.set_layout_engine(None)?; // back to fixed positions
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! With no engine, [`Figure::subplots_adjust`] sets the margins and the
//! spacing of the grid by hand.
//!
//! # Colorbars
//!
//! [`Figure::colorbar`] adds a colorbar for anything colour-mapped (an
//! image, a scatter plot coloured by value, filled contours). It makes a
//! new axes and takes the room from the axes of the mapped artist, or
//! from the axes you name. One colorbar can serve several axes when they
//! share a colour scale.
//!
//! ```
//! use plt::colorbar::ColorbarAxes;
//! use plt::colors::ColorizerOptions;
//! use plt::prelude::*;
//!
//! let fig = Figure::new();
//! let axs = fig.subplots(1, 2)?;
//! let same_scale = || ImshowOptions {
//!     colorizer: ColorizerOptions::default().limits(Some(0.0), Some(100.0)),
//!     ..ImshowOptions::default()
//! };
//! let low = axs[(0, 0)].imshow(Grid::from_fn(10, 10, |r, c| (r * c) as f64), same_scale())?;
//! axs[(0, 1)].imshow(Grid::from_fn(10, 10, |r, c| (r + c) as f64 * 5.0), same_scale())?;
//! let options = ColorbarOptions {
//!     axes: ColorbarAxes::Parents(axs.as_slice().to_vec()),
//!     label: "value".into(),
//!     ..ColorbarOptions::default()
//! };
//! let colorbar = fig.colorbar(&low, options)?;
//! assert_eq!(colorbar.ax().ylim(), (0.0, 100.0));
//! assert_eq!(fig.axes().len(), 3, "the colorbar has an axes of its own");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Both images share the limits 0 to 100, set through their
//! [`ColorizerOptions`], so one colorbar reads for both. Gallery:
//! `shared_colorbar` in `layout.rs`.
//!
//! # Insets
//!
//! [`Axes::inset_axes`] places a small axes inside another one, at a
//! rectangle given in fractions of the parent (left, bottom, width,
//! height). Set its limits to a detail of the data and
//! [`Axes::indicate_inset_zoom`] draws a box around that detail in the
//! parent, with lines to the inset.
//!
//! ```
//! use plt::prelude::*;
//!
//! let fig = Figure::new();
//! let ax = fig.subplot();
//! let x = linspace(0.0, 10.0, 500);
//! let y: Vec<f64> = x.iter().map(|x| x.sin() + 0.05 * (20.0 * x).sin()).collect();
//! ax.plot(&x, &y)?;
//!
//! let inset = ax.inset_axes([0.6, 0.6, 0.35, 0.35]);
//! inset.plot(&x, &y)?;
//! inset.set_xlim(1.2, 2.0)?.set_ylim(0.85, 1.1)?;
//! ax.indicate_inset_zoom(&inset, PatchProps::default());
//! assert_eq!(fig.axes(), [ax], "an inset belongs to its parent axes");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Gallery: `inset` and `broken_axes` in `layout.rs`; `zoomed_inset` in
//! `grids.rs`.
//!
//! # Subfigures
//!
//! A subfigure is a figure inside a figure: a region with its own axes,
//! its own suptitle, its own background and its own layout.
//! [`Figure::subfigures`] divides a figure into a grid of them, returned
//! as [`Figure`] handles, so everything above works inside each one.
//!
//! ```
//! use plt::figure::SubfigureOptions;
//! use plt::prelude::*;
//!
//! let fig = Figure::with_size(9.0, 4.0, 100.0);
//! let options = SubfigureOptions {
//!     width_ratios: Some(vec![2.0, 1.0]),
//!     ..SubfigureOptions::default()
//! };
//! let parts = fig.subfigures(1, 2, options)?;
//! let (left, right) = (&parts[(0, 0)], &parts[(0, 1)]);
//! left.subplots(2, 2)?;
//! left.suptitle("Four small axes");
//! right.subplot();
//! right.suptitle("One large axes");
//! right.set_facecolor("0.95");
//! assert_eq!(left.axes().len(), 4);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Gallery: `subfigures` in `layout.rs`. The
//! [`porting::figures`](crate::porting::figures) page lists the layout
//! methods and their options.
//!
//! Next: [text and annotation](super::text).
//!
//! [`Figure`]: crate::figure::Figure
//! [`Figure::subplots`]: crate::figure::Figure::subplots
//! [`Figure::add_subplot`]: crate::figure::Figure::add_subplot
//! [`Figure::suptitle`]: crate::figure::Figure::suptitle
//! [`Figure::supxlabel`]: crate::figure::Figure::supxlabel
//! [`Figure::supylabel`]: crate::figure::Figure::supylabel
//! [`Figure::subplots_shared`]: crate::figure::Figure::subplots_shared
//! [`Figure::subplot_mosaic`]: crate::figure::Figure::subplot_mosaic
//! [`Figure::subplots_adjust`]: crate::figure::Figure::subplots_adjust
//! [`Figure::colorbar`]: crate::figure::Figure::colorbar
//! [`Figure::subfigures`]: crate::figure::Figure::subfigures
//! [`Grid`]: crate::numeric::Grid
//! [`Grid::as_slice`]: crate::numeric::Grid::as_slice
//! [`Axes`]: crate::axes::Axes
//! [`Axes::label_outer`]: crate::axes::Axes::label_outer
//! [`Axes::twinx`]: crate::axes::Axes::twinx
//! [`Axes::inset_axes`]: crate::axes::Axes::inset_axes
//! [`Axes::indicate_inset_zoom`]: crate::axes::Axes::indicate_inset_zoom
//! [`Share`]: crate::figure::Share
//! [`Mosaic`]: crate::figure::Mosaic
//! [`GridSpec`]: crate::gridspec::GridSpec
//! [`GridSpec::span`]: crate::gridspec::GridSpec::span
//! [`GridSpec::nested`]: crate::gridspec::GridSpec::nested
//! [`LayoutEngine::constrained`]: crate::layout::LayoutEngine::constrained
//! [`LayoutEngine::tight`]: crate::layout::LayoutEngine::tight
//! [`ColorizerOptions`]: crate::colors::ColorizerOptions
