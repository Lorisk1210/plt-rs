//! The look of a 3-D axis a program sets through matplotlib's
//! `axis._axinfo` (`grid`, `tick`, `label`), handed over before every
//! draw, and the box a program's `_get_coord_info` gives it, handed over
//! as every draw begins.

use pltrs::colors::Color;
use pltrs::mplot3d::{Axes3D, AxisBox, AxisInfo};

use super::super::lines::PyLineStyle;
use super::super::text::parse_name;
use super::axis3;
use crate::binding::prelude::*;

/// Draws axis `name` with the `grid` `(color, linewidth, linestyle)`, the
/// `tick` `(major width, minor width, inward factor, outward factor)` and
/// the `label` `(ha, va, rotation_mode)` of its `_axinfo`.
fn set_axis_info(
    axes: Axes3D,
    name: String,
    grid: (Color, f64, PyLineStyle),
    tick: (f64, f64, f64, f64),
    label: (String, String, String),
) -> PyResult<()> {
    let (grid_color, grid_width, PyLineStyle(grid_style)) = grid;
    let (major_tick_width, minor_tick_width, tick_inward, tick_outward) = tick;
    let (ha, va, rotation_mode) = label;
    axes.set_axis_info(
        axis3(&name)?,
        AxisInfo {
            grid_color,
            grid_width,
            grid_style,
            major_tick_width,
            minor_tick_width,
            tick_inward,
            tick_outward,
            label_halign: parse_name(&ha)?,
            label_valign: parse_name(&va)?,
            label_rotation_mode: parse_name(&rotation_mode)?,
        },
    );
    Ok(())
}

/// The box `(mins, maxs, corners, highs, centers, deltas)` axis `name`
/// is drawn on (matplotlib's `_get_coord_info` and `_calc_centers_deltas`:
/// three numbers each but for the eight projected corners and the three
/// flags), or `None` for the axes' own.
type GivenBox = (
    Vec<f64>,
    Vec<f64>,
    Vec<Vec<f64>>,
    Vec<bool>,
    Vec<f64>,
    Vec<f64>,
);

/// Draws axis `name` on `given` (see [`GivenBox`]) from the next draw on.
fn set_axis_box(axes: Axes3D, name: String, given: Option<GivenBox>) -> PyResult<()> {
    let given = match given {
        None => None,
        Some((mins, maxs, corners, highs, centers, deltas)) => {
            let corners: Vec<[f64; 3]> = corners
                .iter()
                .map(|corner| three(corner))
                .collect::<PyResult<_>>()?;
            Some(AxisBox {
                mins: three(&mins)?,
                maxs: three(&maxs)?,
                corners: corners
                    .try_into()
                    .map_err(|_| PyErr::value_error("a box has eight corners"))?,
                highs: highs
                    .try_into()
                    .map_err(|_| PyErr::value_error("a box has three planes' flags"))?,
                centers: three(&centers)?,
                deltas: three(&deltas)?,
            })
        }
    };
    axes.set_axis_box(axis3(&name)?, given);
    Ok(())
}

/// The three coordinates of `values`.
fn three(values: &[f64]) -> PyResult<[f64; 3]> {
    values
        .try_into()
        .map_err(|_| PyErr::value_error("expected three coordinates"))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("set_axis_info", set_axis_info)?;
    module.add("set_axis_box", set_axis_box)
}
