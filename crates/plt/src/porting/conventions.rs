//! Conventions: the rules that turn a matplotlib call into a plt-rs call.
//!
//! A handful of rules cover most of the translation. Once you know them
//! you can usually guess the plt-rs name of a matplotlib function, and
//! rustdoc's search does the rest.
//!
//! * **Getters drop `get_`.** `ax.get_xlim()` is `ax.xlim()`,
//!   `line.get_color()` is `line.color()`. Setters keep `set_` and return
//!   `&Self` so calls chain; setters that create an artist, such as
//!   `set_title`, return its handle instead.
//! * **Keyword arguments become option structs.** Each plotting method
//!   takes a struct of its keywords: either a property set with builder
//!   methods ([`LineProps`], [`PatchProps`], [`CollectionProps`],
//!   [`TextProps`], [`LegendOptions`]) or a plain struct with public fields
//!   filled with struct update syntax (`..Default::default()`). Unset
//!   options fall back to rcParams, as in matplotlib.
//! * **Strings become enums or parsed types.** `linestyle="--"` is
//!   [`LineStyle::Dashed`]; matplotlib's spelling still works through
//!   [`str::parse`], which validates it. Colours are the exception: a
//!   [`Color`] converts from any string and is validated when resolved.
//! * **Errors are `Result`s.** Invalid data or options return an error
//!   enum of the module ([`PlotError`], [`ImageError`], [`LegendError`],
//!   ...). All of them implement [`std::error::Error`], so `?` into
//!   `Box<dyn Error>` works everywhere.
//! * **Handles, not objects.** `Figure`, `Axes`, `Line2D` and every other
//!   artist type are cheap handles into a figure. Cloning a handle gives a
//!   second reference to the same artist, never a copy.
//! * **The per-axis API takes an axis argument.** Where matplotlib has
//!   `get_xmargin` and `get_ymargin`, plt-rs has `margin(AxisName::X)`.
//!   The most common pairs (`set_xlim`, `set_xlabel`, `set_xscale`,
//!   `twinx`, ...) keep their x and y spellings.
//!
//! # Equivalences
//!
//! | matplotlib                              | plt-rs                                           |
//! |-----------------------------------------|--------------------------------------------------|
//! | `ax.get_xlim()`                         | [`Axes::xlim`]                                   |
//! | `ax.get_title()`                        | [`Axes::shown_title`] (where it is, and its text) |
//! | `ax.get_title(loc="left")`              | [`Axes::title`]`(TitleLocation::Left)`           |
//! | `line.get_linewidth()`                  | [`Line2D::linewidth`]                            |
//! | `ax.get_legend()`                       | [`Axes::get_legend`] (`legend()` makes one)      |
//! | `ax.set(xlim=..., xlabel=...)`          | chained setters: `ax.set_xlim(..).set_xlabel(..)` |
//! | `line.set(color=..., lw=...)`, `plt.setp` | [`Line2D::update_props`]                       |
//! | `plt.getp(artist, "color")`             | the getter: [`Line2D::color`]                    |
//! | `ax.plot(x, y, color="r", lw=2)`        | [`Axes::plot_with`] with [`LineProps`]           |
//! | `ax.plot(x, y, "o--")`                  | [`LineProps::fmt`]`("o--")?`                     |
//! | `linestyle="--"`                        | [`LineStyle::Dashed`], or `"--".parse::<LineStyle>()?` |
//! | `marker="o"`                            | [`MarkerSymbol::Circle`], or `"o".parse::<Marker>()?` |
//! | `arrowprops=dict(arrowstyle="->")`      | `"->".parse::<FancyArrowProps>()?` ([`FancyArrowProps`]) |
//! | `color="tab:red"`                       | `"tab:red"` (any `Into<`[`Color`]`>`); [`Color::parse`] validates now |
//! | `ValueError` from a plotting call       | `Err(`[`PlotError`]`::...)`                      |
//! | `ax.get_xmargin()`, `ax.set_ymargin(m)` | [`Axes::margin`], [`Axes::set_margin`]           |
//! | `ax.get_xbound()`, `ax.set_ybound(lo, hi)` | [`Axes::bound`], [`Axes::set_bound`]          |
//! | `ax.get_autoscalex_on()`                | [`Axes::autoscale_on`]`(Some(AxisName::X))`      |
//! | `ax.set_autoscaley_on(False)`           | [`Axes::set_axis_autoscale_on`]`(AxisName::Y, false)` |
//! | `ax.invert_yaxis()`, `ax.yaxis_inverted()` | [`Axis::set_inverted`], [`Axis::inverted`] on [`Axes::yaxis`] |
//! | `ax.format_xdata(v)`                    | [`Axes::format_data`]`(AxisName::X, v)`          |
//! | `ax.get_shared_x_axes()`                | [`Axes::shared_axes`]`(AxisName::X)`             |
//! | `ax.xaxis`, `ax.get_yaxis()`            | [`Axes::xaxis`], [`Axes::yaxis`], [`Axes::axis_of`] |
//! | `artist.remove()`                       | [`ArtistHandle::remove`]                         |
//! | `artist.axes`, `artist.figure`          | kept by the caller; [`Axes::figure`]            |
//!
//! # Getters and setters
//!
//! ```python
//! ax.set_title("Signal")
//! ax.set_xlim(0, 10)
//! line, = ax.plot([0, 10], [0, 1])
//! line.set_linewidth(3)
//! line.set_color("tab:green")
//! assert ax.get_title() == "Signal"
//! assert ax.get_xlim() == (0, 10)
//! assert line.get_linewidth() == 3
//! ```
//!
//! ```
//! use plt::colors::Color;
//! use plt::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! ax.set_title("Signal");
//! ax.set_xlim(0.0, 10.0).set_ylim(-1.0, 1.0);
//! let line = ax.plot([0.0, 10.0], [0.0, 1.0])?;
//! line.set_linewidth(3.0).set_color("tab:green");
//!
//! // `axes.titlelocation` decides where `set_title` puts the title.
//! assert_eq!(ax.shown_title().1, "Signal");
//! assert_eq!(ax.xlim(), (0.0, 10.0));
//! assert_eq!(line.linewidth(), 3.0);
//! assert_eq!(line.color(), Color::from("tab:green"));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Keyword arguments
//!
//! A property set is built with chained methods named after matplotlib's
//! keywords (without the abbreviations). Plain option structs are filled
//! with struct update syntax; their fields carry matplotlib's keyword
//! names too.
//!
//! ```python
//! ax.plot(x, y, "o--", color="tab:red", linewidth=2, markersize=4, label="data")
//! ax.errorbar(x, y, yerr=0.1, capsize=3)
//! ```
//!
//! ```
//! use plt::axes::ErrorbarOptions;
//! use plt::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! let x = [1.0, 2.0, 3.0];
//! let y = [2.0, 1.0, 3.0];
//! let props = LineProps::fmt("o--")?
//!     .color("tab:red")
//!     .linewidth(2.0)
//!     .markersize(4.0)
//!     .label("data");
//! let line = ax.plot_with(x, y, props)?;
//! assert_eq!(line.linestyle(), LineStyle::Dashed);
//! assert_eq!(line.markersize(), 4.0);
//!
//! let options = ErrorbarOptions {
//!     yerr: Some(0.1.into()),
//!     capsize: Some(3.0),
//!     ..ErrorbarOptions::default()
//! };
//! let bars = ax.errorbar(x, y, options)?;
//! assert_eq!(bars.caplines.len(), 2, "a lower and an upper cap line");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Strings, enums and parsing
//!
//! Enums name every value matplotlib accepts. When the value comes from a
//! string (a config file, a user), parse it: the error says what was
//! wrong instead of failing later.
//!
//! ```python
//! line.set_linestyle("--")
//! line.set_marker("o")
//! ax.annotate("peak", xy=(1, 1), xytext=(2, 2), arrowprops=dict(arrowstyle="->"))
//! ```
//!
//! ```
//! use plt::markers::{Marker, MarkerSymbol};
//! use plt::prelude::*;
//! use plt::text::{AnnotateOptions, ArrowProps, FancyArrowProps};
//!
//! let ax = Figure::new().subplot();
//! let line = ax.plot([0.0, 1.0], [0.0, 1.0])?;
//! line.set_linestyle(LineStyle::Dashed).set_marker(MarkerSymbol::Circle);
//! assert_eq!("--".parse::<LineStyle>()?, LineStyle::Dashed);
//! assert_eq!("o".parse::<Marker>()?, Marker::Symbol(MarkerSymbol::Circle));
//! assert!("not a marker".parse::<Marker>().is_err());
//!
//! let arrow: FancyArrowProps = "->".parse()?;
//! let options = AnnotateOptions {
//!     xytext: Some((2.0, 2.0).into()),
//!     arrowprops: Some(ArrowProps::Fancy(arrow)),
//!     ..AnnotateOptions::default()
//! };
//! let note = ax.annotate("peak", (1.0, 1.0), options)?;
//! assert_eq!(note.text(), "peak");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Errors
//!
//! matplotlib raises an exception, plt-rs returns an error value. Errors
//! are enums with one variant per problem, so you can match on them; their
//! `Display` reads like matplotlib's message.
//!
//! ```python
//! ax.plot([1, 2, 3], [1, 2])
//! # ValueError: x and y must have same first dimension, but have shapes (3,) and (2,)
//! ```
//!
//! ```
//! use plt::axes::PlotError;
//! use plt::colors::Color;
//! use plt::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! let error = ax.plot([1.0, 2.0, 3.0], [1.0, 2.0]).unwrap_err();
//! assert_eq!(error, PlotError::LengthMismatch { x: 3, y: 2 });
//! assert_eq!(
//!     error.to_string(),
//!     "x and y must have same first dimension, but have shapes (3,) and (2,)"
//! );
//! assert!(Color::parse("tab:nope").is_err());
//! ```
//!
//! # Handles
//!
//! Every artist lives in its figure; what you hold is a handle to it.
//! Handles are cheap to clone, compare equal when they refer to the same
//! artist, and see each other's changes. A figure and its handles stay on
//! the thread that made them, like pyplot's state. After
//! [`ArtistHandle::remove`] the artist is gone: using its handle again
//! panics, the way reading a closed file fails.
//!
//! ```python
//! line, = ax.plot([0, 1], [0, 1])
//! same = line                     # another name for the same object
//! same.set_color("tab:red")
//! assert line.get_color() == "tab:red"
//! line.remove()
//! assert len(ax.lines) == 0
//! ```
//!
//! ```
//! use plt::artist::ArtistHandle;
//! use plt::colors::Color;
//! use plt::prelude::*;
//!
//! let fig = Figure::new();
//! let ax = fig.subplot();
//! let line = ax.plot([0.0, 1.0], [0.0, 1.0])?;
//! let same = line.clone();
//! same.set_color("tab:red");
//! assert_eq!(line.color(), Color::from("tab:red"));
//! assert_eq!(ax.lines(), [line.clone()]);
//! assert_eq!(ax.figure(), fig);
//!
//! line.remove();
//! assert!(ax.lines().is_empty());
//! assert!(!same.raw().is_alive());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # The per-axis API
//!
//! Methods that exist once per axis in matplotlib take an [`AxisName`]
//! (or an [`Axis3`] on 3-D axes). The [`Axis`] handle returned by
//! [`Axes::xaxis`] and [`Axes::yaxis`] carries what matplotlib keeps on
//! `ax.xaxis`: ticks, locators, formatters, the label, inversion.
//!
//! ```python
//! ax.set_xmargin(0.2)
//! ax.invert_yaxis()
//! assert ax.get_xmargin() == 0.2
//! assert ax.yaxis_inverted()
//! assert ax.get_autoscalex_on()
//! ```
//!
//! ```
//! use plt::prelude::*;
//! use plt::ticker::AxisName;
//!
//! let ax = Figure::new().subplot();
//! ax.plot([0.0, 1.0], [0.0, 1.0])?;
//! ax.set_margin(AxisName::X, 0.2)?;
//! ax.yaxis().set_inverted(true);
//!
//! assert_eq!(ax.margin(AxisName::X), 0.2);
//! assert!(ax.yaxis().inverted());
//! assert!(ax.autoscale_on(Some(AxisName::X)));
//! let (low, high) = ax.bound(AxisName::Y);
//! assert!(low < high, "bounds are increasing even on an inverted axis");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # What was left out, and why
//!
//! * **Aliases.** `lw`, `ls`, `c`, `ec`, `fc`, `mec`, `mfc`, `ms`, `aa`,
//!   and the 3-D aliases `plot3D`, `scatter3D`, `text3D` and `contour3D`
//!   are gone. Each property has one name, the long one. Only the string
//!   keys of [`pyplot::rc`] still accept matplotlib's short forms, since
//!   they are rc-file text.
//! * **Introspection.** `setp`, `getp` and `get_plot_commands` exist
//!   because Python objects are dynamic. Typed getters and setters take
//!   their place, and `Artist.set(**kwargs)` becomes `update_props`.
//! * **Change tracking.** `pchanged`, `add_callback`,
//!   `Transform.invalidate` and frozen transforms have no counterpart.
//!   plt-rs resolves transforms and recomputes the layout each time a
//!   figure is drawn, so nothing is invalidated by hand. Handles share
//!   state, so there is nothing for `update_from` to copy.
//! * **Back-pointers.** Tickers do not point to their axis; a
//!   [`TickContext`] gives them what they need when they run. Artists do
//!   not know their axes or figure either, so keep the [`Axes`] you
//!   plotted on.
//!
//! [`LineProps`]: crate::lines::LineProps
//! [`LineProps::fmt`]: crate::lines::LineProps::fmt
//! [`PatchProps`]: crate::patches::PatchProps
//! [`CollectionProps`]: crate::collections::CollectionProps
//! [`TextProps`]: crate::text::TextProps
//! [`LegendOptions`]: crate::legend::LegendOptions
//! [`LineStyle::Dashed`]: crate::lines::LineStyle::Dashed
//! [`Color`]: crate::colors::Color
//! [`Color::parse`]: crate::colors::Color::parse
//! [`PlotError`]: crate::axes::PlotError
//! [`ImageError`]: crate::image::ImageError
//! [`LegendError`]: crate::legend::LegendError
//! [`Axes::xlim`]: crate::axes::Axes::xlim
//! [`Axes::title`]: crate::axes::Axes::title
//! [`Axes::shown_title`]: crate::axes::Axes::shown_title
//! [`Line2D::linewidth`]: crate::lines::Line2D::linewidth
//! [`Line2D::color`]: crate::lines::Line2D::color
//! [`Line2D::update_props`]: crate::lines::Line2D::update_props
//! [`Axes::get_legend`]: crate::axes::Axes::get_legend
//! [`Axes::plot_with`]: crate::axes::Axes::plot_with
//! [`MarkerSymbol::Circle`]: crate::markers::MarkerSymbol::Circle
//! [`FancyArrowProps`]: crate::text::FancyArrowProps
//! [`Axes::margin`]: crate::axes::Axes::margin
//! [`Axes::set_margin`]: crate::axes::Axes::set_margin
//! [`Axes::bound`]: crate::axes::Axes::bound
//! [`Axes::set_bound`]: crate::axes::Axes::set_bound
//! [`Axes::autoscale_on`]: crate::axes::Axes::autoscale_on
//! [`Axes::set_axis_autoscale_on`]: crate::axes::Axes::set_axis_autoscale_on
//! [`Axis::set_inverted`]: crate::axes::Axis::set_inverted
//! [`Axis::inverted`]: crate::axes::Axis::inverted
//! [`Axes::xaxis`]: crate::axes::Axes::xaxis
//! [`Axes::yaxis`]: crate::axes::Axes::yaxis
//! [`Axes::axis_of`]: crate::axes::Axes::axis_of
//! [`Axes::format_data`]: crate::axes::Axes::format_data
//! [`Axes::shared_axes`]: crate::axes::Axes::shared_axes
//! [`Axes::figure`]: crate::axes::Axes::figure
//! [`ArtistHandle::remove`]: crate::artist::ArtistHandle::remove
//! [`AxisName`]: crate::ticker::AxisName
//! [`Axis3`]: crate::mplot3d::Axis3
//! [`Axis`]: crate::axes::Axis
//! [`TickContext`]: crate::ticker::TickContext
//! [`Axes`]: crate::axes::Axes
//! [`pyplot::rc`]: crate::pyplot::rc
