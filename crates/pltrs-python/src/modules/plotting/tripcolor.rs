//! Pseudocolour plots of triangulations: `tripcolor`.

use pltrs::axes::{Axes, TriShading, TriValues, Tripcolor, TripcolorError, TripcolorOptions};
use pltrs::collections::{Collection, TriMesh};
use pltrs::tri::Triangulation;

use super::mapping::{Mapping, colorizer};
use crate::binding::prelude::*;

artist_handle!(TriMesh);

impl From<TripcolorError> for PyErr {
    /// matplotlib's `ValueError` for values that fit neither the points
    /// nor the triangles.
    fn from(err: TripcolorError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

/// The triangles of `tri` coloured by `values` (matplotlib's
/// `tripcolor`): at the points, or (`faces`) one per triangle, mapped as
/// `mapping` says, flat or (`gouraud`) Gouraud-shaded. Returns the flat
/// polygons or the Gouraud mesh (the other `None`).
pub(super) fn tripcolor(
    axes: Axes,
    tri: Triangulation,
    (values, faces): (NdArray, bool),
    mapping: Mapping,
    gouraud: bool,
) -> PyResult<(Option<Collection>, Option<TriMesh>)> {
    let values = values.into_vec()?;
    let options = TripcolorOptions {
        colorizer: colorizer(mapping),
        shading: if gouraud {
            TriShading::Gouraud
        } else {
            TriShading::Flat
        },
        ..TripcolorOptions::default()
    };
    let values = if faces {
        TriValues::Faces(values)
    } else {
        TriValues::Auto(values)
    };
    Ok(match axes.tripcolor(&tri, values, options)? {
        Tripcolor::Flat(polygons) => (Some(polygons), None),
        Tripcolor::Gouraud(mesh) => (None, Some(mesh)),
    })
}
