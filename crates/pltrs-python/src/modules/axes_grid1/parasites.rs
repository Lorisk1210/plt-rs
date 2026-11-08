//! Host axes and their parasites (matplotlib's
//! `axes_grid1.parasite_axes`): the parasites pltrs (Rust) makes on a
//! host, which draws them.

use pltrs::axes::{Axes, AxisName};
use pltrs::axes_grid1::{HostAxes, ParasiteAxes, ParasiteError, ParasiteLink};
use pltrs_core::Bbox;

use super::super::transforms::affine;
use crate::binding::prelude::*;

impl From<ParasiteError> for PyErr {
    /// matplotlib's `ValueError` for an aux transform pltrs cannot follow.
    fn from(err: ParasiteError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

/// How a parasite relates to its host, as the Python side gives it:
/// `"independent"`, `"equal"`, or the 3×3 matrix of an affine aux
/// transform.
struct PyLink(ParasiteLink);

impl FromPy<'_> for PyLink {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        if let Ok(name) = object.extract::<String>() {
            return match name.as_str() {
                "independent" => Ok(Self(ParasiteLink::Independent)),
                "equal" => Ok(Self(ParasiteLink::Equal)),
                _ => Err(PyErr::value_error(format!("no parasite link {name:?}"))),
            };
        }
        Ok(Self(ParasiteLink::Transform(affine(&object.extract()?)?)))
    }
}

/// The parasite's axes.
fn parasite_axes(parasite: ParasiteAxes) -> Axes {
    parasite.axes().clone()
}

/// A parasite of `host` linked as `link` says, every decoration hidden
/// (`get_aux_axes`).
fn aux_axes(host: Axes, link: PyLink) -> PyResult<Axes> {
    Ok(parasite_axes(HostAxes::new(host).aux_axes(link.0)?))
}

/// A twin of `host`: sharing the x axis (`"x"`, `twinx`), the y axis
/// (`"y"`, `twiny`), or none, mapped by the affine `aux` matrix (`"both"`,
/// `twin`), its sides and the host's as they were (`_add_twin_axes`): the
/// Python methods show and hide them through the axes' `axis` dicts, as
/// matplotlib's do.
fn twin(host: Axes, kind: String, aux: Option<NdArray>) -> PyResult<Axes> {
    let host = HostAxes::new(host);
    let made = match kind.as_str() {
        "x" => host.shared_twin(AxisName::X),
        "y" => host.shared_twin(AxisName::Y),
        "both" => host.linked_twin(aux.as_ref().map(affine).transpose()?)?,
        _ => return Err(PyErr::value_error(format!("no twin {kind:?}"))),
    };
    Ok(parasite_axes(made))
}

/// The parasites of `host`, in creation order (`parasites`).
fn parasites(host: Axes) -> PyResult<Vec<Axes>> {
    Ok(HostAxes::new(host)
        .parasites()
        .into_iter()
        .map(parasite_axes)
        .collect())
}

/// Removes the parasite among the parasites of `host` whose axes is
/// `parasite`, showing the host's far sides again for a twin
/// (`_remove_any_twin`).
fn remove_parasite(host: Axes, parasite: Axes) -> PyResult<()> {
    if let Some(found) = HostAxes::new(host)
        .parasites()
        .into_iter()
        .find(|candidate| *candidate.axes() == parasite)
    {
        found.remove();
    }
    Ok(())
}

/// The parasite `axes` is, or matplotlib's `TypeError` for another axes.
fn parasite_of(axes: &Axes) -> PyResult<ParasiteAxes> {
    ParasiteAxes::of(axes).ok_or_else(|| PyErr::type_error("the axes is no parasite axes"))
}

/// `(left, bottom, right, top)` of the position the parasite `axes` has
/// of its own, in figure fractions (its `get_position(original=True)`).
fn own_position(axes: Axes) -> PyResult<(f64, f64, f64, f64)> {
    Ok(parasite_of(&axes)?.own_position().extents())
}

/// Gives the parasite `axes` the position `rect`, `[left, bottom, width,
/// height]` in figure fractions, as its own (its `set_position`).
fn set_own_position(axes: Axes, rect: (f64, f64, f64, f64)) -> PyResult<()> {
    let (left, bottom, width, height) = rect;
    parasite_of(&axes)?.set_own_position(Bbox::from_bounds(left, bottom, width, height));
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("aux_axes", aux_axes)?;
    module.add("twin", twin)?;
    module.add("parasites", parasites)?;
    module.add("own_position", own_position)?;
    module.add("set_own_position", set_own_position)?;
    module.add("remove_parasite", remove_parasite)
}
