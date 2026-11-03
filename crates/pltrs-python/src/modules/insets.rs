//! `_native.insets`: axes inside axes and their placement (matplotlib's
//! `Axes.inset_axes`, `secondary_xaxis`/`secondary_yaxis`, `child_axes`
//! and `set_axes_locator`), by pltrs (Rust)'s inset and secondary axes,
//! and the indicators of the area an inset shows (matplotlib's
//! `InsetIndicator`, its rectangle and its connectors).

use pltrs::artist::RawHandle;
use pltrs::axes::{Axes, SecondaryLocation};
use pltrs::inset::InsetIndicator;
use pltrs::patches::{ConnectionPatch, Patch, Rectangle};
use pltrs::scale::FuncTransform;
use pltrs::transforms::Transform;

use super::axes_grid1::side_named;
use super::figure::projection_kind;
use crate::binding::prelude::*;

mod program_locators;

artist_handle!(InsetIndicator);

/// Lets the axes keep its original position: no divider, inset locator
/// or colorbar places it any more (`set_axes_locator(None)`).
fn clear_axes_locator(axes: Axes) -> PyResult<()> {
    axes.set_axes_locator(None);
    Ok(())
}

/// The axes placed inside `axes`, in creation order (`child_axes`).
fn child_axes(axes: Axes) -> PyResult<Vec<Axes>> {
    Ok(axes.child_axes())
}

/// Adds `child`, an axes made outside the figure or removed from it,
/// inside `parent` where it is now (`add_child_axes`).
fn add_child_axes(parent: Axes, child: Axes) -> PyResult<()> {
    parent.add_child_axes(&child)?;
    Ok(())
}

/// A child axes of `parent` at `bounds` in its axes fractions, with the
/// built-in `projection` if given (`inset_axes`).
fn inset_axes(
    parent: Axes,
    (left, bottom, width, height): (f64, f64, f64, f64),
    projection: Option<String>,
) -> PyResult<Axes> {
    let bounds = [left, bottom, width, height];
    match projection {
        None => Ok(parent.inset_axes(bounds)),
        Some(name) => parent
            .inset_axes_with_projection(bounds, projection_kind(&name)?)
            .map_err(|err| PyErr::value_error(err.to_string())),
    }
}

/// Places `inset` on `bounds` in the coordinates of `transform` at every
/// layout pass (`_TransformedBoundsLocator`).
fn place_inset(
    parent: Axes,
    inset: Axes,
    (left, bottom, width, height): (f64, f64, f64, f64),
    transform: Transform,
) -> PyResult<()> {
    parent.place_inset(&inset, [left, bottom, width, height], transform);
    Ok(())
}

/// A secondary axis location as the Python side gives it: a side name, a
/// fraction of the parent, or `("data", value)` for a data value of the
/// parent across the axis.
struct PyLocation(SecondaryLocation);

impl FromPy<'_> for PyLocation {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        if let Ok(name) = object.extract::<String>() {
            return Ok(Self(SecondaryLocation::Side(side_named(&name)?)));
        }
        if let Ok(fraction) = object.extract::<f64>() {
            return Ok(Self(SecondaryLocation::Fraction(fraction)));
        }
        let (_, value): (String, f64) = object.extract()?;
        Ok(Self(SecondaryLocation::Data(value)))
    }
}

/// The conversion of host values by the Python functions `(forward,
/// inverse)`, each called with one value; a value a function fails on
/// becomes NaN (its exception goes to the native call running).
fn functions(pair: Option<(Owned, Owned)>) -> PyResult<Option<FuncTransform>> {
    pair.map(|(forward, inverse)| {
        Python::with_gil(|py| super::archive::func_transform(&forward.bind(py), &inverse.bind(py)))
    })
    .transpose()
}

/// A secondary axis of `parent` mirroring its `orientation` axis (`"x"`
/// or `"y"`) at `location`, converting the parent's range with
/// `functions` (`secondary_xaxis`, `secondary_yaxis`).
fn secondary_axis(
    parent: Axes,
    orientation: String,
    location: PyLocation,
    pair: Option<(Owned, Owned)>,
) -> PyResult<Axes> {
    let made = match orientation.as_str() {
        "x" => parent.secondary_xaxis(location.0, functions(pair)?),
        "y" => parent.secondary_yaxis(location.0, functions(pair)?),
        _ => return Err(PyErr::value_error(format!("no axis {orientation:?}"))),
    };
    Ok(made?)
}

/// Converts the parent's range with `functions` from now on
/// (`SecondaryAxis.set_functions`).
fn set_secondary_functions(axes: Axes, pair: Option<(Owned, Owned)>) -> PyResult<()> {
    axes.set_secondary_functions(functions(pair)?);
    Ok(())
}

/// Moves the secondary axis to `location` (`SecondaryAxis.set_location`).
fn set_secondary_location(axes: Axes, location: PyLocation) -> PyResult<()> {
    axes.set_secondary_location(location.0)?;
    Ok(())
}

/// Adds the indicator made outside any figure to `axes`
/// (`add_artist(InsetIndicator(...))`): `artist` gives its common
/// properties and `rectangle` becomes its frame, following the view limits
/// of `inset` when `follows_inset`.
fn adopt_indicator(
    axes: Axes,
    artist: RawHandle,
    rectangle: RawHandle,
    follows_inset: bool,
    inset: Option<Axes>,
) -> PyResult<InsetIndicator> {
    Ok(axes.adopt_indicator(&artist, &rectangle, follows_inset, inset.as_ref())?)
}

/// The indicator's frame (`rectangle`).
fn indicator_rectangle(indicator: InsetIndicator) -> PyResult<Patch<Rectangle>> {
    Ok(indicator.rectangle())
}

/// The indicator's connectors, their visibility chosen if it was not yet
/// (`connectors`); `None` without an inset.
fn indicator_connectors(
    indicator: InsetIndicator,
) -> PyResult<Option<Vec<Patch<ConnectionPatch>>>> {
    Ok(indicator.connectors())
}

/// The indicator's connectors as they are, their visibility not chosen
/// (the connectors `InsetIndicator._shared_setter` styles).
fn indicator_parts(indicator: InsetIndicator) -> PyResult<Vec<Patch<ConnectionPatch>>> {
    Ok(indicator.connector_patches())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    program_locators::register(module)?;
    module.add("clear_axes_locator", clear_axes_locator)?;
    module.add("child_axes", child_axes)?;
    module.add("add_child_axes", add_child_axes)?;
    module.add("inset_axes", inset_axes)?;
    module.add("place_inset", place_inset)?;
    module.add("secondary_axis", secondary_axis)?;
    module.add("set_secondary_functions", set_secondary_functions)?;
    module.add("set_secondary_location", set_secondary_location)?;
    module.add("adopt_indicator", adopt_indicator)?;
    module.add("indicator_rectangle", indicator_rectangle)?;
    module.add("indicator_connectors", indicator_connectors)?;
    module.add("indicator_parts", indicator_parts)
}
