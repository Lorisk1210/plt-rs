//! `_native.insets`: axes inside axes and their placement (matplotlib's
//! `Axes.inset_axes`, `secondary_xaxis`/`secondary_yaxis`, `child_axes`
//! and `set_axes_locator`), by pltrs (Rust)'s inset and secondary axes,
//! and the indicators of the area an inset shows (`indicate_inset`).

use pltrs::axes::{Axes, SecondaryLocation};
use pltrs::inset::InsetIndicator;
use pltrs::patches::Patch;
use pltrs::scale::FuncTransform;
use pltrs::transforms::Transform;

use super::axes_grid1::side_named;
use super::figure::projection_kind;
use super::patches::PatchKwargs;
use crate::binding::prelude::*;

artist_handle!(Patch<InsetIndicator>);

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
fn functions(pair: Option<(Callback, Callback)>) -> Option<FuncTransform> {
    pair.map(|(forward, inverse)| {
        FuncTransform::new(
            move |x| forward.call((x,)).unwrap_or(f64::NAN),
            move |x| inverse.call((x,)).unwrap_or(f64::NAN),
        )
    })
}

/// A secondary axis of `parent` mirroring its `orientation` axis (`"x"`
/// or `"y"`) at `location`, converting the parent's range with
/// `functions` (`secondary_xaxis`, `secondary_yaxis`).
fn secondary_axis(
    parent: Axes,
    orientation: String,
    location: PyLocation,
    pair: Option<(Callback, Callback)>,
) -> PyResult<Axes> {
    let made = match orientation.as_str() {
        "x" => parent.secondary_xaxis(location.0, functions(pair)),
        "y" => parent.secondary_yaxis(location.0, functions(pair)),
        _ => return Err(PyErr::value_error(format!("no axis {orientation:?}"))),
    };
    Ok(made?)
}

/// Converts the parent's range with `functions` from now on
/// (`SecondaryAxis.set_functions`).
fn set_secondary_functions(axes: Axes, pair: Option<(Callback, Callback)>) -> PyResult<()> {
    axes.set_secondary_functions(functions(pair));
    Ok(())
}

/// Moves the secondary axis to `location` (`SecondaryAxis.set_location`).
fn set_secondary_location(axes: Axes, location: PyLocation) -> PyResult<()> {
    axes.set_secondary_location(location.0)?;
    Ok(())
}

/// Marks the area `bounds` of `parent` (data coordinates), or the area
/// `inset` shows when no bounds are given, joined to the corners of
/// `inset` if given, styled by `props` (`indicate_inset`,
/// `indicate_inset_zoom`).
fn indicate_inset(
    parent: Axes,
    bounds: Option<(f64, f64, f64, f64)>,
    inset: Option<Axes>,
    props: PatchKwargs,
) -> PyResult<Patch<InsetIndicator>> {
    match (bounds, inset) {
        (Some((x, y, width, height)), inset) => {
            Ok(parent.indicate_inset([x, y, width, height], inset.as_ref(), props.0))
        }
        (None, Some(inset)) => Ok(parent.indicate_inset_zoom(&inset, props.0)),
        (None, None) => Err(PyErr::value_error(
            "At least one of bounds or inset_ax must be supplied",
        )),
    }
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("clear_axes_locator", clear_axes_locator)?;
    module.add("child_axes", child_axes)?;
    module.add("inset_axes", inset_axes)?;
    module.add("place_inset", place_inset)?;
    module.add("secondary_axis", secondary_axis)?;
    module.add("set_secondary_functions", set_secondary_functions)?;
    module.add("set_secondary_location", set_secondary_location)?;
    module.add("indicate_inset", indicate_inset)
}
