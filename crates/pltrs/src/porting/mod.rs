//! Porting guide: the pltrs (Rust) equivalents of matplotlib calls.
//!
//! pltrs implements the matplotlib model: figures, axes, artists, tickers,
//! transforms and the pyplot state machine. It draws the same pictures
//! with a Rust API. This guide is for programmers who know matplotlib.
//! Each topic explains how pltrs works and gives a table of matplotlib
//! calls and their pltrs equivalents. Each example shows the matplotlib
//! code in Python, then the pltrs code as a doctest.
//!
//! # A first translation
//!
//! ```python
//! import numpy as np
//! import matplotlib.pyplot as plt
//!
//! x = np.linspace(0, 2 * np.pi, 100)
//! fig, ax = plt.subplots(figsize=(6.4, 4.8))
//! ax.plot(x, np.sin(x), color="tab:blue", linewidth=2, label="sin")
//! ax.plot(x, np.cos(x), "--", label="cos")
//! ax.set(xlabel="x", ylabel="y", title="Waves", xlim=(0, 2 * np.pi))
//! ax.legend(loc="upper right")
//! fig.savefig("waves.png", dpi=150)
//! ```
//!
//! ```
//! use std::f64::consts::TAU;
//!
//! use pltrs::figure::SaveOptions;
//! use pltrs::legend::{LegendLoc, LegendOptions};
//! use pltrs::prelude::*;
//!
//! let x = linspace(0.0, TAU, 100);
//! let sin: Vec<f64> = x.iter().map(|v| v.sin()).collect();
//! let cos: Vec<f64> = x.iter().map(|v| v.cos()).collect();
//!
//! let fig = Figure::with_size(6.4, 4.8, 100.0);
//! let ax = fig.subplot();
//! let props = LineProps::default().color("tab:blue").linewidth(2.0).label("sin");
//! ax.plot_with(&x, sin, props)?;
//! ax.plot_with(&x, cos, LineProps::fmt("--")?.label("cos"))?;
//! ax.set_xlabel("x").set_ylabel("y").set_xlim(0.0, TAU)?;
//! ax.set_title("Waves");
//! let legend = ax.legend(LegendOptions::default().loc("upper right".parse::<LegendLoc>()?))?;
//! assert_eq!(legend.labels(), ["sin", "cos"]);
//!
//! // `fig.savefig("waves.png", dpi=150)` writes a file. `to_png_with`
//! // returns the bytes.
//! let options = SaveOptions { dpi: Some(150.0), ..SaveOptions::default() };
//! let png = fig.to_png_with(options)?;
//! assert!(png.starts_with(b"\x89PNG"));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! This script shows the main differences:
//!
//! * The figure and its axes are handles with methods.
//! * Keyword arguments go into an option struct ([`LineProps`]).
//! * The format string ([`LineProps::fmt`]), plotting and saving can fail
//!   and return a `Result`.
//!
//! [`conventions`] explains these rules.
//!
//! # Topics
//!
//! * [`conventions`]: names, getters and setters, option structs, enums,
//!   errors, handles, the per-axis API, and what pltrs leaves out.
//! * [`pyplot`]: the stateful `pltrs.*` interface and the object-oriented
//!   interface. Figures, windows, `show` and `savefig`.
//! * [`figures`]: figures, subplots, gridspecs, mosaics, subfigures and
//!   layout engines.
//! * [`plotting`]: the plotting methods, their options, and the data they
//!   take (slices, [`Grid`], dates, categories, masked values).
//! * [`artists`]: artist handles and their properties for lines, patches
//!   and collections. Z-order, clipping and picking.
//! * [`colors`]: colors, colormaps, norms, style sheets and rcParams.
//! * [`configuration`]: the configuration directory, `matplotlibrc`, the
//!   user style library, and `matplotlib.testing` to test your figures.
//! * [`axes`]: limits, scales, ticks, locators, formatters, spines, twin
//!   and secondary axes, units and dates.
//! * [`text`]: text, fonts, mathtext, text that LaTeX typesets (`usetex`)
//!   and annotations.
//! * [`legends`]: legends, their options and custom handlers.
//! * [`images`]: images, contours, meshes and vector fields.
//! * [`saving`]: output formats and the options of `savefig`.
//! * [`interaction`]: events, widgets, navigation, toolbar tools,
//!   animation and blitting.
//! * [`toolkits`]: `mplot3d`, `axes_grid1`, `axisartist`, polar and
//!   geographic axes, and the other special modules.
//!
//! # Where the matplotlib modules are
//!
//! The modules of pltrs follow the modules of matplotlib. Most imports
//! translate directly:
//!
//! | matplotlib                                   | pltrs                           |
//! |----------------------------------------------|---------------------------------|
//! | `matplotlib.pyplot`                          | [`crate::pyplot`]               |
//! | `matplotlib.figure`                          | [`crate::figure`]               |
//! | `matplotlib.axes`, `matplotlib.axis`         | [`crate::axes`]                 |
//! | `matplotlib.gridspec`                        | [`crate::gridspec`]             |
//! | `matplotlib.layout_engine`                   | [`crate::layout`]               |
//! | `matplotlib.artist`                          | [`crate::artist`]               |
//! | `matplotlib.lines`, `matplotlib.markers`     | [`crate::lines`], [`crate::markers`] |
//! | `matplotlib.patches`                         | [`crate::patches`]              |
//! | `matplotlib.collections`                     | [`crate::collections`]          |
//! | `matplotlib.text`                            | [`crate::text`]                 |
//! | `matplotlib.texmanager`, `matplotlib.dviread` | [`crate::text`] ([`check_usetex`](crate::text::check_usetex), [`TexEngine`](crate::text::TexEngine)), and the TeX code in `pltrs_text::tex` |
//! | `matplotlib.font_manager`                    | [`crate::fonts`]                |
//! | `matplotlib.ft2font`                         | the `pltrs_font` crate (`Face`, `Size`, `FontChain`) |
//! | `matplotlib.colors`, `matplotlib.cm`         | [`crate::colors`]               |
//! | `matplotlib.colorbar`                        | [`crate::colorbar`]             |
//! | `matplotlib.ticker`                          | [`crate::ticker`]               |
//! | `matplotlib.scale`                           | [`crate::scale`]                |
//! | `matplotlib.dates`                           | [`crate::dates`]                |
//! | `matplotlib.units`, `matplotlib.category`    | [`crate::units`]                |
//! | `matplotlib.transforms`                      | [`crate::transforms`]           |
//! | `matplotlib.path`                            | [`crate::path`]                 |
//! | `matplotlib.spines`                          | [`crate::spines`]               |
//! | `matplotlib.image`                           | [`crate::image`]                |
//! | `matplotlib.contour`                         | [`crate::contour`]              |
//! | `matplotlib.tri`                             | [`crate::tri`]                  |
//! | `matplotlib.quiver`                          | [`crate::quiver`]               |
//! | `matplotlib.streamplot`                      | [`crate::streamplot`]           |
//! | `matplotlib.legend`, `matplotlib.legend_handler` | [`crate::legend`]           |
//! | `matplotlib.offsetbox`                       | [`crate::offsetbox`]            |
//! | `matplotlib.container`                       | [`crate::container`]            |
//! | `matplotlib.patheffects`                     | [`crate::patheffects`]          |
//! | `matplotlib.table`                           | [`crate::table`]                |
//! | `matplotlib.sankey`                          | [`crate::sankey`]               |
//! | `matplotlib.projections.polar`               | [`crate::polar`]                |
//! | `matplotlib.projections.geo`                 | [`crate::geo`]                  |
//! | `matplotlib.rcParams`, `matplotlib.rcsetup`  | [`crate::rc`]                   |
//! | `get_configdir`, `matplotlib_fname`          | [`crate::config`]               |
//! | `matplotlib.style`                           | [`crate::style`]                |
//! | `matplotlib.testing`                         | [`crate::testing`]              |
//! | `matplotlib.backend_bases` (events)          | [`crate::events`]               |
//! | `NavigationToolbar2`                         | [`crate::navigation`]           |
//! | `matplotlib.backend_tools`, `backend_managers` | [`crate::gui`] ([`ToolSpec`](crate::gui::ToolSpec), the tool methods of [`Figure`]) |
//! | GUI backends, `webagg`                       | [`crate::gui`], [`crate::web`]  |
//! | `matplotlib.backends.backend_pdf.PdfPages`   | [`crate::figure::PdfPages`]     |
//! | `matplotlib.backends.backend_pgf.PdfPages`   | [`crate::figure::PgfPages`]     |
//! | the renderers of `matplotlib.backends`, `PdfFile` | the `pltrs_render` crate ([`Renderer`](pltrs_render::Renderer) and its backends) |
//! | `matplotlib.widgets`                         | [`crate::widgets`]              |
//! | `matplotlib.animation`                       | [`crate::animation`]            |
//! | `matplotlib.mlab`                            | [`crate::mlab`]                 |
//! | `matplotlib.cbook` statistics, `np.histogram` | [`crate::stats`]               |
//! | `np.linspace`, `np.meshgrid`, 2-D arrays     | [`crate::numeric`]              |
//! | `mpl_toolkits.mplot3d`                       | [`crate::mplot3d`]              |
//! | `mpl_toolkits.axes_grid1`                    | [`crate::axes_grid1`]           |
//! | `mpl_toolkits.axisartist`                    | [`crate::axisartist`]           |
//!
//! [`crate::prelude`] imports the names that most programs use
//! ([`Figure`], [`Axes`], [`LineProps`], [`linspace`], ...).
//!
//! # The full matplotlib API
//!
//! The tables in this guide show the features that programs use most. The
//! file `spec/api-audit/coverage.md` of the repository covers all of the
//! public API of matplotlib 3.11, about 6,800 items. For each item, it
//! gives the pltrs item, the difference in pltrs, or the reason why the
//! item does not apply to Rust. The script `spec/api-audit/run` writes the
//! file again.
//!
//! # Looks
//!
//! pltrs draws in the modern look by default (see [`crate::look`]). The two
//! looks differ only in the default style. To compare a port with the
//! matplotlib output pixel for pixel, set `PLTRS_LOOK=matplotlib`. The
//! examples in this guide pass in both looks.
//!
//! [`LineProps`]: crate::lines::LineProps
//! [`LineProps::fmt`]: crate::lines::LineProps::fmt
//! [`Grid`]: crate::numeric::Grid
//! [`Figure`]: crate::figure::Figure
//! [`Axes`]: crate::axes::Axes
//! [`linspace`]: crate::numeric::linspace

pub mod artists;
pub mod axes;
pub mod colors;
pub mod configuration;
pub mod conventions;
pub mod figures;
pub mod images;
pub mod interaction;
pub mod legends;
pub mod plotting;
pub mod pyplot;
pub mod saving;
pub mod text;
pub mod toolkits;
