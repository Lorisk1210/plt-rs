//! Projections defined by the program (matplotlib's `matplotlib.projections`
//! and `register_projection`).
//!
//! A [`Projection`] gives axes a coordinate system of its own: how data
//! maps onto the axes box, where gridlines, ticks and their labels go, the
//! shape of the background and frame, how limits, scales and the aspect
//! behave, and what the navigation tools and the status bar do. The
//! built-in polar, geographic and rectilinear axes are the models: a
//! projection can draw a map inside an ellipse, or skew its x axis like a
//! SkewT-logP diagram, with ordinary plotting methods on top.
//!
//! * [`Projection`] — the trait to implement, one hook per matplotlib
//!   method a projection subclass overrides; only the name and the data
//!   transform are required.
//! * [`ProjectionView`] — the view limits and scales the hooks build
//!   their transforms from.
//! * [`AxesOutline`] and [`TickLabelPlacement`] — the shapes and label
//!   positions hooks return; [`PanStart`], [`ZoomBox`] and [`FrozenView`]
//!   what the pan and zoom hooks are given, [`ViewLimits`] what they
//!   answer.
//! * [`register`], [`register_with`] and [`projection_names`] — the
//!   per-thread registry of projections by name;
//!   [`ProjectionKind::Registered`] names one.
//! * [`ProjectionSpec`] — a projection kind with the
//!   [`ProjectionOptions`] a registered one is made with
//!   ([`ProjectionKind::with_options`]), which every constructor below
//!   takes (matplotlib's `projection=name, **kwargs`).
//! * [`Figure::subplot_with_projection`], [`Figure::add_subplot_with_projection`],
//!   [`Figure::add_subplot_spec_with_projection`],
//!   [`Figure::add_axes_with_projection`],
//!   [`Figure::subplots_with_projection`],
//!   [`Figure::subplot_mosaic`] (through [`MosaicOptions`]),
//!   [`Axes::inset_axes_with_projection`] and
//!   [`Axes::twin_with_projection`] — axes of any [`ProjectionKind`],
//!   built-in or registered, as do pyplot's
//!   [`subplot_with_projection`](crate::pyplot::subplot_with_projection),
//!   [`subplots_with_projection`](crate::pyplot::subplots_with_projection)
//!   and [`axes_with_projection`](crate::pyplot::axes_with_projection).
//! * [`Axes::with_projection`] and [`Axes::with_projection_mut`] — the
//!   projection of an axes, for settings of its own.
//!
//! Axes drawn through a projection draw in every backend, take part in
//! tight and constrained layouts, are picked over their outline, pan and
//! zoom through the inverse of their transform unless the projection
//! refuses or does it itself, and report coordinates in the status bar.
//!
//! # Example
//!
//! Axes whose vertical lines lean, as in a SkewT-logP diagram:
//!
//! ```
//! use pltrs::axes::{AxisName, ProjectionKind};
//! use pltrs::prelude::*;
//! use pltrs::projections::{self, Projection, ProjectionView};
//! use pltrs::transforms::{Affine2D, Point, ResolvedTransform};
//!
//! /// Rectilinear axes leaning `degrees` to the right.
//! #[derive(Clone, Debug)]
//! struct Leaning {
//!     degrees: f64,
//! }
//!
//! impl Leaning {
//!     fn lean(&self) -> ResolvedTransform {
//!         ResolvedTransform::affine(Affine2D::identity().skew_deg(self.degrees, 0.0))
//!     }
//! }
//!
//! impl Projection for Leaning {
//!     fn name(&self) -> &'static str {
//!         "leaning"
//!     }
//!
//!     fn data_to_axes(&self, view: &ProjectionView<'_>) -> ResolvedTransform {
//!         view.rectilinear().then(self.lean())
//!     }
//!
//!     /// Gridlines of the x axis lean with the data; those of the y axis
//!     /// stay level.
//!     fn axis_transform(&self, view: &ProjectionView<'_>, axis: AxisName) -> Option<ResolvedTransform> {
//!         let along_x = ResolvedTransform::blend(view.rectilinear(), ResolvedTransform::identity());
//!         (axis == AxisName::X).then(|| along_x.then(self.lean()))
//!     }
//!
//!     fn set_up(&mut self, axes: &Axes) -> Result<(), Box<dyn std::error::Error>> {
//!         axes.grid(true);
//!         Ok(())
//!     }
//! }
//!
//! projections::register(|| Leaning { degrees: 30.0 })?;
//! let fig = Figure::new();
//! let ax = fig.subplot_with_projection(ProjectionKind::Registered("leaning"))?;
//! ax.plot([0.0, 1.0], [0.0, 1.0])?;
//! ax.set_xlim(0.0, 1.0)?.set_ylim(0.0, 1.0)?;
//! assert_eq!(ax.projection().name(), "leaning");
//!
//! // The top of the data leans to the right of the top of the box.
//! let top_left = fig.resolve_transform(&ax.trans_axes())?.transform_point(Point::new(0.0, 1.0));
//! let data = fig.resolve_transform(&ax.trans_data())?;
//! assert!(data.transform_point(Point::new(0.0, 1.0)).x > top_left.x + 100.0);
//!
//! // Settings live in the projection of each axes.
//! ax.with_projection_mut(|leaning: &mut Leaning| leaning.degrees = 0.0);
//! let data = fig.resolve_transform(&ax.trans_data())?;
//! assert!((data.transform_point(Point::new(0.0, 1.0)).x - top_left.x).abs() < 1e-9);
//! assert!(fig.to_svg()?.contains("<svg"));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`Figure::subplot_with_projection`]: crate::figure::Figure::subplot_with_projection
//! [`Figure::add_subplot_with_projection`]: crate::figure::Figure::add_subplot_with_projection
//! [`Figure::add_subplot_spec_with_projection`]: crate::figure::Figure::add_subplot_spec_with_projection
//! [`Figure::add_axes_with_projection`]: crate::figure::Figure::add_axes_with_projection
//! [`Figure::subplots_with_projection`]: crate::figure::Figure::subplots_with_projection
//! [`Figure::subplot_mosaic`]: crate::figure::Figure::subplot_mosaic
//! [`MosaicOptions`]: crate::figure::MosaicOptions
//! [`Axes::inset_axes_with_projection`]: crate::axes::Axes::inset_axes_with_projection
//! [`Axes::twin_with_projection`]: crate::axes::Axes::twin_with_projection
//! [`Axes::with_projection`]: crate::axes::Axes::with_projection
//! [`Axes::with_projection_mut`]: crate::axes::Axes::with_projection_mut
//! [`ProjectionKind`]: crate::axes::ProjectionKind
//! [`ProjectionKind::Registered`]: crate::axes::ProjectionKind::Registered
//! [`ProjectionKind::with_options`]: crate::axes::ProjectionKind::with_options

mod create;
mod error;
mod hooks;
mod navigate;
mod projection;
mod registry;
mod spec;
#[cfg(test)]
mod tests;
mod view;

pub(crate) use create::{OUTLINE_SPINE, Projector, install, reinstall};
pub use error::ProjectionError;
pub use navigate::{FrozenView, PanStart, ViewLimits, ZoomBox};
pub use projection::{AxesOutline, Projection, TickLabelPlacement};
pub(crate) use registry::registered_name;
pub use registry::{projection_names, register, register_with};
pub(crate) use spec::SharedOptions;
pub use spec::{ProjectionOptions, ProjectionSpec};
pub use view::ProjectionView;
