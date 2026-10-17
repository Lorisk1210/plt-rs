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
//! * [`math`] — homogeneous projection matrices (matplotlib's `proj3d`).
//! * [`Axes3D`] — limits, labels, view and decoration settings, and the
//!   plotting methods: `plot`, `scatter`, `plot_surface`,
//!   `plot_wireframe`, `plot_trisurf`, `contour`, `contourf`, `bar3d`,
//!   `bar`, `quiver`, `voxels`, `stem`, `text` and `text2d`.
//! * [`LightSource`] — the light shading surfaces, bars and voxels.
//!
//! ```
//! use plt::figure::Figure;
//! use plt::mplot3d::Axis3;
//! let fig = Figure::new();
//! let ax = fig.subplot_3d();
//! let t: Vec<f64> = (0..50).map(|i| f64::from(i) * 0.2).collect();
//! let (x, y): (Vec<f64>, Vec<f64>) = t.iter().map(|t| (t.cos(), t.sin())).unzip();
//! ax.plot(&x, &y, &t)?;
//! ax.set_label(Axis3::Z, "time");
//! assert_eq!(ax.label(Axis3::Z), "time");
//! # Ok::<(), plt::mplot3d::Axes3DError>(())
//! ```
//!
//! [`Figure::subplot_3d`]: crate::figure::Figure::subplot_3d

mod art3d;
mod axis3d;
mod axis_draw;
mod bars;
mod camera;
mod collections3d;
mod contour;
mod coords;
mod error;
mod geometry;
mod handle;
mod limits;
pub mod math;
mod plot;
mod quiver;
mod scatter;
mod shade;
mod state;
mod stem;
mod surface;
mod trisurf;
mod voxels;

pub use art3d::{ProjectedOutline, ZSort};
pub use bars::{Bar3DOptions, Boxes};
pub use camera::{ProjType, ViewAngles};
pub use collections3d::{FaceColormap, FaceColors};
pub use contour::Contour3DOptions;
pub use coords::Axis3;
pub use error::Axes3DError;
pub use handle::{Axes3D, Axes3DOptions};
pub use math::Vec3;
pub use quiver::Quiver3DOptions;
pub use scatter::Scatter3DOptions;
pub use shade::LightSource;
pub use stem::{Stem3D, Stem3DOptions};
pub use surface::{Sampling, SurfaceOptions, WireframeOptions};
pub use trisurf::TrisurfOptions;
pub use voxels::{Array3, VoxelColors, VoxelOptions};

pub(crate) use state::{axes3d_bbox, prepare_axes3d};
