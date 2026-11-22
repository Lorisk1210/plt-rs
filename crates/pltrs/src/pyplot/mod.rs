//! The stateful interface of `matplotlib.pyplot`.
//!
//! Each thread has a list of numbered figures, a current figure and a
//! current axes in that figure. The functions of this module act on the
//! current figure ([`gcf`]) and the current axes ([`gca`]). Most methods of
//! [`Axes`] and many methods of [`Figure`] have a function here with the
//! same arguments and the same result. Use this module for short scripts.
//! Use [`Figure`] and [`Axes`] directly in programs that make more than one
//! figure.
//!
//! ```
//! use pltrs::pyplot as plt;
//! use pltrs::axes::ImshowOptions;
//! use pltrs::colorbar::ColorbarOptions;
//! use pltrs::numeric::{Grid, linspace};
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
//! plt::hot()?; // the colormap of the current image and of new images
//!
//! plt::suptitle("A pyplot script");
//! let png = plt::gcf().to_png()?;
//! assert!(!png.is_empty());
//! plt::close_all();
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Figures and axes
//!
//! | Task | Functions |
//! |------|-----------|
//! | Make a figure | [`figure`], [`figure_with_size`], [`figure_with_figsize`], [`figure_num`], [`figure_label`], [`figaspect`] |
//! | Get or set the current figure and axes | [`gcf`], [`gca`], [`sca`], [`figure_activate`] |
//! | List the figures | [`get_fignums`], [`get_figlabels`], [`fignum_exists`], [`figlabel_exists`] |
//! | Close figures | [`close`], [`close_num`], [`close_label`], [`close_figure`], [`close_all`] |
//! | Add axes | [`subplots`], [`subplots_shared`], [`subplot`], [`subplot2grid`], [`subplot_mosaic`], [`axes`] |
//! | Add axes with a projection | [`subplots_with_projection`], [`subplot_with_projection`], [`axes_with_projection`], [`polar`] |
//! | Add twin axes | [`twinx`], [`twiny`] |
//! | Clear or remove | [`cla`], [`clf`], [`delaxes`] |
//! | Find artists | [`findobj`] |
//! | Save the current figure | [`savefig`], [`savefig_with`] |
//!
//! # Plotting
//!
//! | Group | Functions |
//! |-------|-----------|
//! | Lines and markers | [`plot`], [`plot_fmt`], [`plot_with`], [`plot_y`], [`loglog`], [`semilogx`], [`semilogy`], [`scatter`], [`scatter_with`], [`step`], [`stairs`], [`stem`], [`errorbar`], [`eventplot`] |
//! | Reference lines and spans | [`hlines`], [`vlines`], [`axhline`], [`axvline`], [`axline`], [`axhspan`], [`axvspan`], [`arrow`] |
//! | Areas | [`fill`], [`fill_between`], [`fill_betweenx`], [`stackplot`] |
//! | Bars and histograms | [`bar`], [`barh`], [`bar_label`], [`grouped_bar`], [`broken_barh`], [`hist`], [`hist2d`], [`hexbin`] |
//! | Statistics | [`boxplot`], [`violinplot`], [`ecdf`], [`pie`], [`pie_label`] |
//! | Spectral analysis | [`psd`], [`csd`], [`cohere`], [`specgram`], [`magnitude_spectrum`], [`angle_spectrum`], [`phase_spectrum`], [`acorr`], [`xcorr`] |
//! | Images | [`imshow`], [`matshow`], [`pcolor`], [`pcolormesh`], [`spy`], [`figimage`], [`imread`], [`imsave`] |
//! | Contours and triangulations | [`contour`], [`contourf`], [`clabel`], [`tricontour`], [`tricontourf`], [`tripcolor`], [`triplot`] |
//! | Vector fields | [`quiver`], [`quiverkey`], [`barbs`], [`streamplot`] |
//!
//! # Colors of the current image
//!
//! A plotting function that maps values to colors makes its artist the
//! current image. These functions act on it:
//!
//! * [`gci`] and [`sci`] get and set the current image.
//! * [`clim`] sets its color limits.
//! * [`colorbar`] adds a colorbar for it.
//! * [`get_cmap`] and [`set_cmap`] get a colormap and set the colormap of
//!   the current image and of new images.
//! * [`viridis`], [`plasma`], [`gray`], [`hot`], [`jet`] and the other
//!   colormap functions call [`set_cmap`] with their name.
//!
//! # Labels, ticks and legends
//!
//! | Task | Functions |
//! |------|-----------|
//! | Titles and labels | [`title`], [`title_at`], [`xlabel`], [`ylabel`], [`suptitle`] |
//! | Limits and scales | [`xlim`], [`ylim`], [`get_xlim`], [`get_ylim`], [`xscale`], [`yscale`], [`margins`], [`autoscale`], [`axis`] |
//! | Ticks | [`xticks`], [`yticks`], [`xticklabels`], [`yticklabels`], [`xticks_minor`], [`yticks_minor`], [`get_xticks`], [`get_yticks`], [`tick_params`], [`ticklabel_format`], [`locator_params`], [`minorticks_on`], [`minorticks_off`] |
//! | Polar grids | [`rgrids`], [`rgrids_labeled`], [`thetagrids`], [`thetagrids_labeled`] |
//! | Grid and frame | [`grid`], [`grid_with`], [`box`](fn.box.html) |
//! | Text, legends and tables | [`text`], [`annotate`], [`legend`], [`table`], [`figtext`], [`figlegend`] |
//! | Figure layout | [`subplots_adjust`], [`tight_layout`] |
//!
//! # Configuration
//!
//! [`rc`], [`rc_context`], [`rc_file`], [`rc_file_context`] and
//! [`rcdefaults`] change the runtime settings. [`xkcd`] draws in the
//! hand-drawn xkcd style.
//!
//! # Windows and interaction
//!
//! [`show`] opens one window for each figure and returns when the user
//! closes them. In interactive mode, new figures open their windows at once
//! and [`show`] returns at once:
//!
//! * [`ion`], [`ioff`], [`isinteractive`] and [`interactive_context`] turn
//!   interactive mode on and off.
//! * [`draw`] and [`draw_if_interactive`] draw the current figure again.
//! * [`pause`] keeps the windows responsive for a time.
//! * [`ginput`] waits for clicks, and [`waitforbuttonpress`] waits for a
//!   click or a key.
//! * [`connect`] and [`disconnect`] add and remove event callbacks.
//! * [`subplot_tool`] opens the sliders that change the subplot layout.
//!
//! The backend selects how figures are shown. [`switch_backend`] and
//! [`get_backend`] set and get it. The `MPLBACKEND` environment variable
//! takes the matplotlib backend names. These are the backends:
//!
//! * [`Backend::Windows`] shows pltrs windows. If no window can open, it
//!   uses the web backend on macOS and Windows, and no backend on Linux.
//! * [`Backend::WebAgg`] shows figures in a web browser, as the matplotlib
//!   backend `webagg` does.
//! * [`Backend::Agg`] shows nothing.
//!
//! [`get_current_fig_manager`] returns the [`FigureManager`] of the current
//! figure: its number, its figure and its window title.
//!
//! These matplotlib functions have no pltrs function: `setp`, `getp`,
//! `get`, `get_plot_commands`, `new_figure_manager`, the display hooks of
//! the interactive shell and `set_loglevel`. Rust types and the window
//! session do their work.
//!
//! Functions that can fail return a [`PyplotError`] or the error of the
//! method that they call.
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
