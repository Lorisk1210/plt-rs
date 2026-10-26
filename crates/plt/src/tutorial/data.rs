//! Chapter 4: data, options and errors.
//!
//! Three conventions run through every plotting call: data goes in as
//! whatever container you have, the rest of what the call needs goes in
//! an option struct, and anything that can be wrong comes back as a
//! `Result`.
//!
//! # Numbers
//!
//! Plotting methods take their data as `impl Into<...>` or
//! `impl IntoIterator<...>` parameters, so you pass the data you have, by
//! value or by reference: an array, a slice, a `Vec`, a range, or an
//! iterator computing the values as they are plotted. Any number type
//! goes: `f64`, `f32` and the integers (`i32`, `u8`, `usize`, ...).
//! Passing a reference keeps the data for later; plotting copies what it
//! needs, as `f64`.
//!
//! [`numeric`](crate::numeric) makes the usual sequences: [`linspace`]
//! (`n` points from `start` to `stop`), [`arange`] (from `start` in steps
//! of `step`, before `stop`), [`logspace`] (evenly spaced powers).
//!
//! ```
//! use plt::prelude::*;
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
//! One value is a list of one, `[2.5]`. An empty list needs its type
//! spelled out, as in `Vec::<f64>::new()`, since nothing in it tells Rust
//! what it holds.
//!
//! A NaN leaves a gap: lines break around it and markers skip it. Use it
//! for missing measurements.
//!
//! # Grids
//!
//! Images, meshes and contours take 2-D data as a [`Grid`]: rows and
//! columns of one type stored in a single `Vec`, row by row, and indexed
//! by `(row, column)`. Build one from a function of the row and column,
//! from nested rows, or from a flat `Vec` and its shape. [`meshgrid`]
//! turns the values along x and along y into the two grids of x and y at
//! every point, from which a function of both is one `from_fn` away.
//!
//! ```
//! use plt::prelude::*;
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
//! The x and y of `plot`, `scatter` and `bar` need not be numbers. Strings
//! are *categories*: the axis places them at 0, 1, 2, ... in the order it
//! first sees them, and labels its ticks with them. [`DateTime`] values
//! make a date axis, whose ticks read as dates (chapter
//! [`scales`](super::scales)). Both are converted to numbers when
//! plotted: categories to their index, dates to days since 1970.
//!
//! ```
//! use plt::dates::{DateTime, TimeDelta};
//! use plt::prelude::*;
//!
//! let fig = Figure::new();
//! let axs = fig.subplots(1, 2)?;
//!
//! let line = axs[(0, 0)].plot(["mon", "tue", "wed"], [3.0, 1.0, 2.0])?;
//! assert_eq!(line.xdata(), [0.0, 1.0, 2.0]);
//!
//! let start = DateTime::new(2024, 3, 1, 0, 0, 0, 0)?;
//! let days = (0..30)
//!     .map(|day| start.checked_add(TimeDelta::new(day, 0, 0)?))
//!     .collect::<Result<Vec<_>, _>>()?;
//! let temperatures = (0..30).map(|day| 8.0 + 0.2 * f64::from(day));
//! let series = axs[(0, 1)].plot(days, temperatures)?;
//! assert_eq!(series.xdata()[0], 19783.0, "2024-03-01 is day 19783");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Behind these parameters is one type, [`AxisValues`]: numbers,
//! category names, dates, or values of your own types converted by a
//! unit converter (chapter [`extending`](super::extending)). Every
//! collection and iterator shown above converts into it, which is why the
//! same method takes all of them. Extents measured from such values, such
//! as bar heights or error sizes, are [`AxisDeltas`]: numbers, durations
//! or user values, one for all the items or a list (an array, a slice or
//! a `Vec`). You rarely name either type; it shows up in signatures.
//! Data that can only be numbers, such as the values of a pie chart,
//! takes any collection or iterator of [`Number`]s.
//!
//! # Options
//!
//! Python has keyword arguments; Rust has structs. A plotting method that
//! takes settings takes one *option struct*, by value. Its public fields
//! are the settings, named after matplotlib's keywords. Its `Default`
//! gives matplotlib's defaults, mostly by leaving fields unset (`None` or
//! empty) so that the runtime settings decide (chapter
//! [`styling`](super::styling)). Fill in the fields you want with struct
//! update syntax:
//!
//! ```
//! use plt::prelude::*;
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
//! The property sets of artists ([`LineProps`], [`PatchProps`],
//! [`CollectionProps`](crate::collections::CollectionProps),
//! [`TextProps`](crate::text::TextProps)) and
//! [`LegendOptions`](crate::legend::LegendOptions) add a builder method
//! per field, as in `LineProps::default().color("k").linewidth(2.0)`.
//! The builders take anything convertible (`impl Into<Color>` for a
//! colour), so plain literals work; struct fields need the `.into()`
//! written out, as in the bar example above.
//!
//! The call consumes its options. To use the same ones twice, clone
//! them, or write a function returning them:
//!
//! ```
//! use plt::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! let thin = LineProps::default().linewidth(0.5).color("0.4");
//! ax.plot_with([0.0, 1.0], [0.0, 1.0], thin.clone())?;
//! ax.plot_with([0.0, 1.0], [1.0, 0.0], thin)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Where matplotlib takes a string from a fixed set, plt-rs takes an enum
//! ([`LineStyle::Dashed`] for `"--"`). Those enums parse matplotlib's
//! strings through [`str::parse`], which suits values read from a file
//! or a command line and reports a bad one at once.
//!
//! ```
//! use plt::prelude::*;
//!
//! assert_eq!("--".parse::<LineStyle>()?, LineStyle::Dashed);
//! assert!("~~".parse::<LineStyle>().is_err());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Errors
//!
//! A call that can fail returns a `Result` whose error is an enum of its
//! module, one variant per problem: [`PlotError`] from plotting methods,
//! [`SaveError`] from saving, [`ImageError`](crate::image::ImageError)
//! from images, [`LegendError`](crate::legend::LegendError) from
//! legends, and so on. They fail where matplotlib raises an exception:
//! lengths that differ, NaN limits, unknown colormaps, files that cannot
//! be written. Calls that cannot fail, such as [`Axes::text`], return
//! their handle directly. Bad input gives an error, never a panic.
//!
//! Every error type implements [`std::error::Error`], so the habit is to
//! propagate with `?` from a function returning
//! `Result<_, Box<dyn Error>>`, and to match on a variant only where you
//! can do something about it. A library of your own may prefer an error
//! enum with a `From` implementation for each plt-rs error it meets.
//!
//! ```
//! use std::error::Error;
//! use std::path::Path;
//!
//! use plt::axes::PlotError;
//! use plt::prelude::*;
//!
//! /// Saves a bar chart of `values` to `path`.
//! fn bar_chart(names: &[&str], values: &[f64], path: &Path) -> Result<(), Box<dyn Error>> {
//!     let fig = Figure::new();
//!     fig.subplot().bar(names, values, BarOptions::default())?; // PlotError
//!     fig.savefig(path)?; // SaveError
//!     Ok(())
//! }
//!
//! // Three names and two values: refused before anything is written.
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
//! The messages read like matplotlib's, so searching for one usually
//! finds the explanation of the matplotlib error it mirrors.
//!
//! Next: [styling](super::styling).
//!
//! [`linspace`]: crate::numeric::linspace
//! [`arange`]: crate::numeric::arange
//! [`logspace`]: crate::numeric::logspace
//! [`meshgrid`]: crate::numeric::meshgrid
//! [`Grid`]: crate::numeric::Grid
//! [`DateTime`]: crate::dates::DateTime
//! [`AxisValues`]: crate::units::AxisValues
//! [`AxisDeltas`]: crate::units::AxisDeltas
//! [`Number`]: crate::numeric::Number
//! [`LineProps`]: crate::lines::LineProps
//! [`PatchProps`]: crate::patches::PatchProps
//! [`LineStyle::Dashed`]: crate::lines::LineStyle::Dashed
//! [`PlotError`]: crate::axes::PlotError
//! [`SaveError`]: crate::figure::SaveError
//! [`Axes::text`]: crate::axes::Axes::text
