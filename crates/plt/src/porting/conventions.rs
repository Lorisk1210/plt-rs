//! Conventions: the rules that turn a matplotlib call into a plt-rs call.
//!
//! A handful of rules cover most of the translation. Once you know them
//! you can usually guess the plt-rs name of a matplotlib function, and
//! rustdoc's search does the rest.
//!
//! * **Getters drop `get_`.** `ax.get_xlim()` is `ax.xlim()`,
//!   `line.get_color()` is `line.color()`. Setters keep `set_` and return
//!   `&Self` so calls chain. A setter whose argument can be invalid
//!   returns `Result<&Self, _>` and chains through `?`:
//!   `ax.set_xlim(0.0, 10.0)?.set_ylim(-1.0, 1.0)?` refuses limits that
//!   are NaN or infinite, where matplotlib raises. Setters that create an
//!   artist, such as `set_title`, return its handle. Where the plain name already
//!   makes something, the getter reads `current_`: `ax.legend()` makes a
//!   legend, as in matplotlib, so `ax.get_legend()` is
//!   [`Axes::current_legend`], the way `plt.gci()` is
//!   [`Axes::current_image`].
//! * **Keyword arguments become option structs, passed by value.** Each
//!   plotting method, artist or widget constructor and save function takes
//!   a struct of its keywords and consumes it; clone a struct to use it
//!   again. The public fields are the API: fill them with struct update
//!   syntax (`..Default::default()`); their names are matplotlib's
//!   keywords. Unset options fall back to rcParams, as in matplotlib.
//!   Builder methods are conveniences a few structs add on top of their
//!   fields: the property sets ([`LineProps`], [`PatchProps`],
//!   [`CollectionProps`], [`TextProps`]) and [`LegendOptions`], whose
//!   fields are all `Option`s set from `impl Into` values, and the
//!   options mapping values to colours, which share matplotlib's `cmap`,
//!   `norm`, `vmin` and `vmax` as one [`ColorizerOptions`] in their
//!   `colorizer` field and its builders through [`Colorize`]
//!   ([`ImshowOptions`], [`ContourOptions`], [`MappedColors`], ...).
//!   Other option structs have none. [`UnitAxisInfo`] has builders too,
//!   which take a locator or a formatter by value as the [`Axis`]
//!   setters do: tickers go as `impl Locator` and `impl Formatter`
//!   wherever they are given.
//! * **Strings become enums or parsed types.** `linestyle="--"` is
//!   [`LineStyle::Dashed`]; matplotlib's spelling still works through
//!   [`str::parse`] (every such type implements [`FromStr`]), which
//!   validates it where it is given, format strings and legend locations
//!   ([`LegendLoc`]) included. Two kinds of names convert from any string
//!   instead: a [`Color`], validated when it is resolved, and a
//!   [`ScaleName`], since a scale can be registered under any name; a
//!   setter refuses an unknown scale.
//! * **Keyword rcParams hold the type of their option.** An rcParam naming
//!   one of a few keywords holds the enum the matching option takes
//!   (`axes.titlelocation` is a [`TitleLocation`]), read and written in
//!   matplotlib's spelling through `FromStr` and `Display`. Where the
//!   spellings carry nothing of their own, aliases or names that differ
//!   per axis, the rcParam keeps them in a type of its own that converts
//!   into the one the API takes, so that `rc::get` gives back what
//!   `rc::set` took: [`XLabelLoc`] and [`YLabelLoc`] convert into the
//!   [`LabelLoc`] of [`Axis::set_label_loc`], [`TextHinting`] into a
//!   [`Hinting`], and `axes.grid.axis` is a [`WhichAxis`] where options
//!   take an `Option<AxisName>` (`None` for both).
//! * **Errors are `Result`s.** Invalid data, options or settings return
//!   an error enum of the module ([`PlotError`], [`ImageError`],
//!   [`LegendError`], [`AxesRectError`], ...), wherever matplotlib
//!   raises. All of them implement [`std::error::Error`], so `?` into
//!   `Box<dyn Error>` works everywhere. A problem plt-rs can name has a
//!   variant of its own carrying the values involved, rather than a
//!   message in a catch-all variant ([`TexError::PostScriptOutput`],
//!   [`TexError::NotInstalled`]), and parsing a name fails with such an
//!   error, never a bare `String` ([`UnknownName`] for the keyword types
//!   of `plt`).
//! * **Limits are public.** Where matplotlib allocates whatever an input
//!   asks for until memory runs out, plt-rs refuses inputs beyond a
//!   `MAX_*` constant of the module ([`MAX_HIST_BARS`],
//!   [`MAX_EQUAL_BINS`], [`MAX_DRAWN_TICKS`], ...), named after what it
//!   counts and documented on the functions and options it bounds. The
//!   error variant is named `TooMany*` after the same things
//!   (`FftTooLong` for a length) and carries what was asked for.
//! * **Data takes every form.** Plotting values that may be numbers,
//!   category names, dates or values of user types are [`AxisValues`]
//!   (one is an [`AxisValue`], two make a [`DataPoint`]): any collection
//!   or iterator of [`AxisItem`]s converts into them, numbers of every
//!   primitive type included, so data goes in borrowed, owned or computed
//!   by an iterator, and one value is a list of one, `[v]`. Extents from
//!   them, numbers, durations or user values, are [`AxisDeltas`] (one is
//!   an [`AxisDelta`]); they, and the types made of them such as
//!   [`ErrorSize`], take one value for all the items or a `Vec`, a slice,
//!   an array or a reference to a `Vec` or an array. Data that can only be
//!   numbers takes `impl IntoIterator<Item: Number>` ([`Number`]), and
//!   one dataset or several, as histograms take them, are [`Datasets`].
//!   Methods take these as they are, option fields through `.into()`, and
//!   setters of data (`Line2D::set_ydata`) take what the plotting method
//!   takes.
//! * **User code may use the figure.** Closures plt-rs runs between draws
//!   (event callbacks, timers, toolbar tools, widgets, animation frames,
//!   pickers, unit converters) may use any handle of the figure. Methods
//!   handing state to a closure ([`Custom::update`],
//!   [`Patch::update_shape`], [`Axes::with_projection_mut`],
//!   [`rc::update`]) give it a copy and store it back, so the figure is
//!   not borrowed meanwhile. Functions and traits that run while a figure
//!   is laid out or drawn (formatters, scale and norm functions, axes
//!   locators, image filters, colorbar label functions, legend handlers,
//!   projections) get what they need as arguments and are `Send`, which
//!   keeps handles out of them.
//! * **Contexts return the closure's result.** Helpers running a closure
//!   in temporary settings return what it returns ([`rc::context`]).
//!   Those that can fail before running it ([`style::context`],
//!   [`pyplot::rc_context`], [`pyplot::xkcd`]) take a closure returning
//!   `Result<T, E>` and convert their own error into `E`, so the call ends
//!   in one `?`.
//! * **Lookups index as `std` does.** Indexing a collection of handles
//!   (`axs[(0, 1)]`, `axd["A"]` of a mosaic) panics on an entry that is
//!   not there, as indexing a slice or a map does; `get` returns an
//!   `Option` instead.
//! * **Type names are unique.** Where matplotlib gives two classes one
//!   name in modules a program may use together, plt-rs prefixes one with
//!   its module: `units.AxisInfo` is [`UnitAxisInfo`], beside the
//!   [`ticker::AxisInfo`] tick helpers read, and `GeoAxes.ThetaFormatter`
//!   is [`GeoThetaFormatter`], beside the polar [`ThetaFormatter`].
//! * **plt-rs' own rcParams start with `plt.`**, then the matplotlib rc
//!   group, module or class they belong to in lower case, then the
//!   keyword whose value matplotlib fixes in code, spelled as that keyword:
//!   `plt.widgets.button.color` for `Button(color=...)`,
//!   `plt.table.facecolor` for a table cell's `facecolor`,
//!   `plt.widgets.radiobuttons.activecolor`. A behaviour of plt-rs' own
//!   is named in lower-case words joined by underscores
//!   (`plt.image.hide_grid`, `plt.tight_layout.keep_constrained`), and a
//!   setting shared by a whole group sits at its level
//!   (`plt.widgets.frame`; `plt.ink` for every mark matplotlib draws
//!   black).
//! * **Handles, not objects.** `Figure`, `Axes`, `Line2D` and every other
//!   artist type are cheap handles into a figure. Cloning a handle gives a
//!   second reference to the same artist, never a copy. Every handle knows
//!   its [`figure`](crate::artist::ArtistHandle::figure) and its
//!   [`axes`](crate::artist::ArtistHandle::axes), and searches what it
//!   owns with [`findobj`](crate::artist::ArtistHandle::findobj).
//! * **The per-axis API takes an axis argument.** Where matplotlib has
//!   `get_xmargin` and `get_ymargin`, plt-rs has `margin(AxisName::X)`.
//!   The most common pairs (`set_xlim`, `set_xlabel`, `set_xscale`,
//!   `twinx`, ...) keep their x and y spellings.
//! * **3-D axes are 2-D axes with a z.** A method of [`Axes3D`] or
//!   [`Axis3D`] doing what a method of [`Axes`] or [`Axis`] does has its
//!   name and takes the same kinds of values, with an [`Axis3`] where the
//!   2-D one takes an [`AxisName`]: plotting values for data, limits and
//!   fixed ticks ([`Axis3D::set_ticks`] as [`Axis::set_ticks`]),
//!   [`Axes3D::autoscale_on`] as [`Axes::autoscale_on`],
//!   [`Axes3D::set_margins`] as [`Axes::set_margins`]. Grids of surfaces
//!   and contours are numbers on both.
//!
//! # Equivalences
//!
//! | matplotlib                              | plt-rs                                           |
//! |-----------------------------------------|--------------------------------------------------|
//! | `ax.get_xlim()`                         | [`Axes::xlim`]                                   |
//! | `ax.get_title()`                        | [`Axes::shown_title`] (where it is, and its text) |
//! | `ax.get_title(loc="left")`              | [`Axes::title`]`(TitleLocation::Left)`           |
//! | `line.get_linewidth()`                  | [`Line2D::linewidth`]                            |
//! | `ax.get_legend()`                       | [`Axes::current_legend`] (`legend()` makes one)  |
//! | `ax.set(xlim=..., xlabel=...)`          | chained setters: `ax.set_xlim(..)?.set_xlabel(..)` |
//! | `line.set(color=..., lw=...)`, `plt.setp` | [`Line2D::update_props`]                       |
//! | `plt.getp(artist, "color")`             | the getter: [`Line2D::color`]                    |
//! | `ax.plot(x, y, color="r", lw=2)`        | [`Axes::plot_with`] with [`LineProps`]           |
//! | `ax.plot(x, y, "o--")`                  | [`LineProps::fmt`]`("o--")?`                     |
//! | `ax.errorbar(..., fmt="o")`             | `fmt: "o".parse()?` ([`ErrorbarFormat`])         |
//! | `linestyle="--"`                        | [`LineStyle::Dashed`], or `"--".parse::<LineStyle>()?` |
//! | `marker="o"`                            | [`MarkerSymbol::Circle`], or `"o".parse::<Marker>()?` |
//! | `arrowprops=dict(arrowstyle="->")`      | `Some("->".parse()?)` ([`ArrowProps`]; [`FancyArrowProps`] for its settings) |
//! | `axd = fig.subplot_mosaic("AB;CC")`, `axd["A"]` | [`Figure::subplot_mosaic`]`("AB;CC", ..)?`, `axd["A"]` |
//! | `ax.set_xlim(date1, date2)`             | [`Axes::set_xlim`]`(date1, date2)?` (any [`AxisValue`]) |
//! | `ax.plot(np.arange(10), counts)`        | `ax.plot(0..10, &counts)?` (integers and iterators) |
//! | `color="tab:red"`                       | `"tab:red"` (any `Into<`[`Color`]`>`); [`Color::parse`] validates now |
//! | `loc="upper left"`                      | `AnchorLoc::UpperLeft`, or `"upper left".parse::<`[`LegendLoc`]`>()?` |
//! | `ax.set_xscale("log")`, `ax.get_xscale()` | [`Axes::set_xscale`]`(ScaleName::Log)` or `("log")`; [`Axes::xscale`] is a [`ScaleName`] |
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
//! | `artist.axes`, `artist.figure`          | [`ArtistHandle::axes`], [`ArtistHandle::figure`] |
//! | `artist.get_figure(root=False)`         | [`ArtistHandle::parent_figure`]                  |
//! | `artist.findobj(match)`                 | [`ArtistHandle::findobj`]                        |
//! | `ax.lines`, `ax.patches`, `ax.artists`  | [`Axes::lines`], [`Axes::patches`], [`Axes::artists`] |
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
//! ax.set_xlim(0.0, 10.0)?.set_ylim(-1.0, 1.0)?;
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
//! names too. Either way the options are moved into the call.
//!
//! ```python
//! ax.plot(x, y, "o--", color="tab:red", linewidth=2, markersize=4, label="data")
//! ax.errorbar(x, y, yerr=0.1, fmt="s", capsize=3)
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
//!     fmt: "s".parse()?,
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
//! use plt::text::AnnotateOptions;
//!
//! let ax = Figure::new().subplot();
//! let line = ax.plot([0.0, 1.0], [0.0, 1.0])?;
//! line.set_linestyle(LineStyle::Dashed).set_marker(MarkerSymbol::Circle);
//! assert_eq!("--".parse::<LineStyle>()?, LineStyle::Dashed);
//! assert_eq!("o".parse::<Marker>()?, Marker::Symbol(MarkerSymbol::Circle));
//! assert!("not a marker".parse::<Marker>().is_err());
//!
//! let options = AnnotateOptions {
//!     xytext: Some((2.0, 2.0).into()),
//!     arrowprops: Some("->".parse()?),
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
//! ax.set_xlim(np.nan, 1)
//! # ValueError: Axis limits cannot be NaN or Inf
//! fig.add_axes([np.nan, 0, 1, 1])
//! # ValueError: all entries in rect must be finite not [nan, 0, 1, 1]
//! ```
//!
//! ```
//! use plt::axes::{AxisName, PlotError};
//! use plt::colors::Color;
//! use plt::figure::AxesRectError;
//! use plt::prelude::*;
//!
//! let fig = Figure::new();
//! let ax = fig.subplot();
//! let error = ax.plot([1.0, 2.0, 3.0], [1.0, 2.0]).unwrap_err();
//! assert_eq!(error, PlotError::LengthMismatch { x: 3, y: 2 });
//! assert_eq!(
//!     error.to_string(),
//!     "x and y must have same first dimension, but have shapes (3,) and (2,)"
//! );
//! assert!(Color::parse("tab:nope").is_err());
//! assert_eq!(
//!     ax.set_xlim(f64::NAN, 1.0).err(),
//!     Some(PlotError::NonFiniteLimit(AxisName::X))
//! );
//! assert!(matches!(
//!     fig.add_axes([f64::NAN, 0.0, 1.0, 1.0]),
//!     Err(AxesRectError::NonFinite(_))
//! ));
//! ```
//!
//! # Handles
//!
//! Every artist lives in its figure; what you hold is a handle to it.
//! Handles are cheap to clone, compare equal when they refer to the same
//! artist, and see each other's changes. A figure and its handles stay on
//! the thread that made them, like pyplot's state. A handle knows the
//! axes and the figure it belongs to.
//!
//! [`ArtistHandle::remove`] takes the artist, with everything it owns, out
//! of the figure, as in matplotlib: the figure no longer draws it, lays it
//! out, autoscales over it or finds it, and its `axes()` are `None`. The
//! handle stays usable. Properties read and change as before, and
//! [`Axes::attach`] or [`Figure::attach`] put the artist back, as
//! matplotlib's `add_artist` does. Removing twice does nothing.
//!
//! Python frees a removed artist once nothing refers to it. A figure
//! cannot tell when the last handle goes, so it keeps its removed artists
//! until it is dropped itself. A program that removes artists by the
//! thousand, an animation clearing its axes every frame for instance,
//! keeps them all; update the artists in place or make a fresh figure.
//!
//! ```python
//! line, = ax.plot([0, 1], [0, 1])
//! same = line                     # another name for the same object
//! same.set_color("tab:red")
//! assert line.get_color() == "tab:red"
//! assert line.axes is ax and line.figure is fig
//! line.remove()
//! assert len(ax.lines) == 0 and line.axes is None
//! line.set_linewidth(3)           # still usable
//! ax.add_line(line)               # and back again
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
//! assert_eq!(line.axes(), Some(ax.clone()));
//! assert_eq!(line.figure(), fig);
//!
//! line.remove();
//! assert!(ax.lines().is_empty() && line.axes().is_none());
//! assert!(!same.raw().is_attached());
//! line.set_linewidth(3.0);
//! ax.attach(&line)?;
//! assert_eq!(ax.lines(), [line]);
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
//! * **Back-pointers of tickers.** Tickers do not point to their axis; a
//!   [`TickContext`] gives them what they need when they run.
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
//! [`LegendLoc`]: crate::legend::LegendLoc
//! [`ScaleName`]: crate::scale::ScaleName
//! [`Axes::set_xscale`]: crate::axes::Axes::set_xscale
//! [`Axes::xscale`]: crate::axes::Axes::xscale
//! [`PlotError`]: crate::axes::PlotError
//! [`ImageError`]: crate::image::ImageError
//! [`LegendError`]: crate::legend::LegendError
//! [`AxesRectError`]: crate::figure::AxesRectError
//! [`MAX_HIST_BARS`]: crate::axes::MAX_HIST_BARS
//! [`MAX_EQUAL_BINS`]: crate::stats::MAX_EQUAL_BINS
//! [`MAX_DRAWN_TICKS`]: crate::ticker::MAX_DRAWN_TICKS
//! [`AxisValues`]: crate::units::AxisValues
//! [`AxisValue`]: crate::units::AxisValue
//! [`DataPoint`]: crate::units::DataPoint
//! [`AxisDeltas`]: crate::units::AxisDeltas
//! [`AxisDelta`]: crate::units::AxisDelta
//! [`ErrorSize`]: crate::axes::ErrorSize
//! [`UnitAxisInfo`]: crate::units::UnitAxisInfo
//! [`ticker::AxisInfo`]: crate::ticker::AxisInfo
//! [`GeoThetaFormatter`]: crate::geo::GeoThetaFormatter
//! [`ThetaFormatter`]: crate::polar::ThetaFormatter
//! [`Axes::xlim`]: crate::axes::Axes::xlim
//! [`Axes::title`]: crate::axes::Axes::title
//! [`Axes::shown_title`]: crate::axes::Axes::shown_title
//! [`Line2D::linewidth`]: crate::lines::Line2D::linewidth
//! [`Line2D::color`]: crate::lines::Line2D::color
//! [`Line2D::update_props`]: crate::lines::Line2D::update_props
//! [`Axes::current_legend`]: crate::axes::Axes::current_legend
//! [`Axes::current_image`]: crate::axes::Axes::current_image
//! [`Axes::lines`]: crate::axes::Axes::lines
//! [`Axes::patches`]: crate::axes::Axes::patches
//! [`Axes::artists`]: crate::axes::Axes::artists
//! [`ErrorbarFormat`]: crate::axes::ErrorbarFormat
//! [`ImshowOptions`]: crate::axes::ImshowOptions
//! [`ContourOptions`]: crate::contour::ContourOptions
//! [`MappedColors`]: crate::axes::MappedColors
//! [`ColorizerOptions`]: crate::colors::ColorizerOptions
//! [`Colorize`]: crate::colors::Colorize
//! [`FromStr`]: std::str::FromStr
//! [`ArtistHandle::axes`]: crate::artist::ArtistHandle::axes
//! [`ArtistHandle::figure`]: crate::artist::ArtistHandle::figure
//! [`ArtistHandle::parent_figure`]: crate::artist::ArtistHandle::parent_figure
//! [`ArtistHandle::findobj`]: crate::artist::ArtistHandle::findobj
//! [`Axes::plot_with`]: crate::axes::Axes::plot_with
//! [`MarkerSymbol::Circle`]: crate::markers::MarkerSymbol::Circle
//! [`FancyArrowProps`]: crate::text::FancyArrowProps
//! [`ArrowProps`]: crate::text::ArrowProps
//! [`AxisItem`]: crate::units::AxisItem
//! [`Number`]: crate::numeric::Number
//! [`Datasets`]: crate::stats::Datasets
//! [`Custom::update`]: crate::artist::Custom::update
//! [`Patch::update_shape`]: crate::patches::Patch::update_shape
//! [`Axes::with_projection_mut`]: crate::axes::Axes::with_projection_mut
//! [`rc::update`]: crate::rc::update
//! [`rc::context`]: crate::rc::context
//! [`style::context`]: crate::style::context
//! [`pyplot::rc_context`]: crate::pyplot::rc_context
//! [`pyplot::xkcd`]: crate::pyplot::xkcd
//! [`Figure::subplot_mosaic`]: crate::figure::Figure::subplot_mosaic
//! [`Axes::set_xlim`]: crate::axes::Axes::set_xlim
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
//! [`ArtistHandle::remove`]: crate::artist::ArtistHandle::remove
//! [`Axes::attach`]: crate::axes::Axes::attach
//! [`Figure::attach`]: crate::figure::Figure::attach
//! [`AxisName`]: crate::ticker::AxisName
//! [`Axis3`]: crate::mplot3d::Axis3
//! [`Axis`]: crate::axes::Axis
//! [`TickContext`]: crate::ticker::TickContext
//! [`pyplot::rc`]: crate::pyplot::rc
//! [`TitleLocation`]: crate::axes::TitleLocation
//! [`XLabelLoc`]: crate::axes::XLabelLoc
//! [`YLabelLoc`]: crate::axes::YLabelLoc
//! [`LabelLoc`]: crate::axes::LabelLoc
//! [`Axis::set_label_loc`]: crate::axes::Axis::set_label_loc
//! [`TextHinting`]: crate::text::TextHinting
//! [`Hinting`]: crate::text::Hinting
//! [`WhichAxis`]: crate::axes::WhichAxis
//! [`TexError::PostScriptOutput`]: crate::text::TexError::PostScriptOutput
//! [`TexError::NotInstalled`]: crate::text::TexError::NotInstalled
//! [`UnknownName`]: crate::image::UnknownName
//! [`Axes3D`]: crate::mplot3d::Axes3D
//! [`Axis3D`]: crate::mplot3d::Axis3D
//! [`Axes`]: crate::axes::Axes
//! [`Axis3D::set_ticks`]: crate::mplot3d::Axis3D::set_ticks
//! [`Axis::set_ticks`]: crate::axes::Axis::set_ticks
//! [`Axes3D::autoscale_on`]: crate::mplot3d::Axes3D::autoscale_on
//! [`Axes3D::set_margins`]: crate::mplot3d::Axes3D::set_margins
//! [`Axes::set_margins`]: crate::axes::Axes::set_margins
