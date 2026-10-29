//! Unstructured triangular grids: Delaunay triangulation, point location,
//! interpolation, refinement and mesh analysis (matplotlib.tri).
//!
//! * [`Triangulation`] — points plus anticlockwise triangles, either given
//!   explicitly or computed with [`Triangulation::delaunay`], with an
//!   optional triangle mask, derived `edges` and `neighbors`, and plane
//!   coefficients for linear interpolation.
//! * [`TriFinder`] — locates the triangle containing a point (the rules of
//!   matplotlib's `TrapezoidMapTriFinder`).
//! * [`LinearTriInterpolator`] / [`CubicTriInterpolator`] — piecewise
//!   linear and C1 cubic (reduced Hsieh–Clough–Tocher) interpolation, both
//!   implementing [`TriInterpolator`].
//! * [`UniformTriRefiner`] — recursive 4-way subdivision of triangles and
//!   of fields defined on them.
//! * [`TriAnalyzer`] — flatness measures and removal of flat border
//!   triangles.
//! * [`TriContourGenerator`] — contour lines and filled contours of a
//!   field on a triangulation.
//!
//! ```
//! use pltrs::tri::{LinearTriInterpolator, TriInterpolator, Triangulation};
//!
//! let tri = Triangulation::delaunay(&[0.0, 1.0, 0.0, 1.0], &[0.0, 0.0, 1.0, 1.0])?;
//! assert_eq!(tri.triangles().len(), 2);
//! let z = [0.0, 1.0, 2.0, 3.0];
//! let interp = LinearTriInterpolator::new(&tri, &z)?;
//! assert_eq!(interp.value(0.5, 0.5), Some(1.5));
//! assert_eq!(interp.value(2.0, 0.5), None);
//! # Ok::<(), pltrs::tri::TriError>(())
//! ```

mod analyzer;
mod contour;
mod delaunay;
mod error;
mod finder;
mod interpolate;
mod refine;
mod triangulation;

pub use analyzer::TriAnalyzer;
pub use contour::TriContourGenerator;
pub(crate) use contour::source as contour_source;
pub use error::TriError;
pub use finder::TriFinder;
pub use interpolate::{CubicKind, CubicTriInterpolator, LinearTriInterpolator, TriInterpolator};
pub use refine::{RefinedField, UniformTriRefiner};
pub use triangulation::Triangulation;
