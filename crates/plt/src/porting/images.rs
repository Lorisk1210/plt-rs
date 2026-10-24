//! Images, contours, meshes and vector fields.
//!
//! 2-D data is a [`Grid`] (row-major, `(rows, cols)`, indexed by
//! `(row, col)`), never a nested `Vec`. Each family of methods wraps its
//! data in a type that says how the grid is placed:
//!
//! * images take an [`ImageData`]: a `Grid<f64>` of scalars to colour-map
//!   (or a [`ScalarField`] with a mask), or a grid of RGB(A) pixels as
//!   floats or bytes;
//! * `pcolor` and `pcolormesh` take a [`Mesh`]: values with optional
//!   column/row or full-grid coordinates ([`Mesh::from_coords`]);
//! * contours take a [`QuadField`]: z with optional coordinates
//!   ([`QuadCoords`]) and mask; triangular grids are a [`Triangulation`];
//! * `quiver` and `barbs` take a [`VectorField`], `streamplot` a
//!   [`StreamField`].
//!
//! The colour-mapping keywords `cmap`, `norm`, `vmin` and `vmax` are a
//! [`ColorizerOptions`] in the `colorizer` field of the options of
//! `imshow`, `pcolormesh`, `tripcolor`, `hexbin`, the contour functions
//! ([`ContourOptions`]), `streamplot`, `imsave`, ... A colormap is a
//! value: `cmap="magma"` is `colormaps::get("magma")?`.
//!
//! Colorbars are made by [`Figure::colorbar`] from any colour-mapped
//! artist (image, collection, contour set) and follow later changes of
//! its colormap, norm and limits.
//!
//! # Equivalences
//!
//! | matplotlib                                       | plt-rs                                           |
//! |--------------------------------------------------|--------------------------------------------------|
//! | `ax.imshow(z, cmap=, vmin=, vmax=, norm=)`       | [`Axes::imshow`] with [`ImshowOptions`] and [`ColorizerOptions`] |
//! | `imshow(..., origin="lower", extent=(l, r, b, t))` | [`ImshowOptions::origin`]`: Some(Origin::Lower)`, [`Extent::new`]`(l, r, b, t)` |
//! | `imshow(..., interpolation="nearest", aspect="auto")` | [`Interpolation::Nearest`], [`ImshowOptions::aspect`] |
//! | `ax.imshow(rgb_uint8_array)`                     | `Grid<[u8; 3]>` or `Grid<[u8; 4]>` as [`ImageData`] |
//! | masked image data                                | [`ScalarField::with_mask`]                       |
//! | `im.set_data(z)`, `im.get_array()`               | [`AxesImage::set_data`], [`AxesImage::array`]    |
//! | `im.set_clim(a, b)`, `im.set_cmap(c)`, `im.set_norm(n)` | [`AxesImage::set_clim`], [`AxesImage::set_cmap`], [`AxesImage::set_norm`] |
//! | `ax.matshow(z)`, `ax.spy(z)`                     | [`Axes::matshow`], [`Axes::spy`]                 |
//! | `fig.figimage(z, xo, yo)`                        | [`Figure::figimage`] with [`FigimageOptions`]    |
//! | `ax.pcolormesh(x, y, c, shading="flat")`         | [`Axes::pcolormesh`]`(Mesh::from_coords(x, y, c), ..)` with [`PcolorOptions`], [`Shading`] |
//! | `ax.pcolor(...)`, `ax.pcolorfast(...)`           | [`Axes::pcolor`], [`Axes::pcolorfast`]           |
//! | `ax.hexbin`, `ax.hist2d`                         | [`Axes::hexbin`], [`Axes::hist2d`]               |
//! | `fig.colorbar(im, ax=ax, label=, location=, shrink=)` | [`Figure::colorbar`] with [`ColorbarOptions`] |
//! | `cb.set_label(s)`, `cb.set_ticks(t)`, `cb.ax`    | [`Colorbar::set_label`], [`Colorbar::set_ticks`], [`Colorbar::ax`] |
//! | `ax.contour(X, Y, Z, levels=[...])`              | [`Axes::contour`] with [`QuadField`] and [`ContourOptions`], [`Levels::Values`] |
//! | `ax.contourf(Z, levels=10)`                      | [`Axes::contourf`], [`Levels::Count`]            |
//! | `ax.clabel(cs, fmt="%.2f")`, `cs.clabel(...)`    | [`ContourSet::clabel`] with [`ClabelOptions`] ([`ClabelFormat`]) |
//! | `cs.levels`, `cs.get_paths()`                    | [`ContourSet::levels`], [`ContourSet::paths`]    |
//! | `Triangulation(x, y)`, `Triangulation(x, y, triangles)` | [`Triangulation::delaunay`], [`Triangulation::new`] |
//! | `ax.triplot(tri, "k-")`, `ax.tripcolor(tri, z)`  | [`Axes::triplot`], [`Axes::tripcolor`] with [`TriValues`] |
//! | `ax.tricontour(tri, z)`, `ax.tricontourf(tri, z)` | [`Axes::tricontour`], [`Axes::tricontourf`]     |
//! | `ax.quiver(X, Y, U, V, C, scale=, pivot="mid")`  | [`Axes::quiver`] with [`VectorField`] and [`QuiverOptions`] |
//! | `ax.quiverkey(q, X, Y, U, label)`                | [`Axes::quiverkey`]                              |
//! | `ax.barbs(X, Y, U, V)`                           | [`Axes::barbs`] with [`BarbsOptions`]            |
//! | `ax.streamplot(x, y, U, V, density=, color=)`    | [`Axes::streamplot`] with [`StreamField`] and [`StreamplotOptions`] |
//! | `plt.imread(path)`, `plt.imsave(path, z, cmap=)` | [`imread`], [`imsave`] with [`ImsaveOptions`]    |
//! | `imsave` into a buffer, reading one back         | [`imsave_bytes`], [`imread_bytes`]               |
//!
//! # Images and colorbars
//!
//! ```python
//! z = np.fromfunction(lambda r, c: r * c, (4, 5))
//! im = ax.imshow(z, cmap="magma", vmin=0, vmax=12, origin="lower",
//!                extent=(0, 5, 0, 4), interpolation="nearest")
//! cb = fig.colorbar(im, ax=ax, label="row × column")
//! im.set_clim(0, 6)
//! assert cb.ax.get_ylim() == (0, 6)
//! ```
//!
//! ```
//! use plt::axes::ImshowOptions;
//! use plt::colorbar::ColorbarOptions;
//! use plt::colors::{ColorizerOptions, colormaps};
//! use plt::image::{Extent, Interpolation, Origin};
//! use plt::numeric::Grid;
//! use plt::prelude::*;
//!
//! let fig = Figure::new();
//! let ax = fig.subplot();
//! let z = Grid::from_fn(4, 5, |r, c| (r * c) as f64);
//! let options = ImshowOptions {
//!     colorizer: ColorizerOptions::default()
//!         .cmap(colormaps::get("magma")?)
//!         .limits(Some(0.0), Some(12.0)),
//!     origin: Some(Origin::Lower),
//!     extent: Some(Extent::new(0.0, 5.0, 0.0, 4.0)),
//!     interpolation: Some(Interpolation::Nearest),
//!     ..ImshowOptions::default()
//! };
//! let image = ax.imshow(z, options)?;
//! let colorbar = fig.colorbar(&image, ColorbarOptions { label: "row × column".into(), ..ColorbarOptions::default() })?;
//! image.set_clim(Some(0.0), Some(6.0));
//! assert_eq!(image.clim(), (Some(0.0), Some(6.0)));
//! assert_eq!(colorbar.ax().ylim(), (0.0, 6.0));
//! assert_eq!(image.shape(), (4, 5));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Meshes
//!
//! ```python
//! x = np.linspace(0, 1, 6)        # 5 cells across
//! y = np.linspace(0, 1, 4)        # 3 cells down
//! c = np.fromfunction(lambda r, c: r + c, (3, 5))
//! mesh = ax.pcolormesh(x, y, c, shading="flat", cmap="viridis")
//! ```
//!
//! ```
//! use plt::axes::{Mesh, PcolorOptions, Shading};
//! use plt::colors::{ColorizerOptions, colormaps};
//! use plt::numeric::Grid;
//! use plt::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! let c = Grid::from_fn(3, 5, |r, c| (r + c) as f64);
//! let options = PcolorOptions {
//!     shading: Some(Shading::Flat),
//!     colorizer: ColorizerOptions::default().cmap(colormaps::get("viridis")?),
//!     ..PcolorOptions::default()
//! };
//! let mesh = ax.pcolormesh(Mesh::from_coords(linspace(0.0, 1.0, 6), linspace(0.0, 1.0, 4), c), options)?;
//! assert_eq!(mesh.array().map(|values| values.len()), Some(15));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Contours
//!
//! ```python
//! X, Y = np.meshgrid(np.linspace(-2, 2, 41), np.linspace(-2, 2, 41))
//! Z = np.exp(-(X**2 + Y**2))
//! cs = ax.contour(X, Y, Z, levels=[0.25, 0.5, 0.75], colors="k")
//! labels = ax.clabel(cs, fmt="%.2f")
//! filled = ax.contourf(X, Y, Z, levels=10, cmap="Blues")
//! ```
//!
//! ```
//! use plt::contour::{ClabelOptions, ContourOptions, ClabelFormat, Levels, QuadCoords, QuadField};
//! use plt::colors::colormaps;
//! use plt::numeric::{Grid, meshgrid};
//! use plt::prelude::*;
//! use plt::ticker::pyfmt::PrintfFormat;
//!
//! let ax = Figure::new().subplot();
//! let axis = linspace(-2.0, 2.0, 41);
//! let (x, y) = meshgrid(&axis, &axis);
//! let z = Grid::from_fn(41, 41, |r, c| (-(x[(r, c)].powi(2) + y[(r, c)].powi(2))).exp());
//! let field = QuadField::new(z).with_coords(QuadCoords::Mesh { x, y });
//!
//! let lines = ContourOptions {
//!     levels: Levels::Values(vec![0.25, 0.5, 0.75]),
//!     colors: Some(vec!["k".into()]),
//!     ..ContourOptions::default()
//! };
//! let cs = ax.contour(&field, lines)?;
//! assert_eq!(cs.levels(), [0.25, 0.5, 0.75]);
//! let format = ClabelOptions { fmt: ClabelFormat::Label("%.2f".into()), ..ClabelOptions::default() };
//! let labels = cs.clabel(format)?;
//! assert!(labels.iter().any(|label| label.text() == "0.50"));
//!
//! let bands = ContourOptions {
//!     levels: Levels::Count(10),
//!     ..ContourOptions::default()
//! }
//! .cmap(colormaps::get("Blues")?);
//! let filled = ax.contourf(&field, bands)?;
//! assert!(filled.filled());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Triangulations
//!
//! ```python
//! tri = Triangulation([0, 1, 0, 1], [0, 0, 1, 1])
//! ax.tripcolor(tri, [0, 1, 2, 3])
//! ax.triplot(tri, "k-")
//! ax.tricontour(tri, [0, 1, 2, 3], levels=[1.5])
//! ```
//!
//! ```
//! use plt::axes::{TriValues, TripcolorOptions};
//! use plt::contour::{ContourOptions, Levels};
//! use plt::prelude::*;
//! use plt::tri::Triangulation;
//!
//! let ax = Figure::new().subplot();
//! let tri = Triangulation::delaunay(&[0.0, 1.0, 0.0, 1.0], &[0.0, 0.0, 1.0, 1.0])?;
//! let z = [0.0, 1.0, 2.0, 3.0];
//! ax.tripcolor(&tri, TriValues::Auto(z.to_vec()), TripcolorOptions::default())?;
//! let edges = ax.triplot(&tri, "k-", LineProps::default())?;
//! assert!(!edges.edges.xdata().is_empty());
//! let options = ContourOptions { levels: Levels::Values(vec![1.5]), ..ContourOptions::default() };
//! let cs = ax.tricontour(&tri, &z, options)?;
//! assert_eq!(cs.levels(), [1.5]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Vector fields
//!
//! ```python
//! X, Y = np.meshgrid(np.linspace(-1, 1, 5), np.linspace(-1, 1, 5))
//! q = ax.quiver(X, Y, -Y, X, pivot="mid", color="tab:blue")
//! x = np.linspace(-1, 1, 20)
//! U, V = np.meshgrid(-x, x)
//! ax2.streamplot(x, x, U, V, density=1.5, color="k")
//! ```
//!
//! ```
//! use plt::numeric::meshgrid;
//! use plt::prelude::*;
//! use plt::quiver::{Pivot, QuiverOptions, VectorField};
//! use plt::streamplot::{StreamColors, StreamField, StreamplotOptions};
//!
//! let fig = Figure::new();
//! let axs = fig.subplots(1, 2)?;
//! let (xx, yy) = meshgrid(&linspace(-1.0, 1.0, 5), &linspace(-1.0, 1.0, 5));
//! let u = yy.map(|y| -y);
//! let options = QuiverOptions { pivot: Pivot::Middle, color: "tab:blue".into(), ..QuiverOptions::default() };
//! let arrows = axs[(0, 0)].quiver(VectorField::on_mesh(&xx, &yy, &u, &xx), options)?;
//! assert_eq!(arrows.len(), 25);
//!
//! let x = linspace(-1.0, 1.0, 20);
//! let (u, v) = meshgrid(&x.iter().map(|v| -v).collect::<Vec<_>>(), &x);
//! let options = StreamplotOptions {
//!     density: (1.5, 1.5),
//!     color: Some(StreamColors::Single("k".into())),
//!     ..StreamplotOptions::default()
//! };
//! let streams = axs[(0, 1)].streamplot(&StreamField::new(x.clone(), x, u, v), options)?;
//! assert!(!streams.lines.paths().is_empty());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Reading and writing images
//!
//! ```python
//! buffer = io.BytesIO()
//! plt.imsave(buffer, z, cmap="gray", format="png")
//! pixels = plt.imread(io.BytesIO(buffer.getvalue()))
//! assert pixels.shape == (2, 3, 4)
//! ```
//!
//! ```
//! use plt::colors::{Colorize, colormaps};
//! use plt::image::{BitmapFormat, ImsaveOptions, imread_bytes, imsave_bytes};
//! use plt::numeric::Grid;
//!
//! let z = Grid::from_fn(2, 3, |r, c| (r + c) as f64);
//! let options = ImsaveOptions::default().cmap(colormaps::get("gray")?);
//! let png = imsave_bytes(z, BitmapFormat::Png, options)?;
//! let pixels = imread_bytes(&png)?;
//! assert_eq!(pixels.shape(), (2, 3));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`Grid`]: crate::numeric::Grid
//! [`ImageData`]: crate::image::ImageData
//! [`ScalarField`]: crate::image::ScalarField
//! [`ScalarField::with_mask`]: crate::image::ScalarField::with_mask
//! [`ColorizerOptions`]: crate::colors::ColorizerOptions
//! [`Extent::new`]: crate::image::Extent::new
//! [`Interpolation::Nearest`]: crate::image::Interpolation::Nearest
//! [`AxesImage::set_data`]: crate::image::AxesImage::set_data
//! [`AxesImage::array`]: crate::image::AxesImage::array
//! [`AxesImage::set_clim`]: crate::image::AxesImage::set_clim
//! [`AxesImage::set_cmap`]: crate::image::AxesImage::set_cmap
//! [`AxesImage::set_norm`]: crate::image::AxesImage::set_norm
//! [`FigimageOptions`]: crate::image::FigimageOptions
//! [`ImsaveOptions`]: crate::image::ImsaveOptions
//! [`imread`]: crate::image::imread
//! [`imsave`]: crate::image::imsave
//! [`imsave_bytes`]: crate::image::imsave_bytes
//! [`imread_bytes`]: crate::image::imread_bytes
//! [`Mesh`]: crate::axes::Mesh
//! [`Mesh::from_coords`]: crate::axes::Mesh::from_coords
//! [`PcolorOptions`]: crate::axes::PcolorOptions
//! [`Shading`]: crate::axes::Shading
//! [`ImshowOptions`]: crate::axes::ImshowOptions
//! [`ImshowOptions::origin`]: crate::axes::ImshowOptions::origin
//! [`ImshowOptions::aspect`]: crate::axes::ImshowOptions::aspect
//! [`TriValues`]: crate::axes::TriValues
//! [`Axes::imshow`]: crate::axes::Axes::imshow
//! [`Axes::matshow`]: crate::axes::Axes::matshow
//! [`Axes::spy`]: crate::axes::Axes::spy
//! [`Axes::pcolormesh`]: crate::axes::Axes::pcolormesh
//! [`Axes::pcolor`]: crate::axes::Axes::pcolor
//! [`Axes::pcolorfast`]: crate::axes::Axes::pcolorfast
//! [`Axes::hexbin`]: crate::axes::Axes::hexbin
//! [`Axes::hist2d`]: crate::axes::Axes::hist2d
//! [`Axes::contour`]: crate::axes::Axes::contour
//! [`Axes::contourf`]: crate::axes::Axes::contourf
//! [`Axes::triplot`]: crate::axes::Axes::triplot
//! [`Axes::tripcolor`]: crate::axes::Axes::tripcolor
//! [`Axes::tricontour`]: crate::axes::Axes::tricontour
//! [`Axes::tricontourf`]: crate::axes::Axes::tricontourf
//! [`Axes::quiver`]: crate::axes::Axes::quiver
//! [`Axes::quiverkey`]: crate::axes::Axes::quiverkey
//! [`Axes::barbs`]: crate::axes::Axes::barbs
//! [`Axes::streamplot`]: crate::axes::Axes::streamplot
//! [`Figure::figimage`]: crate::figure::Figure::figimage
//! [`Figure::colorbar`]: crate::figure::Figure::colorbar
//! [`ColorbarOptions`]: crate::colorbar::ColorbarOptions
//! [`Colorbar::set_label`]: crate::colorbar::Colorbar::set_label
//! [`Colorbar::set_ticks`]: crate::colorbar::Colorbar::set_ticks
//! [`Colorbar::ax`]: crate::colorbar::Colorbar::ax
//! [`QuadField`]: crate::contour::QuadField
//! [`QuadCoords`]: crate::contour::QuadCoords
//! [`ContourOptions`]: crate::contour::ContourOptions
//! [`Levels::Values`]: crate::contour::Levels::Values
//! [`Levels::Count`]: crate::contour::Levels::Count
//! [`ContourSet::clabel`]: crate::contour::ContourSet::clabel
//! [`ClabelFormat`]: crate::contour::ClabelFormat
//! [`ContourSet::levels`]: crate::contour::ContourSet::levels
//! [`ContourSet::paths`]: crate::contour::ContourSet::paths
//! [`ClabelOptions`]: crate::contour::ClabelOptions
//! [`Triangulation`]: crate::tri::Triangulation
//! [`Triangulation::delaunay`]: crate::tri::Triangulation::delaunay
//! [`Triangulation::new`]: crate::tri::Triangulation::new
//! [`VectorField`]: crate::quiver::VectorField
//! [`QuiverOptions`]: crate::quiver::QuiverOptions
//! [`BarbsOptions`]: crate::quiver::BarbsOptions
//! [`StreamField`]: crate::streamplot::StreamField
//! [`StreamplotOptions`]: crate::streamplot::StreamplotOptions
