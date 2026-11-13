//! `_native.navigation`: how an axes takes part in interactive navigation
//! (the navigation methods of matplotlib's `axes._base`): whether the
//! toolbar acts on it, panning and zooming it, and the coordinates a
//! status bar shows for it. `axes/_navigation.py` calls it.
//!
//! A pan in progress is a native [`AxesPan`] the Python axes keeps as its
//! `_pan_start`, as matplotlib keeps the frozen state of the axes there.

use std::rc::Rc;

use pltrs::axes::{Axes, AxisName};
use pltrs::navigation::{
    AxesPan, AxesZoom, DragKey, PanLimits, PointerButton, ZoomDirection, ZoomTarget,
};
use pltrs_core::{Bbox, Point};

use crate::binding::prelude::*;
use crate::binding::wrap_native;

impl IntoPy for AxesPan {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        wrap_native(py, self)
    }
}

/// The axis called `name` (`"x"` or `"y"`).
fn axis_name(name: &str) -> PyResult<AxisName> {
    match name {
        "x" => Ok(AxisName::X),
        "y" => Ok(AxisName::Y),
        _ => Err(PyErr::value_error(format!("no axis named {name:?}"))),
    }
}

/// The button with matplotlib's number `number` (`MouseButton`), if any.
fn button(number: Option<u8>) -> Option<PointerButton> {
    number.and_then(PointerButton::from_number)
}

/// The key held while dragging, as matplotlib names it in `event.key`
/// (`"x"`, `"y"`, `"control"`, `"shift"`); others constrain nothing.
fn drag_key(key: Option<&str>) -> DragKey {
    match key {
        Some("x") => DragKey::X,
        Some("y") => DragKey::Y,
        Some("control") => DragKey::Control,
        Some("shift") => DragKey::Shift,
        _ => DragKey::None,
    }
}

/// Whether the navigation toolbar acts on the axes (`get_navigate`).
fn get_navigate(axes: Axes) -> PyResult<bool> {
    Ok(axes.navigate())
}

/// Lets the navigation toolbar act on the axes or not (`set_navigate`).
fn set_navigate(axes: Axes, navigate: bool) -> PyResult<()> {
    axes.set_navigate(navigate);
    Ok(())
}

/// Whether panning and zooming the axes reach the axes below it, `None`
/// for `"auto"` (`get_forward_navigation_events`).
fn get_forward_navigation_events(axes: Axes) -> PyResult<Option<bool>> {
    Ok(axes.forward_navigation_events())
}

/// Sets whether panning and zooming reach the axes below, `None` for
/// `"auto"` (`set_forward_navigation_events`).
fn set_forward_navigation_events(axes: Axes, forward: Option<bool>) -> PyResult<()> {
    axes.set_forward_navigation_events(forward);
    Ok(())
}

/// The axes and those twinned with it (`_twinned_axes.get_siblings`).
fn twins(axes: Axes) -> PyResult<Vec<Axes>> {
    Ok(axes.twins())
}

/// Counts `other` among the twins of `axes` (`_twinned_axes.join`).
fn join_twins(axes: Axes, other: Axes) -> PyResult<()> {
    axes.join_twins(&other);
    Ok(())
}

/// Whether the pan/zoom tool works on the axes (`can_pan`).
fn can_pan(axes: Axes) -> PyResult<bool> {
    Ok(axes.can_pan())
}

/// Whether the zoom-to-rectangle tool works on the axes (`can_zoom`).
fn can_zoom(axes: Axes) -> PyResult<bool> {
    Ok(axes.can_zoom())
}

/// Starts panning the axes from display point `(x, y)` with the button
/// numbered `number` (`start_pan`); `None` for no button or an axes not
/// laid out.
fn start_pan(axes: Axes, x: f64, y: f64, number: Option<u8>) -> PyResult<Option<AxesPan>> {
    Ok(button(number).and_then(|button| axes.start_pan(Point::new(x, y), button)))
}

/// The limits a pan gives with the pointer at `(x, y)` (`_get_pan_points`):
/// `((x0, x1), (y0, y1))`, each end `None` where it would not be finite;
/// `None` when nothing moves.
fn pan_limits(
    pan: Native<AxesPan>,
    axes: Axes,
    number: Option<u8>,
    key: Option<String>,
    x: f64,
    y: f64,
) -> PyResult<Option<PanLimits>> {
    let pan = pan.borrow();
    Ok(button(number)
        .and_then(|button| pan.limits(&axes, button, drag_key(key.as_deref()), Point::new(x, y))))
}

/// The pointer moved to `(x, y)` during a pan (`drag_pan`).
fn drag_pan(
    pan: Native<AxesPan>,
    axes: Axes,
    number: Option<u8>,
    key: Option<String>,
    x: f64,
    y: f64,
) -> PyResult<()> {
    if let Some(button) = button(number) {
        pan.borrow()
            .drag(&axes, button, drag_key(key.as_deref()), Point::new(x, y));
    }
    Ok(())
}

/// The pan ended (`end_pan`).
fn end_pan(pan: Native<AxesPan>, axes: Axes) -> PyResult<()> {
    pan.borrow().end(&axes);
    Ok(())
}

