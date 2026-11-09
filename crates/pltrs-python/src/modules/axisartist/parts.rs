//! What the parts of an axis artist are like now: where the tick labels
//! land and how large they are, and the directions of the tick labels and
//! the label; what its helper places.

use pltrs::axisartist::{AxisArtist, LabelDirection};

use super::super::path::{PathArrays, path_arrays};

use crate::binding::prelude::*;

/// The tick labels of `level` (`"major"` or `"minor"`).
fn tick_labels(artist: &AxisArtist, level: &str) -> pltrs::axisartist::TickLabels {
    if level == "minor" {
        artist.minor_ticklabels()
    } else {
        artist.major_ticklabels()
    }
}

/// The display boxes `(x0, y0, x1, y1)` of the tick labels of `level`
/// (`TickLabels.get_window_extents`).
fn ticklabel_extents(artist: AxisArtist, level: String) -> PyResult<Vec<(f64, f64, f64, f64)>> {
    Ok(tick_labels(&artist, &level)
        .window_extents()
        .iter()
        .map(|b| (b.x0(), b.y0(), b.x1(), b.y1()))
        .collect())
}

/// The width, height and descent in pixels of each non-empty tick label
/// of `level` (`TickLabels.get_texts_widths_heights_descents`).
fn ticklabel_metrics(artist: AxisArtist, level: String) -> PyResult<Vec<(f64, f64, f64)>> {
    Ok(tick_labels(&artist, &level).texts_widths_heights_descents())
}

/// How far the tick labels of `level` sat from their ticks, and the room
/// they left the axis label, in pixels, when last laid out
/// (`TickLabels._offset_radius` and `_axislabel_pad`).
fn ticklabel_offsets(artist: AxisArtist, level: String) -> PyResult<(f64, f64)> {
    Ok(tick_labels(&artist, &level).laid_out_offsets())
}

/// The direction `"+"` or `"-"` names.
fn direction(name: &str) -> PyResult<LabelDirection> {
    match name {
        "+" => Ok(LabelDirection::Plus),
        "-" => Ok(LabelDirection::Minus),
        _ => Err(PyErr::value_error(format!(
            "{name:?} is not a valid direction; supported values are '+', '-'"
        ))),
    }
}

/// Puts the tick labels on the `"+"` or `"-"` side of the line
/// (`set_ticklabel_direction`).
fn set_ticklabel_direction(artist: AxisArtist, name: String) -> PyResult<()> {
    artist.set_ticklabel_direction(direction(&name)?);
    Ok(())
}

/// Puts the label on the `"+"` or `"-"` side of the line
/// (`set_axislabel_direction`).
fn set_axislabel_direction(artist: AxisArtist, name: String) -> PyResult<()> {
    artist.set_axislabel_direction(direction(&name)?);
    Ok(())
}

/// The name of `direction`, `"+"` or `"-"`.
fn direction_name(direction: LabelDirection) -> &'static str {
    match direction {
        LabelDirection::Plus => "+",
        LabelDirection::Minus => "-",
    }
}

/// The sides of the line the tick labels and the label go, `"+"` or
/// `"-"` each (matplotlib's `_ticklabel_add_angle` and
/// `_axislabel_add_angle`, 0 and 180).
fn get_label_directions(artist: AxisArtist) -> PyResult<(&'static str, &'static str)> {
    Ok((
        direction_name(artist.ticklabel_direction()),
        direction_name(artist.axislabel_direction()),
    ))
}

/// A tick of a helper: `((x, y), angle_normal, angle_tangent, label)`.
type HelperTick = ((f64, f64), f64, f64, String);

/// The major and minor ticks the helper of the axis artist places
/// (`get_tick_iterators`).
fn helper_ticks(artist: AxisArtist) -> PyResult<(Vec<HelperTick>, Vec<HelperTick>)> {
    let ticks = |ticks: Vec<pltrs::axisartist::TickInfo>| -> Vec<HelperTick> {
        ticks
            .into_iter()
            .map(|t| ((t.loc.x, t.loc.y), t.angle_normal, t.angle_tangent, t.label))
            .collect()
    };
    let (major, minor) = artist.helper_ticks();
    Ok((ticks(major), ticks(minor)))
}

/// The line the helper of the axis artist runs along (`get_line`), as
/// `(vertices, codes)`; `None` when the axes is gone.
fn helper_line(artist: AxisArtist) -> PyResult<Option<PathArrays>> {
    artist
        .helper_line()
        .map(|(path, _)| path_arrays(&path, false))
        .transpose()
}

/// Where the helper of the axis artist puts the label, in display
/// pixels, and the direction of the line there (`get_axislabel_pos_angle`).
fn helper_label_position(artist: AxisArtist) -> PyResult<Option<((f64, f64), f64)>> {
    Ok(artist
        .helper_label_position()
        .map(|(at, angle)| ((at.x, at.y), angle)))
}

/// What matplotlib's `AxisArtist` computes when it is made (`_init_line`).
fn init_line(artist: AxisArtist) -> PyResult<()> {
    artist.init_line();
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("init_line", init_line)?;
    module.add("ticklabel_extents", ticklabel_extents)?;
    module.add("ticklabel_metrics", ticklabel_metrics)?;
    module.add("ticklabel_offsets", ticklabel_offsets)?;
    module.add("set_ticklabel_direction", set_ticklabel_direction)?;
    module.add("get_label_directions", get_label_directions)?;
    module.add("set_axislabel_direction", set_axislabel_direction)?;
    module.add("helper_ticks", helper_ticks)?;
    module.add("helper_line", helper_line)?;
    module.add("helper_label_position", helper_label_position)
}
