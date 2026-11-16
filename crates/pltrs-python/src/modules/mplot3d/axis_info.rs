//! The look of a 3-D axis a program sets through matplotlib's
//! `axis._axinfo` (`grid`, `tick`, `label`), handed over before every
//! draw.

use pltrs::colors::Color;
use pltrs::mplot3d::{Axes3D, AxisInfo};

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

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("set_axis_info", set_axis_info)
}
