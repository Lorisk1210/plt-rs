//! A stateful interface in the style of `matplotlib.pyplot`: each thread
//! has a list of numbered figures, a current figure and, in it, a current
//! axes; the functions here act on them.
//!
//! ```
//! use plt::pyplot as plt;
//! use ::plt::axes::ImshowOptions;
//! use ::plt::colorbar::ColorbarOptions;
//! use ::plt::numeric::{Grid, linspace};
//!
//! let x = linspace(0.0, 6.0, 50);
//! let y: Vec<f64> = x.iter().map(|v| v.sin()).collect();
//! plt::subplot(1, 2, 1)?;
//! plt::plot(x, y)?;
//! plt::title("A sine");
//! plt::xlabel("x");
//! plt::grid(true);
//!
//! plt::subplot(1, 2, 2)?;
//! let ramp = Grid::from_fn(8, 8, |row, col| (row * col) as f64);
//! plt::imshow(ramp, ImshowOptions::default())?;
//! plt::colorbar(None, ColorbarOptions::default())?;
//! plt::hot()?; // the colormap of the current image and of new ones
//!
//! plt::suptitle("A pyplot script");
//! let png = plt::gcf().to_png()?;
//! assert!(!png.is_empty());
//! plt::close_all();
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Like matplotlib's `pyplot`, nearly every method of [`Axes`] and many of
//! [`Figure`] have a function here acting on the current axes ([`gca`]) or
//! figure ([`gcf`]), with the method's arguments and result:
//!
//! * figures and axes — [`figure`], [`subplots`], [`subplot`],
//!   [`subplot2grid`], [`subplot_mosaic`], [`axes`], [`twinx`], [`close`],
//!   [`findobj`], [`savefig`], ...;
//! * lines, markers, fills and reference lines — [`plot`], [`scatter`],
//!   [`errorbar`], [`fill_between`], [`axhline`], [`polar`], ...;
//! * bars and statistics — [`bar`], [`hist`], [`boxplot`],
//!   [`violinplot`], [`pie`], ...; spectral analysis — [`psd`],
//!   [`specgram`], [`xcorr`], ...;
//! * images, fields and contours — [`imshow`], [`pcolormesh`],
//!   [`contourf`], [`quiver`], [`streamplot`], [`tripcolor`], [`imread`],
//!   [`imsave`], ...;
//! * the current image — [`gci`], [`sci`], [`colorbar`], [`clim`],
//!   [`set_cmap`] and the colormap shortcuts ([`viridis`], [`gray`], ...):
//!   the plotting functions that colour-map make their artist current;
//! * decorations — [`title`], [`xlabel`], [`xlim`], [`xticks`],
//!   [`legend`], [`annotate`], [`suptitle`], [`tight_layout`], ...;
//! * configuration — [`rc`], [`rc_context`], [`rc_file`], [`rc_file_context`],
//!   [`rcdefaults`], [`xkcd`].
//!
//! [`show`] opens a window per figure and waits until they are closed. In
//! interactive mode ([`ion`], or for a while [`interactive_context`]) new
//! figures open their windows at once and
//! [`show`] returns immediately; [`pause`] keeps the windows responsive
//! for a while, and [`ginput`] and [`waitforbuttonpress`] wait for clicks
//! and keys in them; [`subplot_tool`] opens the sliders adjusting the
//! subplot layout of a figure.
//!
//! The backend decides how figures are shown ([`switch_backend`],
//! [`get_backend`], or the `MPLBACKEND` environment variable with
//! matplotlib's backend names): plt-rs's windows ([`Backend::Windows`]);
//! a web browser ([`Backend::WebAgg`], matplotlib's `webagg`, which the
//! windows fall back to without a window system); or nothing
//! ([`Backend::Agg`]).
//!
//! [`get_current_fig_manager`] returns the [`FigureManager`] of the current
//! figure: its number, its figure and its window title.
//! matplotlib's introspection helpers (`setp`, `getp`, `get`,
//! `get_plot_commands`) and the rest of its backend machinery
//! (`new_figure_manager`, the REPL display hooks, `set_loglevel`) have no
//! counterpart: Rust's types and the window session take their place.
//!
//! [`Axes`]: crate::axes::Axes
//! [`Figure`]: crate::figure::Figure

mod backend;
mod config;
mod decorations;
mod error;
mod fields;
mod figsize;
mod host;
mod images;
mod interactive;
mod layout;
mod lines;
mod manager;
mod mappables;
mod registry;
mod spectral;
mod stats;
mod subplots;

pub use backend::*;
pub use config::*;
pub use decorations::*;
pub use error::PyplotError;
pub use fields::*;
pub use figsize::*;
pub(crate) use host::show_until_closed;
pub use images::*;
pub use interactive::*;
pub use layout::*;
pub use lines::*;
pub use manager::*;
pub use mappables::*;
pub use registry::*;
pub use spectral::*;
pub use stats::*;
pub use subplots::*;
