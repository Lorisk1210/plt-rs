//! `_native.spines`: the spines framing an axes (matplotlib's `spines`).

use pltrs::artist::ArtistHandle;
use pltrs::axes::Axes;
use pltrs::colors::Color;
use pltrs::spines::{Spine, SpinePosition, SpineSide};

use super::lines::PyLineStyle;
use super::path::{PathArrays, path_arrays};
use crate::binding::prelude::*;

mod patch;

artist_handle!(Spine);

/// The name of a side, as matplotlib's `spine_type`.
pub(super) fn side_name(side: SpineSide) -> &'static str {
    match side {
        SpineSide::Left => "left",
        SpineSide::Right => "right",
        SpineSide::Bottom => "bottom",
        SpineSide::Top => "top",
    }
}

/// The spines of an axes with their names, in matplotlib's order: left,
/// right, bottom, top; for polar axes the outer arc (`polar`), the start
/// and the end of the sector, and the inner arc, which pltrs (Rust) keeps
/// as the top, left, right and bottom spines.
fn axes_spines(axes: Axes) -> PyResult<Vec<(&'static str, Spine)>> {
    let spines = axes.spines();
    let named: [(&'static str, SpineSide); 4] = if axes.as_polar().is_some() {
        [
            ("polar", SpineSide::Top),
            ("start", SpineSide::Left),
            ("end", SpineSide::Right),
            ("inner", SpineSide::Bottom),
        ]
    } else {
        [
            SpineSide::Left,
            SpineSide::Right,
            SpineSide::Bottom,
            SpineSide::Top,
        ]
        .map(|side| (side_name(side), side))
    };
    Ok(named
        .into_iter()
        .map(|(name, side)| (name, spines.get(side).clone()))
        .collect())
}

/// The side the spine frames (`spine_type`).
fn side(spine: Spine) -> PyResult<&'static str> {
    Ok(side_name(spine.side()))
}

/// A new spine of `axes` on the side `name` (`"left"`, `"right"`,
/// `"bottom"` or `"top"`), which the axes does not draw until it replaces
/// one (matplotlib's `Spine(axes, ...)`).
fn new_spine(axes: Axes, name: String) -> PyResult<Spine> {
    let side = SpineSide::from_name(&name)
        .ok_or_else(|| PyErr::value_error(format!("no side {name:?}")))?;
    Ok(axes.new_spine(side))
}

/// Draws `spine` in the place of `old` among the spines of `axes`
/// (matplotlib's `axes.spines[name] = spine`); whether it did.
fn replace_spine(axes: Axes, old: Spine, spine: Spine) -> PyResult<bool> {
    Ok(axes.replace_spine(&old, &spine))
}

/// Draws `spine`, a new spine of `axes`, beside the spines it draws as a
/// spine framing no side (a spine of a type naming no side in
/// matplotlib's `axes.spines`); whether it did.
fn add_spine(axes: Axes, spine: Spine) -> PyResult<bool> {
    Ok(axes.add_spine(&spine))
}

/// Whether the spine is one of the four of the sides of its axes, not a
/// spine of no side (`add_spine`).
fn frames_side(spine: Spine) -> PyResult<bool> {
    Ok(spine.frames_side())
}

/// Places the spine: `("outward", points)`, `("axes", fraction)` or
/// `("data", value)` (`set_position`).
fn set_position(spine: Spine, (kind, amount): (String, f64)) -> PyResult<()> {
    let position = match kind.as_str() {
        "outward" => SpinePosition::Outward(amount),
        "axes" => SpinePosition::Axes(amount),
        "data" => SpinePosition::Data(amount),
        _ => {
            return Err(PyErr::value_error(
                "position[0] should be one of 'outward', 'axes', or 'data' ",
            ));
        }
    };
    spine.set_position(position);
    Ok(())
}

/// Where the spine is placed, as `set_position` takes it.
fn get_position(spine: Spine) -> PyResult<(&'static str, f64)> {
    Ok(match spine.position() {
        SpinePosition::Outward(points) => ("outward", points),
        SpinePosition::Axes(fraction) => ("axes", fraction),
        SpinePosition::Data(value) => ("data", value),
    })
}

