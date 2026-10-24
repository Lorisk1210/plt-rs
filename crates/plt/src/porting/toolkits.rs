//! `mplot3d`, `axes_grid1`, `axisartist`, polar and geographic axes, and
//! the other specialised modules.
//!
//! matplotlib picks special axes with `projection=` or `axes_class=`;
//! plt-rs gives each kind its own handle and constructor:
//!
//! * [`Axes3D`] from [`Figure::subplot_3d`] (and [`Figure::add_subplot_3d`],
//!   [`Figure::add_axes_3d`]): its own plotting methods taking x, y and z,
//!   per-axis methods taking an [`Axis3`], and the camera ([`ViewAngles`],
//!   [`ProjType`]). Limit setters check their arguments and return a
//!   `Result`.
//! * [`PolarAxes`] from [`Figure::polar_subplot`] and [`GeoAxes`] from
//!   [`Figure::geo_subplot`]: they dereference to [`Axes`], so every 2-D
//!   plotting method works on them, and add the settings of their
//!   projection. Compass directions and orientations are enums
//!   ([`Compass`], [`ThetaDirection`]).
//! * [`crate::axes_grid1`] keeps matplotlib's functions
//!   ([`make_axes_locatable`], [`inset_axes`], [`zoomed_inset_axes`],
//!   [`mark_inset`], [`host_subplot`]) and classes ([`ImageGrid`],
//!   [`Divider`]); sizes like `"5%"` are a [`SizeSpec`], sides a
//!   [`SpineSide`].
//! * [`crate::axisartist`] wraps an ordinary axes in an
//!   [`axisartist::Axes`](crate::axisartist::Axes) ([`Axes::new`](crate::axisartist::Axes::new),
//!   or [`Axes::zero`](crate::axisartist::Axes::zero) for `AxesZero`);
//!   `ax.axis["left"]` is [`axis("left")`](crate::axisartist::Axes::axis),
//!   keyed by name since new axis lines get names of your choosing.
//!
//! # Equivalences
//!
//! | matplotlib                                        | plt-rs                                         |
//! |---------------------------------------------------|------------------------------------------------|
//! | `fig.add_subplot(projection="3d")`                | [`Figure::subplot_3d`], [`Figure::add_subplot_3d`] |
//! | `fig.add_subplot(projection="3d", elev=, azim=, proj_type=)` | [`Figure::subplot_3d_with`] and [`Axes3DOptions`] |
//! | `ax.plot(x, y, z)`, `ax.scatter(x, y, z, ...)`    | [`Axes3D::plot`], [`Axes3D::scatter_with`] with [`Scatter3DOptions`] |
//! | `ax.plot_surface(X, Y, Z, cmap=, norm=, vmin=)`   | [`Axes3D::plot_surface`] with [`SurfaceOptions`], [`FaceColors::Mapped`] |
//! | `ax.plot_wireframe`, `ax.plot_trisurf`            | [`Axes3D::plot_wireframe`], [`Axes3D::plot_trisurf`] |
//! | `ax.contour(X, Y, Z, zdir=, offset=)`             | [`Axes3D::contour`] with [`Contour3DOptions`]  |
//! | `ax.bar3d`, `ax.voxels`, `ax.quiver`, `ax.stem`   | [`Axes3D::bar3d`], [`Axes3D::voxels`], [`Axes3D::quiver`], [`Axes3D::stem`] |
//! | `ax.text(x, y, z, s)`, `ax.text2D(x, y, s)`       | [`Axes3D::text`], [`Axes3D::text2d`]           |
//! | `ax.set_zlim(a, b)`, `ax.get_zlim()`              | [`Axes3D::set_zlim`], [`Axes3D::zlim`]         |
//! | `ax.set_zlabel(s)`, `ax.set_zscale("log")`        | [`Axes3D::set_zlabel`], [`Axes3D::set_zscale`] |
//! | `ax.get_zbound()`, `ax.zaxis_inverted()`          | [`Axes3D::bound`]`(Axis3::Z)`, [`Axes3D::inverted`]`(Axis3::Z)` |
//! | `ax.zaxis`, `ax.get_zticks()`                     | [`Axes3D::zaxis`], [`Axis3D::ticklocs`]        |
//! | `ax.view_init(elev, azim, roll)`                  | [`Axes3D::view_init`], [`Axes3D::view`]        |
//! | `ax.set_box_aspect((1, 1, 1))`, `ax.set_aspect("equal")` | [`Axes3D::set_box_aspect`], [`Axes3D::set_aspect`] with [`Aspect3D`] |
//! | `ax.set_proj_type("ortho")`                       | [`Axes3D::set_proj_type`]`(ProjType::Ortho)`   |
//! | `ax.sharez(other)`, `ax.shareview(other)`         | [`Axes3D::share_z`], [`Axes3D::share_view`]    |
//! | `ax.get_proj()`                                   | [`Axes3D::proj`]                               |
//! | `fig.add_subplot(projection="polar")`             | [`Figure::polar_subplot`], [`Figure::add_polar_subplot`] |
//! | `ax.set_theta_zero_location("N")`                 | [`PolarAxes::set_theta_zero_location`]`(Compass::N, 0.0)` |
//! | `ax.set_theta_direction(-1)`                      | [`PolarAxes::set_theta_direction`]`(ThetaDirection::Clockwise)` |
//! | `ax.set_rmax(r)`, `ax.set_rlim(a, b)`, `ax.set_rticks(t)` | [`PolarAxes::set_rmax`], [`PolarAxes::set_rlim`], [`PolarAxes::set_rticks`] |
//! | `ax.set_rlabel_position(deg)`, `ax.set_thetamin(deg)` | [`PolarAxes::set_rlabel_position`], [`PolarAxes::set_thetamin`] |
//! | `plt.polar(theta, r)`, `plt.thetagrids`, `plt.rgrids` | [`crate::pyplot::polar`], [`crate::pyplot::thetagrids`], [`crate::pyplot::rgrids`] |
//! | `fig.add_subplot(projection="mollweide")`         | [`Figure::geo_subplot`]`(GeoProjection::Mollweide)` |
//! | `ax.set_longitude_grid(30)`, `ax.set_latitude_grid(15)` | [`GeoAxes::set_longitude_grid`], [`GeoAxes::set_latitude_grid`] |
//! | `make_axes_locatable(ax)`                         | [`make_axes_locatable`]                        |
//! | `divider.append_axes("right", size="5%", pad=0.05)` | [`AxesDivider::append_axes`]`(SpineSide::Right, SizeSpec::Percent(5.0), Some(0.05.into()))` |
//! | `inset_axes(ax, width="30%", height=1, loc=...)`  | [`inset_axes`] with [`SizeSpec`] and [`InsetOptions`] |
//! | `zoomed_inset_axes(ax, zoom)`, `mark_inset(...)`  | [`zoomed_inset_axes`], [`mark_inset`]          |
//! | `ImageGrid(fig, 111, nrows_ncols=(2, 2), axes_pad=0.1)` | [`ImageGrid::new`] with [`GridRect`] and [`ImageGridOptions`] |
//! | `Divider(fig, rect, horizontal, vertical)`, `Size.Fixed(1)` | [`Divider::new`], [`Size::Fixed`]        |
//! | `host_subplot(111)`, `host.twinx()`               | [`host_subplot`], [`HostAxes`]                 |
//! | `AnchoredSizeBar`, `AnchoredDirectionArrows`      | [`crate::axes_grid1::anchored_artists`]        |
//! | `fig.add_subplot(axes_class=axisartist.Axes)`     | [`axisartist::Axes::new`](crate::axisartist::Axes::new)`(fig.subplot())` |
//! | `axes_class=AxesZero`                             | [`axisartist::Axes::zero`](crate::axisartist::Axes::zero) |
//! | `ax.axis["right"].set_visible(False)`             | `ax.axis("right")` ([`axisartist::Axes::axis`](crate::axisartist::Axes::axis)) and [`ArtistHandle::set_visible`] |
//! | `ax.axis["x"].set_axisline_style("-\|>")`         | [`AxisArtist::set_axisline_style`]             |
//! | `ax.new_floating_axis(0, 0.5)`                    | [`axisartist::Axes::new_floating_axis`](crate::axisartist::Axes::new_floating_axis) |
//! | `GridHelperCurveLinear(tr)`                       | [`axisartist::GridHelper`](crate::axisartist::GridHelper) |
//! | `ax.get_aux_axes(tr)`, `grid_finder.get_grid_info(bbox)` | [`axisartist::Axes::aux_axes`](crate::axisartist::Axes::aux_axes), [`GridFinder::grid_info`](crate::axisartist::grid_finder::GridFinder::grid_info) |
//! | `ax.inset_axes([x, y, w, h])`, `ax.indicate_inset_zoom(axins)` | [`Axes::inset_axes`], [`Axes::indicate_inset_zoom`] |
//! | `Sankey(ax).add(flows=...).finish()`              | [`Sankey::new`], [`Sankey::add`], [`Sankey::finish`] |
//! | `ax.table(cellText=...)`                          | [`Axes::table`] with [`TableOptions`]          |
//! | `AnchoredText`, `AnchoredOffsetbox`, `AnnotationBbox` | [`Axes::add_anchored_box`], [`Axes::add_annotation_bbox`] ([`crate::offsetbox`]) |
//! | `patheffects.withStroke(linewidth=3, foreground="w")` | [`PathEffect::with_stroke`] ([`crate::patheffects`]) |
//!
//! # 3-D axes
//!
//! ```python
//! ax = fig.add_subplot(projection="3d")
//! t = np.linspace(0, 10, 50)
//! ax.plot(np.cos(t), np.sin(t), t)
//! X, Y = np.meshgrid(np.linspace(-1, 1, 10), np.linspace(-1, 1, 10))
//! ax.plot_surface(X, Y, X * Y, cmap="viridis")
//! ax.set_zlim(-1, 10)
//! ax.set_zlabel("z")
//! ax.view_init(elev=20, azim=-60)
//! ```
//!
//! ```
//! use plt::colors::{Colorize, ColorizerOptions, colormaps};
//! use plt::mplot3d::{Axis3, FaceColors, SurfaceOptions};
//! use plt::numeric::{Grid, linspace, meshgrid};
//! use plt::figure::Figure;
//!
//! let fig = Figure::new();
//! let ax = fig.subplot_3d();
//! let t = linspace(0.0, 10.0, 50);
//! let (x, y): (Vec<f64>, Vec<f64>) = t.iter().map(|t| (t.cos(), t.sin())).unzip();
//! ax.plot(&x, &y, &t)?;
//!
//! let (xx, yy) = meshgrid(&linspace(-1.0, 1.0, 10), &linspace(-1.0, 1.0, 10));
//! let zz = Grid::from_fn(10, 10, |r, c| xx[(r, c)] * yy[(r, c)]);
//! let options = SurfaceOptions {
//!     colors: FaceColors::from(ColorizerOptions::default().cmap(colormaps::get("viridis")?)),
//!     ..SurfaceOptions::default()
//! };
//! ax.plot_surface(&xx, &yy, &zz, options)?;
//! ax.set_zlim(-1.0, 10.0)?;
//! ax.set_zlabel("z");
//! ax.view_init(20.0, -60.0, 0.0);
//!
//! assert_eq!(ax.zlim(), (-1.0, 10.0));
//! assert_eq!(ax.label(Axis3::Z), "z");
//! assert_eq!(ax.view().elev, 20.0);
//! assert!(ax.set_zlim(f64::NAN, 1.0).is_err());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Polar and geographic axes
//!
//! ```python
//! ax = fig.add_subplot(1, 2, 1, projection="polar")
//! ax.plot(theta, r)
//! ax.set_theta_zero_location("N")
//! ax.set_theta_direction(-1)
//! ax.set_rmax(2)
//! ax.set_rlabel_position(-22.5)
//! world = fig.add_subplot(1, 2, 2, projection="mollweide")
//! world.set_longitude_grid(30)
//! ```
//!
//! ```
//! use plt::figure::Figure;
//! use plt::geo::GeoProjection;
//! use plt::numeric::linspace;
//! use plt::polar::{Compass, ThetaDirection};
//!
//! let fig = Figure::new();
//! let ax = fig.add_polar_subplot(1, 2, 1)?;
//! let theta = linspace(0.0, std::f64::consts::TAU, 100);
//! let r: Vec<f64> = theta.iter().map(|t| 1.0 + 0.5 * t.sin()).collect();
//! ax.plot(&theta, r)?; // a 2-D plotting method, through `Deref<Target = Axes>`
//! ax.set_theta_zero_location(Compass::N, 0.0)
//!     .set_theta_direction(ThetaDirection::Clockwise)
//!     .set_rmax(2.0)
//!     .set_rlabel_position(-22.5);
//! assert_eq!(ax.rmax(), 2.0);
//!
//! let world = fig.add_geo_subplot(GeoProjection::Mollweide, 1, 2, 2)?;
//! world.set_longitude_grid(30.0)?;
//! assert_eq!(world.xlim(), (-std::f64::consts::PI, std::f64::consts::PI));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # `axes_grid1`
//!
//! ```python
//! im = ax.imshow(z)
//! divider = make_axes_locatable(ax)
//! cax = divider.append_axes("right", size="5%", pad=0.05)
//! fig.colorbar(im, cax=cax)
//! axins = inset_axes(ax, width="30%", height="40%", loc="upper right")
//! grid = ImageGrid(fig2, 111, nrows_ncols=(2, 2), axes_pad=0.1)
//! ```
//!
//! ```
//! use plt::axes::ImshowOptions;
//! use plt::axes_grid1::{
//!     GridRect, ImageGrid, ImageGridOptions, InsetOptions, SizeSpec, inset_axes, make_axes_locatable,
//! };
//! use plt::colorbar::{ColorbarAxes, ColorbarOptions};
//! use plt::numeric::Grid;
//! use plt::offsetbox::AnchorLoc;
//! use plt::prelude::*;
//! use plt::spines::SpineSide;
//!
//! let fig = Figure::new();
//! let ax = fig.subplot();
//! let image = ax.imshow(Grid::from_fn(4, 4, |r, c| (r + c) as f64), ImshowOptions::default())?;
//! let divider = make_axes_locatable(&ax);
//! let cax = divider.append_axes(SpineSide::Right, SizeSpec::Percent(5.0), Some(SizeSpec::Inches(0.05)));
//! let colorbar = fig.colorbar(&image, ColorbarOptions { axes: ColorbarAxes::Cax(cax.clone()), ..ColorbarOptions::default() })?;
//! assert_eq!(colorbar.ax(), &cax);
//!
//! let options = InsetOptions { loc: AnchorLoc::UpperRight, ..InsetOptions::default() };
//! let axins = inset_axes(&ax, SizeSpec::Percent(30.0), SizeSpec::Percent(40.0), options);
//! axins.plot([0.0, 1.0], [0.0, 1.0])?;
//!
//! let other = Figure::new();
//! let grid_options = ImageGridOptions { axes_pad: (0.1, 0.1), ..ImageGridOptions::default() };
//! let grid = ImageGrid::new(&other, &GridRect::Rect([0.1, 0.1, 0.8, 0.8]), (2, 2), grid_options)?;
//! assert_eq!(grid.axes().len(), 4);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # `axisartist`
//!
//! ```python
//! ax = fig.add_subplot(axes_class=AxesZero)
//! for name in ["xzero", "yzero"]:
//!     ax.axis[name].set_axisline_style("-|>")
//!     ax.axis[name].set_visible(True)
//! for name in ["left", "right", "bottom", "top"]:
//!     ax.axis[name].set_visible(False)
//! ```
//!
//! ```
//! use plt::axisartist;
//! use plt::prelude::*;
//!
//! let fig = Figure::new();
//! let ax = axisartist::Axes::zero(fig.subplot());
//! for name in ["xzero", "yzero"] {
//!     let axis = ax.axis(name).ok_or("an axis of AxesZero")?;
//!     axis.set_axisline_style(Some("-|>".parse()?));
//!     axis.set_visible(true);
//! }
//! for name in ["left", "right", "bottom", "top"] {
//!     ax.axis(name).ok_or("an edge axis")?.set_visible(false);
//! }
//! assert!(ax.axis("xzero").ok_or("xzero")?.visible());
//! assert!(ax.axis("diagonal").is_none());
//! fig.to_png()?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`Axes`]: crate::axes::Axes
//! [`Axes::inset_axes`]: crate::axes::Axes::inset_axes
//! [`Axes::indicate_inset_zoom`]: crate::axes::Axes::indicate_inset_zoom
//! [`Axes::table`]: crate::axes::Axes::table
//! [`Axes::add_anchored_box`]: crate::axes::Axes::add_anchored_box
//! [`Axes::add_annotation_bbox`]: crate::axes::Axes::add_annotation_bbox
//! [`ArtistHandle::set_visible`]: crate::artist::ArtistHandle::set_visible
//! [`Figure::subplot_3d`]: crate::figure::Figure::subplot_3d
//! [`Figure::subplot_3d_with`]: crate::figure::Figure::subplot_3d_with
//! [`Figure::add_subplot_3d`]: crate::figure::Figure::add_subplot_3d
//! [`Figure::add_axes_3d`]: crate::figure::Figure::add_axes_3d
//! [`Figure::polar_subplot`]: crate::figure::Figure::polar_subplot
//! [`Figure::add_polar_subplot`]: crate::figure::Figure::add_polar_subplot
//! [`Figure::geo_subplot`]: crate::figure::Figure::geo_subplot
//! [`Axes3D`]: crate::mplot3d::Axes3D
//! [`Axes3D::plot`]: crate::mplot3d::Axes3D::plot
//! [`Axes3D::scatter_with`]: crate::mplot3d::Axes3D::scatter_with
//! [`Axes3D::plot_surface`]: crate::mplot3d::Axes3D::plot_surface
//! [`Axes3D::plot_wireframe`]: crate::mplot3d::Axes3D::plot_wireframe
//! [`Axes3D::plot_trisurf`]: crate::mplot3d::Axes3D::plot_trisurf
//! [`Axes3D::contour`]: crate::mplot3d::Axes3D::contour
//! [`Axes3D::bar3d`]: crate::mplot3d::Axes3D::bar3d
//! [`Axes3D::voxels`]: crate::mplot3d::Axes3D::voxels
//! [`Axes3D::quiver`]: crate::mplot3d::Axes3D::quiver
//! [`Axes3D::stem`]: crate::mplot3d::Axes3D::stem
//! [`Axes3D::text`]: crate::mplot3d::Axes3D::text
//! [`Axes3D::text2d`]: crate::mplot3d::Axes3D::text2d
//! [`Axes3D::set_zlim`]: crate::mplot3d::Axes3D::set_zlim
//! [`Axes3D::zlim`]: crate::mplot3d::Axes3D::zlim
//! [`Axes3D::set_zlabel`]: crate::mplot3d::Axes3D::set_zlabel
//! [`Axes3D::set_zscale`]: crate::mplot3d::Axes3D::set_zscale
//! [`Axes3D::bound`]: crate::mplot3d::Axes3D::bound
//! [`Axes3D::inverted`]: crate::mplot3d::Axes3D::inverted
//! [`Axes3D::zaxis`]: crate::mplot3d::Axes3D::zaxis
//! [`Axes3D::view_init`]: crate::mplot3d::Axes3D::view_init
//! [`Axes3D::view`]: crate::mplot3d::Axes3D::view
//! [`Axes3D::set_box_aspect`]: crate::mplot3d::Axes3D::set_box_aspect
//! [`Axes3D::set_aspect`]: crate::mplot3d::Axes3D::set_aspect
//! [`Axes3D::set_proj_type`]: crate::mplot3d::Axes3D::set_proj_type
//! [`Axes3D::share_z`]: crate::mplot3d::Axes3D::share_z
//! [`Axes3D::share_view`]: crate::mplot3d::Axes3D::share_view
//! [`Axes3D::proj`]: crate::mplot3d::Axes3D::proj
//! [`Axis3D::ticklocs`]: crate::mplot3d::Axis3D::ticklocs
//! [`Axis3`]: crate::mplot3d::Axis3
//! [`Axes3DOptions`]: crate::mplot3d::Axes3DOptions
//! [`Scatter3DOptions`]: crate::mplot3d::Scatter3DOptions
//! [`SurfaceOptions`]: crate::mplot3d::SurfaceOptions
//! [`FaceColors::Mapped`]: crate::mplot3d::FaceColors::Mapped
//! [`Contour3DOptions`]: crate::mplot3d::Contour3DOptions
//! [`ViewAngles`]: crate::mplot3d::ViewAngles
//! [`ProjType`]: crate::mplot3d::ProjType
//! [`Aspect3D`]: crate::mplot3d::Aspect3D
//! [`PolarAxes`]: crate::polar::PolarAxes
//! [`PolarAxes::set_theta_zero_location`]: crate::polar::PolarAxes::set_theta_zero_location
//! [`PolarAxes::set_theta_direction`]: crate::polar::PolarAxes::set_theta_direction
//! [`PolarAxes::set_rmax`]: crate::polar::PolarAxes::set_rmax
//! [`PolarAxes::set_rlim`]: crate::polar::PolarAxes::set_rlim
//! [`PolarAxes::set_rticks`]: crate::polar::PolarAxes::set_rticks
//! [`PolarAxes::set_rlabel_position`]: crate::polar::PolarAxes::set_rlabel_position
//! [`PolarAxes::set_thetamin`]: crate::polar::PolarAxes::set_thetamin
//! [`Compass`]: crate::polar::Compass
//! [`ThetaDirection`]: crate::polar::ThetaDirection
//! [`GeoAxes`]: crate::geo::GeoAxes
//! [`GeoAxes::set_longitude_grid`]: crate::geo::GeoAxes::set_longitude_grid
//! [`GeoAxes::set_latitude_grid`]: crate::geo::GeoAxes::set_latitude_grid
//! [`make_axes_locatable`]: crate::axes_grid1::make_axes_locatable
//! [`AxesDivider::append_axes`]: crate::axes_grid1::AxesDivider::append_axes
//! [`inset_axes`]: crate::axes_grid1::inset_axes
//! [`zoomed_inset_axes`]: crate::axes_grid1::zoomed_inset_axes
//! [`mark_inset`]: crate::axes_grid1::mark_inset
//! [`host_subplot`]: crate::axes_grid1::host_subplot
//! [`HostAxes`]: crate::axes_grid1::HostAxes
//! [`ImageGrid`]: crate::axes_grid1::ImageGrid
//! [`ImageGrid::new`]: crate::axes_grid1::ImageGrid::new
//! [`ImageGridOptions`]: crate::axes_grid1::ImageGridOptions
//! [`GridRect`]: crate::axes_grid1::GridRect
//! [`Divider`]: crate::axes_grid1::Divider
//! [`Divider::new`]: crate::axes_grid1::Divider::new
//! [`Size::Fixed`]: crate::axes_grid1::Size::Fixed
//! [`SizeSpec`]: crate::axes_grid1::SizeSpec
//! [`InsetOptions`]: crate::axes_grid1::InsetOptions
//! [`SpineSide`]: crate::spines::SpineSide
//! [`AxisArtist::set_axisline_style`]: crate::axisartist::AxisArtist::set_axisline_style
//! [`Sankey::new`]: crate::sankey::Sankey::new
//! [`Sankey::add`]: crate::sankey::Sankey::add
//! [`Sankey::finish`]: crate::sankey::Sankey::finish
//! [`TableOptions`]: crate::table::TableOptions
//! [`PathEffect::with_stroke`]: crate::patheffects::PathEffect::with_stroke
