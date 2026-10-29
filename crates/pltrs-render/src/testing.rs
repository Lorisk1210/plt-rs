//! Test helpers shared by the crate's unit tests.

use pltrs_core::{Path, Point};

/// The closed square with corners `(0, 0)` and `(10, 10)`.
pub(crate) fn square() -> Path {
    Path::polygon([
        Point::new(0.0, 0.0),
        Point::new(10.0, 0.0),
        Point::new(10.0, 10.0),
        Point::new(0.0, 10.0),
    ])
}
