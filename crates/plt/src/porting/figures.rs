//! Figures, subplots, gridspecs, mosaics, subfigures and layout engines.
//!
//! A [`Figure`] is made directly ([`Figure::new`], [`Figure::with_size`])
//! or through pyplot. Axes are added to it the way matplotlib adds them,
//! but the overloaded `add_subplot` splits into one method per form:
//! [`Figure::subplot`] for the single default subplot,
//! [`Figure::add_subplot`] for `(nrows, ncols, index)`,
//! [`Figure::add_subplot_spec`] for a cell of a grid, and one family per
//! projection ([`Figure::polar_subplot`], [`Figure::subplot_3d`],
//! [`Figure::geo_subplot`]) instead of `projection="..."`.
//!
//! [`Figure::subplots`] returns a [`Grid`] of axes, indexed by
//! `(row, col)` where matplotlib indexes a numpy array with `[row, col]`;
//! [`Grid::as_slice`] plays the part of `axs.flat`. Sharing is a [`Share`]
//! value per axis.
//!
//! A [`GridSpec`] is a plain value. Build it with its ratios and margins,
//! take blocks of cells with [`GridSpec::cell`] and [`GridSpec::span`]
//! (Rust ranges instead of slices), and add axes on them. Mosaics are
//! parsed from matplotlib's strings ([`Mosaic::parse`]); subfigures are
//! `Figure` handles of their own.
//!
//! Layout engines are values of [`LayoutEngine`]. The modern look turns
//! constrained layout on by default; matplotlib's look starts without an
//! engine, as matplotlib does.
//!
//! # Equivalences
//!
//! | matplotlib                                      | plt-rs                                        |
//! |-------------------------------------------------|-----------------------------------------------|
//! | `Figure()`                                      | [`Figure::new`]                               |
//! | `Figure(figsize=(w, h), dpi=d)`                 | [`Figure::with_size`]`(w, h, d)`              |
//! | `fig.get_size_inches()`, `fig.set_size_inches(w, h)` | [`Figure::size_inches`], [`Figure::set_size_inches`] |
//! | `fig.dpi`, `fig.set_dpi(d)`                     | [`Figure::dpi`], [`Figure::set_dpi`]          |
//! | `fig.set_facecolor(c)`, `fig.set_edgecolor(c)`  | [`Figure::set_facecolor`], [`Figure::set_edgecolor`] (resolved [`Rgba`]) |
//! | `fig.add_subplot()`                             | [`Figure::subplot`]                           |
//! | `fig.add_subplot(r, c, i)`, `fig.add_subplot(rci)` | [`Figure::add_subplot`]`(r, c, i)?`        |
//! | `fig.add_subplot(gs[0, :])`                     | [`Figure::add_subplot_spec`]                  |
//! | `fig.add_subplot(projection="polar")`           | [`Figure::polar_subplot`], [`Figure::add_polar_subplot`] |
//! | `fig.add_subplot(projection="3d")`              | [`Figure::subplot_3d`], [`Figure::add_subplot_3d`] |
//! | `fig.add_subplot(projection="mollweide")`       | [`Figure::geo_subplot`]`(GeoProjection::Mollweide)` |
//! | `fig.add_axes([l, b, w, h])`                    | [`Figure::add_axes`]                          |
//! | `fig.subplots(r, c)`                            | [`Figure::subplots`] → [`Grid`]`<Axes>`       |
//! | `fig.subplots(r, c, sharex=True, sharey="row")` | [`Figure::subplots_shared`]`(r, c, Share::All, Share::Row)` |
//! | `fig.subplots(..., gridspec_kw={...})`          | [`GridSpec::with_ratios`] and [`Figure::subplots_on`] |
//! | `axs[i, j]`, `axs.flat`, `axs.shape`            | `axs[(i, j)]`, [`Grid::as_slice`], [`Grid::shape`] |
//! | `fig.add_gridspec(r, c)`                        | [`Figure::add_gridspec`]                      |
//! | `GridSpec(r, c, width_ratios=..., left=...)`    | [`GridSpec::with_ratios`] with [`SubplotOverrides`] |
//! | `gs[i, j]`, `gs[a:b, c:d]`                      | [`GridSpec::cell`]`(i, j)?`, [`GridSpec::span`]`(a..b, c..d)?` |
//! | `gs.subgridspec(r, c)`, `GridSpecFromSubplotSpec` | [`GridSpec::nested`]                        |
//! | `gs.update(left=...)`                           | [`GridSpec::update`]                          |
//! | `ax.get_subplotspec()`, `ss.get_gridspec()`     | [`Axes::subplot_spec`], [`SubplotSpec::gridspec`] |
//! | `fig.subplot_mosaic("AB;CC", ...)`              | [`Figure::subplot_mosaic`] with [`Mosaic::parse`] and [`MosaicOptions`] |
//! | `axd["A"]`                                      | [`MosaicAxes::get`]`("A")`                    |
//! | `fig.subfigures(r, c, wspace=..., width_ratios=...)` | [`Figure::subfigures`] with [`SubfigureOptions`] |
//! | `fig.add_subfigure(gs[...])`                    | [`Figure::add_subfigure`]                     |
//! | `fig.axes`, `fig.gca()`, `fig.sca(ax)`          | [`Figure::axes`], [`Figure::gca`], [`Figure::sca`] |
//! | `fig.delaxes(ax)`, `fig.clear()`                | [`Figure::delaxes`], [`Figure::clear`]        |
//! | `fig.suptitle`, `fig.supxlabel`, `fig.supylabel` | [`Figure::suptitle`], [`Figure::supxlabel`], [`Figure::supylabel`] |
//! | `fig.text(x, y, s)`                             | [`Figure::text`]                              |
//! | `layout="constrained"`, `"compressed"`, `"tight"` | [`LayoutEngine::constrained`], [`LayoutEngine::compressed`], [`LayoutEngine::tight`] |
//! | `fig.set_layout_engine(...)`, `fig.get_layout_engine()` | [`Figure::set_layout_engine`], [`Figure::layout_engine`] |
//! | `ConstrainedLayoutEngine(w_pad=..., ...)`       | [`LayoutEngine::Constrained`]`(`[`ConstrainedLayout`]`)` |
//! | `fig.tight_layout(pad=...)`                     | [`Figure::tight_layout`]`(&TightLayout { .. })` ([`TightLayout`]) |
//! | `fig.subplots_adjust(left=..., wspace=...)`     | [`Figure::subplots_adjust`]`(&SubplotOverrides { .. })` |
//! | `fig.subplotpars`                               | [`Figure::subplot_params`]                    |
//! | `fig.align_labels()`, `fig.align_titles()`      | [`Figure::align_labels`]`(None)`, [`Figure::align_titles`]`(None)` |
//! | `fig.autofmt_xdate()`                           | [`Figure::autofmt_xdate`]`(0.2, 30.0, HorizontalAlignment::Right)` |
//! | `fig.draw_without_rendering()`                  | [`Figure::layout`]                            |
//! | `ax.get_position()`, `ax.set_position(rect)`    | [`Axes::position`], [`Axes::set_position`]    |
//! | `ax.label_outer()`                              | [`Axes::label_outer`]                         |
//!
//! # Figures
//!
//! ```python
//! fig = plt.figure(figsize=(8, 3), dpi=120, facecolor="0.9")
//! fig.set_size_inches(6, 4)
//! assert tuple(fig.get_size_inches()) == (6, 4)
//! assert fig.dpi == 120
//! ```
//!
//! ```
//! use plt::colors::Color;
//! use plt::figure::Figure;
//!
//! let fig = Figure::with_size(8.0, 3.0, 120.0);
//! fig.set_facecolor(Color::from("0.9").resolve()?);
//! fig.set_size_inches(6.0, 4.0);
//! assert_eq!(fig.size_inches(), (6.0, 4.0));
//! assert_eq!(fig.dpi(), 120.0);
//! assert_eq!(fig.width_height(), (720, 480));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Grids of subplots
//!
//! ```python
//! fig, axs = plt.subplots(2, 3, sharex=True, sharey="row")
//! axs[1, 2].plot([0, 1], [0, 1])
//! for ax in axs.flat:
//!     ax.grid(True)
//! assert axs.shape == (2, 3)
//! ```
//!
//! ```
//! use plt::figure::{Figure, Share};
//! use plt::ticker::AxisName;
//!
//! let fig = Figure::new();
//! let axs = fig.subplots_shared(2, 3, Share::All, Share::Row)?;
//! axs[(1, 2)].plot([0.0, 1.0], [0.0, 1.0])?;
//! for ax in axs.as_slice() {
//!     ax.grid(true);
//! }
//! assert_eq!(axs.shape(), (2, 3));
//! assert!(axs[(0, 0)].shared_axes(AxisName::X).contains(&axs[(1, 2)]));
//! assert!(!axs[(0, 0)].shared_axes(AxisName::Y).contains(&axs[(1, 0)]));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Gridspecs
//!
//! ```python
//! gs = fig.add_gridspec(2, 3, width_ratios=[2, 1, 1])
//! big = fig.add_subplot(gs[:, 0])
//! top = fig.add_subplot(gs[0, 1:])
//! inner = gs[1, 1:].subgridspec(1, 2)
//! fig.add_subplot(inner[0, 0])
//! fig.add_subplot(inner[0, 1])
//! ```
//!
//! ```
//! use plt::figure::Figure;
//! use plt::gridspec::{GridSpec, SubplotOverrides};
//!
//! let fig = Figure::new();
//! let ratios = Some(vec![2.0, 1.0, 1.0]);
//! let gs = GridSpec::with_ratios(2, 3, ratios, None, SubplotOverrides::default())?;
//! let big = fig.add_subplot_spec(&gs.span(0..2, 0..1)?)?;
//! let top = fig.add_subplot_spec(&gs.span(0..1, 1..3)?)?;
//! let inner = GridSpec::nested(&gs.span(1..2, 1..3)?, 1, 2, None, None, (None, None))?;
//! fig.add_subplot_spec(&inner.cell(0, 0)?)?;
//! fig.add_subplot_spec(&inner.cell(0, 1)?)?;
//!
//! assert_eq!(fig.axes().len(), 4);
//! let spec = big.subplot_spec().ok_or("placed on a grid")?;
//! assert_eq!((spec.rows(), spec.cols()), (0..2, 0..1));
//! assert!(top.subplot_spec().ok_or("placed on a grid")?.is_first_row());
//! assert!(gs.cell(2, 0).is_err(), "out of range");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Mosaics
//!
//! ```python
//! axd = fig.subplot_mosaic("AAB;C.B", width_ratios=[1, 1, 2], sharex=True)
//! axd["A"].set_title("A")
//! assert sorted(axd) == ["A", "B", "C"]
//! ```
//!
//! ```
//! use plt::figure::{Figure, Mosaic, MosaicOptions};
//!
//! let fig = Figure::new();
//! let options = MosaicOptions {
//!     width_ratios: Some(vec![1.0, 1.0, 2.0]),
//!     sharex: true,
//!     ..MosaicOptions::default()
//! };
//! let axd = fig.subplot_mosaic(&Mosaic::parse("AAB;C.B")?, &options)?;
//! axd.get("A").ok_or("label A")?.set_title("A");
//! let labels: Vec<&str> = axd.iter().map(|(label, _)| label).collect();
//! assert_eq!(labels, ["A", "B", "C"]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Subfigures
//!
//! ```python
//! left, right = fig.subfigures(1, 2, width_ratios=[2, 1])
//! left.subplots(2, 1)
//! right.subplots()
//! right.suptitle("Right")
//! ```
//!
//! ```
//! use plt::figure::{Figure, SubfigureOptions};
//!
//! let fig = Figure::new();
//! let options = SubfigureOptions { width_ratios: Some(vec![2.0, 1.0]), ..SubfigureOptions::default() };
//! let subfigs = fig.subfigures(1, 2, &options)?;
//! let (left, right) = (&subfigs[(0, 0)], &subfigs[(0, 1)]);
//! left.subplots(2, 1)?;
//! right.subplot();
//! right.suptitle("Right");
//! assert_eq!(left.axes().len(), 2);
//! assert_eq!(right.suptitle_text(), "Right");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Layout engines and margins
//!
//! Set the engine explicitly when a port depends on it, because the
//! default depends on the look.
//!
//! ```python
//! fig = plt.figure(layout="constrained")
//! assert isinstance(fig.get_layout_engine(), ConstrainedLayoutEngine)
//! fig.set_layout_engine(None)
//! fig.subplots_adjust(left=0.2, wspace=0.4)
//! fig.tight_layout(pad=0.5)
//! fig.align_labels()
//! ```
//!
//! ```
//! use plt::figure::Figure;
//! use plt::gridspec::SubplotOverrides;
//! use plt::layout::{LayoutEngine, TightLayout};
//!
//! let fig = Figure::new();
//! fig.set_layout_engine(LayoutEngine::constrained());
//! assert!(matches!(fig.layout_engine(), Some(LayoutEngine::Constrained(_))));
//!
//! fig.set_layout_engine(None);
//! fig.subplots(2, 2)?;
//! let margins = SubplotOverrides { left: Some(0.2), wspace: Some(0.4), ..SubplotOverrides::default() };
//! fig.subplots_adjust(&margins)?;
//! assert_eq!(fig.subplot_params().left, 0.2);
//!
//! fig.tight_layout(&TightLayout { pad: 0.5, ..TightLayout::default() })?;
//! fig.align_labels(None);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`Figure`]: crate::figure::Figure
//! [`Figure::new`]: crate::figure::Figure::new
//! [`Figure::with_size`]: crate::figure::Figure::with_size
//! [`Figure::size_inches`]: crate::figure::Figure::size_inches
//! [`Figure::set_size_inches`]: crate::figure::Figure::set_size_inches
//! [`Figure::dpi`]: crate::figure::Figure::dpi
//! [`Figure::set_dpi`]: crate::figure::Figure::set_dpi
//! [`Figure::set_facecolor`]: crate::figure::Figure::set_facecolor
//! [`Figure::set_edgecolor`]: crate::figure::Figure::set_edgecolor
//! [`Figure::subplot`]: crate::figure::Figure::subplot
//! [`Figure::add_subplot`]: crate::figure::Figure::add_subplot
//! [`Figure::add_subplot_spec`]: crate::figure::Figure::add_subplot_spec
//! [`Figure::polar_subplot`]: crate::figure::Figure::polar_subplot
//! [`Figure::add_polar_subplot`]: crate::figure::Figure::add_polar_subplot
//! [`Figure::subplot_3d`]: crate::figure::Figure::subplot_3d
//! [`Figure::add_subplot_3d`]: crate::figure::Figure::add_subplot_3d
//! [`Figure::geo_subplot`]: crate::figure::Figure::geo_subplot
//! [`Figure::add_axes`]: crate::figure::Figure::add_axes
//! [`Figure::subplots`]: crate::figure::Figure::subplots
//! [`Figure::subplots_shared`]: crate::figure::Figure::subplots_shared
//! [`Figure::subplots_on`]: crate::figure::Figure::subplots_on
//! [`Figure::add_gridspec`]: crate::figure::Figure::add_gridspec
//! [`Figure::subplot_mosaic`]: crate::figure::Figure::subplot_mosaic
//! [`Figure::subfigures`]: crate::figure::Figure::subfigures
//! [`Figure::add_subfigure`]: crate::figure::Figure::add_subfigure
//! [`Figure::axes`]: crate::figure::Figure::axes
//! [`Figure::gca`]: crate::figure::Figure::gca
//! [`Figure::sca`]: crate::figure::Figure::sca
//! [`Figure::delaxes`]: crate::figure::Figure::delaxes
//! [`Figure::clear`]: crate::figure::Figure::clear
//! [`Figure::suptitle`]: crate::figure::Figure::suptitle
//! [`Figure::supxlabel`]: crate::figure::Figure::supxlabel
//! [`Figure::supylabel`]: crate::figure::Figure::supylabel
//! [`Figure::text`]: crate::figure::Figure::text
//! [`Figure::set_layout_engine`]: crate::figure::Figure::set_layout_engine
//! [`Figure::layout_engine`]: crate::figure::Figure::layout_engine
//! [`Figure::tight_layout`]: crate::figure::Figure::tight_layout
//! [`Figure::subplots_adjust`]: crate::figure::Figure::subplots_adjust
//! [`Figure::subplot_params`]: crate::figure::Figure::subplot_params
//! [`Figure::align_labels`]: crate::figure::Figure::align_labels
//! [`Figure::align_titles`]: crate::figure::Figure::align_titles
//! [`Figure::autofmt_xdate`]: crate::figure::Figure::autofmt_xdate
//! [`Figure::layout`]: crate::figure::Figure::layout
//! [`Rgba`]: plt_core::Rgba
//! [`Grid`]: crate::numeric::Grid
//! [`Grid::as_slice`]: crate::numeric::Grid::as_slice
//! [`Grid::shape`]: crate::numeric::Grid::shape
//! [`Share`]: crate::figure::Share
//! [`GridSpec`]: crate::gridspec::GridSpec
//! [`GridSpec::with_ratios`]: crate::gridspec::GridSpec::with_ratios
//! [`GridSpec::cell`]: crate::gridspec::GridSpec::cell
//! [`GridSpec::span`]: crate::gridspec::GridSpec::span
//! [`GridSpec::nested`]: crate::gridspec::GridSpec::nested
//! [`GridSpec::update`]: crate::gridspec::GridSpec::update
//! [`SubplotOverrides`]: crate::gridspec::SubplotOverrides
//! [`SubplotSpec::gridspec`]: crate::gridspec::SubplotSpec::gridspec
//! [`Axes::subplot_spec`]: crate::axes::Axes::subplot_spec
//! [`Axes::position`]: crate::axes::Axes::position
//! [`Axes::set_position`]: crate::axes::Axes::set_position
//! [`Axes::label_outer`]: crate::axes::Axes::label_outer
//! [`Mosaic::parse`]: crate::figure::Mosaic::parse
//! [`MosaicOptions`]: crate::figure::MosaicOptions
//! [`MosaicAxes::get`]: crate::figure::MosaicAxes::get
//! [`SubfigureOptions`]: crate::figure::SubfigureOptions
//! [`LayoutEngine`]: crate::layout::LayoutEngine
//! [`LayoutEngine::constrained`]: crate::layout::LayoutEngine::constrained
//! [`LayoutEngine::compressed`]: crate::layout::LayoutEngine::compressed
//! [`LayoutEngine::tight`]: crate::layout::LayoutEngine::tight
//! [`LayoutEngine::Constrained`]: crate::layout::LayoutEngine::Constrained
//! [`ConstrainedLayout`]: crate::layout::ConstrainedLayout
//! [`TightLayout`]: crate::layout::TightLayout
