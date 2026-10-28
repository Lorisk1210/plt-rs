//! Chapter 3: plot types, and when to use which.
//!
//! Each kind of plot is a method of [`Axes`] named after matplotlib's
//! function: `plot`, `scatter`, `bar`, `hist`, `fill_between`, `imshow`,
//! `contour`, and many more. Each returns a handle to what it drew, or a
//! struct of handles when it drew several artists. The choice of plot
//! depends on the shape of the data and on the question the figure
//! answers:
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
//! [`Axes::plot`] joins the points in the order given, which suits data
//! measured along an ordered x: a time series, a function, a sweep. A
//! *marker* is a symbol drawn at each point. Add markers when the points
//! themselves matter (few, irregular, measured), and drop the line when
//! the order means nothing. [`Axes::plot_fmt`] takes matplotlib's short
//! format strings: `"o-"` is circles joined by a solid line, `"s"` squares
//! alone, `"r--"` a red dashed line.
//!
//! ```
//! use plt::prelude::*;
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
//! A NaN in the data leaves a gap in the line. Related methods:
//! [`Axes::step`] for values that hold until the next point,
//! [`Axes::errorbar`] for points with error bars, [`Axes::semilogy`] and
//! friends for logarithmic axes (chapter [`scales`](super::scales)).
//! Gallery: `line_plot`, `steps`, `errorbars` and `stem` in `lines.rs`.
//!
//! # Scatter plots
//!
//! [`Axes::scatter`] draws one marker per point and joins nothing. It
//! shows how two quantities relate, and can show two more through the
//! size and the colour of each marker. A colour that stands for a number
//! goes through a *colormap*, a function from numbers to colours;
//! [`colormaps::get`](crate::colors::colormaps::get) finds one by name
//! (`"viridis"`, `"plasma"`, `"coolwarm"`, ...). A *colorbar* beside the
//! axes shows the mapping.
//!
//! ```
//! use plt::prelude::*;
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
//! A scatter plot is a [`Collection`](crate::collections::Collection):
//! one artist drawing many markers, much faster than one line per point.
//! Use `plot` with markers and no line instead when all markers look the
//! same. Gallery: `polar_scatter` in `projections.rs`, `scatter_3d` in
//! `three_d.rs`.
//!
//! # Bars
//!
//! [`Axes::bar`] compares one value per category. The x values may be
//! numbers or names: names become categories, placed at 0, 1, 2, ... in
//! the order they first appear. [`Axes::barh`] draws horizontal bars,
//! easier to read with long names. [`Axes::bar_label`] writes the values
//! on the bars.
//!
//! ```
//! use plt::axes::BarLabelOptions;
//! use plt::prelude::*;
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
//! `bars.rs`; `categorical` in `units.rs`.
//!
//! # Histograms
//!
//! A histogram shows how many samples of one quantity fall in each
//! interval (bin) of its range. [`Axes::hist`] counts and draws the bars;
//! it takes one dataset, or several to draw side by side, stacked or
//! overlapping. The number of bins changes what the plot shows: too few
//! hide the shape, too many turn it into noise.
//!
//! ```
//! use plt::prelude::*;
//! use plt::stats::Bins;
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
//! [`stats::hist`](crate::stats::hist) counts without drawing. For two
//! quantities at once, [`Axes::hist2d`] and [`Axes::hexbin`] count in
//! squares or hexagons; for comparing distributions, [`Axes::boxplot`]
//! and [`Axes::violinplot`] summarise each one. Gallery: `histograms`,
//! `box_and_violin` and `ecdf` in `stats.rs`; `hist2d` in `density.rs`.
//!
//! # Filled areas
//!
//! [`Axes::fill_between`] fills the area between two curves sharing an
//! x: a range of measurements, an uncertainty band, the area under a
//! curve (with a constant lower curve, one value for every x: `[0.0]`).
//! Some transparency lets the grid show through.
//!
//! ```
//! use plt::collections::CollectionProps;
//! use plt::prelude::*;
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
//! [`Axes::fill`] fills a polygon, [`Axes::stackplot`] stacks areas,
//! [`Axes::axvspan`] shades a range of x across the whole axes. Gallery:
//! `confidence_band`, `fill_between_where` and `stackplot` in
//! `lines.rs`.
//!
//! # Images and colormaps
//!
//! [`Axes::imshow`] draws a 2-D array as a grid of coloured cells, row 0
//! at the top as in a picture. The values go through a colormap, as in a
//! colour-mapped scatter plot. The array is a [`Grid`]: rows and columns
//! of one type, made from a function of the row and column or from rows
//! (chapter [`data`](super::data)). A grid of RGB(A) pixels is drawn as
//! the picture it is. The cells sit at the column and row numbers unless
//! an `extent` (`[left, right, bottom, top]`) places the image in data
//! coordinates. The colormap has a builder method, the other options are
//! fields, so struct update syntax fills them around the builder:
//!
//! ```
//! use plt::prelude::*;
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
//! An image keeps its cells square unless told otherwise
//! ([`ImshowOptions::aspect`](crate::axes::ImshowOptions::aspect)).
//! [`Axes::pcolormesh`] draws cells of any size and shape. Colormaps come
//! in families: *sequential* ones (`viridis`, `magma`) for values from
//! low to high, *diverging* ones (`coolwarm`, `RdBu`) for values on both
//! sides of a centre, *qualitative* ones (`tab10`) for categories.
//! Gallery: `image_colorbar` and `imshow_interpolation` in `images.rs`,
//! `colormaps` in `colour.rs`.
//!
//! # Contours
//!
//! A contour line joins the points where a function of two variables has
//! one value, as on a map of heights. [`Axes::contour`] draws the lines,
//! [`Axes::contourf`] fills the bands between them. Both take a
//! [`QuadField`](crate::contour::QuadField): the values on a grid, with
//! the x and y of the grid points; [`meshgrid`] makes those from the
//! values along each axis.
//!
//! ```
//! use plt::contour::{ClabelOptions, ContourOptions, Levels, QuadCoords, QuadField};
//! use plt::prelude::*;
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
//! For scattered points rather than a grid, [`Axes::tricontour`] works on
//! a triangulation. Gallery: `contours` and `tricontour` in
//! `contours.rs`.
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
//!   their own plotting methods (gallery: `projections.rs`,
//!   `three_d.rs`; [`porting::toolkits`](crate::porting::toolkits)).
//!
//! The [`porting::plotting`](crate::porting::plotting) and
//! [`porting::images`](crate::porting::images) pages list every plotting
//! method with its options.
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
