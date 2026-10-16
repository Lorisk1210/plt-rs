//! Marker shapes for lines and scatter plots (matplotlib.markers).
//!
//! A [`Marker`] is what users ask for (`"o"`, a polygon tuple, a custom
//! path); [`MarkerStyle`] resolves it, together with a [`FillStyle`], into
//! the exact paths and transforms matplotlib draws.

mod spec;
mod style;

pub use spec::{FillStyle, Marker, MarkerError, MarkerKey, MarkerSymbol, PolygonStyle};
pub use style::MarkerStyle;
