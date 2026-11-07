//! The camera, the box and the decorations of a 3-D axes (matplotlib's
//! `view_init`, `set_proj_type`, `set_box_aspect`, `set_aspect`, the axis
//! labels, `set_axis_off`, `grid`, the pane colours, `get_proj`) and what
//! the status bar shows over it (`format_coord`).

use pltrs::axes::Adjustable;
use pltrs::colors::Color;
use pltrs::mplot3d::{Aspect3D, Axes3D};
use pltrs::navigation::DragKey;
use pltrs::transforms::Bbox;
use pltrs_core::Point;
use pltrs_core::path::DataLimits;

use super::super::axes_base::aspect::anchor;
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

/// The ratio of the box edges as matplotlib keeps it, the vertical axis
/// last (`get_box_aspect`, `_box_aspect`).
fn get_box_aspect(axes: Axes3D) -> PyResult<(f64, f64, f64)> {
    let [x, y, z] = axes.stored_box_aspect();
    Ok((x, y, z))
}

/// Whether the x, y and z axes are drawn on their high side in the view
/// of the last draw, `None` before it (`get_axis_position`).
fn get_axis_position(axes: Axes3D) -> PyResult<Option<(bool, bool, bool)>> {
    Ok(axes.drawn_axis_position().map(|[x, y, z]| (x, y, z)))
}

/// A point of a pane in data coordinates and the index of the axis the
/// pane is normal to.
type PanePoint = ((f64, f64, f64), usize);

/// The point of the pane under projected point `(xv, yv)`, in data
/// coordinates, and the index of the axis the pane is normal to, through
/// the projection of the last draw and its inverse (`_calc_coord`);
/// `None` without them.
fn calc_coord(axes: Axes3D, xv: f64, yv: f64) -> PyResult<Option<PanePoint>> {
    Ok(axes
        .drawn_pane_point(xv, yv)
        .map(|([x, y, z], pane)| ((x, y, z), pane.index())))
}

/// Places the square box at the anchor named `code`, or at the fractions
/// `(x, y)` of its placement (`set_anchor`).
fn set_anchor(axes: Axes3D, code: Option<String>, fractions: (f64, f64)) -> PyResult<()> {
    axes.set_anchor(anchor(code, fractions)?);
    Ok(())
}

/// `(left, bottom, right, top)` of the axes in figure fractions: its box
/// made square, or before it is when `original` (`get_position`).
fn position(axes: Axes3D, original: bool) -> PyResult<(f64, f64, f64, f64)> {
    let bbox = if original {
        axes.original_position()
    } else {
        axes.squared_position()
    };
    Ok(bbox.extents())
}

/// The 2-D data limits of the projection plane, `((x0, y0, x1, y1),
/// (minposx, minposy), ignores)` (`ignores`: whether the next data
/// replaces them), to give back with [`restore_plane_data_lim`].
fn plane_data_lim(axes: Axes3D) -> PyResult<PlaneLimits> {
    let plane = axes.projection_plane();
    let limits = plane.data_lim();
    Ok((
        limits.bbox.extents(),
        (limits.minpos.x, limits.minpos.y),
        plane.ignores_data_limits(),
    ))
}

/// Gives the projection plane back its 2-D data limits taken by
/// [`plane_data_lim`] (matplotlib's `Axes3D.update_datalim` does nothing).
fn restore_plane_data_lim(
    axes: Axes3D,
    ((x0, y0, x1, y1), (minx, miny), ignores): PlaneLimits,
) -> PyResult<()> {
    let limits = DataLimits {
        bbox: Bbox::from_extents(x0, y0, x1, y1),
        minpos: Point::new(minx, miny),
    };
    axes.restore_plane_data_lim(limits, ignores);
    Ok(())
}

/// The 2-D data limits of a projection plane as Python holds them, and
/// whether the next data replaces them.
type PlaneLimits = ((f64, f64, f64, f64), (f64, f64), bool);

