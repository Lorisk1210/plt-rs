//! 3-D plotting (matplotlib's `mpl_toolkits.mplot3d`).
//!
//! An [`Axes3D`] — created with [`Figure::subplot_3d`] and friends —
//! projects data from a 3-D box onto the screen through a camera
//! ([`ViewAngles`], [`ProjType`], box aspect) and draws the box's panes,
//! gridlines and axes around it.
//!
//! Like matplotlib, every 3-D artist is a 2-D artist drawn in the
//! projected coordinates of the axes (its *projection plane*) plus a 3-D
//! geometry. Before each draw the layout pass projects the geometry onto
//! the 2-D artist; collections and patches are then ordered by depth
//! ("computed z-order") so that nearer ones are drawn on top, and the
//! polygons and markers inside a collection are sorted back to front.
//!
//! Each axis has a scale (linear, log, symlog, logit, ...): data are
//! mapped through the scales before the projection, and autoscaling,
//! panning, zooming and the status-bar readout work in the transformed
//! coordinates, as in matplotlib 3.11.
//!
//! Artists draw beyond the panes when the data exceed the view limits,
//! unless clipped to them (matplotlib 3.10's `axlim_clip`: an option field
//! of the plotting methods, or [`Axes3D::set_axlim_clip`]): their points
//! outside the limits are then hidden.
//!
//! * [`math`] — homogeneous projection matrices (matplotlib's `proj3d`).
//! * [`Axes3D`] — limits, bounds, margins, scales, equal aspects
//!   ([`Aspect3D`]), labels, view and decoration settings, sharing of axes
//!   and views, and the plotting methods: `plot`, `scatter`,
//!   `plot_surface`, `plot_wireframe`, `plot_trisurf`, `contour`,
//!   `contourf`, `tricontour`, `tricontourf`, `fill_between`, `errorbar`,
//!   `bar3d`, `bar`, `quiver`, `voxels`, `stem`, `text`, `text2d` and
//!   `add_collection3d`.
//! * [`Axis3D`] — one axis: tick locators and formatters (major and
//!   minor), fixed ticks and labels, tick parameters, its tick marks and
//!   gridlines, and unit conversion: the point-plotting methods take
//!   dates, category names and values with units as 2-D plotting methods
//!   do, converted on the axis each lies along
//!   ([`Axis3D::convert_units`]).
//! * [`LightSource`] — the light shading surfaces, bars and voxels.
//!
//! ```
//! use pltrs::figure::Figure;
//! use pltrs::mplot3d::Axis3;
//! let fig = Figure::new();
//! let ax = fig.subplot_3d();
//! let t: Vec<f64> = (0..50).map(|i| f64::from(i) * 0.2).collect();
//! let (x, y): (Vec<f64>, Vec<f64>) = t.iter().map(|t| (t.cos(), t.sin())).unzip();
//! ax.plot(&x, &y, &t)?;
//! ax.set_label(Axis3::Z, "time");
//! assert_eq!(ax.label(Axis3::Z), "time");
//! # Ok::<(), pltrs::mplot3d::Axes3DError>(())
//! ```
//!
//! [`Figure::subplot_3d`]: crate::figure::Figure::subplot_3d

mod adopt;
mod art3d;
mod aspect;
mod axis3d;
mod axis_artist;
mod axis_draw;
mod axis_handle;
mod axis_parts;
mod axis_ticks;
mod axlim;
mod bars;
mod bounds;
mod camera;
mod clear;
mod collections3d;
mod contour;
mod coords;
mod create;
mod error;
mod errorbar;
mod fill_between;
mod geometry;
mod handle;
mod layout;
mod lift;
mod limits;
pub mod math;
mod mouse;
mod patch3d;
mod path_patch;
mod plot;
mod quaternion;
mod quiver;
mod readout;
mod scales;
mod scaling;
mod scatter;
mod shade;
mod share;
mod state;
mod stem;
mod surface;
mod test_data;
mod trisurf;
#[cfg(test)]
mod unit_tests;
mod units;
mod update;
mod update_collections;
mod voxels;

pub(crate) use state::Axes3DState;

pub use adopt::Geometry3D;
pub use art3d::{ProjectedOutline, ProjectedPath, ZSort};
pub use aspect::Aspect3D;
pub use axis_draw::{AxisText3D, AxisTextPart, GridLine3D, TickLine3D};
pub use axis_handle::Axis3D;
pub use bars::{Bar3DOptions, Boxes};
pub use camera::{ProjType, ViewAngles};
pub use collections3d::FaceColors;
pub use contour::Contour3DOptions;
pub use coords::Axis3;
pub use create::Axes3DOptions;
pub use error::Axes3DError;
pub use errorbar::{Errorbar3D, Errorbar3DOptions};
pub use fill_between::{Curve3D, FillBetween3DOptions, FillMode, all_points_on_plane};
pub use geometry::AxisPosition;
pub use handle::Axes3D;
pub use lift::Poly3DOptions;
pub use math::Vec3;
pub(crate) use mouse::MouseAction;
pub use mouse::{MouseButtons, RotationStyle};
pub use quiver::Quiver3DOptions;
pub use scatter::Scatter3DOptions;
pub use shade::{LightSource, shade_faces};
pub(crate) use share::leave_shares;
pub use stem::{Stem3D, Stem3DOptions};
pub use surface::{Sampling, SurfaceOptions, WireframeOptions};
pub use test_data::test_data;
pub use trisurf::TrisurfOptions;
pub use voxels::{Array3, VoxelColors, VoxelOptions};

pub(crate) use layout::{axes3d_layout_extents, axes3d_placement, set_axes3d_position};
pub(crate) use state::{
    axes3d_bbox, axes3d_fresh_tight_bbox, axes3d_ids, axes3d_subplot_spec, axes3d_tight_bbox,
    is_axes3d, place_axes3d, prepare_axes3d,
};
