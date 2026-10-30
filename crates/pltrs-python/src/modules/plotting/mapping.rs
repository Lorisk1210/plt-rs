//! Colour mapping as plotting methods take it: a colormap, a norm and
//! the limits.

use pltrs::colors::{ColorizerOptions, Colormap, NormName, NormSpec};

use crate::binding::prelude::*;

/// `(cmap, norm, vmin, vmax)`: a Python `Colormap` (`None`:
/// `image.cmap`), a Python norm or the name of a scale (`None`: linear
/// between the limits), and the limits (`None`: those of the values).
pub(crate) struct Mapping(ColorizerOptions);

impl FromPy<'_> for Mapping {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let (cmap, norm, vmin, vmax): (Option<Colormap>, Bound<'_>, Option<f64>, Option<f64>) =
            object.extract()?;
        let norm = if norm.is_none() {
            None
        } else if let Ok(name) = norm.extract::<String>() {
            Some(NormSpec::Name(
                name.parse::<NormName>()
                    .map_err(|err| PyErr::value_error(err.to_string()))?,
            ))
        } else {
            Some(NormSpec::Norm(norm.extract()?))
        };
        Ok(Self(ColorizerOptions {
            cmap,
            norm,
            vmin,
            vmax,
        }))
    }
}

/// The colorizer options of a [`Mapping`].
pub(crate) fn colorizer(mapping: Mapping) -> ColorizerOptions {
    mapping.0
}
