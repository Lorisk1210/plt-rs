//! Geographic axes (`projections.geo` in matplotlib).
//!
//! A geographic axes shows longitude in radians along x and latitude in
//! radians along y, through a map projection. [`GeoAxes`] lists what is
//! different from a rectangular axes and all geographic settings. Start
//! there.
//!
//! * [`GeoAxes`]: the handle of a geographic axes. It dereferences to
//!   [`Axes`] for plotting.
//! * [`GeoProjection`]: the four projections Aitoff, Hammer, Lambert and
//!   Mollweide, with their forward and inverse maps.
//! * [`GeoThetaFormatter`]: the degree labels of the gridlines.
//! * [`GeoError`]: the errors of the geographic settings.
//!
//! Geographic axes draw through the same [`Projection`] trait as the
//! projections that you define.
//!
//! [`Projection`]: crate::projections::Projection

mod api;
mod axes_projection;
mod placement;
mod projection;
mod ticker;
mod transform;
pub(crate) use transform::GeoMap;

pub use api::{GeoAxes, GeoError};
pub use projection::GeoProjection;
pub use ticker::GeoThetaFormatter;

use crate::axes::Axes;

/// Where longitude gridlines stop by default, in degrees of latitude
/// (matplotlib's `set_longitude_grid_ends(75)` in `GeoAxes.clear`).
const DEFAULT_LONGITUDE_GRID_ENDS: f64 = 75.0;

/// Longitudes run from `-LONGITUDE_EDGE` to `LONGITUDE_EDGE` degrees.
const LONGITUDE_EDGE: f64 = 180.0;

/// Latitudes run from `-LATITUDE_EDGE` to `LATITUDE_EDGE` degrees.
const LATITUDE_EDGE: f64 = 90.0;

/// Default spacing of longitude gridlines in degrees.
const DEFAULT_LONGITUDE_GRID: f64 = 30.0;

/// Default spacing of latitude gridlines in degrees.
const DEFAULT_LATITUDE_GRID: f64 = 15.0;

/// The geographic settings of an axes.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GeoState {
    /// The map projection.
    pub(crate) projection: GeoProjection,
    /// The latitude, in radians, beyond which longitude gridlines are not
    /// drawn (matplotlib's `_longitude_cap`).
    pub(crate) longitude_cap: f64,
}

pltrs_core::archive_struct!(GeoState {
    projection,
    longitude_cap
});

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
