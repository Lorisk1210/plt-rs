//! Paths: the polylines and Bézier curves artists are drawn from
//! (matplotlib's `matplotlib.path`), from `plt-core`.
//!
//! ```
//! use plt::path::{Path, PathBuilder};
//! use plt::transforms::Point;
//!
//! let triangle = Path::polygon([Point::new(0.0, 0.0), Point::new(1.0, 0.0), Point::new(0.0, 1.0)]);
//! assert_eq!(triangle.vertices()[1], Point::new(1.0, 0.0));
//! let curve = PathBuilder::default()
//!     .move_to(Point::new(0.0, 0.0))
//!     .quad_to(Point::new(0.5, 1.0), Point::new(1.0, 0.0))
//!     .build();
//! assert_eq!(curve.vertices().len(), 3);
//! ```

pub use plt_core::{Path, PathBuilder, PathCode, PathError, Segment};
