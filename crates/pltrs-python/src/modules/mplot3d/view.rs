//! The camera, the box and the decorations of a 3-D axes (matplotlib's
//! `view_init`, `set_proj_type`, `set_box_aspect`, `set_aspect`, the axis
//! labels, `set_axis_off`, `grid`, the pane colours, `get_proj`) and what
//! the status bar shows over it (`format_coord`).

use pltrs::axes::Adjustable;
use pltrs::colors::Color;
use pltrs::mplot3d::{Aspect3D, Axes3D};
use pltrs::navigation::DragKey;
use pltrs_core::Point;

use super::axis3;
use super::create::proj_type_of;
use crate::binding::prelude::*;

/// Turns the camera to `(elev, azim, roll)` with `vertical` up, the axes
/// sharing the view following when `share` (`view_init`).
fn view_init(
    axes: Axes3D,
    (elev, azim, roll): (f64, f64, f64),
    vertical: String,
    share: bool,
) -> PyResult<()> {
    axes.set_vertical_axis(axis3(&vertical)?);
    if share {
        axes.view_init_shared(elev, azim, roll);
    } else {
        axes.view_init(elev, azim, roll);
    }
    Ok(())
}

/// The view angles `(elev, azim, roll)` (`ax.elev`, `ax.azim`,
/// `ax.roll`).
fn get_view(axes: Axes3D) -> PyResult<(f64, f64, f64)> {
    let view = axes.view();
    Ok((view.elev, view.azim, view.roll))
}

/// Sets the projection, `"persp"` with `focal_length` (`None`: 1) or
/// `"ortho"` (`set_proj_type`).
fn set_proj_type(axes: Axes3D, name: String, focal_length: Option<f64>) -> PyResult<()> {
    axes.set_proj_type(proj_type_of(&name, focal_length)?)?;
    Ok(())
}

/// Sets the x:y:z ratio of the box edges (`None`: 4:4:3) and its zoom
/// (`set_box_aspect`).
fn set_box_aspect(axes: Axes3D, aspect: Option<(f64, f64, f64)>, zoom: f64) -> PyResult<()> {
    axes.set_box_aspect(aspect.map(|(x, y, z)| [x, y, z]), zoom)?;
    Ok(())
}

/// The x:y:z ratio of the box edges (`get_box_aspect`).
fn get_box_aspect(axes: Axes3D) -> PyResult<(f64, f64, f64)> {
    let [x, y, z] = axes.box_aspect();
    Ok((x, y, z))
}

/// Sets the aspect (`"auto"`, `"equal"`, `"equalxy"`, `"equalxz"`,
/// `"equalyz"`) and what keeps it (`"box"` or `"datalim"`;
/// `set_aspect`).
fn set_aspect(axes: Axes3D, aspect: String, adjustable: String) -> PyResult<()> {
    let adjustable = match adjustable.as_str() {
        "box" => Adjustable::Box,
        "datalim" => Adjustable::Datalim,
        _ => {
            return Err(PyErr::value_error(format!(
                "{adjustable:?} is not a valid value for adjustable; supported values are \
                 'box', 'datalim'"
            )));
        }
    };
    axes.set_aspect(aspect.parse::<Aspect3D>()?, adjustable)?;
    Ok(())
}

/// The label of axis `name` (`get_zlabel`, ...).
fn get_label(axes: Axes3D, name: String) -> PyResult<String> {
    Ok(axes.label(axis3(&name)?))
}

/// Sets the label of axis `name` (`set_zlabel`, ...).
fn set_label(axes: Axes3D, name: String, text: String) -> PyResult<()> {
    axes.set_label(axis3(&name)?, text);
    Ok(())
}

/// Shows (`on`) or hides the panes, axes and gridlines (`set_axis_on`,
/// `set_axis_off`).
fn set_axis_on(axes: Axes3D, on: bool) -> PyResult<()> {
    if on {
        axes.set_axis_on();
    } else {
        axes.set_axis_off();
    }
    Ok(())
}

/// Whether the panes, axes and gridlines are drawn.
fn get_axis_on(axes: Axes3D) -> PyResult<bool> {
    Ok(axes.axis_on())
}

/// Shows or hides the gridlines on the panes (`grid`).
fn grid(axes: Axes3D, visible: bool) -> PyResult<()> {
    axes.grid(visible);
    Ok(())
}

/// Sets the colour of the pane normal to axis `name` (`set_pane_color`).
fn set_pane_color(axes: Axes3D, name: String, color: Color) -> PyResult<()> {
    axes.set_pane_color(axis3(&name)?, color);
    Ok(())
}

/// Orders collections and patches by depth or by their own z-order
/// (`computed_zorder`).
fn set_computed_zorder(axes: Axes3D, computed: bool) -> PyResult<()> {
    axes.set_computed_zorder(computed);
    Ok(())
}

/// The projection matrix of the current view, a 4 × 4 array (`get_proj`).
fn get_proj(axes: Axes3D) -> PyResult<NdArray> {
    let rows = axes.proj();
    NdArray::with_shape(rows.rows().iter().flatten().copied().collect(), vec![4, 4])
}

