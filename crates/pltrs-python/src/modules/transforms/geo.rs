//! The map projections of geographic axes (matplotlib's
//! `AitoffTransform`, `HammerTransform`, `LambertTransform`,
//! `MollweideTransform` and their inverses), computed by pltrs (Rust),
//! and the grids of the axes (`set_longitude_grid`, ...,
//! `GeoAxes.ThetaFormatter`).

use pltrs::axes::Axes;
use pltrs::figure::Figure;
use pltrs::geo::{GeoAxes, GeoError, GeoProjection, GeoThetaFormatter};
use pltrs::gridspec::SubplotSpec;

use super::{matrix, points, points_array};
use crate::binding::prelude::*;

impl From<GeoError> for PyErr {
    fn from(err: GeoError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

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

/// The Lambert projection centred at `(longitude, latitude)` in radians.
fn lambert((center_longitude, center_latitude): (f64, f64)) -> GeoProjection {
    GeoProjection::Lambert {
        center_longitude,
        center_latitude,
    }
}

/// Lambert axes centred at `center` on the block `spec` of `figure`
/// (`add_subplot(projection='lambert', center_longitude=...,
/// center_latitude=...)`).
fn add_lambert_subplot(figure: Figure, spec: SubplotSpec, center: (f64, f64)) -> PyResult<Axes> {
    figure
        .add_geo_subplot_spec(lambert(center), &spec)
        .map(|geo| geo.axes().clone())
        .map_err(|err| PyErr::value_error(err.to_string()))
}

/// Lambert axes centred at `center` at `[left, bottom, width, height]` of
/// `figure` (`add_axes(rect, projection='lambert', center_longitude=...)`).
fn add_lambert_axes(
    figure: Figure,
    (left, bottom, width, height): (f64, f64, f64, f64),
    center: (f64, f64),
) -> PyResult<Axes> {
    figure
        .add_geo_axes(lambert(center), [left, bottom, width, height])
        .map(|geo| geo.axes().clone())
        .map_err(|err| PyErr::value_error(err.to_string()))
}

/// The geographic axes `axes` is.
fn geo_axes(axes: &Axes) -> PyResult<GeoAxes> {
    axes.as_geo()
        .ok_or_else(|| PyErr::type_error("the axes is not a geographic axes"))
}

/// The map from the projected plane onto the unit square of `axes`
/// (matplotlib's `GeoAxes._get_affine_transform()`), as a 3×3 matrix.
fn geo_affine(axes: Axes) -> PyResult<NdArray> {
    matrix(geo_axes(&axes)?.geo_projection().affine())
}

/// `GeoAxes.set_longitude_grid(degrees)`.
fn set_longitude_grid(axes: Axes, degrees: f64) -> PyResult<()> {
    geo_axes(&axes)?.set_longitude_grid(degrees)?;
    Ok(())
}

/// `GeoAxes.set_latitude_grid(degrees)`.
fn set_latitude_grid(axes: Axes, degrees: f64) -> PyResult<()> {
    geo_axes(&axes)?.set_latitude_grid(degrees)?;
    Ok(())
}

/// `GeoAxes.set_longitude_grid_ends(degrees)`.
fn set_longitude_grid_ends(axes: Axes, degrees: f64) -> PyResult<()> {
    geo_axes(&axes)?.set_longitude_grid_ends(degrees);
    Ok(())
}

/// `GeoAxes.ThetaFormatter(round_to)(radians)`: the angle in degrees,
/// rounded to a multiple of `round_to`, with a degree sign.
fn geo_theta_label(round_to: f64, radians: f64) -> PyResult<String> {
    Ok(GeoThetaFormatter::new(round_to).label(radians))
}

/// Registers the functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("geo_forward", geo_forward)?;
    module.add("geo_inverse", geo_inverse)?;
    module.add("lambert_center", lambert_center)?;
    module.add("add_lambert_subplot", add_lambert_subplot)?;
    module.add("add_lambert_axes", add_lambert_axes)?;
    module.add("geo_affine", geo_affine)?;
    module.add("set_longitude_grid", set_longitude_grid)?;
    module.add("set_latitude_grid", set_latitude_grid)?;
    module.add("set_longitude_grid_ends", set_longitude_grid_ends)?;
    module.add("geo_theta_label", geo_theta_label)
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
