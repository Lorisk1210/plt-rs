//! Legends, their options and custom handlers.
//!
//! [`Axes::legend`] and [`Figure::legend`] take a [`LegendOptions`]: one
//! builder method per keyword of matplotlib's `legend()`, unset ones read
//! from the `legend.*` rcParams. Locations keep matplotlib's spellings
//! (`"upper left"`, `"best"`, a code, `"outside upper center"` for figure
//! legends); an unknown one is reported when the legend is made
//! ([`LegendError::InvalidLocation`]).
//!
//! Without explicit handles the legend lists the labelled artists and
//! containers, skipping labels that start with an underscore, as
//! matplotlib does. Explicit handles are anything convertible into a
//! [`LegendHandle`]: an artist handle (`&line`), a proxy drawn only in
//! the legend ([`LegendHandle::line`], [`LegendHandle::patch`]), a tuple
//! of handles ([`LegendHandle::tuple`]) or any object with a handler.
//!
//! Handlers are values implementing [`LegendHandler`]. The built-in ones
//! carry matplotlib's names ([`HandlerPatch`], [`HandlerTuple`],
//! [`HandlerLine2D`], ...); a handler is attached to a kind of artist with
//! [`LegendOptions::handler`] and a [`HandlerKey`], or to one handle with
//! [`LegendHandle::with_handler`].
//!
//! # Equivalences
//!
//! | matplotlib                                         | plt-rs                                        |
//! |----------------------------------------------------|-----------------------------------------------|
//! | `ax.legend()`                                      | [`Axes::legend`]`(LegendOptions::default())`  |
//! | `fig.legend(...)`, `plt.figlegend(...)`            | [`Figure::legend`], [`crate::pyplot::figlegend`] |
//! | `loc="upper left"`, `loc=2`, `loc=(0.1, 0.2)`      | [`LegendOptions::loc`] (a name, a code or a point) |
//! | `bbox_to_anchor=(1.02, 1)`, `bbox_transform=`      | [`LegendOptions::bbox_to_anchor`], [`LegendOptions::bbox_transform`] |
//! | `ncols=`, `mode="expand"`, `reverse=True`          | [`LegendOptions::ncols`], [`LegendOptions::mode`]`(LegendMode::Expand)`, [`LegendOptions::reverse`] |
//! | `title=`, `title_fontsize=`, `alignment="left"`    | [`LegendOptions::title`], [`LegendOptions::title_fontsize`], [`LegendOptions::alignment`] |
//! | `fontsize="small"`, `prop=fp`, `labelcolor="linecolor"` | [`LegendOptions::fontsize`], [`LegendOptions::prop`], [`LegendOptions::labelcolor`] |
//! | `frameon=`, `fancybox=`, `shadow=`, `framealpha=`  | [`LegendOptions::frameon`], [`LegendOptions::fancybox`], [`LegendOptions::shadow`], [`LegendOptions::framealpha`] |
//! | `facecolor=`, `edgecolor=`                         | [`LegendOptions::facecolor`], [`LegendOptions::edgecolor`] |
//! | `numpoints=`, `scatterpoints=`, `markerscale=`, `markerfirst=` | [`LegendOptions::numpoints`], [`LegendOptions::scatterpoints`], [`LegendOptions::markerscale`], [`LegendOptions::markerfirst`] |
//! | `borderpad=`, `labelspacing=`, `handlelength=`, `columnspacing=`, ... | the builder method of the same name |
//! | `draggable=True`, `leg.set_draggable(True)`        | [`LegendOptions::draggable`], [`Legend::set_draggable`] |
//! | `handles=[...]`, `labels=[...]`                    | [`LegendOptions::handles`], [`LegendOptions::labels`] |
//! | `ax.get_legend_handles_labels()`                   | [`Axes::legend_handles_labels`]               |
//! | `ax.get_legend()`                                  | [`Axes::get_legend`]                          |
//! | `[t.get_text() for t in leg.get_texts()]`          | [`Legend::labels`]; [`Legend::texts`] for the text handles |
//! | `leg.set_title(s)`, `leg.get_frame()`              | [`Legend::set_title`], [`Legend::frame`]      |
//! | `ax.add_artist(first_legend)`                      | [`Axes::keep_legend`]                         |
//! | `label="_nolegend_"`                               | any label starting with `_`                   |
//! | `mpatches.Patch(color=, label=)` as a proxy        | [`LegendHandle::patch`]`(PatchProps::default().color(..).label(..))` |
//! | `mlines.Line2D([], [], marker=, label=)` as a proxy | [`LegendHandle::line`]`(LineProps::default()..)` |
//! | `handles=[(line, dots)]`, `HandlerTuple(ndivide=None)` | [`LegendHandle::tuple`] with [`HandlerTuple`] |
//! | `handler_map={Line2D: HandlerLine2D(numpoints=1)}` | [`LegendOptions::handler`]`(HandlerKey::Line, ..)` |
//! | `handler_map={line: HandlerLine2D(numpoints=4)}`   | [`LegendHandle::with_handler`], [`LegendHandle::with_numpoints`] |
//! | `class MyHandler(HandlerBase): def legend_artist(...)` | `impl` [`LegendHandler`] (see its example)  |
//! | `Legend.update_default_handler_map(...)`           | [`Legend::update_default_handler_map`]        |
//! | `sc.legend_elements(prop="sizes")`                 | [`Collection::legend_elements`]               |
//!
//! # Placing and styling a legend
//!
//! ```python
//! ax.plot(x, np.sin(x), label="sin")
//! ax.plot(x, np.cos(x), label="_nolegend_")
//! leg = ax.legend(loc="upper left", bbox_to_anchor=(1.02, 1), title="Curves",
//!                 frameon=False, fontsize="small")
//! assert [t.get_text() for t in leg.get_texts()] == ["sin"]
//! assert ax.get_legend() is leg
//! ax.legend(loc="top middle")   # ValueError
//! ```
//!
//! ```
//! use plt::fonts::RelativeSize;
//! use plt::legend::{LegendError, LegendOptions};
//! use plt::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! let x = linspace(0.0, 6.0, 30);
//! ax.plot_with(&x, x.iter().map(|v| v.sin()).collect::<Vec<_>>(), LineProps::default().label("sin"))?;
//! ax.plot_with(&x, x.iter().map(|v| v.cos()).collect::<Vec<_>>(), LineProps::default().label("_nolegend_"))?;
//! let options = LegendOptions::default()
//!     .loc("upper left")
//!     .bbox_to_anchor((1.02, 1.0))
//!     .title("Curves")
//!     .frameon(false)
//!     .fontsize(RelativeSize::Small);
//! let legend = ax.legend(options)?;
//! assert_eq!(legend.labels(), ["sin"]);
//! assert_eq!(ax.get_legend(), Some(legend));
//!
//! let error = ax.legend(LegendOptions::default().loc("top middle")).unwrap_err();
//! assert!(matches!(error, LegendError::InvalidLocation(_)));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Explicit handles and proxy artists
//!
//! ```python
//! line, = ax.plot([0, 1], [0, 1], label="data")
//! handles, labels = ax.get_legend_handles_labels()
//! red = mpatches.Patch(color="red", label="red area")
//! stars = mlines.Line2D([], [], color="b", marker="*", linestyle="None", label="stars")
//! ax.legend(handles=[line, red, stars])
//! ```
//!
//! ```
//! use plt::legend::{LegendHandle, LegendOptions};
//! use plt::markers::MarkerSymbol;
//! use plt::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! let line = ax.plot_with([0.0, 1.0], [0.0, 1.0], LineProps::default().label("data"))?;
//! let (handles, labels) = ax.legend_handles_labels();
//! assert_eq!((handles.len(), labels), (1, vec!["data".to_owned()]));
//!
//! let red = LegendHandle::patch(PatchProps::default().color("red").label("red area"));
//! let stars = LegendHandle::line(
//!     LineProps::default()
//!         .color("b")
//!         .marker(MarkerSymbol::Star)
//!         .linestyle(LineStyle::None)
//!         .label("stars"),
//! );
//! let legend = ax.legend(LegendOptions::default().handles([LegendHandle::from(&line), red, stars]))?;
//! assert_eq!(legend.labels(), ["data", "red area", "stars"]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Several legends, and figure legends
//!
//! ```python
//! first = ax.legend(handles=[a], loc="upper left")
//! ax.add_artist(first)
//! second = ax.legend(handles=[b], loc="lower right")
//! fig.legend(handles=[a, b], loc="outside upper center", ncols=2)
//! ```
//!
//! ```
//! use plt::legend::LegendOptions;
//! use plt::prelude::*;
//!
//! let fig = Figure::new();
//! let ax = fig.subplot();
//! let a = ax.plot_with([0.0, 1.0], [0.0, 1.0], LineProps::default().label("a"))?;
//! let b = ax.plot_with([0.0, 1.0], [1.0, 0.0], LineProps::default().label("b"))?;
//! let first = ax.legend(LegendOptions::default().handles([&a]).loc("upper left"))?;
//! ax.keep_legend(&first);
//! let second = ax.legend(LegendOptions::default().handles([&b]).loc("lower right"))?;
//! assert_eq!(ax.get_legend(), Some(second));
//!
//! let options = LegendOptions::default().handles([&a, &b]).loc("outside upper center").ncols(2);
//! fig.legend(options)?;
//! assert_eq!(fig.legends().len(), 1);
//! fig.to_png()?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Handlers
//!
//! ```python
//! from matplotlib.legend_handler import HandlerLine2D, HandlerTuple
//! line, = ax.plot([0, 1], [0, 1], marker="o")
//! dots = ax.scatter([0.5], [0.5])
//! ax.legend([(line, dots), line], ["line and dot", "four markers"],
//!           handler_map={tuple: HandlerTuple(ndivide=None), line: HandlerLine2D(numpoints=4)})
//! ```
//!
//! ```
//! use plt::legend::{HandlerTuple, LegendHandle, LegendOptions};
//! use plt::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! let line = ax.plot_fmt([0.0, 1.0], [0.0, 1.0], "o-")?;
//! let dots = ax.scatter(vec![0.5], vec![0.5])?;
//! let side_by_side = HandlerTuple { ndivide: None, ..HandlerTuple::default() };
//! let both = LegendHandle::tuple([LegendHandle::from(&line), LegendHandle::from(&dots)], side_by_side);
//! let four = LegendHandle::from(&line).with_numpoints(4);
//! let legend = ax.legend(
//!     LegendOptions::default()
//!         .handles([both, four])
//!         .labels(["line and dot", "four markers"]),
//! )?;
//! assert_eq!(legend.labels(), ["line and dot", "four markers"]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! A handler of your own implements [`LegendHandler::create_artists`];
//! the trait's documentation ports matplotlib's `HandlerEllipse`.
//!
//! [`Axes::legend`]: crate::axes::Axes::legend
//! [`Axes::get_legend`]: crate::axes::Axes::get_legend
//! [`Axes::keep_legend`]: crate::axes::Axes::keep_legend
//! [`Axes::legend_handles_labels`]: crate::axes::Axes::legend_handles_labels
//! [`Figure::legend`]: crate::figure::Figure::legend
//! [`Collection::legend_elements`]: crate::collections::Collection::legend_elements
//! [`Legend::labels`]: crate::legend::Legend::labels
//! [`Legend::texts`]: crate::legend::Legend::texts
//! [`Legend::set_title`]: crate::legend::Legend::set_title
//! [`Legend::frame`]: crate::legend::Legend::frame
//! [`Legend::set_draggable`]: crate::legend::Legend::set_draggable
//! [`Legend::update_default_handler_map`]: crate::legend::Legend::update_default_handler_map
//! [`LegendError::InvalidLocation`]: crate::legend::LegendError::InvalidLocation
//! [`LegendHandle`]: crate::legend::LegendHandle
//! [`LegendHandle::line`]: crate::legend::LegendHandle::line
//! [`LegendHandle::patch`]: crate::legend::LegendHandle::patch
//! [`LegendHandle::tuple`]: crate::legend::LegendHandle::tuple
//! [`LegendHandle::with_handler`]: crate::legend::LegendHandle::with_handler
//! [`LegendHandle::with_numpoints`]: crate::legend::LegendHandle::with_numpoints
//! [`LegendHandler`]: crate::legend::LegendHandler
//! [`LegendHandler::create_artists`]: crate::legend::LegendHandler::create_artists
//! [`HandlerKey`]: crate::legend::HandlerKey
//! [`HandlerPatch`]: crate::legend::HandlerPatch
//! [`HandlerTuple`]: crate::legend::HandlerTuple
//! [`HandlerLine2D`]: crate::legend::HandlerLine2D
//! [`LegendOptions`]: crate::legend::LegendOptions
//! [`LegendOptions::loc`]: crate::legend::LegendOptions::loc
//! [`LegendOptions::bbox_to_anchor`]: crate::legend::LegendOptions::bbox_to_anchor
//! [`LegendOptions::bbox_transform`]: crate::legend::LegendOptions::bbox_transform
//! [`LegendOptions::ncols`]: crate::legend::LegendOptions::ncols
//! [`LegendOptions::mode`]: crate::legend::LegendOptions::mode
//! [`LegendOptions::reverse`]: crate::legend::LegendOptions::reverse
//! [`LegendOptions::title`]: crate::legend::LegendOptions::title
//! [`LegendOptions::title_fontsize`]: crate::legend::LegendOptions::title_fontsize
//! [`LegendOptions::alignment`]: crate::legend::LegendOptions::alignment
//! [`LegendOptions::fontsize`]: crate::legend::LegendOptions::fontsize
//! [`LegendOptions::prop`]: crate::legend::LegendOptions::prop
//! [`LegendOptions::labelcolor`]: crate::legend::LegendOptions::labelcolor
//! [`LegendOptions::frameon`]: crate::legend::LegendOptions::frameon
//! [`LegendOptions::fancybox`]: crate::legend::LegendOptions::fancybox
//! [`LegendOptions::shadow`]: crate::legend::LegendOptions::shadow
//! [`LegendOptions::framealpha`]: crate::legend::LegendOptions::framealpha
//! [`LegendOptions::facecolor`]: crate::legend::LegendOptions::facecolor
//! [`LegendOptions::edgecolor`]: crate::legend::LegendOptions::edgecolor
//! [`LegendOptions::numpoints`]: crate::legend::LegendOptions::numpoints
//! [`LegendOptions::scatterpoints`]: crate::legend::LegendOptions::scatterpoints
//! [`LegendOptions::markerscale`]: crate::legend::LegendOptions::markerscale
//! [`LegendOptions::markerfirst`]: crate::legend::LegendOptions::markerfirst
//! [`LegendOptions::draggable`]: crate::legend::LegendOptions::draggable
//! [`LegendOptions::handles`]: crate::legend::LegendOptions::handles
//! [`LegendOptions::labels`]: crate::legend::LegendOptions::labels
//! [`LegendOptions::handler`]: crate::legend::LegendOptions::handler
