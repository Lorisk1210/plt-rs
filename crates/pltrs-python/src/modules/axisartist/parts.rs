//! What the parts of an axis artist are like now: the colours and widths
//! they take of their own or follow from their reference artists
//! (matplotlib's `AttributeCopier`), where the tick labels land, and the
//! directions of the tick labels and the label.

use pltrs::axisartist::{AxisArtist, LabelDirection};
use pltrs::colors::Color;

use super::super::path::{PathArrays, path_arrays};
use super::artists::{label_part, ticks};
use crate::binding::prelude::*;

/// A colour as it was given: its string, or its RGBA components.
enum GivenColor {
    Spec(String),
    Rgba((f64, f64, f64, f64)),
}

impl IntoPy for GivenColor {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        match self {
            Self::Spec(spec) => spec.into_py(py),
            Self::Rgba(rgba) => rgba.into_py(py),
        }
    }
}

/// A colour as it was given, `None` for one that follows the reference
/// artist.
type Rgba = Option<GivenColor>;

/// `color` as it was given.
fn rgba(color: Option<Color>) -> PyResult<Rgba> {
    Ok(color.map(|color| match color {
        Color::Spec(spec) => GivenColor::Spec(spec),
        Color::Rgba(c) => GivenColor::Rgba((c.r, c.g, c.b, c.a)),
    }))
}

/// The colour and width of the tick marks of `level` set on them, each
/// `None` when it follows the reference axis' ticks (`"auto"`).
fn get_tick_style(artist: AxisArtist, level: String) -> PyResult<(Rgba, Option<f64>)> {
    let ticks = ticks(&artist, &level);
    Ok((rgba(ticks.color())?, ticks.markeredgewidth()))
}

/// The colour set on a text part, `None` for the default (`"auto"`).
fn get_label_color(artist: AxisArtist, part: String) -> PyResult<Rgba> {
    rgba(label_part(&artist, &part)?.color())
}

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

/// Moves the tick labels of both levels to the other side of the line,
/// with the alignment of the opposite direction
/// (`invert_ticklabel_direction`).
fn invert_ticklabel_direction(artist: AxisArtist) -> PyResult<()> {
    artist.invert_ticklabel_direction();
    Ok(())
}

/// Puts the label on the `"+"` or `"-"` side of the line
/// (`set_axislabel_direction`).
fn set_axislabel_direction(artist: AxisArtist, name: String) -> PyResult<()> {
    artist.set_axislabel_direction(direction(&name)?);
    Ok(())
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

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("get_tick_style", get_tick_style)?;
    module.add("get_label_color", get_label_color)?;
    module.add("ticklabel_extents", ticklabel_extents)?;
    module.add("ticklabel_metrics", ticklabel_metrics)?;
    module.add("set_ticklabel_direction", set_ticklabel_direction)?;
    module.add("invert_ticklabel_direction", invert_ticklabel_direction)?;
    module.add("set_axislabel_direction", set_axislabel_direction)?;
    module.add("helper_ticks", helper_ticks)?;
    module.add("helper_line", helper_line)?;
    module.add("helper_label_position", helper_label_position)
}
