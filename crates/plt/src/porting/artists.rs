//! Artists and their properties.
//!
//! Every visible thing is an artist owned by its figure, and the values
//! you hold are handles to them (see [`super::conventions`]). The
//! properties every artist shares (visibility, alpha, z-order, label,
//! clipping, transform, picking, path effects, ...) come from the
//! [`ArtistHandle`] trait, which [`crate::prelude`] imports. Each artist
//! type adds its own: [`Line2D`] for lines, [`Patch`] for shapes,
//! [`Collection`] for batches of shapes, [`Text`] for texts,
//! [`AxesImage`] for images.
//!
//! matplotlib constructs an artist and then adds it (`ax.add_patch(
//! Rectangle(...))`); plt-rs adds a *shape* and returns the artist's
//! handle. A patch is a [`Patch<S>`](crate::patches::Patch) whose shape `S` is a plain
//! struct ([`Rectangle`], [`Circle`], [`Polygon`], [`Wedge`],
//! [`FancyBboxPatch`], ...): read it with [`Patch::shape`], change it with
//! [`Patch::update_shape`]. Its paint is set through a [`PatchProps`] when
//! it is added, or through the setters later.
//!
//! matplotlib's collection classes are constructors of one [`Collection`]
//! artist: [`CollectionSpec::lines`] is a `LineCollection`,
//! [`CollectionSpec::polygons`] a `PolyCollection`, and so on.
//!
//! Your own `Artist` subclasses are types implementing the [`Artist`]
//! trait, added with [`Axes::add_custom_artist`]; [`Custom`] shows a
//! complete example.
//!
//! # Equivalences
//!
//! | matplotlib                                     | plt-rs                                          |
//! |------------------------------------------------|-------------------------------------------------|
//! | `a.set_visible(False)`, `a.get_visible()`      | [`ArtistHandle::set_visible`], [`ArtistHandle::visible`] |
//! | `a.set_alpha(0.5)`, `a.set_alpha(None)`        | [`ArtistHandle::set_alpha`]`(Some(0.5))`, `(None)` |
//! | `a.set_zorder(3)`, `a.get_zorder()`            | [`ArtistHandle::set_zorder`], [`ArtistHandle::zorder`] |
//! | `a.set_label("x")`, `a.get_label()`            | [`ArtistHandle::set_label`], [`ArtistHandle::label`] |
//! | `a.set_clip_on(False)`                         | [`ArtistHandle::set_clip_on`]                   |
//! | `a.set_clip_path(patch)`                       | [`ArtistHandle::set_clip_path`]`(Some(ClipPathSpec::Artist(patch.id())))` |
//! | `a.set_transform(ax.transAxes)`                | [`ArtistHandle::set_transform`]`(ax.trans_axes())` |
//! | `a.set_rasterized(True)`                       | [`ArtistHandle::set_rasterized`]                |
//! | `a.set_picker(5)`, `a.set_picker(True)`        | [`ArtistHandle::set_picker`]`(Some(Picker::Radius(5.0)))`, `Picker::Default` |
//! | `a.set_path_effects([...])`                    | [`ArtistHandle::set_path_effects`]              |
//! | `a.set_sketch_params(scale, length, randomness)` | [`ArtistHandle::set_sketch_params`]`(Some(Sketch { .. }))` |
//! | `a.set_agg_filter(f)`                          | [`ArtistHandle::set_agg_filter`]                |
//! | `a.set_gid`, `a.set_url`                       | [`ArtistHandle::set_gid`], [`ArtistHandle::set_url`] |
//! | `a.set_animated(True)`, `a.set_in_layout(False)` | [`ArtistHandle::set_animated`], [`ArtistHandle::set_in_layout`] |
//! | `a.get_window_extent()`                        | [`ArtistHandle::window_extent`]                 |
//! | `a.contains(event)`                            | [`ArtistHandle::contains`]`(point)`             |
//! | `a.remove()`                                   | [`ArtistHandle::remove`]                        |
//! | `a.set(color=..., lw=...)`                     | `update_props`, e.g. [`Line2D::update_props`]   |
//! | `fig.findobj(Line2D)`, `ax.findobj(...)`       | [`ArtistHandle::findobj`] and [`RawHandle::downcast`] |
//! | `a.axes`, `a.figure`                           | [`ArtistHandle::axes`], [`ArtistHandle::figure`] |
//! | `isinstance(a, Line2D)`                        | `a.downcast::<Line2D>().is_some()`              |
//! | `ax.lines`, `ax.images`, `ax.containers`       | [`Axes::lines`], [`Axes::images`], [`Axes::containers`] |
//! | `ax.patches`, `ax.collections`, `ax.texts`     | [`Axes::patches`], [`Axes::collections`], [`Axes::texts`] |
//! | `ax.tables`, `ax.artists`                      | [`Axes::tables`], [`Axes::artists`]             |
//! | `ax.child_axes`                                | [`Axes::child_axes`]                            |
//! | `ax.add_line(Line2D(x, y, ...))`               | [`Axes::add_line`]`(x, y, props)`               |
//! | `ax.add_patch(Rectangle(...))`                 | [`Axes::add_patch`], [`Axes::add_patch_with`]   |
//! | `ax.add_artist(patch)`                         | [`Axes::add_artist`] (data limits untouched)    |
//! | `Rectangle(xy, w, h, angle=30)`                | [`Rectangle::new`]`(xy, w, h).rotated(30.0, RotationPoint::Xy)` |
//! | `Rectangle(xy, w, h)` of dates or user values  | [`DataRectangle::new`]`(xy, w, h)?`, converted by [`Axes::add_patch`] |
//! | `rect.set_width(3)`, `rect.get_width()`        | [`Patch::update_shape`], [`Patch::shape`]       |
//! | `patch.set_facecolor(c)`, `patch.set_hatch("//")` | [`Patch::set_facecolor`], [`Patch::set_hatch`] |
//! | `patch.get_facecolor()`                        | [`Patch::colors`] (resolved), [`Patch::style`] (as set) |
//! | `Circle`, `Ellipse`, `Polygon`, `Wedge`, `Arc`, `RegularPolygon`, `Annulus` | [`Circle`], [`Ellipse`], [`Polygon`], [`Wedge`], [`Arc`], [`RegularPolygon`], [`Annulus`] |
//! | `FancyBboxPatch`, `FancyArrowPatch`, `ConnectionPatch` | [`FancyBboxPatch`], [`FancyArrowPatch`], [`ConnectionPatch`] |
//! | `LineCollection`, `PolyCollection`             | [`CollectionSpec::lines`], [`CollectionSpec::polygons`] |
//! | `PatchCollection`, `EllipseCollection`, `CircleCollection` | [`CollectionSpec::patches`], [`CollectionSpec::ellipses`], [`CollectionSpec::circles`] |
//! | `RegularPolyCollection`, `EventCollection`     | [`CollectionSpec::regular_polygons`], [`CollectionSpec::events`] |
//! | `ax.add_collection(c)`                         | [`Axes::add_collection`]                        |
//! | `coll.set_array(values)`, `coll.set_offsets(xy)` | [`Collection::set_array`], [`Collection::set_offsets`] |
//! | `coll.set_array(None)`, `coll.get_segments()`  | [`Collection::clear_array`], [`Collection::segments`] |
//! | `coll.set_paths(paths)`, `polys.set_verts(verts)` | [`Collection::set_paths`], [`Collection::set_verts`]  |
//! | `coll.get_datalim(ax.transData)`               | [`Collection::datalim`]                         |
//! | `events.set_positions(p)`, `events.switch_orientation()` | [`EventCollection::set_positions`], [`EventCollection::switch_orientation`] (downcast the collection) |
//! | `ellipses.set_widths(w)`, `ellipses.get_angles()` | [`EllipseCollection::set_widths`], [`EllipseCollection::angles`] |
//! | `class MyArtist(Artist)`                       | `impl` [`Artist`], [`Axes::add_custom_artist`] → [`Custom`] |
//! | `a.update_from(other)`, `a.pchanged()`         | not needed: handles share state                 |
//!
//! # Common properties
//!
//! ```python
//! line, = ax.plot([0, 1], [0, 1], label="data")
//! line.set_alpha(0.5)
//! line.set_zorder(3)
//! line.set_visible(False)
//! assert line.get_label() == "data" and line.get_zorder() == 3
//! line.set(color="k", linewidth=0.5)
//! ```
//!
//! ```
//! use plt::colors::Color;
//! use plt::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! let line = ax.plot_with([0.0, 1.0], [0.0, 1.0], LineProps::default().label("data"))?;
//! line.set_alpha(Some(0.5)).set_zorder(3.0).set_visible(false);
//! assert_eq!(line.label(), "data");
//! assert_eq!(line.zorder(), 3.0);
//! assert_eq!(line.alpha(), Some(0.5));
//! assert!(!line.visible());
//!
//! line.update_props(LineProps::default().color("k").linewidth(0.5));
//! assert_eq!(line.color(), Color::from("k"));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Patches
//!
//! ```python
//! rect = ax.add_patch(Rectangle((0, 0), 2, 1, facecolor="tab:blue",
//!                               edgecolor="k", hatch="//"))
//! rect.set_width(3)
//! circle = ax.add_patch(Circle((1, 1), 0.5, fill=False))
//! ax.add_patch(Polygon([[0, 0], [1, 0], [0, 1]], closed=True))
//! ```
//!
//! ```
//! use plt::patches::{Circle, PatchProps, Polygon, Rectangle};
//! use plt::prelude::*;
//! use plt::transforms::Point;
//!
//! let ax = Figure::new().subplot();
//! let paint = PatchProps::default().facecolor("tab:blue").edgecolor("k").hatch("//");
//! let rect = ax.add_patch_with(Rectangle::new((0.0, 0.0), 2.0, 1.0), paint);
//! rect.update_shape(|r| r.width = 3.0);
//! assert_eq!(rect.shape().width, 3.0);
//! assert_eq!(rect.style().hatch.as_deref(), Some("//"));
//!
//! let circle = ax.add_patch_with(Circle::new((1.0, 1.0), 0.5), PatchProps::default().fill(false));
//! assert!(!circle.style().fill);
//! let corners = [Point::new(0.0, 0.0), Point::new(1.0, 0.0), Point::new(0.0, 1.0)];
//! let triangle = ax.add_patch(Polygon::new(corners, true));
//! // As in matplotlib's `get_xy`, a closed polygon repeats its first vertex.
//! assert_eq!(triangle.shape().vertices().len(), 4);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Collections
//!
//! ```python
//! from matplotlib.collections import LineCollection
//! lc = LineCollection([[(0, 0), (1, 1)], [(0, 1), (1, 0)]],
//!                     colors=["tab:red", "tab:blue"], linewidths=2)
//! ax.add_collection(lc)
//! ax.autoscale_view()
//! ```
//!
//! ```
//! use plt::collections::{CollectionProps, CollectionSpec, EdgeColors};
//! use plt::prelude::*;
//! use plt::transforms::Point;
//!
//! let ax = Figure::new().subplot();
//! let segments = vec![
//!     vec![Point::new(0.0, 0.0), Point::new(1.0, 1.0)],
//!     vec![Point::new(0.0, 1.0), Point::new(1.0, 0.0)],
//! ];
//! let props = CollectionProps {
//!     edgecolors: Some(EdgeColors::Colors(vec!["tab:red".into(), "tab:blue".into()])),
//!     linewidths: Some(vec![2.0]),
//!     ..CollectionProps::default()
//! };
//! let lc = ax.add_collection(CollectionSpec::lines(segments).with_props(props));
//! ax.autoscale_view(None);
//! assert_eq!(lc.paths().len(), 2);
//! assert_eq!(lc.style().linewidths, Some(vec![2.0]));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Finding artists, clipping and z-order
//!
//! ```python
//! circle = ax.add_patch(Circle((0.5, 0.5), 0.3, transform=ax.transAxes, visible=False))
//! line, = ax.plot([0, 1], [0, 1], zorder=3)
//! line.set_clip_path(circle)
//! assert fig.findobj(Line2D) == [line]
//! assert line.axes is ax
//! ```
//!
//! ```
//! use plt::artist::ClipPathSpec;
//! use plt::patches::{Circle, PatchProps};
//! use plt::prelude::*;
//!
//! let fig = Figure::new();
//! let ax = fig.subplot();
//! let hidden = PatchProps::default().transform(ax.trans_axes()).visible(false);
//! let circle = ax.add_patch_with(Circle::new((0.5, 0.5), 0.3), hidden);
//! let line = ax.plot_with([0.0, 1.0], [0.0, 1.0], LineProps::default().zorder(3.0))?;
//! line.set_clip_path(Some(ClipPathSpec::Artist(circle.id())));
//!
//! let lines: Vec<Line2D> = fig.findobj(|_| true).iter().filter_map(|a| a.downcast()).collect();
//! assert_eq!(lines, [line.clone()]);
//! assert_eq!(line.axes(), Some(ax));
//! fig.to_png()?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`ArtistHandle`]: crate::artist::ArtistHandle
//! [`ArtistHandle::set_visible`]: crate::artist::ArtistHandle::set_visible
//! [`ArtistHandle::visible`]: crate::artist::ArtistHandle::visible
//! [`ArtistHandle::set_alpha`]: crate::artist::ArtistHandle::set_alpha
//! [`ArtistHandle::set_zorder`]: crate::artist::ArtistHandle::set_zorder
//! [`ArtistHandle::zorder`]: crate::artist::ArtistHandle::zorder
//! [`ArtistHandle::set_label`]: crate::artist::ArtistHandle::set_label
//! [`ArtistHandle::label`]: crate::artist::ArtistHandle::label
//! [`ArtistHandle::set_clip_on`]: crate::artist::ArtistHandle::set_clip_on
//! [`ArtistHandle::set_clip_path`]: crate::artist::ArtistHandle::set_clip_path
//! [`ArtistHandle::set_transform`]: crate::artist::ArtistHandle::set_transform
//! [`ArtistHandle::set_rasterized`]: crate::artist::ArtistHandle::set_rasterized
//! [`ArtistHandle::set_picker`]: crate::artist::ArtistHandle::set_picker
//! [`ArtistHandle::set_path_effects`]: crate::artist::ArtistHandle::set_path_effects
//! [`ArtistHandle::set_sketch_params`]: crate::artist::ArtistHandle::set_sketch_params
//! [`ArtistHandle::set_agg_filter`]: crate::artist::ArtistHandle::set_agg_filter
//! [`ArtistHandle::set_gid`]: crate::artist::ArtistHandle::set_gid
//! [`ArtistHandle::set_url`]: crate::artist::ArtistHandle::set_url
//! [`ArtistHandle::set_animated`]: crate::artist::ArtistHandle::set_animated
//! [`ArtistHandle::set_in_layout`]: crate::artist::ArtistHandle::set_in_layout
//! [`ArtistHandle::window_extent`]: crate::artist::ArtistHandle::window_extent
//! [`ArtistHandle::contains`]: crate::artist::ArtistHandle::contains
//! [`ArtistHandle::remove`]: crate::artist::ArtistHandle::remove
//! [`Artist`]: crate::artist::Artist
//! [`Custom`]: crate::artist::Custom
//! [`RawHandle::downcast`]: crate::artist::RawHandle::downcast
//! [`Line2D`]: crate::lines::Line2D
//! [`Line2D::update_props`]: crate::lines::Line2D::update_props
//! [`Text`]: crate::text::Text
//! [`AxesImage`]: crate::image::AxesImage
//! [`ArtistHandle::findobj`]: crate::artist::ArtistHandle::findobj
//! [`ArtistHandle::axes`]: crate::artist::ArtistHandle::axes
//! [`ArtistHandle::figure`]: crate::artist::ArtistHandle::figure
//! [`Axes::patches`]: crate::axes::Axes::patches
//! [`Axes::collections`]: crate::axes::Axes::collections
//! [`Axes::texts`]: crate::axes::Axes::texts
//! [`Axes::tables`]: crate::axes::Axes::tables
//! [`Axes::artists`]: crate::axes::Axes::artists
//! [`Axes::lines`]: crate::axes::Axes::lines
//! [`Axes::images`]: crate::axes::Axes::images
//! [`Axes::containers`]: crate::axes::Axes::containers
//! [`Axes::child_axes`]: crate::axes::Axes::child_axes
//! [`Axes::add_line`]: crate::axes::Axes::add_line
//! [`Axes::add_patch`]: crate::axes::Axes::add_patch
//! [`Axes::add_patch_with`]: crate::axes::Axes::add_patch_with
//! [`Axes::add_artist`]: crate::axes::Axes::add_artist
//! [`Axes::add_collection`]: crate::axes::Axes::add_collection
//! [`Axes::add_custom_artist`]: crate::axes::Axes::add_custom_artist
//! [`Patch`]: crate::patches::Patch
//! [`Patch::shape`]: crate::patches::Patch::shape
//! [`Patch::update_shape`]: crate::patches::Patch::update_shape
//! [`Patch::set_facecolor`]: crate::patches::Patch::set_facecolor
//! [`Patch::set_hatch`]: crate::patches::Patch::set_hatch
//! [`Patch::colors`]: crate::patches::Patch::colors
//! [`Patch::style`]: crate::patches::Patch::style
//! [`PatchProps`]: crate::patches::PatchProps
//! [`Rectangle`]: crate::patches::Rectangle
//! [`Rectangle::new`]: crate::patches::Rectangle::new
//! [`DataRectangle::new`]: crate::patches::DataRectangle::new
//! [`Circle`]: crate::patches::Circle
//! [`Ellipse`]: crate::patches::Ellipse
//! [`Polygon`]: crate::patches::Polygon
//! [`Wedge`]: crate::patches::Wedge
//! [`Arc`]: crate::patches::Arc
//! [`RegularPolygon`]: crate::patches::RegularPolygon
//! [`Annulus`]: crate::patches::Annulus
//! [`FancyBboxPatch`]: crate::patches::FancyBboxPatch
//! [`FancyArrowPatch`]: crate::patches::FancyArrowPatch
//! [`ConnectionPatch`]: crate::patches::ConnectionPatch
//! [`Collection`]: crate::collections::Collection
//! [`Collection::set_array`]: crate::collections::Collection::set_array
//! [`Collection::set_offsets`]: crate::collections::Collection::set_offsets
//! [`Collection::clear_array`]: crate::collections::Collection::clear_array
//! [`Collection::segments`]: crate::collections::Collection::segments
//! [`Collection::datalim`]: crate::collections::Collection::datalim
//! [`EventCollection::set_positions`]: crate::collections::EventCollection::set_positions
//! [`EventCollection::switch_orientation`]: crate::collections::EventCollection::switch_orientation
//! [`EllipseCollection::set_widths`]: crate::collections::EllipseCollection::set_widths
//! [`EllipseCollection::angles`]: crate::collections::EllipseCollection::angles
//! [`CollectionSpec::lines`]: crate::collections::CollectionSpec::lines
//! [`CollectionSpec::polygons`]: crate::collections::CollectionSpec::polygons
//! [`CollectionSpec::patches`]: crate::collections::CollectionSpec::patches
//! [`CollectionSpec::ellipses`]: crate::collections::CollectionSpec::ellipses
//! [`CollectionSpec::circles`]: crate::collections::CollectionSpec::circles
//! [`CollectionSpec::regular_polygons`]: crate::collections::CollectionSpec::regular_polygons
//! [`CollectionSpec::events`]: crate::collections::CollectionSpec::events
//! [`Collection::set_paths`]: crate::collections::Collection::set_paths
//! [`Collection::set_verts`]: crate::collections::Collection::set_verts
