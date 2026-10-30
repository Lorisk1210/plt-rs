//! Dividers (matplotlib's `axes_grid1.axes_divider`): the Python
//! `Divider` classes hold a native [`Divider`] of their figure and keep it
//! in step with their sizes; their locators place axes through it.

use pltrs::axes::Axes;
use pltrs::axes_grid1::{AxesDivider, Divider};
use pltrs::figure::Figure;
use pltrs::gridspec::SubplotSpec;

use super::sizes::{PySize, side_named, unwrap_sizes};
use crate::binding::prelude::*;

native_value!(Divider);

/// A divider of the rectangle `[left, bottom, width, height]` of `figure`
/// (`Divider(fig, pos, horizontal, vertical)`).
fn new(
    figure: Figure,
    (left, bottom, width, height): (f64, f64, f64, f64),
    horizontal: Vec<PySize>,
    vertical: Vec<PySize>,
) -> PyResult<Divider> {
    Ok(Divider::new(
        &figure,
        [left, bottom, width, height],
        unwrap_sizes(horizontal),
        unwrap_sizes(vertical),
        false,
    ))
}

/// A divider of the cells of `spec`, of the kind `kind`: `"grid"`
/// (`SubplotDivider`), `"hbox"` (`HBoxDivider`) or `"vbox"`
/// (`VBoxDivider`).
fn subplot(
    figure: Figure,
    spec: SubplotSpec,
    kind: String,
    horizontal: Vec<PySize>,
    vertical: Vec<PySize>,
) -> PyResult<Divider> {
    let (horizontal, vertical) = (unwrap_sizes(horizontal), unwrap_sizes(vertical));
    Ok(match kind.as_str() {
        "grid" => Divider::subplot(&figure, &spec, horizontal, vertical, false),
        "hbox" => Divider::hbox(&figure, &spec, horizontal, vertical),
        "vbox" => Divider::vbox(&figure, &spec, horizontal, vertical),
        _ => return Err(PyErr::value_error(format!("no divider kind {kind:?}"))),
    })
}

/// The divider of `axes`' original position (`AxesDivider(axes)`), which
/// does not place `axes` yet.
fn of_axes(axes: Axes) -> PyResult<Divider> {
    Ok(AxesDivider::new(&axes).divider())
}

/// Replaces the column widths (`set_horizontal`).
fn set_horizontal(divider: Divider, sizes: Vec<PySize>) -> PyResult<()> {
    divider.set_horizontal(unwrap_sizes(sizes));
    Ok(())
}

/// Replaces the row heights (`set_vertical`).
fn set_vertical(divider: Divider, sizes: Vec<PySize>) -> PyResult<()> {
    divider.set_vertical(unwrap_sizes(sizes));
    Ok(())
}

/// Adds `size` on `side` (`append_size`).
fn append_size(divider: Divider, side: String, size: PySize) -> PyResult<()> {
    divider.append_size(side_named(&side)?, size.0);
    Ok(())
}

/// Whether the relative sizes keep their proportions (`set_aspect`).
fn set_aspect(divider: Divider, aspect: bool) -> PyResult<()> {
    divider.set_aspect(aspect);
    Ok(())
}

/// Where the cells sit when they do not fill the rectangle
/// (`set_anchor`): a code such as `"SW"` or a pair of fractions.
fn set_anchor(divider: Divider, anchor: Owned) -> PyResult<()> {
    let anchor = Python::with_gil(|py| super::super::transforms::anchor(&anchor.bind(py)))?;
    divider.set_anchor(anchor);
    Ok(())
}

/// Divides the rectangle `[left, bottom, width, height]` from now on
/// (`set_position`).
fn set_position(
    divider: Divider,
    (left, bottom, width, height): (f64, f64, f64, f64),
) -> PyResult<()> {
    divider.set_position([left, bottom, width, height]);
    Ok(())
}

/// Divides the cells of `spec` from now on (`set_subplotspec`).
fn set_subplotspec(divider: Divider, spec: SubplotSpec) -> PyResult<()> {
    divider.set_subplot_spec(&spec);
    Ok(())
}

/// Places `axes` over the block of cells `columns` × `rows`, counted as
/// the divider's own `new_locator` counts them
/// (`ax.set_axes_locator(divider.new_locator(...))`).
fn place(
    divider: Divider,
    axes: Axes,
    columns: (isize, isize),
    rows: (isize, isize),
) -> PyResult<()> {
    divider
        .place_spanning(&axes, columns.0..columns.1, rows.0..rows.1)
        .map_err(|err| PyErr::value_error(err.to_string()))?;
    Ok(())
}

/// Where the block of cells `columns` × `rows` is now, as `(x0, y0,
/// width, height)` in figure fractions (calling a divider's locator).
fn locate(
    divider: Divider,
    columns: (isize, isize),
    rows: (isize, isize),
) -> PyResult<(f64, f64, f64, f64)> {
    divider
        .locate(columns.0..columns.1, rows.0..rows.1)
        .map(|cell| cell.bounds())
        .ok_or_else(|| PyErr::new(ExceptionKind::IndexError, "index out of range"))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("divider_new", new)?;
    module.add("divider_subplot", subplot)?;
    module.add("divider_of_axes", of_axes)?;
    module.add("divider_set_horizontal", set_horizontal)?;
    module.add("divider_set_vertical", set_vertical)?;
    module.add("divider_append_size", append_size)?;
    module.add("divider_set_aspect", set_aspect)?;
    module.add("divider_set_anchor", set_anchor)?;
    module.add("divider_set_position", set_position)?;
    module.add("divider_set_subplotspec", set_subplotspec)?;
    module.add("divider_place", place)?;
    module.add("divider_locate", locate)
}
