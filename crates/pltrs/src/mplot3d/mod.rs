//! 3-D plotting (`mpl_toolkits.mplot3d` in matplotlib).
//!
//! An [`Axes3D`] projects data from a 3-D box onto the screen through a
//! camera. Its page lists all 3-D plot types, the view settings and the axes
//! settings. Start there. [`Figure::subplot_3d`] makes one.
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
//! # How a 3-D axes draws
//!
//! Each 3-D artist is a 2-D artist with a 3-D geometry. Before each draw,
//! pltrs projects the geometry onto the 2-D artist. Then it sorts the
//! collections and patches by depth, so that nearer artists cover farther
//! ones. It also sorts the polygons and markers in a collection from back to
//! front.
//!
//! Each axis has a scale, such as linear or log. pltrs maps the data through
//! the scales before the projection. Autoscaling, panning, zooming and the
//! coordinate readout use the scaled coordinates.
//!
//! Artists can extend past the panes if the data is outside the view limits.
//! To hide those parts, set the option field `axlim_clip` of a plotting
//! method, or call [`Axes3D::set_axlim_clip`].
//!
//! # Types
//!
//! | Group | Types |
//! |-------|-------|
//! | The axes | [`Axes3D`], [`Axis3D`] (one axis), [`Axis3`] (the name of an axis), [`SharedAxes`] |
//! | View | [`ViewAngles`], [`ProjType`], [`Aspect3D`], [`PaneStyle`] |
//! | Options of the plot types | [`Axes3DOptions`], [`Scatter3DOptions`], [`SurfaceOptions`], [`WireframeOptions`], [`TrisurfOptions`], [`Contour3DOptions`], [`FillBetween3DOptions`], [`Errorbar3DOptions`], [`Bar3DOptions`], [`Quiver3DOptions`], [`VoxelOptions`], [`Stem3DOptions`], [`Poly3DOptions`] |
//! | Data | [`Vec3`], [`Curve3D`], [`Boxes`], [`Array3`] |
//! | Results | [`Errorbar3D`], [`Stem3D`] |
//! | Shading and depth | [`LightSource`], [`ZSort`] |
//! | Mouse | [`MouseButtons`], [`RotationStyle`] |
//! | Errors | [`Axes3DError`], [`DrawFailure`] |
//!
//! The module [`math`] has the projection matrices (`proj3d` in matplotlib).
//!
//! [`Figure::subplot_3d`]: crate::figure::Figure::subplot_3d

mod adopt;
mod art3d;
mod aspect;
mod axis3d;
mod axis_artist;
mod axis_box;
mod axis_draw;
mod axis_handle;
mod axis_info;
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
mod drawn;
mod error;
mod errorbar;
mod failures;
mod fill_between;
mod flat_share;
mod geometry;
mod handle;
mod kept;
mod layout;
mod lift;
mod limits;
pub mod math;
mod mouse;
mod patch3d;
mod path_patch;
mod pending;
mod plot;
mod program;
mod projection_warnings;
mod quaternion;
mod quiver;
mod readd;
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
mod tick_props;
mod tick_sites;
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
pub use axis_box::AxisBox;
pub use axis_draw::{AxisText3D, AxisTextPart, GridLine3D, TickLine3D};
pub use axis_handle::Axis3D;
pub use axis_info::AxisInfo;
pub use axis3d::PaneStyle;
pub use bars::{Bar3DOptions, Boxes};
pub use camera::{ProjType, ViewAngles};
pub use collections3d::FaceColors;
pub use contour::Contour3DOptions;
pub use contour::ContourPlacement;
pub use coords::Axis3;
pub use create::Axes3DOptions;
pub use error::Axes3DError;
pub use errorbar::{Errorbar3D, Errorbar3DOptions};
pub use failures::DrawFailure;
pub use fill_between::{Curve3D, FillBetween3DOptions, FillMode, all_points_on_plane};
pub use flat_share::SharedAxes;
pub use geometry::AxisPosition;
pub use handle::Axes3D;
pub use kept::{Kept3D, Kept3DKind};
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
    axes3d_bbox, axes3d_fresh_tight_bbox, axes3d_ids, axes3d_subplot_spec, is_axes3d, place_axes3d,
    prepare_axes3d, prepare_drawn,
};
