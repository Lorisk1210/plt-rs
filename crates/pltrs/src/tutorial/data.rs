//! Chapter 4: data, options and errors.
//!
//! Plotting calls take data in the container that you have, settings in an
//! option struct, and return errors in a `Result`.
//!
//! # Numbers
//!
//! Plotting methods take data as `impl Into<...>` or
//! `impl IntoIterator<...>`. Pass an array, slice, `Vec`, range or
//! iterator, by value or by reference, of `f64`, `f32` or an integer type.
//! Plotting copies the values as `f64`.
//!
//! [`numeric`](crate::numeric) makes the usual sequences: [`linspace`]
//! (`n` points from `start` to `stop`), [`arange`] (steps of `step` from
//! `start`, before `stop`) and [`logspace`] (evenly spaced powers).
//!
//! ```
//! use pltrs::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! let x = vec![1.0, 2.0, 3.0];
//! ax.plot(&x, [2.0, 4.0, 3.0])?; // a borrowed Vec and an array
//! ax.plot(x.as_slice(), vec![1.0, 1.5, 2.0])?; // a slice and an owned Vec
//!
//! let counts: [u32; 3] = [3, 8, 5];
//! ax.plot(1..4, counts)?; // integers, in a range and an array
//! let squares = ax.plot(&x, x.iter().map(|x| x * x))?; // an iterator
//! assert_eq!(squares.ydata(), [1.0, 4.0, 9.0]);
//!
//! assert_eq!(arange(0.0, 1.0, 0.25), [0.0, 0.25, 0.5, 0.75]);
//! assert_eq!(linspace(0.0, 1.0, 5), [0.0, 0.25, 0.5, 0.75, 1.0]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! A single value is `[2.5]`. An empty list needs its type, as in
//! `Vec::<f64>::new()`. A NaN makes a gap.
//!
//! # Grids
//!
//! Images, meshes and contours take 2-D data as a [`Grid`]: rows and
//! columns of one type in one `Vec`, indexed by `(row, column)`. Make a
//! grid from a function, from nested rows, or from a flat `Vec` and a
//! shape. [`meshgrid`] makes the x grid and the y grid from the values
//! along each axis.
//!
//! ```
//! use pltrs::prelude::*;
//!
//! let a = Grid::from_rows([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]])?;
//! let b = Grid::from_vec(2, 3, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0])?;
//! let c = Grid::from_fn(2, 3, |row, col| (3 * row + col + 1) as f64);
//! assert!(a == b && b == c);
//! assert_eq!((a.shape(), a[(1, 0)]), ((2, 3), 4.0));
//! assert!(Grid::from_rows([vec![1.0], vec![2.0, 3.0]]).is_err(), "ragged rows");
//!
//! let (x, y) = meshgrid(&[0.0, 1.0, 2.0], &[10.0, 20.0]);
//! assert_eq!(x.shape(), (2, 3), "one row per y, one column per x");
//! assert_eq!((x[(1, 2)], y[(1, 2)]), (2.0, 20.0));
//! let sum = Grid::from_fn(2, 3, |r, c| x[(r, c)] + y[(r, c)]);
//! assert_eq!(sum.row(0), [10.0, 11.0, 12.0]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Categories and dates
//!
//! The x and y of `plot`, `scatter` and `bar` can also be strings
//! (*categories*) or dates. The axis places categories at 0, 1, 2, ... in
//! the order of their first appearance, as tick labels.
//! [`DateTime`] values make a date axis (chapter [`scales`](super::scales)),
//! where a date is the number of days since 1970. A [`DateRange`] is a
//! series of date-times a [`TimeDelta`] apart (`TimeDelta::from_hours(6)`,
//! `TimeDelta::from_days(1)?`).
//!
//! ```
//! use pltrs::dates::{DateRange, DateTime, TimeDelta};
//! use pltrs::prelude::*;
//!
//! let fig = Figure::new();
//! let axs = fig.subplots(1, 2)?;
//!
//! let line = axs[(0, 0)].plot(["mon", "tue", "wed"], [3.0, 1.0, 2.0])?;
//! assert_eq!(line.xdata(), [0.0, 1.0, 2.0]);
//!
//! let start = DateTime::new(2024, 3, 1, 0, 0, 0, 0)?;
//! let end = DateTime::new(2024, 3, 31, 0, 0, 0, 0)?;
//! let days = DateRange::new(start, end, TimeDelta::from_days(1)?)?;
//! let temperatures = (0..30).map(|day| 8.0 + 0.2 * f64::from(day));
//! let series = axs[(0, 1)].plot(days, temperatures)?;
//! assert_eq!(series.xdata()[0], 19783.0, "2024-03-01 is day 19783");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! A [`ZonedDateTime`] also gives its time zone to an axis without a unit.
//!
//! The type of these parameters is [`AxisValues`]: numbers, categories,
//! dates, or your own values (chapter [`extending`](super::extending)). Extents such as bar heights or error
//! sizes are [`AxisDeltas`]: one value for all items, or a list. Data that
//! can only be numbers, such as pie values, takes [`Number`]s. 3-D axes
//! take x, y and z in the same way:
//!
//! ```
//! use pltrs::dates::DateTime;
//! use pltrs::prelude::*;
//!
//! let ax = Figure::new().subplot_3d();
//! let day = |d| DateTime::new(2024, 3, d, 0, 0, 0, 0);
//! let line = ax.plot(["north", "south", "east"], [1.0, 2.0, 4.0], [day(1)?, day(2)?, day(3)?])?;
//! let [x, _, z] = ax.line_data(&line)?;
//! assert_eq!(x, [0.0, 1.0, 2.0]);
//! assert_eq!(z[0], 19783.0, "2024-03-01 is day 19783");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Options
//!
//! A plotting method with settings takes one *option struct* by value. Its
//! public fields have the names of the matplotlib keywords. Most fields
//! are unset (`None` or empty) by default, so the runtime settings decide
//! (chapter [`styling`](super::styling)). Use struct update syntax:
//!
//! ```
//! use pltrs::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! let options = BarOptions {
//!     thickness: 0.5.into(),
//!     color: vec!["tab:orange".into()],
//!     label: Some("fruit".into()),
//!     ..BarOptions::default()
//! };
//! let bars = ax.bar(["apples", "pears"], [3.0, 5.0], options)?;
//! assert_eq!(bars.patches[0].shape().width, 0.5);
//!
//! // `Default::default()` alone takes every default.
//! ax.bar(["plums"], [2.0], Default::default())?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`LineProps`], [`PatchProps`],
//! [`CollectionProps`](crate::collections::CollectionProps),
//! [`TextProps`](crate::text::TextProps) and
//! [`LegendOptions`](crate::legend::LegendOptions) also have builder
//! methods, as in `LineProps::default().color("k").linewidth(2.0)`. The
//! builders take `impl Into<...>`, so literals work. A struct field needs
//! `.into()`.
//!
//! The call consumes its options. To use them twice, clone them:
//!
//! ```
//! use pltrs::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! let thin = LineProps::default().linewidth(0.5).color("0.4");
//! ax.plot_with([0.0, 1.0], [0.0, 1.0], thin.clone())?;
//! ax.plot_with([0.0, 1.0], [1.0, 0.0], thin)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! If matplotlib takes a string from a fixed set, pltrs takes an enum
//! ([`LineStyle::Dashed`] for `"--"`). [`str::parse`] reads the matplotlib
//! strings into these enums.
//!
//! ```
//! use pltrs::prelude::*;
//!
//! assert_eq!("--".parse::<LineStyle>()?, LineStyle::Dashed);
//! assert!("~~".parse::<LineStyle>().is_err());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Errors
//!
//! A call that can fail returns a `Result`. Its error is an enum of the
//! module: [`PlotError`] for plotting, [`SaveError`] for saving,
//! [`ImageError`](crate::image::ImageError),
//! [`LegendError`](crate::legend::LegendError) and so on. pltrs returns an
//! error where matplotlib raises an exception, for example for different
//! lengths or an unknown colormap. Bad input never causes a panic. Calls
//! that cannot fail, such as [`Axes::text`], return the handle directly.
//!
//! Each error type implements [`std::error::Error`]. Propagate errors with
//! `?`, and match on a variant only where you can handle it. A library can
//! define its own error enum with a `From` implementation for each pltrs
//! error.
//!
//! ```
//! use std::error::Error;
//! use std::path::Path;
//!
//! use pltrs::axes::PlotError;
//! use pltrs::prelude::*;
//!
//! /// Saves a bar chart of `values` to `path`.
//! fn bar_chart(names: &[&str], values: &[f64], path: &Path) -> Result<(), Box<dyn Error>> {
//!     let fig = Figure::new();
//!     fig.subplot().bar(names, values, BarOptions::default())?; // PlotError
//!     fig.savefig(path)?; // SaveError
//!     Ok(())
//! }
//!
//! // Three names and two values: `bar` fails before `savefig` writes a file.
//! let path = Path::new("never-written.png");
//! let error = bar_chart(&["a", "b", "c"], &[1.0, 2.0], path).unwrap_err();
//! assert!(error.is::<PlotError>());
//!
//! let ax = Figure::new().subplot();
//! match ax.plot([1.0, 2.0, 3.0], [1.0, 2.0]) {
//!     Err(PlotError::LengthMismatch { x, y }) => assert_eq!((x, y), (3, 2)),
//!     other => panic!("expected a length mismatch, got {other:?}"),
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! The messages are similar to the matplotlib messages, so a search for a
//! message usually finds help.
//!
//! Next: [styling](super::styling).
//!
//! [`linspace`]: crate::numeric::linspace
//! [`arange`]: crate::numeric::arange
//! [`logspace`]: crate::numeric::logspace
//! [`meshgrid`]: crate::numeric::meshgrid
//! [`Grid`]: crate::numeric::Grid
//! [`DateTime`]: crate::dates::DateTime
//! [`DateRange`]: crate::dates::DateRange
//! [`TimeDelta`]: crate::dates::TimeDelta
//! [`ZonedDateTime`]: crate::dates::ZonedDateTime
//! [`AxisValues`]: crate::units::AxisValues
//! [`AxisDeltas`]: crate::units::AxisDeltas
//! [`Number`]: crate::numeric::Number
//! [`LineProps`]: crate::lines::LineProps
//! [`PatchProps`]: crate::patches::PatchProps
//! [`LineStyle::Dashed`]: crate::lines::LineStyle::Dashed
//! [`PlotError`]: crate::axes::PlotError
//! [`SaveError`]: crate::figure::SaveError
//! [`Axes::text`]: crate::axes::Axes::text
