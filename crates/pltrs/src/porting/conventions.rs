//! The rules that translate a matplotlib call into a pltrs call.
//!
//! With these rules, you can usually guess the pltrs name of a matplotlib
//! function. The rustdoc search finds the rest.
//!
//! # The rules
//!
//! ## Getters drop `get_`
//!
//! `ax.get_xlim()` is `ax.xlim()`. Setters keep `set_` and return `&Self`
//! for chained calls. A setter that can refuse its argument returns
//! `Result<&Self, _>`: `ax.set_xlim(0.0, 10.0)?.set_ylim(-1.0, 1.0)?`.
//! Values that are not handles ([`FontProperties`], [`ScalarMappable`],
//! [`Triangulation`]) chain through `&mut self`. A setter that makes an
//! artist, such as `set_title`, returns its handle.
//!
//! If the plain name already makes something, the getter starts with
//! `current_`. `ax.legend()` makes a legend, so `ax.get_legend()` is
//! [`Axes::current_legend`], and `plt.gci()` is [`Axes::current_image`].
//!
//! ## Keyword arguments become option structs
//!
//! Each function with keywords takes a struct of them by value. Examples
//! are plotting methods, constructors, save functions, [`Axes::tick_params`],
//! [`Axes::locator_params`], [`Figure::tight_layout`] and
//! [`Figure::subplots_adjust`]. The public fields have the names of the
//! matplotlib keywords. Fill them with struct update syntax
//! (`..Default::default()`), and clone a struct to use it again. An unset
//! option takes its rcParam.
//!
//! Only these structs also have builder methods:
//!
//! * the property sets [`LineProps`], [`PatchProps`], [`CollectionProps`]
//!   and [`TextProps`], and [`LegendOptions`]. Their builders take
//!   `impl Into` values.
//! * the options that map values to colors ([`ImshowOptions`],
//!   [`ContourOptions`], [`MappedColors`], ...). Their field `colorizer` is
//!   a [`ColorizerOptions`] with `cmap`, `norm`, `vmin` and `vmax`. The
//!   trait [`Colorize`] gives them its builders. Put these builders last in
//!   a struct update:
//!   `ImshowOptions { extent: .., ..ImshowOptions::default().cmap(..) }`.
//! * [`UnitAxisInfo`].
//!
//! Tickers are `impl Locator` and `impl Formatter` values. If an option
//! holds a function, its type has a constructor that takes a closure:
//! `func: LegendValueFn::new(|size| size / 20.0)` ([`LegendValueFn`],
//! [`AxesLocatorFn`]). The default does what matplotlib does.
//!
//! ## Strings become enums or parsed types
//!
//! `linestyle="--"` is [`LineStyle::Dashed`]. These types implement
//! [`FromStr`], so [`str::parse`] checks a matplotlib string at once. This
//! includes format strings and legend locations ([`LegendLoc`]). A
//! [`Color`] and a [`ScaleName`] convert from any string. pltrs checks a
//! color when it resolves it. A setter refuses an unknown scale.
//!
//! ## Keyword rcParams hold the type of their option
//!
//! An rcParam that takes one of a few keywords holds an enum:
//! `axes.titlelocation` is a [`TitleLocation`]. `FromStr` and `Display`
//! use the matplotlib spelling. If the rcParam accepts aliases or names
//! for each axis, it has its own type, so that `rc::get` returns what
//! `rc::set` took. That type converts into the type of the API:
//!
//! * [`XLabelLoc`] and [`YLabelLoc`] into the [`LabelLoc`] of
//!   [`Axis::set_label_loc`].
//! * [`TextHinting`] into a [`Hinting`].
//! * the [`WhichAxis`] of `axes.grid.axis` into an `Option<AxisName>`, where
//!   `None` means both axes.
//!
//! The module of the methods also exports such a type: the [`Aspect`] of
//! `image.aspect` is `pltrs::rc::Aspect` and `pltrs::axes::Aspect`
//! ([`Axes::set_aspect`]).
//!
//! ## Errors are `Result`s
//!
//! If matplotlib raises, pltrs returns an error enum of the module
//! ([`PlotError`], [`ImageError`], [`LegendError`], [`AxesRectError`],
//! ...). All error enums implement [`std::error::Error`], so `?` into
//! `Box<dyn Error>` works. Each problem has its own variant with the values
//! involved ([`TexError::NoType1Font`], [`TexError::NotInstalled`]). A
//! failed parse of a name returns such an error, never a `String`
//! ([`UnknownName`] for the keyword types of `pltrs`).
//!
//! ## Limits are public
//!
//! matplotlib allocates what an input asks for, until memory runs out. pltrs
//! refuses inputs above a `MAX_*` constant of the module ([`MAX_HIST_BARS`],
//! [`MAX_EQUAL_BINS`], [`MAX_DRAWN_TICKS`], ...). The documentation of each
//! limited function names its constant. The error variant is `TooMany*`
//! with the same name (`FftTooLong` for a length), and holds the requested
//! value.
//!
//! ## Data takes every form
//!
//! Plotting values are [`AxisValues`]: numbers, category names, dates or
//! user values. One value is an [`AxisValue`], and two make a
//! [`DataPoint`]. Any collection or iterator of [`AxisItem`]s converts into
//! [`AxisValues`], numbers of each primitive type included. Thus data can
//! be borrowed, owned or made by an iterator. One value is a list of one:
//! `[v]`.
//!
//! Extents are [`AxisDeltas`] (one is an [`AxisDelta`]): numbers, durations
//! or user values. They and the types made of them, such as [`ErrorSize`],
//! take one value for all items or a list. A list is a `Vec`, a slice, an
//! array, or a reference to a `Vec` or an array. Data that can only be numbers takes
//! `impl IntoIterator<Item: Number>` ([`Number`]). One or more datasets are
//! [`Datasets`].
//!
//! Option fields take these types through `.into()`. A data setter
//! (`Line2D::set_ydata`) takes what the plotting method takes. A tick
//! setter takes what [`Axes::set_xticks`] takes, also on other projections
//! ([`PolarAxes::set_rticks`]).
//!
//! A list of numbers in a setting converts from an array or a `Vec`. An
//! image [`Extent`] converts from `[left, right, bottom, top]`. The
//! multiples of each `with_subs` convert from `[2.0, 5.0]`
//! ([`LogLocator::with_subs`]). One label
//! or a list of labels converts into [`DatasetLabels`]:
//! [`HistPlotOptions::label`] is `"samples".into()` or `["a", "b"].into()`.
//!
//! ## User code can use the figure
//!
//! Closures that pltrs runs between draws can use any handle of the figure.
//! These are event callbacks, timers, toolbar tools, widgets, animation
//! frames, pickers and unit converters. Methods that give state to a closure
//! ([`Custom::update`], [`Patch::update_shape`],
//! [`Axes::with_projection_mut`], [`rc::update`]) give it a copy and store
//! the copy back. Thus the figure is not borrowed during the call.
//!
//! Code that runs during layout or drawing gets what it needs as arguments.
//! This is formatters, scale and norm functions, axes locators, image
//! filters, colorbar label functions, legend handlers and projections. It is
//! `Send`, which keeps handles out of it.
//!
//! ## Contexts return the result of the closure
//!
//! A helper that runs a closure with temporary settings returns the result
//! of the closure ([`rc::context`]). If the helper can fail before it runs
//! the closure ([`style::context`], [`pyplot::rc_context`],
//! [`pyplot::xkcd`]), the closure returns `Result<T, E>`. The helper
//! converts its own error into `E`, so the call ends in one `?`.
//!
//! ## Indexing works as in `std`
//!
//! `axs[(0, 1)]` and `axd["A"]` panic on a missing entry, as a slice or a
//! map does. `get` returns an `Option`.
//!
//! ## Type names are unique
//!
//! If matplotlib has two classes with one name, pltrs adds the module name
//! to one. `units.AxisInfo` is [`UnitAxisInfo`], beside
//! [`ticker::AxisInfo`]. `GeoAxes.ThetaFormatter` is [`GeoThetaFormatter`],
//! beside the polar [`ThetaFormatter`]. `pltrs` re-exports the types of its
//! lower crates under their original names ([`RotationMode`] in
//! `pltrs::text`).
//!
//! ## The rcParams of pltrs start with `pltrs.`
//!
//! The name continues with the matplotlib rc group, module or class in
//! lower case. It ends with the keyword whose value matplotlib fixes in
//! code: `pltrs.widgets.button.color` for `Button(color=...)`, or
//! `pltrs.table.facecolor` for the `facecolor` of a table cell. A behavior
//! that only pltrs has uses lower-case words and underscores
//! (`pltrs.image.hide_grid`). A setting for a whole group is at the level
//! of the group: `pltrs.widgets.frame`, or `pltrs.ink` for each mark that
//! matplotlib draws black.
//!
//! ## Handles, not objects
//!
//! `Figure`, `Axes`, `Line2D` and all other artist types are cheap handles.
//! A clone refers to the same artist. Each handle knows its
//! [`figure`](crate::artist::ArtistHandle::figure) and its
//! [`axes`](crate::artist::ArtistHandle::axes), and
//! [`findobj`](crate::artist::ArtistHandle::findobj) searches what it owns.
//!
//! Widgets are values. A widget owns its event connections, and they end
//! when you drop it. Thus a widget is not `Clone`. To share one between
//! callbacks, use an `Rc`. Legends take handles owned or borrowed
//! ([`LegendHandle`]), and handles of several types one at a time
//! ([`LegendOptions::handle`]).
//!
//! ## The per-axis API takes an axis argument
//!
//! `get_xmargin` and `get_ymargin` become `margin(AxisName::X)`. The common
//! pairs keep their x and y names (`set_xlim`, `set_xlabel`, `set_xscale`,
//! `twinx`, ...).
//!
//! ## 3-D axes are 2-D axes with a z
//!
//! A method of [`Axes3D`] or [`Axis3D`] has the name of the matching method
//! of [`Axes`] or [`Axis`]. It takes the same kinds of values, with an
//! [`Axis3`] in place of an [`AxisName`]. Examples are
//! [`Axis3D::set_ticks`] and [`Axes3D::autoscale_on`]. Grids of surfaces and
//! contours are numbers on both kinds of axes.
//! # Equivalences
//!
//! | matplotlib                              | pltrs                                            |
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
//! | `arrowprops=dict(arrowstyle="->")`      | `Some("->".parse()?)` ([`ArrowProps`], with [`FancyArrowProps`] for its settings) |
//! | `axd = fig.subplot_mosaic("AB;CC")`, `axd["A"]` | [`Figure::subplot_mosaic`]`("AB;CC", ..)?`, `axd["A"]` |
//! | `ax.set_xlim(date1, date2)`             | [`Axes::set_xlim`]`(date1, date2)?` (any [`AxisValue`]) |
//! | `ax.plot(np.arange(10), counts)`        | `ax.plot(0..10, &counts)?` (integers and iterators) |
//! | `color="tab:red"`                       | `"tab:red"` (any `Into<`[`Color`]`>`), or [`Color::parse`] to check it at once |
//! | `loc="upper left"`                      | `AnchorLoc::UpperLeft`, or `"upper left".parse::<`[`LegendLoc`]`>()?` |
//! | `ax.set_xscale("log")`, `ax.get_xscale()` | [`Axes::set_xscale`]`(ScaleName::Log)` or `("log")`, and [`Axes::xscale`] returns a [`ScaleName`] |
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
//! use pltrs::colors::Color;
//! use pltrs::prelude::*;
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
//! Property sets use builder methods. Other option structs use struct
//! update syntax.
//!
//! ```python
//! ax.plot(x, y, "o--", color="tab:red", linewidth=2, markersize=4, label="data")
//! ax.errorbar(x, y, yerr=0.1, fmt="s", capsize=3)
//! ```
//!
//! ```
//! use pltrs::axes::ErrorbarOptions;
//! use pltrs::prelude::*;
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
//! If a value comes from a string, for example from a configuration file,
//! parse it. The parse error says what is wrong.
//!
//! ```python
//! line.set_linestyle("--")
//! line.set_marker("o")
//! ax.annotate("peak", xy=(1, 1), xytext=(2, 2), arrowprops=dict(arrowstyle="->"))
//! ```
//!
//! ```
//! use pltrs::markers::{Marker, MarkerSymbol};
//! use pltrs::prelude::*;
//! use pltrs::text::AnnotateOptions;
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
//! Errors have one variant for each problem, so you can match on them.
//! Their `Display` text reads like the matplotlib message.
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
//! use pltrs::axes::{AxisName, PlotError};
//! use pltrs::colors::Color;
//! use pltrs::figure::AxesRectError;
//! use pltrs::prelude::*;
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
//! Two handles are equal if they refer to the same artist. A figure and its
//! handles stay on the thread that made them, like the pyplot state.
//!
//! [`ArtistHandle::remove`] takes the artist and all that it owns out of the
//! figure, as in matplotlib. The figure then does not draw it, lay it out,
//! autoscale over it or find it, and its `axes()` are `None`. The handle
//! stays usable. [`Axes::attach`] or
//! [`Figure::attach`] puts the artist back, as `add_artist` does. A second
//! remove does nothing.
//!
//! A figure keeps its removed artists until you drop it, because it cannot
//! know when the last handle goes. Thus an animation that clears its axes
//! in each frame keeps all the old artists. Update the artists in place,
//! or make a new figure.
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
//! use pltrs::artist::ArtistHandle;
//! use pltrs::colors::Color;
//! use pltrs::prelude::*;
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
//! The [`Axis`] handle of [`Axes::xaxis`] and [`Axes::yaxis`] holds what
//! matplotlib keeps on `ax.xaxis`: ticks, locators, formatters, the label
//! and the inversion. On 3-D axes, an [`Axis3`] replaces the [`AxisName`].
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
//! use pltrs::prelude::*;
//! use pltrs::ticker::AxisName;
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
//! # What pltrs leaves out
//!
//! * Aliases. pltrs has no `lw`, `ls`, `c`, `ec`, `fc`, `mec`, `mfc`, `ms`
//!   and `aa`, and no 3-D aliases `plot3D`, `scatter3D`, `text3D` and
//!   `contour3D`. Each property has one name, the long one. Only the string
//!   keys of [`pyplot::rc`] accept the matplotlib short forms, because they
//!   are rc-file text.
//! * Introspection. `setp`, `getp` and `get_plot_commands` exist because
//!   Python objects are dynamic. Typed getters and setters replace them.
//!   `Artist.set(**kwargs)` becomes `update_props`.
//! * Change tracking. `pchanged`, `add_callback`, `Transform.invalidate` and
//!   frozen transforms have no equivalent. pltrs resolves transforms and
//!   computes the layout again at each draw, so you never invalidate
//!   anything. Handles share state, so `update_from` has nothing to copy.
//! * Back-pointers of tickers. Tickers do not point to their axis. A
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
//! [`FontProperties`]: crate::fonts::FontProperties
//! [`ScalarMappable`]: crate::colors::ScalarMappable
//! [`Triangulation`]: crate::tri::Triangulation
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
//! [`TexError::NoType1Font`]: crate::text::TexError::NoType1Font
//! [`TexError::NotInstalled`]: crate::text::TexError::NotInstalled
//! [`UnknownName`]: crate::rc::UnknownName
//! [`Axes3D`]: crate::mplot3d::Axes3D
//! [`Axis3D`]: crate::mplot3d::Axis3D
//! [`Axes`]: crate::axes::Axes
//! [`Axis3D::set_ticks`]: crate::mplot3d::Axis3D::set_ticks
//! [`Axes3D::autoscale_on`]: crate::mplot3d::Axes3D::autoscale_on
//! [`Aspect`]: crate::axes::Aspect
//! [`Axes::set_aspect`]: crate::axes::Axes::set_aspect
//! [`Axes::set_xticks`]: crate::axes::Axes::set_xticks
//! [`PolarAxes::set_rticks`]: crate::polar::PolarAxes::set_rticks
//! [`Extent`]: crate::image::Extent
//! [`LogLocator::with_subs`]: crate::ticker::LogLocator::with_subs
//! [`Axes::tick_params`]: crate::axes::Axes::tick_params
//! [`Axes::locator_params`]: crate::axes::Axes::locator_params
//! [`Figure::tight_layout`]: crate::figure::Figure::tight_layout
//! [`Figure::subplots_adjust`]: crate::figure::Figure::subplots_adjust
//! [`LegendValueFn`]: crate::collections::LegendValueFn
//! [`AxesLocatorFn`]: crate::axes::AxesLocatorFn
//! [`HistPlotOptions::label`]: crate::axes::HistPlotOptions::label
//! [`DatasetLabels`]: crate::axes::DatasetLabels
//! [`RotationMode`]: crate::text::RotationMode
//! [`LegendHandle`]: crate::legend::LegendHandle
//! [`LegendOptions::handle`]: crate::legend::LegendOptions::handle
