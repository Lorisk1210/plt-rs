//! Chapter 11: extending pltrs with your own types.
//!
//! You extend pltrs with traits. Implement [`Artist`] for a new thing to
//! draw and [`Projection`] for a new kind of axes. Implement
//! [`ConversionInterface`] for values of your own types and
//! [`LegendHandler`] for a new legend entry.
//! Toolbar tools are closures. The linked pages have complete examples.
//!
//! # Artists
//!
//! An artist holds an [`ArtistBase`] with the shared properties and draws
//! itself in [`Artist::draw`]. [`Axes::add_custom_artist`] adds it and
//! returns a [`Custom`] handle, which reads or changes your artist through
//! closures.
//!
//! The transform of the artist (data coordinates in an axes) maps its
//! points to pixels. The [`Renderer`] draws paths with a
//! [`GraphicsContext`]: color, width, dashes, alpha and clipping.
//! [`DrawContext::base_gc`] sets the shared properties. The others start
//! at black and 1 point wide. Send a problem, such as a bad color, to
//! [`DrawContext::report`]. pltrs then draws everything else, and the
//! drawing fails with the problems. This bracket marks a range of y, with
//! arms of a fixed length in pixels:
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
//! [`Custom::get`] returns a copy of your artist, so the artist derives
//! `Clone`. [`Custom::update`] changes a copy and puts it back. Your code
//! does not borrow the figure, so it can use any handle. Optional methods
//! make an artist take space in the layout ([`Artist::window_extent`]),
//! react to the mouse ([`Artist::contains`]) or widen the data limits
//! ([`Artist::limit_points`]). They run during drawing or layout, so they
//! get their data as arguments and must not use other handles.
//!
//! # Projections
//!
//! A projection maps data coordinates to the axes rectangle, as polar axes
//! do. Implement [`Projection`] and
//! [`register`](crate::projections::register) it under a name. Then
//! [`Figure::subplot_with_projection`] makes axes with it, and all
//! plotting methods work. The trait has hooks for the data transform, the
//! tick transforms, the outline, the spines, pan and zoom. The hooks get
//! their data as arguments. A projection holds settings, not handles, and
//! is `Send`. [`porting::toolkits`](crate::porting::toolkits) builds
//! skewed axes, and the gallery entry `custom_projection` in
//! `projections.rs` uses them.
//!
//! # Values of your own types
//!
//! Plotting methods also take your own types if they have a *unit
//! converter*. Implement [`ConversionInterface`] for the converter. It
//! converts values to numbers and can give the axis a label and tick
//! helpers. Then implement [`Convertible`] for the value type. Lists of
//! the values then convert into [`AxisValues`](crate::units::AxisValues).
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
//! With a converter for real units, such as meters and feet, an axis can
//! change its unit. [`ConversionInterface`] shows one, and the gallery
//! entry `custom_units` in `units.rs` uses it.
//!
//! # Legend entries
//!
//! [`LegendHandle::line`](crate::legend::LegendHandle::line) and
//! [`LegendHandle::patch`](crate::legend::LegendHandle::patch) make legend
//! entries from properties only, without an artist.
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
//! To draw an entry in a new way, implement [`LegendHandler`]. Its
//! documentation draws an ellipse for circles, like the matplotlib legend
//! guide.
//!
//! # Toolbar tools
//!
//! [`Figure::add_tool`] adds a toolbar button with a keyboard shortcut that
//! calls your closure. A toggle tool stays pressed and gives the closure
//! its state. [`Figure::trigger_tool`] runs a tool without a window.
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
//! scales and style sheets (chapter [`styling`](super::styling)) also
//! register by name.
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