/// Moves the axes to `(left, bottom, width, height)` in figure fractions,
/// its box made square there at the next draw (`set_position`).
fn set_position(axes: Axes3D, (left, bottom, width, height): (f64, f64, f64, f64)) -> PyResult<()> {
    axes.set_position(Bbox::from_bounds(left, bottom, width, height));
    Ok(())
}

/// The tight bounding box of the axes in display coordinates, `(x0, y0,
/// x1, y1)`, or `None` for an invisible axes (`get_tightbbox`).
fn tightbbox(axes: Axes3D, for_layout_only: bool) -> PyResult<Option<(f64, f64, f64, f64)>> {
    Ok(axes
        .tight_bbox(for_layout_only)
        .map(|b| (b.x0(), b.y0(), b.x1(), b.y1())))
}

/// Shares axis `name` (`"x"`, `"y"`, `"z"`) or, for `"view"`, the view
/// angles with `other` (`sharex`, `sharey`, `sharez`, `shareview`).
fn share(axes: Axes3D, name: String, other: Axes3D) -> PyResult<()> {
    if name == "view" {
        axes.share_view(&other)?;
    } else {
        axes.share_axis(axis3(&name)?, &other)?;
    }
    Ok(())
}

/// The axes sharing axis `name` (`"x"`, `"y"`, `"z"`) or, for `"view"`,
/// the view angles with `axes`, `axes` included (the groups of
/// `_shared_axes`).
fn shared(axes: Axes3D, name: String) -> PyResult<Vec<Axes3D>> {
    if name == "view" {
        Ok(axes.shared_views())
    } else {
        Ok(axes.shared_axes(axis3(&name)?))
    }
}

/// The axes axis `name` or, for `"view"`, the view angles of `axes` were
/// explicitly shared with (`_sharex`, `_sharey`, `_sharez`,
/// `_shareview`).
fn share_target(axes: Axes3D, name: String) -> PyResult<Option<Axes3D>> {
    if name == "view" {
        Ok(axes.view_share_target())
    } else {
        Ok(axes.share_target(axis3(&name)?))
    }
}

/// Sets the aspect (`"auto"`, `"equal"`, `"equalxy"`, `"equalxz"`,
/// `"equalyz"`) and what keeps it (`"box"` or `"datalim"`;
/// `set_aspect`).
fn set_aspect(axes: Axes3D, aspect: String, adjustable: String) -> PyResult<()> {
    axes.set_aspect(aspect.parse::<Aspect3D>()?, adjustable_of(&adjustable)?)?;
    Ok(())
}

/// What `"box"` or `"datalim"` names (matplotlib's `adjustable`).
fn adjustable_of(name: &str) -> PyResult<Adjustable> {
    match name {
        "box" => Ok(Adjustable::Box),
        "datalim" => Ok(Adjustable::Datalim),
        _ => Err(PyErr::value_error(format!(
            "{name:?} is not a valid value for adjustable; supported values are 'box', \
             'datalim'"
        ))),
    }
}

/// Sets what keeps the aspect, `"box"` or `"datalim"`, the aspect left as
/// it is (`set_adjustable`).
fn set_adjustable(axes: Axes3D, adjustable: String) -> PyResult<()> {
    axes.set_adjustable(adjustable_of(&adjustable)?);
    Ok(())
}

/// The aspect (`"auto"`, `"equal"`, `"equalxy"`, `"equalxz"` or
/// `"equalyz"`) and what keeps it (`"box"` or `"datalim"`; `get_aspect`,
/// `get_adjustable`).
fn get_aspect(axes: Axes3D) -> PyResult<(&'static str, &'static str)> {
    let aspect = match axes.aspect() {
        Aspect3D::Auto => "auto",
        Aspect3D::Equal => "equal",
        Aspect3D::EqualXY => "equalxy",
        Aspect3D::EqualXZ => "equalxz",
        Aspect3D::EqualYZ => "equalyz",
    };
    let adjustable = match axes.adjustable() {
        Adjustable::Box => "box",
        Adjustable::Datalim => "datalim",
    };
    Ok((aspect, adjustable))
}

