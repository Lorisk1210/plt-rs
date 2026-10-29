//! Chapter 11: extending pltrs with types of your own.
//!
//! pltrs is extended through traits: implement [`Artist`] for something
//! new to draw, [`Projection`] for a new kind of axes,
//! [`ConversionInterface`] to plot values of your own types, and
//! [`LegendHandler`] for a new kind of legend entry. Toolbar tools are
//! closures. This chapter shows the shape of each; the linked pages have
//! complete examples.
//!
//! # Artists
//!
//! An artist is a type implementing [`Artist`]: it holds an
//! [`ArtistBase`] with the properties all artists share, and draws itself
//! in [`Artist::draw`]. [`Axes::add_custom_artist`] adds it and returns a
//! [`Custom`] handle, which has the shared properties of every handle and
//! reads or changes your artist through closures.
//!
//! Drawing happens in display coordinates (pixels). The artist's
//! transform, data coordinates when it was added to an axes, maps its
//! points there; then the [`Renderer`] draws paths with a
//! [`GraphicsContext`], which carries the colour, width, dashes, alpha
//! and clipping. [`DrawContext::base_gc`] fills in what all artists
//! share (alpha, clipping, ...) and leaves the rest at black, 1 point
//! wide, for the artist to set from fields of its own. A problem found
//! while drawing, such as a colour that does not parse, goes to
//! [`DrawContext::report`]: the figure draws everything else and the
//! drawing fails with the problems found. Here, a bracket marking a
//! range of y at one x, with arms of a fixed length in pixels whatever
//! the zoom:
//!
//! ```
//! use pltrs::artist::{Artist, ArtistBase, DrawContext};
//! use pltrs::colors::Color;
//! use pltrs::path::Path;
//! use pltrs::prelude::*;
//! use pltrs::transforms::{Affine2D, Point};
//!
//! /// A bracket from `low` to `high` at `x`, in data coordinates.
//! #[derive(Clone, Debug)]
//! struct Bracket {
//!     base: ArtistBase,
//!     x: f64,
//!     low: f64,
//!     high: f64,
//!     color: Color,
//! }
//!
//! impl Artist for Bracket {
//!     fn base(&self) -> &ArtistBase {
//!         &self.base
//!     }
//!
//!     fn base_mut(&mut self) -> &mut ArtistBase {
//!         &mut self.base
//!     }
//!
//!     fn draw(&self, ctx: &mut DrawContext<'_>) {
//!         let Some(to_pixels) = self.base.transform.as_ref().and_then(|t| ctx.resolve(t)) else {
//!             return;
//!         };
//!         let bottom = to_pixels.transform_point(Point::new(self.x, self.low));
//!         let top = to_pixels.transform_point(Point::new(self.x, self.high));
//!         let arm = 6.0;
//!         let outline = Path::polyline(vec![
//!             Point::new(bottom.x + arm, bottom.y),
//!             bottom,
//!             top,
//!             Point::new(top.x + arm, top.y),
//!         ]);
//!         let mut gc = ctx.base_gc(&self.base);
//!         match self.color.resolve() {
//!             Ok(rgba) => gc.foreground = rgba,
//!             Err(problem) => return ctx.report(self.base.id(), problem),
//!         }
//!         gc.linewidth = 1.5;
//!         ctx.renderer().draw_path(&gc, &outline, &Affine2D::IDENTITY, None);
//!     }
//! }
//!
//! let fig = Figure::new();
//! let ax = fig.subplot();
//! ax.plot([0.0, 1.0, 2.0], [1.0, 4.0, 2.0])?;
//! let bracket = ax.add_custom_artist(Bracket {
//!     base: ArtistBase::default(),
//!     x: 1.2,
//!     low: 1.0,
//!     high: 4.0,
//!     color: "tab:red".into(),
//! });
//! bracket.update(|bracket| bracket.low = 2.0);
//! bracket.set_zorder(3.0);
//! let copy = bracket.get();
//! assert_eq!(copy.high - copy.low, 2.0);
//! fig.to_png()?;
//!
//! bracket.update(|bracket| bracket.color = "not a colour".into());
//! assert!(fig.to_png().is_err(), "reported while drawing");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`Custom::get`] returns a copy of your artist, which is why it derives
//! `Clone`, and [`Custom::update`] changes a copy and puts it back. The
//! figure is not borrowed while your code runs, so it may use any handle
//! meanwhile. The [`Artist`] trait has further methods with defaults,
//! for an artist that takes room in the layout ([`Artist::window_extent`]),
//! answers the mouse ([`Artist::contains`]) or widens its axes' data
//! limits ([`Artist::limit_points`]). They run while the figure is drawn
//! or laid out, so they get what they need as arguments and should leave
//! other handles alone.
//!
//! # Projections
//!
//! A projection decides how data coordinates map to the axes rectangle:
//! polar axes and map projections are projections. Implement
//! [`Projection`], [`register`](crate::projections::register) it under a
//! name, and make axes of it with
//! [`Figure::subplot_with_projection`]; every plotting method then works
//! on them. The trait has one hook for each part a projection may
//! change: the data transform, the transforms of the axes' ticks, the
//! outline and spines, panning and zooming. The hooks run while the
//! figure is laid out and drawn, so they get what they need as
//! arguments; a projection holds settings, not handles, and is `Send`.
//! The [`porting::toolkits`](crate::porting::toolkits) page builds skewed
//! axes, and `custom_projection` in `projections.rs` in
//! the gallery draws a weather sounding on them.
//!
//! # Values of your own types
//!
//! Plotting methods take numbers, names and dates (chapter
//! [`data`](super::data)), and also values of your types once they have
//! a *unit converter*. Implement [`ConversionInterface`] for a converter,
//! which turns values into axis numbers and may give the axis a label
//! and tick helpers, then [`Convertible`] for the value type, naming its
//! converter. Lists of the values then convert into
//! [`AxisValues`](crate::units::AxisValues) like any other data.
//!
//! ```
//! use pltrs::prelude::*;
//! use pltrs::ticker::StrMethodFormatter;
//! use pltrs::units::{ConversionError, ConversionInterface, Convertible, UnitAxis, UnitAxisInfo};
//!
//! /// An amount of money.
//! #[derive(Clone, Copy, Debug, PartialEq)]
//! struct Euros(f64);
//!
//! struct EuroConverter;
//!
//! impl ConversionInterface for EuroConverter {
//!     type Value = Euros;
//!     type Unit = ();
//!
//!     fn convert(
//!         &self,
//!         values: &[Euros],
//!         _: Option<&()>,
//!         _: &UnitAxis,
//!     ) -> Result<Vec<f64>, ConversionError> {
//!         Ok(values.iter().map(|euros| euros.0).collect())
//!     }
//!
//!     fn axisinfo(&self, _: Option<&()>, _: &UnitAxis) -> Option<UnitAxisInfo> {
//!         let format = StrMethodFormatter::new("{x:.2f} €").ok()?;
//!         Some(UnitAxisInfo::default().label("price").majfmt(format))
//!     }
//! }
//!
//! impl Convertible for Euros {
//!     type Converter = EuroConverter;
//!
//!     fn converter() -> EuroConverter {
//!         EuroConverter
//!     }
//! }
//!
//! let ax = Figure::new().subplot();
//! ax.plot(["mon", "tue", "wed"], vec![Euros(3.5), Euros(4.25), Euros(3.9)])?;
//! ax.set_yticks([3.5, 4.0]);
//! assert_eq!(ax.yaxis().label_text(), "price");
//! assert_eq!(ax.yaxis().ticklabels(false)?, ["3.50 €", "4.00 €"]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! A converter with real units (metres and feet, degrees Celsius and
//! Fahrenheit) lets an axis switch unit and its artists follow:
//! [`ConversionInterface`] shows one, and `custom_units` in `units.rs` in
//! the gallery draws it.
//!
//! # Legend entries
//!
//! A legend lists labelled artists, but an entry can stand for anything:
//! [`LegendHandle::line`](crate::legend::LegendHandle::line) and
//! [`LegendHandle::patch`](crate::legend::LegendHandle::patch) make
//! entries from properties alone, for things drawn in another way or not
//! at all.
//!
//! ```
//! use pltrs::legend::LegendHandle;
//! use pltrs::prelude::*;
//!
//! let ax = Figure::new().subplot();
//! let handles = [
//!     LegendHandle::patch(PatchProps::default().facecolor("tab:red").label("over budget")),
//!     LegendHandle::line(
//!         LineProps::default().color("0.5").linestyle(LineStyle::Dashed).label("target"),
//!     ),
//! ];
//! let legend = ax.legend(LegendOptions::default().handles(handles))?;
//! assert_eq!(legend.labels(), ["over budget", "target"]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! To draw an entry in a new way, implement [`LegendHandler`]: its
//! documentation draws an ellipse for circles, as matplotlib's legend
//! guide does.
//!
//! # Toolbar tools
//!
//! [`Figure::add_tool`] adds a button to the toolbar of the figure's
//! window, with a keyboard shortcut, calling your closure. A toggle tool
//! stays pressed and tells the closure whether it is.
//! [`Figure::trigger_tool`] runs a tool as a click would, so tools can be
//! tested without a window.
//!
//! ```
//! use pltrs::gui::{Icon, ToolKind, ToolSpec};
//! use pltrs::prelude::*;
//!
//! let fig = Figure::new();
//! fig.subplots(1, 2)?;
//! let spec = ToolSpec {
//!     name: "grid".into(),
//!     description: "Show the grids".into(),
//!     icon: Icon::new(24.0),
//!     keys: vec!["ctrl+g".into()],
//!     kind: ToolKind::Toggle,
//!     ..ToolSpec::default()
//! };
//! fig.add_tool(spec, |event| {
//!     for ax in event.figure.axes() {
//!         ax.grid(event.toggled);
//!     }
//! })?;
//! fig.trigger_tool("grid");
//! assert_eq!(fig.tool_toggled("grid"), Some(true));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Colormaps ([`colormaps::register`](crate::colors::colormaps::register)),
//! scales and style sheets (chapter [`styling`](super::styling)) are
//! registered by name, after which every call taking a name finds them.
//!
//! Next: [testing your plots](super::testing).
//!
//! [`Artist`]: crate::artist::Artist
//! [`Artist::draw`]: crate::artist::Artist::draw
//! [`Artist::window_extent`]: crate::artist::Artist::window_extent
//! [`Artist::contains`]: crate::artist::Artist::contains
//! [`Artist::limit_points`]: crate::artist::Artist::limit_points
//! [`ArtistBase`]: crate::artist::ArtistBase
//! [`DrawContext::base_gc`]: crate::artist::DrawContext::base_gc
//! [`DrawContext::report`]: crate::artist::DrawContext::report
//! [`GraphicsContext`]: crate::artist::GraphicsContext
//! [`Renderer`]: crate::artist::Renderer
//! [`Custom`]: crate::artist::Custom
//! [`Custom::get`]: crate::artist::Custom::get
//! [`Custom::update`]: crate::artist::Custom::update
//! [`Axes::add_custom_artist`]: crate::axes::Axes::add_custom_artist
//! [`Projection`]: crate::projections::Projection
//! [`Figure::subplot_with_projection`]: crate::figure::Figure::subplot_with_projection
//! [`ConversionInterface`]: crate::units::ConversionInterface
//! [`Convertible`]: crate::units::Convertible
//! [`LegendHandler`]: crate::legend::LegendHandler
//! [`Figure::add_tool`]: crate::figure::Figure::add_tool
//! [`Figure::trigger_tool`]: crate::figure::Figure::trigger_tool
