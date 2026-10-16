//! Patches: filled 2-D shapes with an edge (matplotlib.patches).
//!
//! A patch is a [`PatchShape`] (rectangle, circle, polygon, wedge, path,
//! ...) painted with a [`PatchStyle`]. Adding a shape to an axes returns a
//! typed [`Patch`] handle:
//!
//! ```
//! use plt::prelude::*;
//! use plt::patches::Rectangle;
//! let ax = Figure::new().subplot();
//! let bar = ax.add_patch(Rectangle::new((0.0, 0.0), 2.0, 3.0));
//! bar.set_facecolor("tab:green").set_hatch("//");
//! bar.update_shape(|rect| rect.height = 5.0);
//! assert_eq!(bar.shape().height, 5.0);
//! ```

mod artist;
mod shapes;
mod style;

pub(crate) use artist::PatchArtist;
pub use artist::Patch;
pub use shapes::{Circle, Ellipse, PatchShape, PathShape, Polygon, Rectangle, RegularPolygon, RotationPoint, Wedge};
pub use style::PatchStyle;
