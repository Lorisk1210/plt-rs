//! pyplot and the object-oriented API: figures, windows, `show` and
//! `savefig`.
//!
//! pltrs has both of matplotlib's interfaces. The object-oriented one is
//! the primary API: you make a [`Figure`], add [`Axes`] and call their
//! methods. [`crate::pyplot`] is the stateful interface on top: each
//! thread keeps a list of numbered figures, a current figure and a current
//! axes, and free functions such as [`pyplot::plot`] act on them. Nearly
//! every `Axes` method has a pyplot function with the same arguments; the
//! few that differ are those whose matplotlib function takes other ones
//! (the axes of `twinx(ax)`, the optional mappable of `colorbar`), and
//! their documentation says so.
//!
//! Three spellings change because Rust has no keyword arguments or
//! optional positional arguments:
//!
//! * functions that are both getter and setter in matplotlib split in two:
//!   `plt.xlim(0, 1)` is [`pyplot::xlim`], `plt.xlim()` is
//!   [`pyplot::get_xlim`];
//! * functions whose argument changes meaning by type split by type:
//!   `plt.figure(3)` is [`pyplot::figure_num`], `plt.figure("name")` is
//!   [`pyplot::figure_label`], `plt.close("all")` is [`pyplot::close_all`];
//! * optional arguments become calls of their own, as on the axes:
//!   `plt.xticks(ticks, labels)` is [`pyplot::xticks`] then
//!   [`pyplot::xticklabels`], as `ax.set_xticks(ticks, labels)` is
//!   `set_xticks` then `set_xticklabels`.
//!
//! To read like Python, import the module as `plt`
//! (`use pltrs::pyplot as plt;`), as matplotlib users do; the crate's other
//! items keep their paths, as in `pltrs::legend::LegendOptions`.
//!
//! Figures do not need pyplot to be saved or shown: [`Figure::savefig`]
//! writes a file, [`crate::gui::show`] shows a list of figures with
//! pyplot's backend and its fallback, and [`crate::web::serve`] serves
//! them to a browser.
//!
//! # Equivalences
//!
//! | matplotlib                                   | pltrs                                         |
//! |----------------------------------------------|-----------------------------------------------|
//! | `plt.figure()`                               | [`pyplot::figure`]                            |
//! | `plt.figure(figsize=(w, h), dpi=d)`          | [`pyplot::figure_with_size`]`(w, h, d)`       |
//! | `plt.figure(3)`, `plt.figure("name")`        | [`pyplot::figure_num`], [`pyplot::figure_label`] |
//! | `plt.figure(fig)`                            | [`pyplot::figure_activate`]                   |
//! | `plt.subplots(r, c)`                         | [`pyplot::subplots`] → `(Figure, Grid<Axes>)` |
//! | `plt.subplots(r, c, sharex=True)`            | [`pyplot::subplots_shared`] with [`Share`]    |
//! | `plt.subplot(r, c, i)`, `plt.subplot2grid`   | [`pyplot::subplot`], [`pyplot::subplot2grid`] |
//! | `plt.subplot_mosaic(...)`                    | [`pyplot::subplot_mosaic`]                    |
//! | `plt.axes([l, b, w, h])`                     | [`pyplot::axes`]`(Some([l, b, w, h]))`        |
//! | `plt.gcf()`, `plt.gca()`, `plt.sca(ax)`      | [`pyplot::gcf`], [`pyplot::gca`], [`pyplot::sca`] |
//! | `plt.gci()`, `plt.sci(im)`                   | [`pyplot::gci`], [`pyplot::sci`]              |
//! | `plt.get_fignums()`, `plt.fignum_exists(n)`  | [`pyplot::get_fignums`], [`pyplot::fignum_exists`] |
//! | `plt.close()`, `plt.close(n)`, `plt.close(fig)` | [`pyplot::close`], [`pyplot::close_num`], [`pyplot::close_figure`] |
//! | `plt.close("name")`, `plt.close("all")`      | [`pyplot::close_label`], [`pyplot::close_all`] |
//! | `plt.clf()`, `plt.cla()`, `plt.delaxes(ax)`  | [`pyplot::clf`], [`pyplot::cla`], [`pyplot::delaxes`] |
//! | `plt.plot(y)`, `plt.plot(x, y)`              | [`pyplot::plot_y`], [`pyplot::plot`]          |
//! | `plt.plot(x, y, "r--")`, `plt.plot(x, y, lw=2)` | [`pyplot::plot_fmt`], [`pyplot::plot_with`] |
//! | `plt.xlim(a, b)`, `plt.xlim()`               | [`pyplot::xlim`], [`pyplot::get_xlim`]        |
//! | `plt.xticks(ticks, labels)`, `plt.xticks()`  | [`pyplot::xticks`] then [`pyplot::xticklabels`], [`pyplot::get_xticks`] |
//! | `plt.xticks(ticks, labels, minor=True)`      | [`pyplot::xticks_minor`] then [`pyplot::xticklabels_minor`] |
//! | `plt.margins(x)`, `plt.margins(y=m)`         | [`pyplot::margins`]`(x, None)`, `(None, m)`  |
//! | `plt.title(s)`, `plt.title(s, loc="left")`   | [`pyplot::title`], [`pyplot::title_at`]       |
//! | `plt.axis("off")`, `plt.axis("equal")`       | [`pyplot::axis`]`(AxisMode::Off)`, `AxisMode::Equal` ([`AxisMode`]) |
//! | `plt.box(False)`                             | [`pyplot::box`]`(Some(false))`, written `plt::r#box` |
//! | `plt.suptitle`, `plt.figtext`, `plt.figlegend` | [`pyplot::suptitle`], [`pyplot::figtext`], [`pyplot::figlegend`] |
//! | `plt.tight_layout()`, `plt.subplots_adjust(...)` | [`pyplot::tight_layout`], [`pyplot::subplots_adjust`] |
//! | `plt.colorbar()`, `plt.clim(a, b)`           | [`pyplot::colorbar`]`(None, ..)`, [`pyplot::clim`] |
//! | `plt.set_cmap("hot")`, `plt.hot()`           | [`pyplot::set_cmap`], [`pyplot::hot`]         |
//! | `plt.get_cmap("viridis")`                    | [`pyplot::get_cmap`]`(Some("viridis"), None)` |
//! | `plt.savefig(path, dpi=...)`                 | [`pyplot::savefig`], [`pyplot::savefig_with`] |
//! | `plt.show()`, `plt.draw()`                   | [`pyplot::show`], [`pyplot::draw`]            |
//! | `plt.ion()`, `plt.ioff()`, `plt.isinteractive()` | [`pyplot::ion`], [`pyplot::ioff`], [`pyplot::isinteractive`] |
//! | `plt.pause(t)`, `plt.ginput(n)`, `plt.waitforbuttonpress()` | [`pyplot::pause`], [`pyplot::ginput`], [`pyplot::waitforbuttonpress`] |
//! | `plt.connect(name, f)`, `plt.disconnect(cid)` | [`pyplot::connect`] with an [`EventKind`], [`pyplot::disconnect`] |
//! | `matplotlib.use("agg")`, `plt.switch_backend` | [`pyplot::switch_backend`]`(Backend::Agg)` ([`Backend`]) |
//! | `matplotlib.get_backend()`, `MPLBACKEND`     | [`pyplot::get_backend`]; `MPLBACKEND` works too |
//! | `plt.get_current_fig_manager()`              | [`pyplot::get_current_fig_manager`] → [`FigureManager`] |
//! | `fig.canvas.manager.resize(w, h)`            | [`FigureManager::resize`]`(w, h)?`; [`FigureWindow::resize`], [`Session::resize`] for windows of your own |
//! | `fig.canvas.manager.window.grab()` (Qt)      | [`FigureWindow::screenshot`]`(&fig, 1.0)?`, off screen with no display |
//! | `plt.rc("lines", lw=2)`                      | [`pyplot::rc`]`("lines", &[("lw", "2")])`     |
//! | `plt.rc_context({...})`, `plt.rcdefaults()`  | [`pyplot::rc_context`], [`pyplot::rcdefaults`] |
//! | `with plt.xkcd():`                           | [`pyplot::xkcd`]`(XkcdSketch::default(), \|\| ..)` |
//! | `plt.findobj(match=f)`                       | [`pyplot::findobj`]                           |
//! | `plt.subplot_tool()`                         | [`pyplot::subplot_tool`]                      |
//! | `fig.show()`                                 | [`crate::gui::show`]`(&[fig])`                |
//! | `fig.savefig(path)`                          | [`Figure::savefig`]                           |
//! | `plt.setp`, `plt.getp`, `plt.get_plot_commands`, `plt.set_loglevel` | not available: use the typed setters and getters |
//!
//! # A pyplot script
//!
//! ```python
//! import matplotlib.pyplot as plt
//!
//! plt.figure(figsize=(5, 3))
//! plt.plot([1, 2, 3], [1, 4, 9], "o-", label="squares")
//! plt.xlim(0, 4)
//! plt.title("Squares")
//! plt.legend()
//! assert plt.xlim() == (0, 4)
//! plt.savefig("squares.png")
//! plt.close()
//! ```
//!
//! ```
//! use pltrs::pyplot as plt;
//! use pltrs::legend::LegendOptions;
//! use pltrs::lines::LineProps;
//!
//! plt::figure_with_size(5.0, 3.0, 100.0);
//! plt::plot_with([1.0, 2.0, 3.0], [1.0, 4.0, 9.0], LineProps::fmt("o-")?.label("squares"))?;
//! plt::xlim(0.0, 4.0)?;
//! plt::title("Squares");
//! plt::legend(LegendOptions::default())?;
//! assert_eq!(plt::get_xlim(), (0.0, 4.0));
//!
//! // `plt::savefig("squares.png")` writes the file; here we keep the bytes.
//! let png = plt::gcf().to_png()?;
//! assert!(png.starts_with(b"\x89PNG"));
//! plt::close();
//! assert!(plt::get_fignums().is_empty());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Mixing the two interfaces
//!
//! pyplot hands out the same handles the object-oriented API uses, so the
//! two mix freely: plot on a returned axes, or make it current and go on
//! with pyplot.
//!
//! ```python
//! fig, axs = plt.subplots(1, 2, sharey=True)
//! axs[0].plot([0, 1], [0, 1])
//! plt.sca(axs[1])
//! plt.plot([0, 1], [1, 0])
//! assert plt.gcf() is fig
//! ```
//!
//! ```
//! use pltrs::pyplot as plt;
//! use pltrs::figure::Share;
//!
//! let (fig, axs) = plt::subplots_shared(1, 2, Share::None, Share::All)?;
//! axs[(0, 0)].plot([0.0, 1.0], [0.0, 1.0])?;
//! plt::sca(&axs[(0, 1)]);
//! plt::plot([0.0, 1.0], [1.0, 0.0])?;
//!
//! assert_eq!(plt::gcf(), fig);
//! assert_eq!(plt::gca(), axs[(0, 1)]);
//! assert_eq!(axs[(0, 1)].lines().len(), 1);
//! plt::close_all();
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Numbered and labelled figures
//!
//! ```python
//! plt.figure(1)
//! plt.figure("results")
//! assert plt.get_fignums() == [1, 2]
//! plt.close("results")
//! plt.close("all")
//! ```
//!
//! ```
//! use pltrs::pyplot as plt;
//!
//! let first = plt::figure_num(1);
//! plt::figure_label("results");
//! assert_eq!(plt::get_fignums(), [1, 2]);
//! assert_eq!(plt::figure_num(1), first, "an existing number is reused");
//! plt::close_label("results");
//! assert_eq!(plt::get_fignums(), [1]);
//! plt::close_all();
//! assert!(!plt::fignum_exists(1));
//! ```
//!
//! # Showing figures
//!
//! [`pyplot::show`] shows every open figure with the current backend: a
//! window each ([`Backend::Windows`], pltrs's own windows on X11), a web
//! page each ([`Backend::WebAgg`], also the fallback on systems without
//! X11), or nothing ([`Backend::Agg`], the fallback on a machine without
//! a display). Like matplotlib, it blocks until the windows are closed
//! unless interactive mode is on.
//!
//! ```python
//! import matplotlib
//! matplotlib.use("agg")
//! plt.plot([0, 1], [0, 1])
//! plt.show()                    # nothing to show under agg
//! ```
//!
//! ```
//! use pltrs::pyplot as plt;
//! use pltrs::pyplot::Backend;
//!
//! plt::switch_backend(Backend::Agg);
//! assert_eq!(plt::get_backend(), Backend::Agg);
//! plt::plot([0.0, 1.0], [0.0, 1.0])?;
//! plt::show()?;
//!
//! plt::ion();
//! assert!(plt::isinteractive());
//! plt::pause(0.0)?; // under agg, pause only sleeps
//! plt::ioff();
//! plt::close_all();
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Without pyplot, [`crate::gui::show`] takes the figures to show, in
//! windows or as [`pyplot::show`] falls back without them, and
//! [`crate::web::serve`] serves them to a browser:
//!
//! ```no_run
//! use pltrs::figure::Figure;
//! use pltrs::web::{WebOptions, serve};
//!
//! let fig = Figure::new();
//! fig.subplot().plot([0.0, 1.0, 2.0], [0.0, 1.0, 4.0])?;
//! pltrs::gui::show(&[fig.clone()])?; // returns when the window is closed, at once headless
//! serve(&[fig], WebOptions::from_rc()?)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Configuration through pyplot
//!
//! [`pyplot::rc`] takes string values, exactly as rc files spell them, and
//! keeps matplotlib's short group aliases (`lw`, `c`, `fc`, ...).
//!
//! ```python
//! with plt.rc_context({"lines.linewidth": 3}):
//!     plt.rc("axes", grid=True)
//!     line, = plt.plot([0, 1], [0, 1])
//! assert line.get_linewidth() == 3
//! ```
//!
//! ```
//! use pltrs::pyplot as plt;
//!
//! let line = plt::rc_context(&[("lines.linewidth", "3")], || {
//!     plt::rc("axes", &[("grid", "True")])?;
//!     Ok::<_, Box<dyn std::error::Error>>(plt::plot([0.0, 1.0], [0.0, 1.0])?)
//! })?;
//! assert_eq!(line.linewidth(), 3.0);
//! plt::close_all();
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`Figure`]: crate::figure::Figure
//! [`Figure::savefig`]: crate::figure::Figure::savefig
//! [`Axes`]: crate::axes::Axes
//! [`Share`]: crate::figure::Share
//! [`AxisMode`]: crate::axes::AxisMode
//! [`EventKind`]: crate::events::EventKind
//! [`Backend`]: crate::pyplot::Backend
//! [`Backend::Windows`]: crate::pyplot::Backend::Windows
//! [`Backend::WebAgg`]: crate::pyplot::Backend::WebAgg
//! [`Backend::Agg`]: crate::pyplot::Backend::Agg
//! [`FigureManager`]: crate::pyplot::FigureManager
//! [`FigureManager::resize`]: crate::pyplot::FigureManager::resize
//! [`FigureWindow::resize`]: crate::gui::FigureWindow::resize
//! [`FigureWindow::screenshot`]: crate::gui::FigureWindow::screenshot
//! [`Session::resize`]: crate::gui::Session::resize
//! [`pyplot::figure`]: crate::pyplot::figure
//! [`pyplot::figure_with_size`]: crate::pyplot::figure_with_size
//! [`pyplot::figure_num`]: crate::pyplot::figure_num
//! [`pyplot::figure_label`]: crate::pyplot::figure_label
//! [`pyplot::figure_activate`]: crate::pyplot::figure_activate
//! [`pyplot::subplots`]: crate::pyplot::subplots
//! [`pyplot::subplots_shared`]: crate::pyplot::subplots_shared
//! [`pyplot::subplot`]: crate::pyplot::subplot
//! [`pyplot::subplot2grid`]: crate::pyplot::subplot2grid
//! [`pyplot::subplot_mosaic`]: crate::pyplot::subplot_mosaic
//! [`pyplot::axes`]: crate::pyplot::axes
//! [`pyplot::gcf`]: crate::pyplot::gcf
//! [`pyplot::gca`]: crate::pyplot::gca
//! [`pyplot::sca`]: crate::pyplot::sca
//! [`pyplot::gci`]: crate::pyplot::gci
//! [`pyplot::sci`]: crate::pyplot::sci
//! [`pyplot::get_fignums`]: crate::pyplot::get_fignums
//! [`pyplot::fignum_exists`]: crate::pyplot::fignum_exists
//! [`pyplot::close`]: crate::pyplot::close
//! [`pyplot::close_num`]: crate::pyplot::close_num
//! [`pyplot::close_figure`]: crate::pyplot::close_figure
//! [`pyplot::close_label`]: crate::pyplot::close_label
//! [`pyplot::close_all`]: crate::pyplot::close_all
//! [`pyplot::clf`]: crate::pyplot::clf
//! [`pyplot::cla`]: crate::pyplot::cla
//! [`pyplot::delaxes`]: crate::pyplot::delaxes
//! [`pyplot::plot`]: crate::pyplot::plot
//! [`pyplot::plot_y`]: crate::pyplot::plot_y
//! [`pyplot::plot_fmt`]: crate::pyplot::plot_fmt
//! [`pyplot::plot_with`]: crate::pyplot::plot_with
//! [`pyplot::xlim`]: crate::pyplot::xlim
//! [`pyplot::get_xlim`]: crate::pyplot::get_xlim
//! [`pyplot::xticks`]: crate::pyplot::xticks
//! [`pyplot::xticklabels`]: crate::pyplot::xticklabels
//! [`pyplot::xticks_minor`]: crate::pyplot::xticks_minor
//! [`pyplot::xticklabels_minor`]: crate::pyplot::xticklabels_minor
//! [`pyplot::margins`]: crate::pyplot::margins
//! [`pyplot::get_xticks`]: crate::pyplot::get_xticks
//! [`pyplot::title`]: crate::pyplot::title
//! [`pyplot::title_at`]: crate::pyplot::title_at
//! [`pyplot::axis`]: crate::pyplot::axis
//! [`pyplot::box`]: crate::pyplot::box
//! [`pyplot::suptitle`]: crate::pyplot::suptitle
//! [`pyplot::figtext`]: crate::pyplot::figtext
//! [`pyplot::figlegend`]: crate::pyplot::figlegend
//! [`pyplot::tight_layout`]: crate::pyplot::tight_layout
//! [`pyplot::subplots_adjust`]: crate::pyplot::subplots_adjust
//! [`pyplot::colorbar`]: crate::pyplot::colorbar
//! [`pyplot::clim`]: crate::pyplot::clim
//! [`pyplot::set_cmap`]: crate::pyplot::set_cmap
//! [`pyplot::hot`]: crate::pyplot::hot
//! [`pyplot::get_cmap`]: crate::pyplot::get_cmap
//! [`pyplot::savefig`]: crate::pyplot::savefig
//! [`pyplot::savefig_with`]: crate::pyplot::savefig_with
//! [`pyplot::show`]: crate::pyplot::show
//! [`pyplot::draw`]: crate::pyplot::draw
//! [`pyplot::ion`]: crate::pyplot::ion
//! [`pyplot::ioff`]: crate::pyplot::ioff
//! [`pyplot::isinteractive`]: crate::pyplot::isinteractive
//! [`pyplot::pause`]: crate::pyplot::pause
//! [`pyplot::ginput`]: crate::pyplot::ginput
//! [`pyplot::waitforbuttonpress`]: crate::pyplot::waitforbuttonpress
//! [`pyplot::connect`]: crate::pyplot::connect
//! [`pyplot::disconnect`]: crate::pyplot::disconnect
//! [`pyplot::switch_backend`]: crate::pyplot::switch_backend
//! [`pyplot::get_backend`]: crate::pyplot::get_backend
//! [`pyplot::get_current_fig_manager`]: crate::pyplot::get_current_fig_manager
//! [`pyplot::rc`]: crate::pyplot::rc
//! [`pyplot::rc_context`]: crate::pyplot::rc_context
//! [`pyplot::rcdefaults`]: crate::pyplot::rcdefaults
//! [`pyplot::xkcd`]: crate::pyplot::xkcd
//! [`pyplot::findobj`]: crate::pyplot::findobj
//! [`pyplot::subplot_tool`]: crate::pyplot::subplot_tool
