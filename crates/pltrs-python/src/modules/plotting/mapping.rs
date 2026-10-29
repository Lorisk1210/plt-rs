//! Colour mapping as plotting methods take it: a colormap and a norm by
//! name, and the limits.

use pltrs::colors::{ColorizerOptions, NormName, NormSpec, colormaps};

use crate::binding::prelude::*;

/// `(cmap, norm, vmin, vmax)`: the colormap and norm names (`None` for
/// the defaults) and the limits.
pub(crate) type Mapping = (Option<String>, Option<String>, Option<f64>, Option<f64>);

/// The colorizer options of a [`Mapping`]; `ValueError` for an unknown
/// colormap or norm name.
pub(crate) fn colorizer((cmap, norm, vmin, vmax): Mapping) -> PyResult<ColorizerOptions> {
    let cmap = cmap
        .map(|name| colormaps::get(&name).map_err(|err| PyErr::value_error(err.to_string())))
        .transpose()?;
    let norm = norm
        .map(|name| {
            name.parse::<NormName>()
                .map(NormSpec::Name)
                .map_err(|err| PyErr::value_error(err.to_string()))
        })
        .transpose()?;
    Ok(ColorizerOptions {
        cmap,
        norm,
        vmin,
        vmax,
    })
}
