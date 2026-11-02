//! The shape of a spine (matplotlib's `Spine.set_patch_arc`,
//! `set_patch_circle`, `set_patch_line`) and the coordinates of its path
//! (`get_spine_transform`).

use pltrs::spines::{Spine, SpinePatch};
use pltrs::transforms::{Point, Transform};

use crate::binding::prelude::*;

/// Draws the spine as an arc of the circle of `radius` around `center`
/// (axes fractions) from `theta1` to `theta2` degrees.
fn set_patch_arc(
    spine: Spine,
    (x, y): (f64, f64),
    radius: f64,
    (theta1, theta2): (f64, f64),
) -> PyResult<()> {
    spine.set_patch_arc(Point::new(x, y), radius, theta1, theta2);
    Ok(())
}

/// Draws the spine as the circle of `radius` around `center`.
fn set_patch_circle(spine: Spine, (x, y): (f64, f64), radius: f64) -> PyResult<()> {
    spine.set_patch_circle(Point::new(x, y), radius);
    Ok(())
}

/// Draws the spine as a line along its side.
fn set_patch_line(spine: Spine) -> PyResult<()> {
    spine.set_patch_line();
    Ok(())
}

/// The kind of the spine's shape: `"line"`, `"arc"` or `"circle"`
/// (matplotlib's `_patch_type`).
fn patch_type(spine: Spine) -> PyResult<&'static str> {
    Ok(match spine.patch() {
        SpinePatch::Line => "line",
        SpinePatch::Arc { .. } => "arc",
        SpinePatch::Circle { .. } => "circle",
    })
}

/// The coordinates of the spine's path, `None` outside any axes.
fn spine_transform(spine: Spine) -> PyResult<Option<Transform>> {
    Ok(spine.spine_transform())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("set_patch_arc", set_patch_arc)?;
    module.add("set_patch_circle", set_patch_circle)?;
    module.add("set_patch_line", set_patch_line)?;
    module.add("patch_type", patch_type)?;
    module.add("spine_transform", spine_transform)
}