/// The data interval the spine is limited to, if any.
fn get_bounds(spine: Spine) -> PyResult<Option<(f64, f64)>> {
    Ok(spine.bounds())
}

/// Sets the spine's colour (`set_color`, its edge colour).
fn set_color(spine: Spine, color: Color) -> PyResult<()> {
    spine.set_color(color);
    Ok(())
}

/// The spine's colour as an RGBA tuple.
fn get_edgecolor(spine: Spine) -> PyResult<(f64, f64, f64, f64)> {
    let color = spine
        .edgecolor()
        .resolve()
        .map_err(|err| PyErr::value_error(err.to_string()))?;
    Ok((color.r, color.g, color.b, color.a))
}

/// Sets the spine's width in points.
fn set_linewidth(spine: Spine, width: f64) -> PyResult<()> {
    spine.set_linewidth(width);
    Ok(())
}

/// Sets the spine's line style: a name, a short form or an
/// `(offset, dashes)` tuple.
fn set_linestyle(spine: Spine, style: PyLineStyle) -> PyResult<()> {
    spine.set_linestyle(style.0);
    Ok(())
}

/// Limits the spine to the data interval from `low` to `high`, an end
/// given as `None` keeping the bound set before or else the view limit
/// (`set_bounds`).
fn set_bound_ends(spine: Spine, low: Option<f64>, high: Option<f64>) -> PyResult<()> {
    spine.set_bound_ends(low, high);
    Ok(())
}

/// The spine's display box with the tick marks drawn on it
/// (`get_window_extent`).
fn window_extent(spine: Spine) -> PyResult<Option<(f64, f64, f64, f64)>> {
    Ok(spine
        .window_extent()
        .map(|bbox| (bbox.x0(), bbox.y0(), bbox.x1(), bbox.y1())))
}

/// The spine line in display pixels, if drawn.
fn display_path(spine: Spine) -> PyResult<Option<PathArrays>> {
    spine
        .display_path()
        .map(|path| path_arrays(&path, false))
        .transpose()
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("axes_spines", axes_spines)?;
    module.add("side", side)?;
    module.add("new_spine", new_spine)?;
    module.add("replace_spine", replace_spine)?;
    module.add("frames_side", frames_side)?;
    module.add("add_spine", add_spine)?;
    module.add("set_position", set_position)?;
    module.add("get_position", get_position)?;
    module.add("set_bound_ends", set_bound_ends)?;
    module.add("window_extent", window_extent)?;
    module.add("display_path", display_path)?;
    module.add("get_bounds", get_bounds)?;
    module.add("set_color", set_color)?;
    module.add("get_edgecolor", get_edgecolor)?;
    module.add("set_linewidth", set_linewidth)?;
    module.add("set_linestyle", set_linestyle)?;
    patch::register(module)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs::figure::Figure;

    #[test]
    fn spines_come_in_matplotlibs_order_and_move() {
        let ax = Figure::new().subplot();
        let spines = axes_spines(ax).unwrap();
        let names: Vec<&str> = spines.iter().map(|(name, _)| *name).collect();
        assert_eq!(names, ["left", "right", "bottom", "top"]);
        let left = spines[0].1.clone();
        set_position(left.clone(), ("outward".into(), 10.0)).unwrap();
        assert_eq!(get_position(left.clone()).unwrap(), ("outward", 10.0));
        assert!(set_position(left.clone(), ("inward".into(), 1.0)).is_err());
        set_bound_ends(left.clone(), Some(0.25), None).unwrap();
        assert_eq!(get_bounds(left.clone()).unwrap(), Some((0.25, 1.0)));
        assert!(window_extent(left.clone()).unwrap().is_some());
        let path = display_path(left).unwrap().expect("a drawn spine");
        assert_eq!(path.0.shape, [2, 2]);
    }

    #[test]
    fn polar_spines_take_matplotlibs_names() {
        let ax = Figure::new().polar_subplot().axes().clone();
        let names: Vec<&str> = axes_spines(ax)
            .unwrap()
            .iter()
            .map(|(name, _)| *name)
            .collect();
        assert_eq!(names, ["polar", "start", "end", "inner"]);
    }
}
