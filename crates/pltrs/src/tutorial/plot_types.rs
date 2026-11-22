//! Chapter 3: plot types, and when to use each.
//!
//! Each kind of plot is a method of [`Axes`] with the matplotlib name, for
//! example `plot`, `scatter` or `hist`. It returns a handle, or a struct of
//! handles. Select the plot from the data and the question:
//!
//! | data                                   | question                         | plot |
//! |----------------------------------------|----------------------------------|------|
//! | y as a function of an ordered x        | how does y change?               | [line](#lines-and-markers) |
//! | pairs of values, in no order           | how do two quantities relate?    | [scatter](#scatter-plots) |
//! | one value per category                 | which is larger?                 | [bars](#bars) |
//! | many samples of one quantity           | how are they spread?             | [histogram](#histograms) |
//! | a range of y at each x                 | how uncertain, how much between? | [filled area](#filled-areas) |
//! | values on a regular 2-D grid           | where is it high or low?         | [image](#images-and-colormaps) |
//! | a smooth function of two variables     | where are the levels?            | [contours](#contours) |
//!
//! # Lines and markers
//!
//! [`Axes::plot`] joins the points in order, with an optional *marker* at
//! each point. [`Axes::plot_fmt`] accepts matplotlib format
//! strings: `"o-"` gives circles and a solid line, `"s"` gives squares only
//! and `"r--"` gives a red dashed line.
//!
//! ```
//! use pltrs::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! let x = [1.0, 2.0, 3.0, 4.0, 5.0];
//! ax.plot_fmt(x, [3.0, 1.0, 4.0, 1.5, 5.0], "o-")?;
//!
//! let squares = LineProps::default()
//!     .marker(MarkerSymbol::Square)
//!     .linestyle(LineStyle::None)
//!     .label("markers only");
//! let markers = ax.plot_with(x, [2.0, 2.5, 2.0, 3.5, 3.0], squares)?;
//! assert_eq!(markers.linestyle(), LineStyle::None);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! A NaN makes a gap in the line. [`Axes::step`] draws steps,
//! [`Axes::errorbar`] draws error bars, and [`Axes::semilogy`] and similar
//! methods use logarithmic axes (chapter [`scales`](super::scales)).
//! Gallery: `line_plot`, `steps`, `errorbars` and `stem` in `lines.rs`.
//!
//! # Scatter plots
//!
//! [`Axes::scatter`] draws one marker per point, and the size and color of
//! each marker can show more data. A *colormap* maps numbers to
//! colors. [`colormaps::get`](crate::colors::colormaps::get) finds one by
//! name (`"viridis"`, `"plasma"`, ...). A *colorbar* shows the mapping.
//!
//! ```
//! use pltrs::prelude::*;
//!
//! let fig = Figure::new();
//! let ax = fig.subplot();
//! let x: Vec<f64> = (0..50).map(|i| f64::from(i) / 5.0).collect();
//! let y: Vec<f64> = x.iter().map(|x| x.sin() + 0.1 * x).collect();
//! let options = ScatterOptions {
//!     sizes: Some(x.iter().map(|x| 10.0 + 8.0 * x).collect()),
//!     colors: MappedColors::new(&y).cmap(colormaps::get("viridis")?).into(),
//!     ..ScatterOptions::default()
//! };
//! let points = ax.scatter_with(&x, &y, options)?;
//! fig.colorbar(&points, ColorbarOptions::default())?;
//! assert_eq!(points.offsets().len(), 50);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! A scatter plot is one [`Collection`](crate::collections::Collection)
//! artist. If all markers look the same, `plot` with markers and no line
//! is enough. Gallery: `polar_scatter` in `projections.rs`, `scatter_3d` in
//! `three_d.rs`.
//!
//! # Bars
//!
//! [`Axes::bar`] draws one bar per category. The x values can be numbers
//! or names. pltrs places names at 0, 1, 2, ... in the order of their first
//! appearance. [`Axes::barh`] draws horizontal bars. [`Axes::bar_label`]
//! writes the values on the bars.
//!
//! ```
//! use pltrs::axes::BarLabelOptions;
//! use pltrs::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! let bars = ax.bar(["apples", "pears", "plums"], [12.0, 7.0, 9.0], BarOptions::default())?;
//! let labels = ax.bar_label(&bars, BarLabelOptions::default())?;
//!
//! assert_eq!(bars.patches.len(), 3);
//! assert_eq!(labels[1].text(), "7");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Gallery: `grouped_bars`, `stacked_bars` and `horizontal_bars` in
//! `bars.rs`, `categorical` in `units.rs`.
//!
//! # Histograms
//!
//! A histogram counts the samples in each interval (bin). [`Axes::hist`]
//! counts and draws. It accepts one dataset, or several datasets side by
//! side, stacked or overlapping.
//!
//! ```
//! use pltrs::prelude::*;
//! use pltrs::stats::Bins;
//!
//! // 1000 samples shaped like a bell curve, without a random-number
//! // generator: each is the sum of three evenly spread sequences.
//! let samples: Vec<f64> = (0..1000)
//!     .map(|i| [0.618_034, 0.414_214, 0.732_051].iter().map(|k| (f64::from(i) * k).fract()).sum())
//!     .collect();
//!
//! let ax = Figure::new().subplot();
//! let options = HistPlotOptions {
//!     bins: Some(Bins::Count(30)),
//!     label: "samples".into(), // one per dataset for several: ["a", "b"].into()
//!     ..HistPlotOptions::default()
//! };
//! let hist = ax.hist(&samples, options)?;
//!
//! assert_eq!(hist.edges.len(), 31, "30 bins have 31 edges");
//! assert_eq!(hist.tops[0].iter().sum::<f64>(), 1000.0);
//! assert_eq!(ax.legend(LegendOptions::default())?.labels(), ["samples"]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`stats::hist`](crate::stats::hist) counts without drawing.
//! [`Axes::hist2d`] and [`Axes::hexbin`] count in squares or hexagons.
//! [`Axes::boxplot`] and [`Axes::violinplot`] summarize distributions. Gallery: `histograms`, `box_and_violin`
//! and `ecdf` in `stats.rs`, `hist2d` in `density.rs`.
//!
//! # Filled areas
//!
//! [`Axes::fill_between`] fills the area between two curves with the same
//! x. For a constant curve, give one value, for example `[0.0]`.
//!
//! ```
//! use pltrs::collections::CollectionProps;
//! use pltrs::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! let x = linspace(0.0, 10.0, 100);
//! let mean: Vec<f64> = x.iter().map(|x| x.sin()).collect();
//! let band = FillBetweenOptions {
//!     props: CollectionProps::default().alpha(0.3).label("± 0.3"),
//!     ..FillBetweenOptions::default()
//! };
//! let (low, high) = (mean.iter().map(|m| m - 0.3), mean.iter().map(|m| m + 0.3));
//! ax.fill_between(&x, low, high, band)?;
//! ax.plot_with(&x, &mean, LineProps::default().label("mean"))?;
//! assert_eq!(ax.collections().len(), 1);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`Axes::fill`] fills a polygon. [`Axes::stackplot`] stacks areas.
//! [`Axes::axvspan`] shades a range of x across the axes. Gallery:
//! `confidence_band`, `fill_between_where` and `stackplot` in `lines.rs`.
//!
//! # Images and colormaps
//!
//! [`Axes::imshow`] draws a [`Grid`] (chapter [`data`](super::data)) of
//! values as colored cells, with row 0 at the top. A colormap gives the
//! colors. pltrs draws a grid of RGB or RGBA pixels as a picture. An
//! `extent` (`[left, right, bottom, top]`) places the image in data
//! coordinates. Without it, the cells are at their column and row numbers.
//! The colormap is a builder method and the other options are fields:
//!
//! ```
//! use pltrs::prelude::*;
//!
//! let fig = Figure::new();
//! let ax = fig.subplot();
//! let z = Grid::from_fn(40, 60, |row, col| (col as f64 / 10.0).sin() * (row as f64 / 10.0).cos());
//! let options = ImshowOptions {
//!     extent: Some([0.0, 6.0, 0.0, 4.0].into()),
//!     ..ImshowOptions::default().cmap(colormaps::get("magma")?)
//! };
//! let image = ax.imshow(z, options)?;
//! fig.colorbar(&image, ColorbarOptions::default())?;
//! assert_eq!(image.shape(), (40, 60));
//! assert_eq!(ax.xlim(), (0.0, 6.0));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Cells are square by default
//! ([`ImshowOptions::aspect`](crate::axes::ImshowOptions::aspect)).
//! [`Axes::pcolormesh`] draws cells of any size and shape. Use a
//! sequential colormap (`viridis`, `magma`) for values from low to high.
//! Use a diverging colormap (`coolwarm`, `RdBu`) for values around a
//! center, and a qualitative colormap (`tab10`) for categories.
//!
//! Gallery: `image_colorbar` and `imshow_interpolation` in `images.rs`,
//! `colormaps` in `colour.rs`.
//!
//! # Contours
//!
//! A contour line joins the points where a function of two variables has
//! one value. [`Axes::contour`] draws the lines, and [`Axes::contourf`]
//! fills the bands. Both take a [`QuadField`](crate::contour::QuadField):
//! values on a grid, with the x and y of the grid points. [`meshgrid`]
//! makes the grid points.
//!
//! ```
//! use pltrs::contour::{ClabelOptions, ContourOptions, Levels, QuadCoords, QuadField};
//! use pltrs::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! let along = linspace(-2.0, 2.0, 81);
//! let (x, y) = meshgrid(&along, &along);
//! let z = Grid::from_fn(81, 81, |r, c| (-(x[(r, c)].powi(2) + y[(r, c)].powi(2))).exp());
//! let field = QuadField::new(z).with_coords(QuadCoords::Mesh { x, y });
//!
//! let bands = ContourOptions { levels: Levels::Count(8), ..ContourOptions::default() };
//! ax.contourf(&field, bands)?;
//! let lines = ContourOptions {
//!     levels: Levels::Values(vec![0.25, 0.5, 0.75]),
//!     colors: Some(vec!["k".into()]),
//!     ..ContourOptions::default()
//! };
//! let contours = ax.contour(&field, lines)?;
//! contours.clabel(ClabelOptions::default())?;
//! assert_eq!(contours.levels(), [0.25, 0.5, 0.75]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! For scattered points, [`Axes::tricontour`] uses a triangulation.
//! Gallery: `contours` and `tricontour` in `contours.rs`.
//!
//! # More plot types
//!
//! * Statistics: [`Axes::boxplot`], [`Axes::violinplot`], [`Axes::ecdf`],
//!   [`Axes::pie`], [`Axes::eventplot`].
//! * Vector fields: [`Axes::quiver`] (arrows), [`Axes::barbs`],
//!   [`Axes::streamplot`] (gallery: `vectors.rs`).
//! * Signals: [`Axes::specgram`], [`Axes::psd`] (gallery: `spectrogram`
//!   in `density.rs`).
//! * Other projections: polar and map axes ([`Figure::polar_subplot`],
//!   [`Figure::geo_subplot`]) and 3-D axes ([`Figure::subplot_3d`]), with
//!   their own plotting methods (gallery: `projections.rs`, `three_d.rs`,
//!   and [`porting::toolkits`](crate::porting::toolkits)).
//!
//! The pages [`porting::plotting`](crate::porting::plotting) and
//! [`porting::images`](crate::porting::images) list each plotting method
//! with its options.
//!
//! Next: [data and options](super::data).
//!
//! [`Axes`]: crate::axes::Axes
//! [`Axes::plot`]: crate::axes::Axes::plot
//! [`Axes::plot_fmt`]: crate::axes::Axes::plot_fmt
//! [`Axes::step`]: crate::axes::Axes::step
//! [`Axes::errorbar`]: crate::axes::Axes::errorbar
//! [`Axes::semilogy`]: crate::axes::Axes::semilogy
//! [`Axes::scatter`]: crate::axes::Axes::scatter
//! [`Axes::bar`]: crate::axes::Axes::bar
//! [`Axes::barh`]: crate::axes::Axes::barh
//! [`Axes::bar_label`]: crate::axes::Axes::bar_label
//! [`Axes::hist`]: crate::axes::Axes::hist
//! [`Axes::hist2d`]: crate::axes::Axes::hist2d
//! [`Axes::hexbin`]: crate::axes::Axes::hexbin
//! [`Axes::boxplot`]: crate::axes::Axes::boxplot
//! [`Axes::violinplot`]: crate::axes::Axes::violinplot
//! [`Axes::fill_between`]: crate::axes::Axes::fill_between
//! [`Axes::fill`]: crate::axes::Axes::fill
//! [`Axes::stackplot`]: crate::axes::Axes::stackplot
//! [`Axes::axvspan`]: crate::axes::Axes::axvspan
//! [`Axes::imshow`]: crate::axes::Axes::imshow
//! [`Axes::pcolormesh`]: crate::axes::Axes::pcolormesh
//! [`Axes::contour`]: crate::axes::Axes::contour
//! [`Axes::contourf`]: crate::axes::Axes::contourf
//! [`Axes::tricontour`]: crate::axes::Axes::tricontour
//! [`Axes::ecdf`]: crate::axes::Axes::ecdf
//! [`Axes::pie`]: crate::axes::Axes::pie
//! [`Axes::eventplot`]: crate::axes::Axes::eventplot
//! [`Axes::quiver`]: crate::axes::Axes::quiver
//! [`Axes::barbs`]: crate::axes::Axes::barbs
//! [`Axes::streamplot`]: crate::axes::Axes::streamplot
//! [`Axes::specgram`]: crate::axes::Axes::specgram
//! [`Axes::psd`]: crate::axes::Axes::psd
//! [`Figure::polar_subplot`]: crate::figure::Figure::polar_subplot
//! [`Figure::geo_subplot`]: crate::figure::Figure::geo_subplot
//! [`Figure::subplot_3d`]: crate::figure::Figure::subplot_3d
//! [`Grid`]: crate::numeric::Grid
//! [`meshgrid`]: crate::numeric::meshgrid