/// The zoom the arguments of `_set_view_from_bbox` describe: `bbox` is
/// `(x0, y0, x1, y1)` in display coordinates or `(x, y, scale)`.
fn zoom(
    bbox: &[f64],
    direction: &str,
    mode: Option<&str>,
    twinx: bool,
    twiny: bool,
) -> PyResult<AxesZoom> {
    let target = match *bbox {
        [x, y, scale] => ZoomTarget::Around {
            at: Point::new(x, y),
            scale,
        },
        [x0, y0, x1, y1] => ZoomTarget::Box(Bbox::from_extents(x0, y0, x1, y1)),
        _ => {
            return Err(PyErr::value_error(
                "the zoom box is a tuple of length 3 or 4",
            ));
        }
    };
    let direction = match direction {
        "in" => ZoomDirection::In,
        "out" => ZoomDirection::Out,
        _ => {
            return Err(PyErr::value_error(format!(
                "{direction:?} is not a valid value for direction; supported values are 'in', \
                 'out'"
            )));
        }
    };
    Ok(AxesZoom {
        target,
        direction,
        mode: drag_key(mode),
        twinx,
        twiny,
    })
}

/// The x and the y bounds of an axes.
type Bounds = ((f64, f64), (f64, f64));

/// The x and y bounds a zoom gives (`_prepare_view_from_bbox`), `None` if
/// the axes is not laid out.
fn zoomed_bounds(
    axes: Axes,
    bbox: Vec<f64>,
    direction: String,
    mode: Option<String>,
    twinx: bool,
    twiny: bool,
) -> PyResult<Option<Bounds>> {
    let zoom = zoom(&bbox, &direction, mode.as_deref(), twinx, twiny)?;
    Ok(axes.zoomed_bounds(&zoom))
}

/// Zooms the axes (`_set_view_from_bbox`).
fn zoom_view(
    axes: Axes,
    bbox: Vec<f64>,
    direction: String,
    mode: Option<String>,
    twinx: bool,
    twiny: bool,
) -> PyResult<()> {
    axes.zoom_view(&zoom(&bbox, &direction, mode.as_deref(), twinx, twiny)?);
    Ok(())
}

/// Whether display point `(x, y)` is over the axes background
/// (`contains_point`).
fn contains_point(axes: Axes, x: f64, y: f64) -> PyResult<bool> {
    Ok(axes.contains_point(Point::new(x, y)))
}

/// The data of the topmost visible artist with `mouseover` set under
/// display point `(x, y)`, as a status bar shows it.
fn cursor_text(axes: Axes, x: f64, y: f64) -> PyResult<Option<String>> {
    Ok(axes.cursor_text(Point::new(x, y)))
}

/// Formats the coordinates along axis `name` with the Python function
/// `function`, called with the value, or with the axis' major formatter
/// again for `None` (`fmt_xdata`, `fmt_ydata`).
fn set_fmt_data(axes: Axes, name: String, function: Option<Owned>) -> PyResult<()> {
    let formatter = function
        .map(|function| {
            Python::with_gil(|py| super::archive::data_formatter(&function.bind(py)))
                .map(|formatter| Rc::new(formatter) as Rc<dyn pltrs::ticker::Formatter>)
        })
        .transpose()?;
    axes.set_fmt_data(axis_name(&name)?, formatter);
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("get_navigate", get_navigate)?;
    module.add("set_navigate", set_navigate)?;
    module.add(
        "get_forward_navigation_events",
        get_forward_navigation_events,
    )?;
    module.add(
        "set_forward_navigation_events",
        set_forward_navigation_events,
    )?;
    module.add("twins", twins)?;
    module.add("join_twins", join_twins)?;
    module.add("can_pan", can_pan)?;
    module.add("can_zoom", can_zoom)?;
    module.add("start_pan", start_pan)?;
    module.add("pan_limits", pan_limits)?;
    module.add("drag_pan", drag_pan)?;
    module.add("end_pan", end_pan)?;
    module.add("zoomed_bounds", zoomed_bounds)?;
    module.add("zoom_view", zoom_view)?;
    module.add("contains_point", contains_point)?;
    module.add("cursor_text", cursor_text)?;
    module.add("set_fmt_data", set_fmt_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs::figure::Figure;

    #[test]
    fn zoom_arguments_are_checked() {
        assert!(zoom(&[1.0, 2.0], "in", None, false, false).is_err());
        assert!(zoom(&[1.0, 2.0, 3.0, 4.0], "sideways", None, false, false).is_err());
        let around = zoom(&[1.0, 2.0, 3.0], "in", Some("x"), true, false).unwrap();
        assert_eq!(around.mode, DragKey::X);
        assert!(
            matches!(around.target, ZoomTarget::Around { scale, .. } if scale.to_bits() == 3.0_f64.to_bits())
        );
    }

    #[test]
    fn a_zoom_into_a_box_gives_its_bounds() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_xlim(0.0, 10.0).unwrap();
        ax.set_ylim(0.0, 10.0).unwrap();
        let corner = |x, y| ax.data_to_display(Point::new(x, y)).unwrap();
        let (a, b) = (corner(2.0, 3.0), corner(4.0, 7.0));
        let bounds = zoomed_bounds(
            ax.clone(),
            vec![a.x, a.y, b.x, b.y],
            "in".into(),
            Some("x".into()),
            false,
            false,
        )
        .unwrap()
        .unwrap();
        assert!((bounds.0.0 - 2.0).abs() < 1e-9 && (bounds.0.1 - 4.0).abs() < 1e-9);
        assert_eq!(bounds.1, (0.0, 10.0), "mode 'x' keeps y");
        assert_eq!(drag_key(Some("control")), DragKey::Control);
        assert_eq!(button(Some(3)), Some(PointerButton::Right));
        assert_eq!(button(Some(7)), None);
    }
}