/// The status-bar text for projected point `(xv, yv)` (`format_coord`).
fn format_coord(axes: Axes3D, xv: f64, yv: f64) -> PyResult<String> {
    Ok(axes.format_coord(xv, yv))
}

/// The view angles as shown while rotating (`_rotation_coords`).
fn rotation_coords(axes: Axes3D) -> PyResult<String> {
    Ok(axes.rotation_coords())
}

/// The coordinate `value` along axis `name` as the status bar shows it
/// (`format_xdata`, ...); `None` when it cannot be formatted.
fn format_data(axes: Axes3D, name: String, value: f64) -> PyResult<Option<String>> {
    Ok(axes.format_data(axis3(&name)?, value))
}

/// Turns the view as dragging the rotate button from `start` to `end`
/// does (projected coordinates); `snap` rounds the angles to
/// `axes3d.snap_rotation` (`_on_move` with the rotate button).
fn drag_rotate(axes: Axes3D, start: (f64, f64), end: (f64, f64), snap: bool) -> PyResult<()> {
    axes.drag_rotate(Point::new(start.0, start.1), Point::new(end.0, end.1), snap);
    Ok(())
}

/// Zooms as dragging the zoom button from height `start_y` to `end_y`
/// does (projected coordinates; `_on_move` with the zoom button).
fn drag_zoom(axes: Axes3D, start_y: f64, end_y: f64) -> PyResult<()> {
    axes.drag_zoom(start_y, end_y)?;
    Ok(())
}

/// Moves the view by `(du, dv)` in projected coordinates (`drag_pan`).
fn pan_view(axes: Axes3D, du: f64, dv: f64) -> PyResult<()> {
    axes.pan_view(du, dv)?;
    Ok(())
}

/// Zooms into (or, with `out`, out of) the display rectangle `(x0, y0,
/// x1, y1)`, keeping the full height for `mode` `"x"` and the full width
/// for `"y"` (`_set_view_from_bbox`).
fn zoom_to_rect(
    axes: Axes3D,
    (x0, y0, x1, y1): (f64, f64, f64, f64),
    out: bool,
    mode: Option<String>,
) -> PyResult<()> {
    let key = match mode.as_deref() {
        Some("x") => DragKey::X,
        Some("y") => DragKey::Y,
        _ => DragKey::None,
    };
    axes.zoom_to_rect(Point::new(x0, y0), Point::new(x1, y1), out, key);
    Ok(())
}

/// Clears the axes and restores its defaults (`clear`, `cla`).
fn clear(axes: Axes3D) -> PyResult<()> {
    axes.clear()?;
    Ok(())
}

/// Stops the mouse from rotating, panning and zooming the axes
/// (`disable_mouse_rotation`).
fn disable_mouse_rotation(axes: Axes3D) -> PyResult<()> {
    axes.disable_mouse_rotation();
    Ok(())
}

/// Registers the functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("view_init", view_init)?;
    module.add("get_view", get_view)?;
    module.add("set_proj_type", set_proj_type)?;
    module.add("set_box_aspect", set_box_aspect)?;
    module.add("get_box_aspect", get_box_aspect)?;
    module.add("set_aspect", set_aspect)?;
    module.add("get_label", get_label)?;
    module.add("set_label", set_label)?;
    module.add("set_axis_on", set_axis_on)?;
    module.add("get_axis_on", get_axis_on)?;
    module.add("grid", grid)?;
    module.add("set_pane_color", set_pane_color)?;
    module.add("set_computed_zorder", set_computed_zorder)?;
    module.add("get_proj", get_proj)?;
    module.add("format_coord", format_coord)?;
    module.add("rotation_coords", rotation_coords)?;
    module.add("format_data", format_data)?;
    module.add("disable_mouse_rotation", disable_mouse_rotation)?;
    module.add("clear", clear)?;
    module.add("drag_rotate", drag_rotate)?;
    module.add("drag_zoom", drag_zoom)?;
    module.add("pan_view", pan_view)?;
    module.add("zoom_to_rect", zoom_to_rect)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs::figure::Figure;

    #[test]
    fn the_camera_and_labels_round_trip() {
        let ax = Figure::new().subplot_3d();
        view_init(ax.clone(), (10.0, 20.0, 30.0), "y".into(), false).unwrap();
        assert_eq!(get_view(ax.clone()).unwrap(), (10.0, 20.0, 30.0));
        set_label(ax.clone(), "z".into(), "depth".into()).unwrap();
        assert_eq!(get_label(ax.clone(), "z".into()).unwrap(), "depth");
        assert!(set_proj_type(ax.clone(), "persp".into(), Some(0.0)).is_err());
        assert!(set_aspect(ax.clone(), "equal".into(), "sideways".into()).is_err());
        set_axis_on(ax.clone(), false).unwrap();
        assert!(!get_axis_on(ax).unwrap());
    }
}
