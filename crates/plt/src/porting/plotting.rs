//! Plotting functions: how their keywords map to options, and the data
//! they take.
//!
//! The plotting methods of [`Axes`] carry matplotlib's names. Where a
//! matplotlib function behaves differently depending on which arguments it
//! gets, plt-rs has one method per behaviour:
//!
//! * [`Axes::plot`] takes x and y; [`Axes::plot_y`] only y (x is `0, 1,
//!   2, ...`); [`Axes::plot_fmt`] adds a format string; [`Axes::plot_with`]
//!   takes a [`LineProps`]. One call draws one line: `plot(x, y1, x, y2)`
//!   is two calls.
//! * [`Axes::scatter`] uses the defaults, [`Axes::scatter_with`] takes a
//!   [`ScatterOptions`]. The same `_with` pattern appears wherever a
//!   matplotlib function is commonly called without keywords.
//! * `axhline(y, xmin, xmax)` becomes [`Axes::axhline`] and
//!   [`Axes::axhline_between`]; the same for `axvline`, `axhspan` and
//!   `axvspan`.
//!
//! Keywords map in three ways. Artist properties (`color`, `linewidth`,
//! `alpha`, `label`, `zorder`, ...) go into the property set of the artist
//! the method makes: [`LineProps`] for lines, [`PatchProps`] for patches,
//! [`CollectionProps`] for collections. Keywords of the method itself
//! (`bins`, `where`, `capsize`, ...) are fields of its option struct, named
//! as in matplotlib except where the name was ambiguous: `bar(width=,
//! bottom=)` is [`BarOptions::thickness`] and [`BarOptions::base`], shared
//! by `bar` and `barh`, and `fill_between(where=)` is
//! [`FillBetweenOptions::mask`]. Values that take several forms in
//! matplotlib are enums: `bins=10`, `bins="auto"` or `bins=[...]` is a
//! [`Bins`]; scatter's `c=` is a [`ScatterColors`].
//!
//! matplotlib returns lists, tuples and containers; plt-rs returns one
//! typed value: a [`Line2D`], a [`Collection`], a [`Patch`] or a
//! container struct with named fields ([`BarContainer`],
//! [`ErrorbarContainer`], [`HistPlot`], [`BoxplotArtists`], ...).
//!
//! # Equivalences
//!
//! | matplotlib                                         | plt-rs                                            |
//! |----------------------------------------------------|---------------------------------------------------|
//! | `ax.plot(y)`, `ax.plot(x, y)`                      | [`Axes::plot_y`], [`Axes::plot`]                  |
//! | `ax.plot(x, y, "r--")`, `ax.plot(x, y, lw=2)`      | [`Axes::plot_fmt`], [`Axes::plot_with`]           |
//! | `ax.loglog`, `ax.semilogx`, `ax.semilogy`          | [`Axes::loglog`], [`Axes::semilogx`], [`Axes::semilogy`] |
//! | `ax.step(x, y, where="mid")`                       | [`Axes::step`]`(x, y, StepWhere::Mid, props)` ([`StepWhere`]) |
//! | `ax.stairs(values, edges)`                         | [`Axes::stairs`] with [`StairsOptions`]           |
//! | `ax.scatter(x, y, s=, c=, marker=, cmap=, vmin=)`  | [`Axes::scatter_with`] with [`ScatterOptions`], [`MappedColors`] |
//! | `ax.errorbar(x, y, yerr=, xerr=, fmt=, capsize=)`  | [`Axes::errorbar`] with [`ErrorbarOptions`], [`ErrorSize`], [`ErrorbarFormat`] |
//! | `ax.bar(x, h, width=, bottom=, align=, color=)`    | [`Axes::bar`] with [`BarOptions`]                 |
//! | `ax.barh(y, w, height=, left=)`                    | [`Axes::barh`] (same [`BarOptions`])              |
//! | `ax.bar_label(bars, fmt=, label_type=, padding=)`  | [`Axes::bar_label`] with [`BarLabelOptions`]; `fmt` is a [`LabelFormat`] |
//! | `ax.grouped_bar(heights, ...)`                     | [`Axes::grouped_bar`] with [`GroupedBarOptions`]  |
//! | `ax.broken_barh(xranges, (y, h))`                  | [`Axes::broken_barh`]                             |
//! | `ax.fill(x, y, color=)`                            | [`Axes::fill`] with [`PatchProps`]                |
//! | `ax.fill_between(x, y1, y2, where=, interpolate=, step=)` | [`Axes::fill_between`] with [`FillBetweenOptions`] |
//! | `ax.fill_betweenx(y, x1, x2)`                      | [`Axes::fill_betweenx`]                           |
//! | `ax.stackplot(x, ys, baseline=)`                   | [`Axes::stackplot`] with [`StackplotOptions`]     |
//! | `ax.stem(x, y, linefmt=, markerfmt=, basefmt=)`    | [`Axes::stem`] with [`StemOptions`] (formats are [`FormatString`]s) |
//! | `ax.eventplot(positions, orientation=, ...)`       | [`Axes::eventplot`] with [`EventplotOptions`]     |
//! | `ax.hlines(y, xmin, xmax)`, `ax.vlines(...)`       | [`Axes::hlines`], [`Axes::vlines`]                |
//! | `ax.axhline(y)`, `ax.axhline(y, xmin, xmax)`       | [`Axes::axhline`], [`Axes::axhline_between`]      |
//! | `ax.axvspan(a, b)`, `ax.axvspan(a, b, ymin, ymax)` | [`Axes::axvspan`], [`Axes::axvspan_between`]      |
//! | `ax.axline(p1, p2)`, `ax.axline(p1, slope=s)`      | [`Axes::axline`] with [`AxLineThrough`]           |
//! | `line.set_xy1(p)`, `set_xy2(p)`, `set_slope(s)`    | [`Line2D::set_axline_xy1`], [`Line2D::set_axline_xy2`], [`Line2D::set_axline_slope`] |
//! | `ax.hist(x, bins=, range=, density=, histtype=)`   | [`Axes::hist`] with [`HistPlotOptions`] and [`HistOptions`] |
//! | `ax.hist2d(x, y, bins=)`                           | [`Axes::hist2d`] with [`Hist2dOptions`]           |
//! | `ax.hexbin(x, y, C=, gridsize=, mincnt=)`          | [`Axes::hexbin`] with [`HexbinOptions`]           |
//! | `ax.boxplot(data, whis=, tick_labels=, showfliers=)` | [`Axes::boxplot`] with [`BoxplotOptions`] and [`BxpOptions`] |
//! | `ax.bxp(stats)`                                    | [`Axes::bxp`]                                     |
//! | `ax.violinplot(data, showmeans=)`, `ax.violin(stats)` | [`Axes::violinplot`], [`Axes::violin`]         |
//! | `ax.pie(x, labels=, autopct=, explode=)`           | [`Axes::pie`] with [`PieOptions`]; `autopct` is a [`LabelFormat`] |
//! | `ax.pie_label(...)`                                | [`Axes::pie_label`]                               |
//! | `ax.ecdf(x)`                                       | [`Axes::ecdf`]                                    |
//! | `ax.psd`, `ax.csd`, `ax.cohere`, `ax.specgram`     | [`Axes::psd`], [`Axes::csd`], [`Axes::cohere`], [`Axes::specgram`] |
//! | `ax.magnitude_spectrum`, `ax.xcorr`, `ax.acorr`    | [`Axes::magnitude_spectrum`], [`Axes::xcorr`], [`Axes::acorr`] |
//! | `ax.table(cellText=, colLabels=)`                  | [`Axes::table`] with [`TableOptions`]             |
//! | `data=` with column names                          | not available: pass the columns                   |
//!
//! Images, contours, meshes and vector fields are in [`super::images`];
//! text and annotations in [`super::text`].
//!
//! # Data
//!
//! | matplotlib input                       | plt-rs input                                       |
//! |----------------------------------------|----------------------------------------------------|
//! | a list or 1-D array                    | `[f64; N]`, `&[f64]`, `Vec<f64>`, `&Vec<f64>`      |
//! | `np.linspace`, `np.arange`, `np.logspace`, `np.geomspace` | [`linspace`], [`arange`], [`logspace`], [`geomspace`] |
//! | a 2-D array                            | [`Grid`]`<f64>` ([`Grid::from_fn`], [`Grid::from_rows`]) |
//! | `np.meshgrid(x, y)`                    | [`meshgrid`]                                       |
//! | strings (categories)                   | `["a", "b"]`, `Vec<String>`: [`AxisValues::Categories`] |
//! | `datetime`, `np.datetime64`            | `&[DateTime]`, `Vec<Datetime64>`: [`AxisValues::Dates`] |
//! | `np.nan` gaps in a line                | `f64::NAN`                                         |
//! | masked arrays                          | NaN for lines and markers; [`ScalarField::with_mask`] for images; [`QuadField`] masks for contours; [`FillBetweenOptions::mask`]; `Option<f64>` values for norms and collections |
//!
//! The x and y of `plot`, `scatter`, `bar`, `barh` and `step` accept
//! numbers, categories and dates ([`AxisValues`]); most other methods take
//! numbers (`impl AsRef<[f64]>`), converted first with
//! [`Axis::convert_units`] when they are categories or dates.
//!
//! # Lines
//!
//! ```python
//! ax.plot([0, 1, 4, 9])                       # x = 0, 1, 2, 3
//! ax.plot(x, y1, "r--")
//! ax.plot(x, y2, color="0.5", linewidth=1, marker="s", markevery=2)
//! ax.plot(x, y1, x, y2)                       # two lines
//! ```
//!
//! ```
//! use plt::lines::MarkEvery;
//! use plt::markers::MarkerSymbol;
//! use plt::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! let x = [0.0, 1.0, 2.0, 3.0];
//! let y1 = [0.0, 1.0, 4.0, 9.0];
//! let y2 = [9.0, 4.0, 1.0, 0.0];
//!
//! let implicit = ax.plot_y(y1);
//! assert_eq!(implicit.xdata(), x);
//! ax.plot_fmt(x, y1, "r--")?;
//! let props = LineProps::default()
//!     .color("0.5")
//!     .linewidth(1.0)
//!     .marker(MarkerSymbol::Square)
//!     .markevery(MarkEvery::Every { start: 0, step: 2 });
//! ax.plot_with(x, y2, props)?;
//! for y in [y1, y2] {
//!     ax.plot(x, y)?;
//! }
//! assert_eq!(ax.lines().len(), 5);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Scatter plots
//!
//! A numeric `c` with its `cmap`, `norm`, `vmin` and `vmax` is a
//! [`MappedColors`]; one colour is [`ScatterColors::Single`], a colour per
//! point [`ScatterColors::PerPoint`].
//!
//! ```python
//! points = ax.scatter(x, y, s=[20, 40, 60, 80], c=[0.1, 0.4, 0.7, 1.0],
//!                     cmap="plasma", vmin=0, vmax=1, edgecolors="face")
//! ```
//!
//! ```
//! use plt::axes::{MappedColors, ScatterOptions};
//! use plt::collections::EdgeColors;
//! use plt::colors::colormaps;
//! use plt::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! let options = ScatterOptions {
//!     sizes: Some(vec![20.0, 40.0, 60.0, 80.0]),
//!     colors: MappedColors::new(vec![0.1, 0.4, 0.7, 1.0])
//!         .cmap(colormaps::get("plasma")?)
//!         .limits(Some(0.0), Some(1.0))
//!         .into(),
//!     edgecolors: Some(EdgeColors::Face),
//!     ..ScatterOptions::default()
//! };
//! let points = ax.scatter_with([0.0, 1.0, 2.0, 3.0], [3.0, 1.0, 2.0, 0.0], options)?;
//! assert_eq!(points.offsets().len(), 4);
//! assert_eq!(points.clim(), (Some(0.0), Some(1.0)));
//! assert_eq!(points.sizes(), Some(vec![20.0, 40.0, 60.0, 80.0]));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Bars
//!
//! Per-bar values are vectors holding one value for all bars or one per
//! bar, as matplotlib broadcasts its scalars and sequences.
//!
//! ```python
//! bars = ax.bar(["apples", "pears", "plums"], [3, 5, 2], width=0.5,
//!               color="tab:orange", label="fruit")
//! ax.bar_label(bars, fmt="%.1f")
//! ax.barh([0, 1], [2, 4], height=0.4, left=1)
//! ```
//!
//! ```
//! use plt::axes::{BarLabelOptions, BarOptions};
//! use plt::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! let options = BarOptions {
//!     thickness: vec![0.5],
//!     color: vec!["tab:orange".into()],
//!     label: Some("fruit".into()),
//!     ..BarOptions::default()
//! };
//! let bars = ax.bar(["apples", "pears", "plums"], [3.0, 5.0, 2.0], options)?;
//! assert_eq!(bars.datavalues, [3.0, 5.0, 2.0]);
//! assert_eq!(bars.patches[1].shape().width, 0.5);
//!
//! let format = BarLabelOptions { fmt: "%.1f".into(), ..BarLabelOptions::default() };
//! let labels = ax.bar_label(&bars, format)?;
//! assert_eq!(labels[1].text(), "5.0");
//!
//! let other = Figure::new().subplot();
//! let options = BarOptions { thickness: vec![0.4], base: vec![1.0], ..BarOptions::default() };
//! let horizontal = other.barh([0.0, 1.0], [2.0, 4.0], options)?;
//! assert_eq!(horizontal.patches[0].shape().xy.x, 1.0);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Histograms
//!
//! [`Axes::hist`] takes a slice of datasets, so a single dataset is
//! `&[data]`. The binning options are shared with [`crate::stats::hist`].
//!
//! ```python
//! counts, edges, patches = ax.hist(data, bins=5, range=(0, 10),
//!                                  density=True, histtype="step")
//! ```
//!
//! ```
//! use plt::axes::{HistArtists, HistPlotOptions};
//! use plt::prelude::*;
//! use plt::stats::{Bins, HistOptions, HistType};
//!
//! let ax = Figure::new().subplot();
//! let data = [1.0, 2.0, 2.5, 3.0, 7.0, 9.5];
//! let options = HistPlotOptions {
//!     layout: HistOptions {
//!         bins: Some(Bins::Count(5)),
//!         range: Some((0.0, 10.0)),
//!         density: true,
//!         histtype: HistType::Step,
//!         ..HistOptions::default()
//!     },
//!     ..HistPlotOptions::default()
//! };
//! let hist = ax.hist(&[data], options)?;
//! assert_eq!(hist.edges, [0.0, 2.0, 4.0, 6.0, 8.0, 10.0]);
//! let area: f64 = hist.tops[0].iter().map(|density| density * 2.0).sum();
//! assert!((area - 1.0).abs() < 1e-12);
//! assert!(matches!(hist.artists, HistArtists::Steps(_)));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Filled areas
//!
//! ```python
//! ax.fill_between(x, y1, y2, where=y1 > y2, interpolate=True,
//!                 color="tab:green", alpha=0.3)
//! ```
//!
//! ```
//! use plt::axes::FillBetweenOptions;
//! use plt::collections::CollectionProps;
//! use plt::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! let x = linspace(0.0, 6.0, 50);
//! let y1: Vec<f64> = x.iter().map(|v| v.sin()).collect();
//! let y2: Vec<f64> = x.iter().map(|v| v.cos()).collect();
//! let mask = y1.iter().zip(&y2).map(|(a, b)| a > b).collect();
//! let options = FillBetweenOptions {
//!     mask: Some(mask),
//!     interpolate: true,
//!     props: CollectionProps::default().color("tab:green").alpha(0.3),
//!     ..FillBetweenOptions::default()
//! };
//! let band = ax.fill_between(&x, &y1, &y2, options)?;
//! assert_eq!(band.paths().len(), 1, "one region where sin x > cos x");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Statistics plots
//!
//! ```python
//! parts = ax.boxplot([a, b], tick_labels=["a", "b"], whis=1.5, showfliers=False)
//! wedges, texts, autotexts = ax2.pie([1, 2, 1], labels=["x", "y", "z"], autopct="%1.0f%%")
//! ```
//!
//! ```
//! use plt::axes::{BoxplotOptions, BxpOptions, PieOptions};
//! use plt::prelude::*;
//! use plt::stats::Whis;
//!
//! let ax = Figure::new().subplot();
//! let a = [1.0, 2.0, 3.0, 4.0, 30.0];
//! let b = [2.0, 3.0, 4.0, 5.0, 6.0];
//! let options = BoxplotOptions {
//!     tick_labels: Some(vec!["a".into(), "b".into()]),
//!     whis: Some(Whis::Factor(1.5)),
//!     draw: BxpOptions { showfliers: Some(false), ..BxpOptions::default() },
//!     ..BoxplotOptions::default()
//! };
//! let parts = ax.boxplot(&[a, b], options)?;
//! assert_eq!(parts.medians.len(), 2);
//! assert!(parts.fliers.iter().all(|line| line.xdata().is_empty()));
//!
//! let ax2 = Figure::new().subplot();
//! let options = PieOptions {
//!     labels: vec!["x".into(), "y".into(), "z".into()],
//!     autopct: Some("%1.0f%%".into()),
//!     ..PieOptions::default()
//! };
//! let pie = ax2.pie(&[1.0, 2.0, 1.0], options)?;
//! assert_eq!(pie.pct_labels[1].text(), "50%");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Categories, dates and gaps
//!
//! ```python
//! from datetime import datetime
//! ax1.plot(["mon", "tue", "wed"], [3, 1, 2])
//! ax2.plot([datetime(2024, 1, 1), datetime(2024, 3, 1)], [1, 3])
//! ax3.plot([0, 1, 2, 3], [0, np.nan, 2, 3])   # a gap at x = 1
//! ```
//!
//! ```
//! use plt::dates::DateTime;
//! use plt::prelude::*;
//! use plt::units::UnitConverter;
//!
//! let fig = Figure::new();
//! let axs = fig.subplots(1, 3)?;
//! let days = axs[(0, 0)].plot(["mon", "tue", "wed"], [3.0, 1.0, 2.0])?;
//! assert_eq!(days.xdata(), [0.0, 1.0, 2.0]);
//! assert_eq!(axs[(0, 0)].xaxis().converter(), Some(UnitConverter::Categories));
//!
//! let dates = [DateTime::new(2024, 1, 1, 0, 0, 0, 0)?, DateTime::new(2024, 3, 1, 0, 0, 0, 0)?];
//! let line = axs[(0, 1)].plot(&dates[..], [1.0, 3.0])?;
//! assert_eq!(line.xdata(), [19723.0, 19783.0], "days since 1970-01-01");
//! assert_eq!(axs[(0, 1)].xaxis().converter(), Some(UnitConverter::Dates));
//!
//! let gap = axs[(0, 2)].plot([0.0, 1.0, 2.0, 3.0], [0.0, f64::NAN, 2.0, 3.0])?;
//! assert!(gap.ydata()[1].is_nan());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`Axes`]: crate::axes::Axes
//! [`Axis::convert_units`]: crate::axes::Axis::convert_units
//! [`Axes::plot`]: crate::axes::Axes::plot
//! [`Axes::plot_y`]: crate::axes::Axes::plot_y
//! [`Axes::plot_fmt`]: crate::axes::Axes::plot_fmt
//! [`Axes::plot_with`]: crate::axes::Axes::plot_with
//! [`Axes::loglog`]: crate::axes::Axes::loglog
//! [`Axes::semilogx`]: crate::axes::Axes::semilogx
//! [`Axes::semilogy`]: crate::axes::Axes::semilogy
//! [`Axes::step`]: crate::axes::Axes::step
//! [`Axes::stairs`]: crate::axes::Axes::stairs
//! [`Axes::scatter`]: crate::axes::Axes::scatter
//! [`Axes::scatter_with`]: crate::axes::Axes::scatter_with
//! [`Axes::errorbar`]: crate::axes::Axes::errorbar
//! [`Axes::bar`]: crate::axes::Axes::bar
//! [`Axes::barh`]: crate::axes::Axes::barh
//! [`Axes::bar_label`]: crate::axes::Axes::bar_label
//! [`Axes::grouped_bar`]: crate::axes::Axes::grouped_bar
//! [`Axes::broken_barh`]: crate::axes::Axes::broken_barh
//! [`Axes::fill`]: crate::axes::Axes::fill
//! [`Axes::fill_between`]: crate::axes::Axes::fill_between
//! [`Axes::fill_betweenx`]: crate::axes::Axes::fill_betweenx
//! [`Axes::stackplot`]: crate::axes::Axes::stackplot
//! [`Axes::stem`]: crate::axes::Axes::stem
//! [`ErrorbarFormat`]: crate::axes::ErrorbarFormat
//! [`FormatString`]: crate::lines::FormatString
//! [`Axes::eventplot`]: crate::axes::Axes::eventplot
//! [`Axes::hlines`]: crate::axes::Axes::hlines
//! [`Axes::vlines`]: crate::axes::Axes::vlines
//! [`Axes::axhline`]: crate::axes::Axes::axhline
//! [`Axes::axhline_between`]: crate::axes::Axes::axhline_between
//! [`Axes::axvspan`]: crate::axes::Axes::axvspan
//! [`Axes::axvspan_between`]: crate::axes::Axes::axvspan_between
//! [`Axes::axline`]: crate::axes::Axes::axline
//! [`Axes::hist`]: crate::axes::Axes::hist
//! [`Axes::hist2d`]: crate::axes::Axes::hist2d
//! [`Axes::hexbin`]: crate::axes::Axes::hexbin
//! [`Axes::boxplot`]: crate::axes::Axes::boxplot
//! [`Axes::bxp`]: crate::axes::Axes::bxp
//! [`Axes::violinplot`]: crate::axes::Axes::violinplot
//! [`Axes::violin`]: crate::axes::Axes::violin
//! [`Axes::pie`]: crate::axes::Axes::pie
//! [`Axes::pie_label`]: crate::axes::Axes::pie_label
//! [`Axes::ecdf`]: crate::axes::Axes::ecdf
//! [`Axes::psd`]: crate::axes::Axes::psd
//! [`Axes::csd`]: crate::axes::Axes::csd
//! [`Axes::cohere`]: crate::axes::Axes::cohere
//! [`Axes::specgram`]: crate::axes::Axes::specgram
//! [`Axes::magnitude_spectrum`]: crate::axes::Axes::magnitude_spectrum
//! [`Axes::xcorr`]: crate::axes::Axes::xcorr
//! [`Axes::acorr`]: crate::axes::Axes::acorr
//! [`Axes::table`]: crate::axes::Axes::table
//! [`LineProps`]: crate::lines::LineProps
//! [`Line2D`]: crate::lines::Line2D
//! [`StepWhere`]: crate::lines::StepWhere
//! [`AxLineThrough`]: crate::lines::AxLineThrough
//! [`Line2D::set_axline_xy1`]: crate::lines::Line2D::set_axline_xy1
//! [`Line2D::set_axline_xy2`]: crate::lines::Line2D::set_axline_xy2
//! [`Line2D::set_axline_slope`]: crate::lines::Line2D::set_axline_slope
//! [`PatchProps`]: crate::patches::PatchProps
//! [`Patch`]: crate::patches::Patch
//! [`CollectionProps`]: crate::collections::CollectionProps
//! [`Collection`]: crate::collections::Collection
//! [`ScatterOptions`]: crate::axes::ScatterOptions
//! [`ScatterColors`]: crate::axes::ScatterColors
//! [`ScatterColors::Single`]: crate::axes::ScatterColors::Single
//! [`ScatterColors::PerPoint`]: crate::axes::ScatterColors::PerPoint
//! [`MappedColors`]: crate::axes::MappedColors
//! [`ErrorbarOptions`]: crate::axes::ErrorbarOptions
//! [`ErrorbarContainer`]: crate::axes::ErrorbarContainer
//! [`ErrorSize`]: crate::axes::ErrorSize
//! [`BarOptions`]: crate::axes::BarOptions
//! [`BarOptions::thickness`]: crate::axes::BarOptions::thickness
//! [`BarOptions::base`]: crate::axes::BarOptions::base
//! [`BarContainer`]: crate::axes::BarContainer
//! [`BarLabelOptions`]: crate::axes::BarLabelOptions
//! [`LabelFormat`]: crate::ticker::LabelFormat
//! [`GroupedBarOptions`]: crate::axes::GroupedBarOptions
//! [`FillBetweenOptions`]: crate::axes::FillBetweenOptions
//! [`FillBetweenOptions::mask`]: crate::axes::FillBetweenOptions::mask
//! [`StackplotOptions`]: crate::axes::StackplotOptions
//! [`StairsOptions`]: crate::axes::StairsOptions
//! [`StemOptions`]: crate::axes::StemOptions
//! [`EventplotOptions`]: crate::axes::EventplotOptions
//! [`HistPlotOptions`]: crate::axes::HistPlotOptions
//! [`HistPlot`]: crate::axes::HistPlot
//! [`HistOptions`]: crate::stats::HistOptions
//! [`Bins`]: crate::stats::Bins
//! [`Hist2dOptions`]: crate::axes::Hist2dOptions
//! [`HexbinOptions`]: crate::axes::HexbinOptions
//! [`BoxplotOptions`]: crate::axes::BoxplotOptions
//! [`BxpOptions`]: crate::axes::BxpOptions
//! [`BoxplotArtists`]: crate::axes::BoxplotArtists
//! [`PieOptions`]: crate::axes::PieOptions
//! [`TableOptions`]: crate::table::TableOptions
//! [`linspace`]: crate::numeric::linspace
//! [`arange`]: crate::numeric::arange
//! [`logspace`]: crate::numeric::logspace
//! [`geomspace`]: crate::numeric::geomspace
//! [`meshgrid`]: crate::numeric::meshgrid
//! [`Grid`]: crate::numeric::Grid
//! [`Grid::from_fn`]: crate::numeric::Grid::from_fn
//! [`Grid::from_rows`]: crate::numeric::Grid::from_rows
//! [`AxisValues`]: crate::units::AxisValues
//! [`AxisValues::Categories`]: crate::units::AxisValues::Categories
//! [`AxisValues::Dates`]: crate::units::AxisValues::Dates
//! [`ScalarField::with_mask`]: crate::image::ScalarField::with_mask
//! [`QuadField`]: crate::contour::QuadField
