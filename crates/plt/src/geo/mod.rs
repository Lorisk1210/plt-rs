//! Geographic axes (matplotlib's `projections.geo`): longitude (the x
//! axis) and latitude (the y axis) in radians drawn through a map
//! projection inside an ellipse or a circle.
//!
//! * [`GeoAxes`] — the handle returned by
//!   [`Figure::geo_subplot`](crate::figure::Figure::geo_subplot); it
//!   dereferences to [`Axes`] for plotting and adds the grid settings
//!   (`set_longitude_grid`, `set_latitude_grid`,
//!   `set_longitude_grid_ends`).
//! * [`GeoProjection`] — the four projections (Aitoff, Hammer, Lambert,
//!   Mollweide) with their forward and inverse maps.
//! * [`ThetaFormatter`] — the degree labels of the grid.
//!
//! The view limits are fixed to the whole sphere, `[-π, π]` × `[-π/2,
//! π/2]`; every path drawn in data coordinates is subdivided before it is
//! projected, so straight segments follow the curvature of the map.

mod api;
mod placement;
mod projection;
mod ticker;
mod transform;

pub use api::{GeoAxes, GeoError};
pub(crate) use placement::{tick_label_anchor, tick_transform};
pub use projection::GeoProjection;
pub use ticker::ThetaFormatter;
pub(crate) use transform::{OUTLINE_SPINE, outline};

use crate::axes::Axes;

/// Where longitude gridlines stop by default, in degrees of latitude
/// (matplotlib's `set_longitude_grid_ends(75)` in `GeoAxes.clear`).
const DEFAULT_LONGITUDE_GRID_ENDS: f64 = 75.0;

/// The geographic settings of an axes.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GeoState {
    /// The map projection.
    pub(crate) projection: GeoProjection,
    /// The latitude, in radians, beyond which longitude gridlines are not
    /// drawn (matplotlib's `_longitude_cap`).
    pub(crate) longitude_cap: f64,
}

impl GeoState {
    /// Settings of fresh axes drawn through `projection`.
    pub(crate) fn new(projection: GeoProjection) -> Self {
        Self {
            projection,
            longitude_cap: DEFAULT_LONGITUDE_GRID_ENDS.to_radians(),
        }
    }
}

impl Axes {
    /// This axes as geographic axes, if it is one.
    pub fn as_geo(&self) -> Option<GeoAxes> {
        GeoAxes::new(self.clone())
    }
}
