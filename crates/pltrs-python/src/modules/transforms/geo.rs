//! The map projections of geographic axes (matplotlib's
//! `AitoffTransform`, `HammerTransform`, `LambertTransform`,
//! `MollweideTransform` and their inverses), computed by pltrs (Rust).

use pltrs::axes::Axes;
use pltrs::geo::GeoProjection;

use super::{points, points_array};
use crate::binding::prelude::*;

/// The projection `name` names (`"aitoff"`, `"hammer"`, `"lambert"` with
/// its centre `(longitude, latitude)` in radians, or `"mollweide"`).
fn projection(
    name: &str,
    (center_longitude, center_latitude): (f64, f64),
) -> PyResult<GeoProjection> {
    Ok(match name {
        "aitoff" => GeoProjection::Aitoff,
        "hammer" => GeoProjection::Hammer,
        "lambert" => GeoProjection::Lambert {
            center_longitude,
            center_latitude,
        },
        "mollweide" => GeoProjection::Mollweide,
        _ => {
            return Err(PyErr::value_error(format!(
                "{name:?} is not a valid value for projection; supported values are 'aitoff', \
                 'hammer', 'lambert', 'mollweide'"
            )));
        }
    })
}

/// The points `lonlat`, an `(n, 2)` array of longitudes and latitudes in
/// radians, projected by the projection `name` centred at `center`
/// (`transform_non_affine`).
fn geo_forward(name: String, center: (f64, f64), lonlat: NdArray) -> PyResult<NdArray> {
    let map = projection(&name, center)?;
    let projected: Vec<_> = points(&lonlat)?
        .into_iter()
        .map(|p| map.forward(p))
        .collect();
    points_array(&projected, vec![projected.len(), 2])
}

/// The longitudes and latitudes in radians of the projected points `xy`,
/// an `(n, 2)` array (the inverse transform's `transform_non_affine`).
fn geo_inverse(name: String, center: (f64, f64), xy: NdArray) -> PyResult<NdArray> {
    let map = projection(&name, center)?;
    let lonlat: Vec<_> = points(&xy)?.into_iter().map(|p| map.inverse(p)).collect();
    points_array(&lonlat, vec![lonlat.len(), 2])
}

/// The centre `(longitude, latitude)` in radians of the Lambert
/// projection of `axes`, if it is drawn in one.
fn lambert_center(axes: Axes) -> PyResult<Option<(f64, f64)>> {
    Ok(axes.as_geo().and_then(|geo| match geo.geo_projection() {
        GeoProjection::Lambert {
            center_longitude,
            center_latitude,
        } => Some((center_longitude, center_latitude)),
        _ => None,
    }))
}

/// Registers the functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("geo_forward", geo_forward)?;
    module.add("geo_inverse", geo_inverse)?;
    module.add("lambert_center", lambert_center)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projections_are_named_by_matplotlib() {
        assert_eq!(
            projection("mollweide", (0.0, 0.0)).ok(),
            Some(GeoProjection::Mollweide)
        );
        assert_eq!(
            projection("lambert", (0.5, 0.25)).ok(),
            Some(GeoProjection::Lambert {
                center_longitude: 0.5,
                center_latitude: 0.25
            })
        );
        assert!(projection("mercator", (0.0, 0.0)).is_err());
    }
}
