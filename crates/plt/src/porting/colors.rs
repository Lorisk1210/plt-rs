//! Colours, colormaps, norms, style sheets and rcParams.
//!
//! **Colours.** A [`Color`] is anything matplotlib accepts as a colour: a
//! name (`"tab:blue"`, `"xkcd:sky blue"`), a hex code, a grey level
//! (`"0.5"`), a cycle reference (`"C2"`), `"none"`, or RGB(A) tuples and
//! arrays. Strings convert infallibly, so every colour argument takes a
//! plain `&str`; the colour is checked when it is resolved to an RGBA
//! value ([`Color::resolve`]) or up front with [`Color::parse`]. Resolved
//! colours are [`Rgba`] values; a few figure-level settings
//! ([`Figure::set_facecolor`], [`SaveOptions::facecolor`]) take one
//! directly.
//!
//! **Colormaps and norms.** [`Colormap`] is one type for matplotlib's
//! `ListedColormap` and `LinearSegmentedColormap`, built with
//! [`Colormap::listed`], [`Colormap::from_list`] or
//! [`Colormap::segmented`], and looked up by name in [`colormaps`]. Norms
//! implement the [`Norm`] trait; they map `f64` data to [`Normalized`]
//! positions, which distinguish fractions, direct indices
//! ([`BoundaryNorm`]) and masked values. Plotting options take a
//! colormap value, not a name: `cmap="plasma"` is
//! `colormaps::get("plasma")?`.
//!
//! **rcParams.** [`RcParams`] has one typed field per parameter
//! (`lines.linewidth` is `lines_linewidth`), and also matplotlib's
//! string-keyed access with matplotlib's validation ([`rc::set`],
//! [`RcParams::get`]). The current parameters belong to the thread, like
//! pyplot's state. Style sheets apply on top ([`style::use_style`],
//! [`style::context`]).
//!
//! **Looks.** plt-rs starts threads from its modern look; [`rc::reset`]
//! restores matplotlib's defaults and [`rc::restore_startup`] the look's.
//! See [`crate::look`] to choose the look for a whole program.
//!
//! # Equivalences
//!
//! | matplotlib                                       | plt-rs                                         |
//! |--------------------------------------------------|------------------------------------------------|
//! | `"tab:blue"`, `"#ff8800"`, `"0.5"`, `"C2"`       | `"tab:blue"` etc.: any `Into<`[`Color`]`>`     |
//! | `(0.1, 0.2, 0.3)`, `(r, g, b, a)`, `"none"`      | `(0.1, 0.2, 0.3).into()`, `[r, g, b, a].into()`, [`Color::none`] |
//! | `mcolors.to_rgba(c, alpha)`                      | [`to_rgba`]`(spec, alpha)`, [`Color::resolve_alpha`] |
//! | `mcolors.to_hex(c, keep_alpha)`                  | [`to_hex`]                                     |
//! | `is_color_like`, `same_color`, `to_rgba_array`   | [`is_color_like`], [`same_color`], [`to_rgba_array`] |
//! | `rgb_to_hsv`, `hsv_to_rgb`                       | [`rgb_to_hsv`], [`hsv_to_rgb`]                 |
//! | `mcolors.CSS4_COLORS`, `TABLEAU_COLORS`, `XKCD_COLORS` | [`ColorTable`], [`named_colors`]         |
//! | `mpl.color_sequences["tab10"]`                   | [`color_sequence`]`("tab10")`                  |
//! | `mpl.colormaps["viridis"]`, `plt.get_cmap(name)` | [`colormaps::get`]                             |
//! | `mpl.colormaps.register(cmap)`, `list(mpl.colormaps)` | [`colormaps::register`], [`colormaps::names`] |
//! | `cmap(0.5)`, `cmap(0.5, bytes=True)`             | [`Colormap::sample`], [`Colormap::lookup_bytes`] |
//! | `cmap(norm(values))`                             | [`Colormap::map`] of [`Norm::normalize`]       |
//! | `cmap.reversed()`, `cmap.resampled(n)`           | [`Colormap::reversed`], [`Colormap::resampled`] |
//! | `cmap.with_extremes(bad=, under=, over=)`        | [`Colormap::with_bad`], [`Colormap::with_under`], [`Colormap::with_over`] |
//! | `ListedColormap(colors)`                         | [`Colormap::listed`]                           |
//! | `LinearSegmentedColormap.from_list(name, colors, N)` | [`Colormap::from_list`]                    |
//! | `LinearSegmentedColormap(name, segmentdata)`     | [`Colormap::segmented`]                        |
//! | `Normalize(vmin, vmax, clip=True)`               | [`Normalize::new`]`(vmin, vmax).with_clip(true)` |
//! | `LogNorm`, `SymLogNorm`, `AsinhNorm`, `FuncNorm` | [`LogNorm`], [`SymLogNorm`], [`AsinhNorm`], [`FuncNorm`] |
//! | `PowerNorm`, `TwoSlopeNorm`, `CenteredNorm`      | [`PowerNorm`], [`TwoSlopeNorm`], [`CenteredNorm`] |
//! | `BoundaryNorm`, `NoNorm`                         | [`BoundaryNorm`], [`NoNorm`]                   |
//! | `norm(values)`, `norm.inverse(x)`                | [`Norm::normalize`], [`Norm::inverse`]         |
//! | `ScalarMappable(norm, cmap)`                     | [`ScalarMappable::with_norm`]                  |
//! | `from_levels_and_colors(levels, colors)`         | [`from_levels_and_colors`]                     |
//! | `mpl.rcParams["lines.linewidth"] = 2`            | [`rc::set`]`("lines.linewidth", "2")?` or [`rc::update`]`(\|p\| p.lines_linewidth = 2.0)` |
//! | `mpl.rcParams["lines.linewidth"]`                | `rc::params().lines_linewidth`, [`RcParams::get`] |
//! | `with mpl.rc_context({...}):`                    | [`rc::context`]`(\|\| ..)`                     |
//! | `mpl.rcdefaults()`, `mpl.rc_file_defaults()`     | [`rc::reset`], [`rc::restore_startup`]         |
//! | `mpl.rcParamsDefault`                            | [`RcParams::matplotlib`]                       |
//! | `mpl.rc_file(path)`                              | [`RcParams::apply_rc_text`] and [`rc::replace`] |
//! | `plt.style.use(name)`, `plt.style.context(name)` | [`style::use_style`], [`style::context`]       |
//! | `plt.style.available`                            | [`style::available`]                           |
//! | `ax.set_prop_cycle(color=[...])`                 | [`Axes::set_prop_cycle`]`(Some(Cycler::colors_of(..)))` |
//! | `cycler(color=a) + cycler(linestyle=b)`          | [`Cycler::new`] and [`Cycler::zip`]            |
//! | `cycler(...) * cycler(...)`                      | [`Cycler::product`]                            |
//!
//! # Colours
//!
//! ```python
//! from matplotlib import colors as mcolors
//! assert mcolors.to_hex("tab:orange") == "#ff7f0e"
//! assert mcolors.to_rgba("red", alpha=0.5) == (1.0, 0.0, 0.0, 0.5)
//! assert not mcolors.is_color_like("tab:nope")
//! assert mcolors.same_color("r", "red")
//! tab10 = mpl.color_sequences["tab10"]
//! ```
//!
//! ```
//! use plt::colors::{Color, color_sequence, is_color_like, same_color, to_hex, to_rgba};
//!
//! let orange = Color::from("tab:orange").resolve()?;
//! assert_eq!(to_hex(orange, false)?, "#ff7f0e");
//! let red = to_rgba("red", Some(0.5))?;
//! assert_eq!((red.r, red.g, red.b, red.a), (1.0, 0.0, 0.0, 0.5));
//! assert!(!is_color_like("tab:nope"));
//! assert!(same_color("r", "red")?);
//! let tab10 = color_sequence("tab10").ok_or("a bundled sequence")?;
//! assert_eq!(tab10.len(), 10);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Colormaps and norms
//!
//! ```python
//! from matplotlib.colors import LinearSegmentedColormap, ListedColormap, LogNorm, Normalize
//! norm = Normalize(vmin=0, vmax=10)
//! assert list(norm([0, 5, 10])) == [0.0, 0.5, 1.0]
//! assert LogNorm(1, 100)(10) == 0.5
//! viridis = mpl.colormaps["viridis"]
//! assert viridis.reversed().name == "viridis_r"
//! assert viridis.resampled(5).N == 5
//! viridis.with_extremes(over="red")
//! ListedColormap(["red", "green", "blue"])
//! LinearSegmentedColormap.from_list("red_blue", ["red", "blue"], N=256)
//! ```
//!
//! ```
//! use plt::colors::{Color, Colormap, LogNorm, Norm, Normalize, Normalized, colormaps};
//!
//! let mut norm = Normalize::new(0.0, 10.0);
//! assert_eq!(
//!     norm.normalize(&[0.0, 5.0, 10.0])?,
//!     [Normalized::Fraction(0.0), Normalized::Fraction(0.5), Normalized::Fraction(1.0)]
//! );
//! assert_eq!(LogNorm::new(1.0, 100.0).normalize(&[10.0])?, [Normalized::Fraction(0.5)]);
//!
//! let viridis = colormaps::get("viridis")?;
//! assert_eq!(viridis.reversed().name(), "viridis_r");
//! assert_eq!(viridis.resampled(5)?.n(), 5);
//! let marked = viridis.with_over("red")?;
//! assert_eq!(marked.over(), Color::from("red").resolve()?);
//!
//! let listed = Colormap::listed("rgb", vec!["red", "green", "blue"])?;
//! assert_eq!(listed.n(), 3);
//! let ramp = Colormap::from_list("red_blue", vec!["red", "blue"], 256, 1.0)?;
//! assert_eq!(ramp.sample(0.0), Color::from("red").resolve()?);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # rcParams
//!
//! ```python
//! with mpl.rc_context():
//!     mpl.rcParams["lines.linewidth"] = 2
//!     mpl.rcParams["axes.grid"] = True
//!     assert mpl.rcParams["lines.linewidth"] == 2
//! mpl.rcParams["lines.linewidth"] = "thick"   # ValueError
//! ```
//!
//! ```
//! use plt::rc;
//!
//! rc::context(|| {
//!     rc::set("lines.linewidth", "2")?;
//!     rc::update(|params| params.axes_grid = true);
//!     assert_eq!(rc::params().lines_linewidth, 2.0);
//!     assert!(rc::params().axes_grid);
//!     assert_eq!(rc::params().get("lines.linewidth").as_deref(), Some("2.0"));
//!     Ok::<(), rc::RcError>(())
//! })?;
//! assert!(rc::set("lines.linewidth", "thick").is_err());
//! # Ok::<(), rc::RcError>(())
//! ```
//!
//! # Style sheets
//!
//! ```python
//! with plt.style.context("ggplot"):
//!     assert mpl.rcParams["axes.grid"]
//! plt.style.use("dark_background")
//! ```
//!
//! ```
//! use plt::{rc, style};
//!
//! rc::context(|| {
//!     style::context("ggplot", || assert!(rc::params().axes_grid))?;
//!     style::use_style("dark_background")?;
//!     assert!(style::use_style("no such style").is_err());
//!     Ok::<(), rc::RcError>(())
//! })?;
//! # Ok::<(), rc::RcError>(())
//! ```
//!
//! # Property cycles
//!
//! ```python
//! from cycler import cycler
//! ax.set_prop_cycle(color=["r", "g", "b"])
//! first, = ax.plot([0, 1], [0, 1])
//! assert first.get_color() == "r"
//! styles = cycler(color=["k", "0.5"]) + cycler(linestyle=["-", "--"])
//! ```
//!
//! ```
//! use plt::colors::Color;
//! use plt::prelude::*;
//! use plt::rc::{CycleValue, Cycler};
//!
//! let ax = Figure::new().subplot();
//! ax.set_prop_cycle(Some(Cycler::colors_of(["r", "g", "b"].map(Color::from))));
//! let first = ax.plot([0.0, 1.0], [0.0, 1.0])?;
//! assert_eq!(first.rgba()?, Color::from("r").resolve()?);
//!
//! let dashes = [LineStyle::Solid, LineStyle::Dashed].map(CycleValue::LineStyle).to_vec();
//! let styles = Cycler::colors_of(["k", "0.5"].map(Color::from)).zip(Cycler::new("linestyle", dashes))?;
//! assert_eq!(styles.len(), 2);
//! assert_eq!(styles.keys(), ["color", "linestyle"]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`Color`]: crate::colors::Color
//! [`Color::none`]: crate::colors::Color::none
//! [`Color::resolve`]: crate::colors::Color::resolve
//! [`Color::resolve_alpha`]: crate::colors::Color::resolve_alpha
//! [`Color::parse`]: crate::colors::Color::parse
//! [`Rgba`]: crate::colors::Rgba
//! [`Figure::set_facecolor`]: crate::figure::Figure::set_facecolor
//! [`SaveOptions::facecolor`]: crate::figure::SaveOptions::facecolor
//! [`to_rgba`]: crate::colors::to_rgba
//! [`to_hex`]: crate::colors::to_hex
//! [`is_color_like`]: crate::colors::is_color_like
//! [`same_color`]: crate::colors::same_color
//! [`to_rgba_array`]: crate::colors::to_rgba_array
//! [`rgb_to_hsv`]: crate::colors::rgb_to_hsv
//! [`hsv_to_rgb`]: crate::colors::hsv_to_rgb
//! [`ColorTable`]: crate::colors::ColorTable
//! [`named_colors`]: crate::colors::named_colors
//! [`color_sequence`]: crate::colors::color_sequence
//! [`colormaps`]: crate::colors::colormaps
//! [`colormaps::get`]: crate::colors::colormaps::get
//! [`colormaps::register`]: crate::colors::colormaps::register
//! [`colormaps::names`]: crate::colors::colormaps::names
//! [`Colormap`]: crate::colors::Colormap
//! [`Colormap::sample`]: crate::colors::Colormap::sample
//! [`Colormap::lookup_bytes`]: crate::colors::Colormap::lookup_bytes
//! [`Colormap::map`]: crate::colors::Colormap::map
//! [`Colormap::reversed`]: crate::colors::Colormap::reversed
//! [`Colormap::resampled`]: crate::colors::Colormap::resampled
//! [`Colormap::with_bad`]: crate::colors::Colormap::with_bad
//! [`Colormap::with_under`]: crate::colors::Colormap::with_under
//! [`Colormap::with_over`]: crate::colors::Colormap::with_over
//! [`Colormap::listed`]: crate::colors::Colormap::listed
//! [`Colormap::from_list`]: crate::colors::Colormap::from_list
//! [`Colormap::segmented`]: crate::colors::Colormap::segmented
//! [`Norm`]: crate::colors::Norm
//! [`Norm::normalize`]: crate::colors::Norm::normalize
//! [`Norm::inverse`]: crate::colors::Norm::inverse
//! [`Normalized`]: crate::colors::Normalized
//! [`Normalize::new`]: crate::colors::Normalize::new
//! [`LogNorm`]: crate::colors::LogNorm
//! [`SymLogNorm`]: crate::colors::SymLogNorm
//! [`AsinhNorm`]: crate::colors::AsinhNorm
//! [`FuncNorm`]: crate::colors::FuncNorm
//! [`PowerNorm`]: crate::colors::PowerNorm
//! [`TwoSlopeNorm`]: crate::colors::TwoSlopeNorm
//! [`CenteredNorm`]: crate::colors::CenteredNorm
//! [`BoundaryNorm`]: crate::colors::BoundaryNorm
//! [`NoNorm`]: crate::colors::NoNorm
//! [`ScalarMappable::with_norm`]: crate::colors::ScalarMappable::with_norm
//! [`from_levels_and_colors`]: crate::colors::from_levels_and_colors
//! [`RcParams`]: crate::rc::RcParams
//! [`RcParams::get`]: crate::rc::RcParams::get
//! [`RcParams::matplotlib`]: crate::rc::RcParams::matplotlib
//! [`RcParams::apply_rc_text`]: crate::rc::RcParams::apply_rc_text
//! [`rc::set`]: crate::rc::set
//! [`rc::update`]: crate::rc::update
//! [`rc::context`]: crate::rc::context
//! [`rc::reset`]: crate::rc::reset
//! [`rc::restore_startup`]: crate::rc::restore_startup
//! [`rc::replace`]: crate::rc::replace
//! [`style::use_style`]: crate::style::use_style
//! [`style::context`]: crate::style::context
//! [`style::available`]: crate::style::available
//! [`Axes::set_prop_cycle`]: crate::axes::Axes::set_prop_cycle
//! [`Cycler::new`]: crate::rc::Cycler::new
//! [`Cycler::zip`]: crate::rc::Cycler::zip
//! [`Cycler::product`]: crate::rc::Cycler::product
