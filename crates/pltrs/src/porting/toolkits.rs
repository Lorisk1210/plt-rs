//! `mplot3d`, `axes_grid1`, `axisartist`, polar and geographic axes,
//! projections of your own, and the other specialised modules.
//!
//! matplotlib picks special axes with `projection=` or `axes_class=`;
//! pltrs gives each kind its own handle and constructor:
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
//! * Projections of your own implement [`Projection`]: one hook per
//!   method matplotlib's projection subclasses override, panning and
//!   zooming included. [`register`] names them, as `register_projection`
//!   does, [`register_with`] those whose `__init__` takes keywords, and
//!   [`Figure::add_subplot_with_projection`] makes axes of any
//!   [`ProjectionKind`], built-in or registered, with the keywords as
//!   options; so do the constructors of grids, mosaics, insets and twins.
//! * [`crate::axes_grid1`] keeps matplotlib's functions
//!   ([`make_axes_locatable`], [`inset_axes`], [`zoomed_inset_axes`],
//!   [`mark_inset`], [`host_subplot`]) and classes ([`ImageGrid`],
//!   [`Divider`]); sizes like `"5%"` are a [`SizeSpec`], sides a
//!   [`SpineSide`].
//! * [`crate::axisartist`] wraps an ordinary axes in an
//!   [`axisartist::AxisArtistAxes`](crate::axisartist::AxisArtistAxes) ([`AxisArtistAxes::new`](crate::axisartist::AxisArtistAxes::new),
//!   or [`AxisArtistAxes::zero`](crate::axisartist::AxisArtistAxes::zero) for `AxesZero`);
//!   `ax.axis["left"]` is [`axis("left")`](crate::axisartist::AxisArtistAxes::axis),
//!   keyed by name since new axis lines get names of your choosing.
//!
//! # Equivalences
//!
//! | matplotlib                                        | pltrs                                          |
//! |---------------------------------------------------|------------------------------------------------|
//! | `fig.add_subplot(projection="3d")`                | [`Figure::subplot_3d`], [`Figure::add_subplot_3d`] |
//! | `fig.add_subplot(projection="3d", elev=, azim=, proj_type=)` | [`Figure::subplot_3d_with`] and [`Axes3DOptions`] |
//! | `ax.plot(x, y, z)`, `ax.scatter(x, y, z, ...)`    | [`Axes3D::plot`], [`Axes3D::scatter_with`] with [`Scatter3DOptions`] |
//! | `x, y, z = ax._process_unit_info([("x", x), ("y", y), ("z", z)])` before plotting dates or names | none: [`Axes3D::plot`], `scatter`, `text`, `bar`, `bar3d`, `stem`, `errorbar` and `fill_between` take dates, names and values with units, as do the limit setters and [`Axis3D::set_ticks`] |
//! | `ax.convert_zunits(z)`, `ax.zaxis.convert_units(z)` | [`Axes3D::zaxis`]`().`[`convert_units`](crate::mplot3d::Axis3D::convert_units)`(z)` |
//! | `ax.zaxis.update_units(v)`, `set_units(u)`, `get_units()` | [`Axis3D::update_units`], [`Axis3D::set_units`], [`Axis3D::units`] |
//! | `ax.zaxis.set_converter(c)`, `get_converter()`    | [`Axis3D::set_converter`], [`Axis3D::converter`]; `set_converter(mdates.ConciseDateConverter())` is `set_converter(`[`DateConverter::Concise`]`)?` |
//! | `ax.zaxis.axis_date(tz)`, `ax.zaxis_date(tz)`     | [`Axis3D::axis_date`]                          |
//! | `ax.fill_between(x1, y1, z1, x2, y2, z2)`         | [`Axes3D::fill_between`]`((x1, y1, z1), (x2, y2, z2), ..)` ([`Curve3D`]) |
//! | `ax.bar3d(x, y, z, dx, dy, dz)`                   | [`Axes3D::bar3d`]`(`[`Boxes::new`]`(x, y, z, dx, dy, dz), ..)` |
//! | `ax.plot_surface(X, Y, Z, cmap=, norm=, vmin=)`   | [`Axes3D::plot_surface`] with [`SurfaceOptions`], [`FaceColors::Mapped`] |
//! | `ax.plot_wireframe`, `ax.plot_trisurf`            | [`Axes3D::plot_wireframe`], [`Axes3D::plot_trisurf`] |
//! | `ax.contour(X, Y, Z, zdir=, offset=)`             | [`Axes3D::contour`] with [`Contour3DOptions`]  |
//! | `ax.bar3d`, `ax.voxels`, `ax.quiver`, `ax.stem`   | [`Axes3D::bar3d`], [`Axes3D::voxels`], [`Axes3D::quiver`], [`Axes3D::stem`] |
//! | `ax.text(x, y, z, s)`, `ax.text2D(x, y, s)`       | [`Axes3D::text`], [`Axes3D::text2d`]           |
//! | `line.set_data_3d(x, y, z)`, `line.get_data_3d()` | [`Axes3D::set_line_data`], [`Axes3D::line_data`] |
//! | `scatter._offsets3d`, `scatter.get_depthshade()`  | [`Axes3D::set_scatter_offsets`], [`Axes3D::scatter_offsets`], [`Axes3D::depthshade`] |
//! | `text3d.get_position_3d()`, `text3d.set_position_3d(p)` | [`Axes3D::text_position`], [`Axes3D::set_text_position`] |
//! | `ax.set_title(s, loc="left")`, `ax.zaxis.get_rotate_label(s)` | [`Axes3D::set_title_at`], [`Axes3D::title_at`], [`Axes3D::rotate_label`] |
//! | `ax.set_zlim(a, b)`, `ax.get_zlim()`              | [`Axes3D::set_zlim`], [`Axes3D::zlim`]         |
//! | `ax.set_zlim(a, b, view_margin=m)`, `ax.set_zbound(a, b, view_margin=m)` | [`Axes3D::set_limits_with_margin`], [`Axes3D::set_bound_with_margin`] |
//! | `ax.set_zlabel(s)`, `ax.set_zscale("log")`        | [`Axes3D::set_zlabel`], [`Axes3D::set_zscale`] |
//! | `ax.get_zbound()`, `ax.zaxis_inverted()`          | [`Axes3D::bound`]`(Axis3::Z)`, [`Axes3D::inverted`]`(Axis3::Z)` |
//! | `ax.margins(x, y, z, tight=True)`, `ax.set_zmargin(m)`, `ax.get_zmargin()` | [`Axes3D::set_margins`]`([Some(x), Some(y), Some(z)], Some(true))`, [`Axes3D::set_margin`], [`Axes3D::margin`] |
//! | `ax.get_autoscalez_on()`, `ax.set_autoscalez_on(b)` | [`Axes3D::autoscale_on`]`(Axis3::Z)`, [`Axes3D::set_axis_autoscale_on`] |
//! | `ax.zaxis`, `ax.get_zticks()`                     | [`Axes3D::zaxis`], [`Axis3D::ticklocs`]        |
//! | `ax.get_zticklines()`, `ax.get_zgridlines()`      | [`Axis3D::ticklines`], [`Axis3D::gridlines`] ([`TickLine3D`], [`GridLine3D`]) |
//! | `col.set_sort_zpos(z)`                            | [`Axes3D::set_sort_zpos`], [`Axes3D::sort_zpos`] |
//! | `lines3d.set_segments(segments)`                  | [`Axes3D::set_segments`]                       |
//! | `poly3d.set_verts(v)`, `set_verts_and_codes(v, c)`, `set_3d_properties()` | [`Axes3D::set_poly_verts`], [`Axes3D::set_poly_verts_and_codes`], [`Axes3D::set_poly_3d_properties`] |
//! | `ax.add_collection3d(PatchCollection(...))`, `patches3d.get_depthshade()`, `set_3d_properties(zs, zdir)` | [`Axes3D::add_collection3d`], [`Axes3D::depthshade`], [`Axes3D::set_scatter_offsets`] |
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
//! | `plt.polar(theta, r)`, `plt.thetagrids`, `plt.rgrids` | [`crate::pyplot::polar`], [`crate::pyplot::thetagrids`], [`crate::pyplot::rgrids`] (`_labeled` with labels) |
//! | `fig.add_subplot(projection="mollweide")`         | [`Figure::geo_subplot`]`(GeoProjection::Mollweide)` |
//! | `ax.set_longitude_grid(30)`, `ax.set_latitude_grid(15)` | [`GeoAxes::set_longitude_grid`], [`GeoAxes::set_latitude_grid`] |
//! | `class SkewXAxes(Axes): name = "skewx"`            | a type implementing [`Projection`]            |
//! | `register_projection(SkewXAxes)`                  | [`register`]`(SkewXAxes::default)`             |
//! | `fig.add_subplot(projection="skewx")`             | [`Figure::subplot_with_projection`]`(ProjectionKind::Registered("skewx"))` |
//! | `fig.add_subplot(2, 1, 1, projection=name)`, `fig.add_axes(rect, projection=name)` | [`Figure::add_subplot_with_projection`], [`Figure::add_axes_with_projection`] |
//! | `def __init__(self, *args, rot=30, **kwargs)`, `register_projection` | [`register_with`]`(\|rot: &Rot\| SkewXAxes::new(rot.0))` with `Rot: Default` |
//! | `fig.add_subplot(projection="skewx", rot=45)`     | [`Figure::subplot_with_projection`]`(kind.with_options(Rot(45.0)))` ([`ProjectionKind::with_options`]) |
//! | `fig.subplots(2, 2, subplot_kw={"projection": name})` | [`Figure::subplots_with_projection`]      |
//! | `fig.subplot_mosaic(..., per_subplot_kw={"A": {"projection": name}})` | [`MosaicOptions::per_subplot`] |
//! | `ax.inset_axes(rect, projection=name)`, `ax.twinx(projection=name)` | [`Axes::inset_axes_with_projection`], [`Axes::twin_with_projection`] |
//! | `_set_lim_and_transforms` (`transData`), `get_xaxis_transform` | [`Projection::data_to_axes`], [`Projection::axis_transform`] |
//! | `clear`, `_gen_axes_patch`, `_gen_axes_spines`    | [`Projection::set_up`], [`Projection::outline`] |
//! | `start_pan`, `drag_pan`, `end_pan`, `_set_view_from_bbox` | [`Projection::start_pan`], [`Projection::drag_pan`], [`Projection::end_pan`], [`Projection::zoom_to_box`] |
//! | `get_projection_names()`                          | [`projection_names`]                           |
//! | `make_axes_locatable(ax)`                         | [`make_axes_locatable`]                        |
//! | `divider.append_axes("right", size="5%", pad=0.05)` | [`AxesDivider::append_axes`]`(SpineSide::Right, SizeSpec::Percent(5.0), Some(0.05.into()))` |
//! | `inset_axes(ax, width="30%", height=1, loc=...)`  | [`inset_axes`] with [`SizeSpec`] and [`InsetOptions`] |
//! | `zoomed_inset_axes(ax, zoom)`, `mark_inset(...)`  | [`zoomed_inset_axes`], [`mark_inset`]          |
//! | `ImageGrid(fig, 111, nrows_ncols=(2, 2), axes_pad=0.1)` | [`ImageGrid::new`] with [`GridRect`] and [`ImageGridOptions`] |
//! | `Divider(fig, rect, horizontal, vertical)`, `Size.Fixed(1)` | [`Divider::new`], [`Size::Fixed`]        |
//! | `divider.get_horizontal()`, `append_size("left", s)`, `get_aspect()` | [`Divider::horizontal`], [`Divider::append_size`], [`Divider::aspect`]; [`AxesDivider::divider`] for an axes divider |
//! | `host_subplot(111)`, `host.twinx()`               | [`host_subplot`], [`HostAxes`]                 |
//! | `AnchoredSizeBar`, `AnchoredDirectionArrows`      | [`crate::axes_grid1::anchored_artists`]        |
//! | `fig.add_subplot(axes_class=axisartist.Axes)`     | [`axisartist::AxisArtistAxes::new`](crate::axisartist::AxisArtistAxes::new)`(fig.subplot())` |
//! | `axes_class=AxesZero`                             | [`axisartist::AxisArtistAxes::zero`](crate::axisartist::AxisArtistAxes::zero) |
//! | `ax.axis["right"].set_visible(False)`             | `ax.axis("right")` ([`axisartist::AxisArtistAxes::axis`](crate::axisartist::AxisArtistAxes::axis)) and [`ArtistHandle::set_visible`] |
//! | `ax.axis["x"].set_axisline_style("-\|>")`         | [`AxisArtist::set_axisline_style`]             |
//! | `ax.new_floating_axis(0, 0.5)`                    | [`axisartist::AxisArtistAxes::new_floating_axis`](crate::axisartist::AxisArtistAxes::new_floating_axis) |
//! | `GridHelperCurveLinear(tr)`                       | [`axisartist::GridHelper`](crate::axisartist::GridHelper) |
//! | `ax.get_aux_axes(tr)`, `grid_finder.get_grid_info(bbox)` | [`axisartist::AxisArtistAxes::aux_axes`](crate::axisartist::AxisArtistAxes::aux_axes), [`GridFinder::grid_info`](crate::axisartist::grid_finder::GridFinder::grid_info) |
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
//! use pltrs::colors::{Colorize, ColorizerOptions, colormaps};
//! use pltrs::mplot3d::{Axis3, FaceColors, SurfaceOptions};
//! use pltrs::numeric::{Grid, linspace, meshgrid};
//! use pltrs::figure::Figure;
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
//! use pltrs::figure::Figure;
//! use pltrs::geo::GeoProjection;
//! use pltrs::numeric::linspace;
//! use pltrs::polar::{Compass, ThetaDirection};
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
//! # Projections of your own
//!
//! ```python
//! class SkewXAxes(Axes):
//!     name = "skewx"
//!
//!     def _set_lim_and_transforms(self):
//!         super()._set_lim_and_transforms()
//!         skew = Affine2D().skew_deg(30, 0)
//!         self.transDataToAxes = self.transScale + self.transLimits + skew
//!         self.transData = self.transDataToAxes + self.transAxes
//!         self._xaxis_transform = (blended_transform_factory(
//!             self.transScale + self.transLimits, IdentityTransform())
//!             + skew + self.transAxes)
//!
//! register_projection(SkewXAxes)
//! ax = fig.add_subplot(projection="skewx")
//! ax.semilogy([10, -20], [1000, 100])
//! ```
//!
//! ```
//! use pltrs::axes::{AxisName, ProjectionKind};
//! use pltrs::prelude::*;
//! use pltrs::projections::{self, Projection, ProjectionView};
//! use pltrs::scale::ScaleName;
//! use pltrs::transforms::{Affine2D, ResolvedTransform};
//!
//! #[derive(Clone, Debug, Default)]
//! struct SkewXAxes;
//!
//! fn skew() -> ResolvedTransform {
//!     ResolvedTransform::affine(Affine2D::identity().skew_deg(30.0, 0.0))
//! }
//!
//! impl Projection for SkewXAxes {
//!     fn name(&self) -> &'static str {
//!         "skewx"
//!     }
//!
//!     fn data_to_axes(&self, view: &ProjectionView<'_>) -> ResolvedTransform {
//!         view.rectilinear().then(skew())
//!     }
//!
//!     fn axis_transform(&self, view: &ProjectionView<'_>, axis: AxisName) -> Option<ResolvedTransform> {
//!         let along_x = ResolvedTransform::blend(view.rectilinear(), ResolvedTransform::identity());
//!         (axis == AxisName::X).then(|| along_x.then(skew()))
//!     }
//! }
//!
//! projections::register(SkewXAxes::default)?;
//! let fig = Figure::new();
//! let ax = fig.subplot_with_projection(ProjectionKind::Registered("skewx"))?;
//! ax.semilogy([10.0, -20.0], [1000.0, 100.0])?;
//! assert_eq!((ax.projection().name(), ax.yscale()), ("skewx", ScaleName::Log));
//! assert!(projections::projection_names().contains(&"skewx"));
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
//! use pltrs::axes::ImshowOptions;
//! use pltrs::axes_grid1::{
//!     GridRect, ImageGrid, ImageGridOptions, InsetOptions, SizeSpec, inset_axes, make_axes_locatable,
//! };
//! use pltrs::colorbar::{ColorbarAxes, ColorbarOptions};
//! use pltrs::numeric::Grid;
//! use pltrs::offsetbox::AnchorLoc;
//! use pltrs::prelude::*;
//! use pltrs::spines::SpineSide;
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
//! use pltrs::axisartist;
//! use pltrs::prelude::*;
//!
//! let fig = Figure::new();
//! let ax = axisartist::AxisArtistAxes::zero(fig.subplot());
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
//! [`Axis3D::ticklines`]: crate::mplot3d::Axis3D::ticklines
//! [`Axis3D::gridlines`]: crate::mplot3d::Axis3D::gridlines
//! [`Axis3D::convert_units`]: crate::mplot3d::Axis3D::convert_units
//! [`Axis3D::update_units`]: crate::mplot3d::Axis3D::update_units
//! [`Axis3D::set_units`]: crate::mplot3d::Axis3D::set_units
//! [`Axis3D::units`]: crate::mplot3d::Axis3D::units
//! [`Axis3D::set_converter`]: crate::mplot3d::Axis3D::set_converter
//! [`Axis3D::converter`]: crate::mplot3d::Axis3D::converter
//! [`Axis3D::axis_date`]: crate::mplot3d::Axis3D::axis_date
//! [`DateConverter::Concise`]: crate::units::DateConverter::Concise
//! [`TickLine3D`]: crate::mplot3d::TickLine3D
//! [`GridLine3D`]: crate::mplot3d::GridLine3D
//! [`Curve3D`]: crate::mplot3d::Curve3D
//! [`Boxes::new`]: crate::mplot3d::Boxes::new
//! [`Axes3D::fill_between`]: crate::mplot3d::Axes3D::fill_between
//! [`Axes3D::set_limits_with_margin`]: crate::mplot3d::Axes3D::set_limits_with_margin
//! [`Axes3D::set_bound_with_margin`]: crate::mplot3d::Axes3D::set_bound_with_margin
//! [`Axes3D::set_sort_zpos`]: crate::mplot3d::Axes3D::set_sort_zpos
//! [`Axes3D::sort_zpos`]: crate::mplot3d::Axes3D::sort_zpos
//! [`Axes3D::set_segments`]: crate::mplot3d::Axes3D::set_segments
//! [`Axes3D::set_poly_verts`]: crate::mplot3d::Axes3D::set_poly_verts
//! [`Axes3D::set_poly_verts_and_codes`]: crate::mplot3d::Axes3D::set_poly_verts_and_codes
//! [`Axes3D::set_poly_3d_properties`]: crate::mplot3d::Axes3D::set_poly_3d_properties
//! [`Axes3D::add_collection3d`]: crate::mplot3d::Axes3D::add_collection3d
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
//! [`Projection`]: crate::projections::Projection
//! [`Projection::data_to_axes`]: crate::projections::Projection::data_to_axes
//! [`Projection::axis_transform`]: crate::projections::Projection::axis_transform
//! [`Projection::set_up`]: crate::projections::Projection::set_up
//! [`Projection::outline`]: crate::projections::Projection::outline
//! [`Projection::start_pan`]: crate::projections::Projection::start_pan
//! [`Projection::drag_pan`]: crate::projections::Projection::drag_pan
//! [`Projection::end_pan`]: crate::projections::Projection::end_pan
//! [`Projection::zoom_to_box`]: crate::projections::Projection::zoom_to_box
//! [`register`]: crate::projections::register
//! [`register_with`]: crate::projections::register_with
//! [`projection_names`]: crate::projections::projection_names
//! [`ProjectionKind`]: crate::axes::ProjectionKind
//! [`ProjectionKind::with_options`]: crate::axes::ProjectionKind::with_options
//! [`Figure::subplot_with_projection`]: crate::figure::Figure::subplot_with_projection
//! [`Figure::add_subplot_with_projection`]: crate::figure::Figure::add_subplot_with_projection
//! [`Figure::add_axes_with_projection`]: crate::figure::Figure::add_axes_with_projection
//! [`Figure::subplots_with_projection`]: crate::figure::Figure::subplots_with_projection
//! [`MosaicOptions::per_subplot`]: crate::figure::MosaicOptions::per_subplot
//! [`Axes::inset_axes_with_projection`]: crate::axes::Axes::inset_axes_with_projection
//! [`Axes::twin_with_projection`]: crate::axes::Axes::twin_with_projection
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
//! [`Axes3D::set_line_data`]: crate::mplot3d::Axes3D::set_line_data
//! [`Axes3D::line_data`]: crate::mplot3d::Axes3D::line_data
//! [`Axes3D::set_scatter_offsets`]: crate::mplot3d::Axes3D::set_scatter_offsets
//! [`Axes3D::scatter_offsets`]: crate::mplot3d::Axes3D::scatter_offsets
//! [`Axes3D::depthshade`]: crate::mplot3d::Axes3D::depthshade
//! [`Axes3D::text_position`]: crate::mplot3d::Axes3D::text_position
//! [`Axes3D::set_text_position`]: crate::mplot3d::Axes3D::set_text_position
//! [`Axes3D::set_title_at`]: crate::mplot3d::Axes3D::set_title_at
//! [`Axes3D::title_at`]: crate::mplot3d::Axes3D::title_at
//! [`Axes3D::rotate_label`]: crate::mplot3d::Axes3D::rotate_label
//! [`Divider::horizontal`]: crate::axes_grid1::Divider::horizontal
//! [`Divider::append_size`]: crate::axes_grid1::Divider::append_size
//! [`Divider::aspect`]: crate::axes_grid1::Divider::aspect
//! [`AxesDivider::divider`]: crate::axes_grid1::AxesDivider::divider
//! [`Axis3D::set_ticks`]: crate::mplot3d::Axis3D::set_ticks
//! [`Axes3D::set_margins`]: crate::mplot3d::Axes3D::set_margins
//! [`Axes3D::set_margin`]: crate::mplot3d::Axes3D::set_margin
//! [`Axes3D::margin`]: crate::mplot3d::Axes3D::margin
//! [`Axes3D::autoscale_on`]: crate::mplot3d::Axes3D::autoscale_on
//! [`Axes3D::set_axis_autoscale_on`]: crate::mplot3d::Axes3D::set_axis_autoscale_on
