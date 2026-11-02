//! Axes made for colorbars beside their parents (matplotlib's
//! `colorbar.make_axes` and `make_axes_gridspec`).

use pltrs::axes::Axes;
use pltrs::colorbar::{ColorbarCax, ColorbarError, ColorbarOptions, make_axes, make_axes_gridspec};

use super::super::transforms::anchor;
use super::{orientation, orientation_name, parent_anchor, side};
use crate::binding::dict_value;
use crate::binding::prelude::*;

/// A colorbar axes made, with the colorbar's `orientation` and
/// `ticklocation` keywords.
type Made = (Axes, &'static str, &'static str);

/// The placement keywords of `make_axes` (`location`, `orientation`,
/// `fraction`, `shrink`, `aspect`, `pad`, `anchor`, `panchor`) as
/// colorbar options.
fn layout_options(spec: &Bound<'_>) -> PyResult<ColorbarOptions> {
    let named = |key: &str| -> PyResult<Option<String>> { dict_value(spec, key) };
    Ok(ColorbarOptions {
        location: named("location")?.as_deref().map(side).transpose()?,
        orientation: named("orientation")?
            .as_deref()
            .map(orientation)
            .transpose()?,
        fraction: dict_value(spec, "fraction")?,
        shrink: dict_value(spec, "shrink")?,
        aspect: dict_value(spec, "aspect")?,
        pad: dict_value(spec, "pad")?,
        anchor: dict_value::<Option<Bound<'_>>>(spec, "anchor")?
            .as_ref()
            .map(anchor)
            .transpose()?,
        panchor: parent_anchor(&dict_value(spec, "panchor")?)?,
        ..ColorbarOptions::default()
    })
}

/// The axes and the keywords of a colorbar axes made.
fn made(made: Result<ColorbarCax, ColorbarError>) -> PyResult<Made> {
    let made = made.map_err(|err| match err {
        ColorbarError::MixedFigures => PyErr::value_error(
            "Unable to create a colorbar Axes as not all parents share the same figure.",
        ),
        ColorbarError::LocationOrientation { .. } => {
            PyErr::type_error("location and orientation are mutually exclusive")
        }
        ColorbarError::NoSubplotSpec => PyErr::new(
            ExceptionKind::AttributeError,
            "'NoneType' object has no attribute 'subgridspec'",
        ),
        other => other.into(),
    })?;
    Ok((
        made.cax,
        orientation_name(made.orientation),
        made.location.name(),
    ))
}

/// An axes for a colorbar beside `parents`, shrunk to free its space
/// (`make_axes`).
fn make_colorbar_axes(parents: Vec<Axes>, spec: Owned) -> PyResult<Made> {
    let options = Python::with_gil(|py| layout_options(&spec.bind(py)))?;
    made(make_axes(&parents, &options))
}

/// An axes for a colorbar in the grid cell of `parent`
/// (`make_axes_gridspec`).
fn make_colorbar_axes_gridspec(parent: Axes, spec: Owned) -> PyResult<Made> {
    let options = Python::with_gil(|py| layout_options(&spec.bind(py)))?;
    made(make_axes_gridspec(&parent, &options))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("make_axes", make_colorbar_axes)?;
    module.add("make_axes_gridspec", make_colorbar_axes_gridspec)
}
