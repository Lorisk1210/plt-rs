//! Porting from matplotlib: a map from what you know to what plt-rs offers.
//!
//! plt-rs implements matplotlib's model (figures, axes, artists, tickers,
//! transforms, the pyplot state machine) and draws the same pictures, but
//! it speaks Rust rather than Python. Keyword arguments become option
//! structs, strings become enums, getters lose their `get_` prefix, and
//! everything that can fail returns a `Result`. This guide is for people
//! who know matplotlib and want to find the plt-rs spelling of it.
//!
//! The guide is documentation only. Each topic below introduces how plt-rs
//! does things, gives an equivalence table of matplotlib calls and their
//! plt-rs counterparts, and shows side-by-side examples: the matplotlib
//! snippet in Python, then the plt-rs version as a doctest that runs and
//! checks its result.
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
//! use plt::figure::SaveOptions;
//! use plt::legend::LegendOptions;
//! use plt::prelude::*;
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
//! let legend = ax.legend(LegendOptions::default().loc("upper right"))?;
//! assert_eq!(legend.labels(), ["sin", "cos"]);
//!
//! // `fig.savefig("waves.png", dpi=150)` writes a file; `to_png_with`
//! // returns the bytes instead.
//! let options = SaveOptions { dpi: Some(150.0), ..SaveOptions::default() };
//! let png = fig.to_png_with(options)?;
//! assert!(png.starts_with(b"\x89PNG"));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! The differences in this small script are the ones you meet everywhere:
//! the figure and its axes are handles you call methods on, keyword
//! arguments live in an option struct ([`LineProps`]), the format string
//! is parsed and can fail ([`LineProps::fmt`]), and so can plotting and
//! saving. [`conventions`] explains each of these rules once.
//!
//! # Topics
//!
//! * [`conventions`]: naming, getters and setters, option structs, enums
//!   instead of strings, errors, handles, the per-axis API, and what was
//!   left out.
//! * [`pyplot`]: the stateful `plt.*` interface next to the
//!   object-oriented one; figures, windows, `show` and `savefig`.
//! * [`figures`]: figures, subplots, gridspecs, mosaics, subfigures and
//!   layout engines.
//! * [`plotting`]: the plotting methods, how their keywords map to
//!   options, and the data they take (slices, [`Grid`], dates,
//!   categories, masked values).
//! * [`artists`]: artist handles and their properties, for lines,
//!   patches and collections; z-order, clipping and picking.
//! * [`colors`]: colours, colormaps, norms, style sheets and rcParams.
//! * [`axes`]: limits, scales, ticks, locators, formatters, spines, twin
//!   and secondary axes, units and dates.
//! * [`text`]: text, fonts, mathtext and annotations.
//! * [`legends`]: legends, their options and custom handlers.
//! * [`images`]: images, contours, meshes and vector fields.
//! * [`saving`]: output formats and the options of `savefig`.
//! * [`interaction`]: events, widgets, navigation, toolbar tools,
//!   animation and blitting.
//! * [`toolkits`]: `mplot3d`, `axes_grid1`, `axisartist`, polar and
//!   geographic axes, and the other specialised modules.
//!
//! # Where matplotlib's modules went
//!
//! The crate is organised like matplotlib, so most imports translate
//! directly:
//!
//! | matplotlib                                   | plt-rs                          |
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
//! | `matplotlib.font_manager`                    | [`crate::fonts`]                |
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
//! | `matplotlib.style`                           | [`crate::style`]                |
//! | `matplotlib.backend_bases` (events)          | [`crate::events`]               |
//! | `NavigationToolbar2`                         | [`crate::navigation`]           |
//! | GUI backends, `webagg`                       | [`crate::gui`], [`crate::web`]  |
//! | `matplotlib.backends.backend_pdf.PdfPages`   | [`crate::figure::PdfPages`]     |
//! | `matplotlib.widgets`                         | [`crate::widgets`]              |
//! | `matplotlib.animation`                       | [`crate::animation`]            |
//! | `matplotlib.mlab`                            | [`crate::mlab`]                 |
//! | `matplotlib.cbook` statistics, `np.histogram` | [`crate::stats`]               |
//! | `np.linspace`, `np.meshgrid`, 2-D arrays     | [`crate::numeric`]              |
//! | `mpl_toolkits.mplot3d`                       | [`crate::mplot3d`]              |
//! | `mpl_toolkits.axes_grid1`                    | [`crate::axes_grid1`]           |
//! | `mpl_toolkits.axisartist`                    | [`crate::axisartist`]           |
//!
//! [`crate::prelude`] imports the names nearly every program uses
//! ([`Figure`], [`Axes`], [`LineProps`], [`linspace`], ...).
//!
//! # Looks
//!
//! plt-rs draws in its own modern look unless told otherwise (see
//! [`crate::look`]). Behaviour is the same in both looks; only the default
//! style differs. To compare a port with matplotlib's output pixel for
//! pixel, run it with `PLT_LOOK=matplotlib`. The examples of this guide
//! hold in either look.
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