/// The label of axis `name` (`get_zlabel`, ...).
fn get_label(axes: Axes3D, name: String) -> PyResult<String> {
    Ok(axes.label(axis3(&name)?))
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

/// Whether the gridlines on the panes are drawn (`_draw_grid`).
fn get_grid(axes: Axes3D) -> PyResult<bool> {
    Ok(axes.grid_on())
}

/// Shows the window of projected coordinates the axes draws in
/// (`set_top_view`).
fn set_top_view(axes: Axes3D) -> PyResult<()> {
    axes.set_top_view();
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

/// The status-bar text for projected point `(xv, yv)` outside a
/// rotation, empty before the first draw (`format_coord`).
fn format_coord(axes: Axes3D, xv: f64, yv: f64) -> PyResult<String> {
    Ok(axes.drawn_format_coord(xv, yv))
}

/// The view angles as shown while rotating (`_rotation_coords`).
fn rotation_coords(axes: Axes3D) -> PyResult<String> {
    Ok(axes.rotation_coords())
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

/// Scales the limits by `scale_u`, `scale_v` and `scale_w` along the
/// screen's horizontal, vertical and depth directions
/// (`_zoom_data_limits`).
fn zoom_data_limits(axes: Axes3D, scale_u: f64, scale_v: f64, scale_w: f64) -> PyResult<()> {
    axes.zoom_data_limits(scale_u, scale_v, scale_w)?;
    Ok(())
}

/// The focal length of the camera, infinite for an orthographic
/// projection (`_focal_length`).
fn get_focal_length(axes: Axes3D) -> PyResult<f64> {
    Ok(axes.focal_length())
}

/// Registers the functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("view_init", view_init)?;
    module.add("get_view", get_view)?;
    module.add("set_proj_type", set_proj_type)?;
    module.add("set_box_aspect", set_box_aspect)?;
    module.add("get_box_aspect", get_box_aspect)?;
    module.add("get_axis_position", get_axis_position)?;
    module.add("calc_coord", calc_coord)?;
    module.add("share", share)?;
    module.add("shared", shared)?;
    module.add("share_target", share_target)?;
    module.add("position", position)?;
    module.add("set_position", set_position)?;
    module.add("plane_data_lim", plane_data_lim)?;
    module.add("restore_plane_data_lim", restore_plane_data_lim)?;
    module.add("set_anchor", set_anchor)?;
    module.add("tightbbox", tightbbox)?;
    module.add("set_aspect", set_aspect)?;
    module.add("set_adjustable", set_adjustable)?;
    module.add("get_aspect", get_aspect)?;
    module.add("get_label", get_label)?;
    module.add("set_axis_on", set_axis_on)?;
    module.add("get_axis_on", get_axis_on)?;
    module.add("grid", grid)?;
    module.add("get_grid", get_grid)?;
    module.add("set_top_view", set_top_view)?;
    module.add("set_pane_color", set_pane_color)?;
    module.add("set_computed_zorder", set_computed_zorder)?;
    module.add("get_proj", get_proj)?;
    module.add("format_coord", format_coord)?;
    module.add("rotation_coords", rotation_coords)?;
    module.add("zoom_data_limits", zoom_data_limits)?;
    module.add("get_focal_length", get_focal_length)?;
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
        ax.set_label(pltrs::mplot3d::Axis3::Z, "depth");
        assert_eq!(get_label(ax.clone(), "z".into()).unwrap(), "depth");
        assert!(set_proj_type(ax.clone(), "persp".into(), Some(0.0)).is_err());
        assert!(set_aspect(ax.clone(), "equal".into(), "sideways".into()).is_err());
        set_axis_on(ax.clone(), false).unwrap();
        assert!(!get_axis_on(ax).unwrap());
    }
}
