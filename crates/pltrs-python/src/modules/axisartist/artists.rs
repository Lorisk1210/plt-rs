//! Axes drawn by axis artists (matplotlib's `axisartist.axislines` and
//! `axis_artist`): the axis artists of an axes, the parts of each (ticks,
//! tick labels, label, line) and tick marks drawn on their own.

use pltrs::artist::RawHandle;
use pltrs::axes::{Axes, AxisName, GridOptions, TickDirection, TickLevel};
use pltrs::axisartist::{
    AxisArtist, AxisArtistAxes, AxislineStyle, AxislineStyleError, LabelBase, TickMarks, Toggle,
};
use pltrs::colors::Color;
use pltrs::text::{HAlign, VAlign};
use pltrs::transforms::Point;

use super::super::axes_grid1::side_named;
use super::super::lines::PyLineStyle;
use crate::binding::prelude::*;

artist_handle!(AxisArtist);

impl From<AxislineStyleError> for PyErr {
    /// matplotlib's `ValueError` for an unknown style or attribute.
    fn from(err: AxislineStyleError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

/// The axis called `name` (`"x"` or `"y"`, or 0 and 1 as matplotlib's
/// `nth_coord`).
pub(super) fn axis_named(name: &str) -> PyResult<AxisName> {
    match name {
        "x" | "0" => Ok(AxisName::X),
        "y" | "1" => Ok(AxisName::Y),
        _ => Err(PyErr::value_error(format!("no axis {name:?}"))),
    }
}

/// The axis artists of `axes`, by name, in creation order.
fn axis_artists(axes: Axes) -> PyResult<Vec<(String, AxisArtist)>> {
    Ok(AxisArtistAxes::of(axes)
        .axis_artists()
        .into_iter()
        .map(|artist| (artist.name(), artist))
        .collect())
}

/// Makes `axes` an axes drawn by axis artists: its spines and axes
/// hidden, gridlines through the rectilinear grid helper and an axis
/// artist on each edge (`axislines.Axes`); with `zero`, also the hidden
/// floating axes `xzero` and `yzero` (`AxesZero`).
fn install(axes: Axes, zero: bool) -> PyResult<()> {
    if zero {
        AxisArtistAxes::zero(axes);
    } else {
        AxisArtistAxes::new(axes);
    }
    Ok(())
}

/// Clears `axes` and puts its axis artists and gridlines back (`clear`).
fn clear(axes: Axes) -> PyResult<()> {
    AxisArtistAxes::of(axes).clear();
    Ok(())
}

/// A new axis artist `name` on edge `loc` of `axes`, moved by `offset`
/// points (`new_fixed_axis`).
fn new_fixed_axis(
    axes: Axes,
    name: String,
    loc: String,
    offset: (f64, f64),
) -> PyResult<AxisArtist> {
    Ok(AxisArtistAxes::of(axes).new_fixed_axis(&name, side_named(&loc)?, offset))
}

/// A new axis artist `name` at `value` of the coordinate `nth`
/// (matplotlib's `nth_coord`, `"x"` or `"y"`), its texts facing
/// `direction` (`new_floating_axis`).
fn new_floating_axis(
    axes: Axes,
    name: String,
    (nth, value): (String, f64),
    direction: String,
) -> PyResult<AxisArtist> {
    Ok(AxisArtistAxes::of(axes).new_floating_axis_facing(
        &name,
        axis_named(&nth)?,
        value,
        side_named(&direction)?,
    ))
}

/// Puts `artist` under `name` among the axis artists of `axes`,
/// replacing the one there (`ax.axis[name] = artist`).
fn assign(axes: Axes, name: String, artist: AxisArtist) -> PyResult<()> {
    AxisArtistAxes::of(axes)
        .assign(&name, &artist)
        .map_err(|err| PyErr::value_error(err.to_string()))?;
    Ok(())
}

/// The gridline style of `grid`: `(color, alpha, linewidth, linestyle)`.
type GridStyle = (Option<Color>, Option<f64>, Option<f64>, Option<PyLineStyle>);

/// Shows or hides the gridlines of `which` level along `axis` (`"x"`,
/// `"y"` or `"both"`), styled by `style` (`grid`).
fn grid(
    axes: Axes,
    visible: bool,
    (which, axis): (String, String),
    (color, alpha, linewidth, linestyle): GridStyle,
) -> PyResult<()> {
    let options = GridOptions {
        which: which
            .parse::<TickLevel>()
            .map_err(|err| PyErr::value_error(err.to_string()))?,
        axis: if axis == "both" {
            None
        } else {
            Some(axis_named(&axis)?)
        },
        color,
        alpha,
        linewidth,
        linestyle: linestyle.map(|style| style.0),
        ..GridOptions::default()
    };
    AxisArtistAxes::of(axes).grid(visible, options);
    Ok(())
}

/// Turns the axis artist's texts to the conventions of an axis on
/// `direction` (`set_axis_direction`).
fn set_axis_direction(artist: AxisArtist, direction: String) -> PyResult<()> {
    artist.set_axis_direction(side_named(&direction)?);
    Ok(())
}

/// Shows or hides the ticks, tick labels and label; `all` first
/// (`toggle`).
fn toggle(
    artist: AxisArtist,
    (all, ticks, ticklabels, label): (Option<bool>, Option<bool>, Option<bool>, Option<bool>),
) -> PyResult<()> {
    artist.toggle(Toggle {
        all,
        ticks,
        ticklabels,
        label,
    });
    Ok(())
}

/// Draws the line as `style` (`"->"`, `"-|>"`, with attributes such as
/// `",size=1.5"`), or plainly for `None` (`set_axisline_style`).
fn set_axisline_style(artist: AxisArtist, style: Option<String>) -> PyResult<()> {
    let style = style
        .map(|text| text.parse::<AxislineStyle>())
        .transpose()?;
    artist.set_axisline_style(style);
    Ok(())
}

/// The ticks of `level` (`"major"` or `"minor"`) of the axis artist.
fn ticks(artist: &AxisArtist, level: &str) -> pltrs::axisartist::Ticks {
    if level == "minor" {
        artist.minor_ticks()
    } else {
        artist.major_ticks()
    }
}

/// Sets the tick property `key` of `level` to `value` (`Ticks.set_*`):
/// `"ticksize"`, `"tick_direction"`, `"visible"`, `"color"`,
/// `"markeredgewidth"` or `"alpha"`.
fn set_ticks(artist: AxisArtist, level: String, key: String, value: Owned) -> PyResult<()> {
    let ticks = ticks(&artist, &level);
    Python::with_gil(|py| {
        let value = value.bind(py);
        match key.as_str() {
            "ticksize" => {
                ticks.set_ticksize(value.extract()?);
            }
            "tick_direction" => {
                let name: String = value.extract()?;
                let direction = name.parse::<TickDirection>().map_err(|_| {
                    PyErr::value_error(format!(
                        "{name:?} is not a valid value for tick_direction; supported values \
                         are 'in', 'out', 'inout'"
                    ))
                })?;
                ticks.set_tick_direction(direction);
            }
            "visible" => {
                ticks.set_visible(value.extract()?);
            }
            "color" => {
                ticks.set_color(value.extract::<Option<Color>>()?);
            }
            "markeredgewidth" => {
                ticks.set_markeredgewidth(value.extract()?);
            }
            "alpha" => {
                ticks.set_alpha(value.extract()?);
            }
            _ => return Err(PyErr::value_error(format!("no tick property {key:?}"))),
        }
        Ok(())
    })
}

/// A tick property of `level`: `(ticksize, tick_direction, visible)`.
fn get_ticks(artist: AxisArtist, level: String) -> PyResult<(f64, String, bool)> {
    let ticks = ticks(&artist, &level);
    Ok((
        ticks.ticksize(),
        ticks.tick_direction().name().to_owned(),
        ticks.visible(),
    ))
}

/// The text part `part` (`"major_ticklabels"`, `"minor_ticklabels"` or
/// `"label"`) of the axis artist.
fn label_part(artist: &AxisArtist, part: &str) -> PyResult<LabelBase> {
    Ok(match part {
        "major_ticklabels" => (*artist.major_ticklabels()).clone(),
        "minor_ticklabels" => (*artist.minor_ticklabels()).clone(),
        "label" => (*artist.label()).clone(),
        _ => return Err(PyErr::value_error(format!("no text part {part:?}"))),
    })
}

/// Sets the property `key` of a text part to `value` (`LabelBase.set_*`):
/// `"axis_direction"`, `"pad"`, `"visible"`, `"color"`, `"rotation"`,
/// `"ha"`, `"va"`, `"size"` or, for the label, `"text"`.
fn set_label(artist: AxisArtist, part: String, key: String, value: Owned) -> PyResult<()> {
    let label = label_part(&artist, &part)?;
    Python::with_gil(|py| {
        let value = value.bind(py);
        match key.as_str() {
            "axis_direction" => {
                label.set_axis_direction(side_named(&value.extract::<String>()?)?);
            }
            "pad" => {
                label.set_pad(value.extract()?);
            }
            "visible" => {
                label.set_visible(value.extract()?);
            }
            "color" => {
                label.set_color(value.extract::<Option<Color>>()?);
            }
            "rotation" => {
                label.set_rotation(value.extract()?);
            }
            "ha" => {
                label.set_ha(alignment::<HAlign>(&value.extract::<String>()?)?);
            }
            "va" => {
                label.set_va(alignment::<VAlign>(&value.extract::<String>()?)?);
            }
            "size" => {
                label.set_size(value.extract()?);
            }
            "text" if part == "label" => {
                artist.label().set_text(value.str()?);
            }
            "invert" if part != "label" => {
                artist.major_ticklabels().invert_axis_direction();
            }
            _ => return Err(PyErr::value_error(format!("no text property {key:?}"))),
        }
        Ok(())
    })
}

/// The alignment called `name`.
fn alignment<T: std::str::FromStr>(name: &str) -> PyResult<T> {
    name.parse()
        .map_err(|_| PyErr::value_error(format!("{name:?} is not a valid alignment")))
}

/// A text part's `(axis_direction, pad, visible, rotation, size, text)`.
fn get_label(
    artist: AxisArtist,
    part: String,
) -> PyResult<(String, f64, bool, f64, f64, Option<String>)> {
    let label = label_part(&artist, &part)?;
    let text = if part == "label" {
        artist.label().text()
    } else {
        None
    };
    Ok((
        label.axis_direction().name().to_owned(),
        label.pad(),
        label.visible(),
        label.rotation(),
        label.size(),
        text,
    ))
}

/// Sets the property `key` of the line to `value` (`line.set_*`):
/// `"color"`, `"linewidth"` or `"visible"`.
fn set_line(artist: AxisArtist, key: String, value: Owned) -> PyResult<()> {
    let line = artist.line();
    Python::with_gil(|py| {
        let value = value.bind(py);
        match key.as_str() {
            "color" => {
                line.set_color(value.extract::<Option<Color>>()?);
            }
            "linewidth" => {
                line.set_linewidth(value.extract()?);
            }
            "visible" => {
                line.set_visible(value.extract()?);
            }
            _ => return Err(PyErr::value_error(format!("no line property {key:?}"))),
        }
        Ok(())
    })
}

/// Whether the line is drawn.
fn get_line_visible(artist: AxisArtist) -> PyResult<bool> {
    Ok(artist.line().visible())
}

/// The extent of what the axis artist draws, as `(x0, y0, x1, y1)` in
/// display pixels (`get_tightbbox`); `None` when it draws nothing.
fn tight_bbox(artist: AxisArtist) -> PyResult<Option<(f64, f64, f64, f64)>> {
    Ok(artist
        .tight_bbox()
        .map(|bbox| (bbox.x0(), bbox.y0(), bbox.x1(), bbox.y1())))
}

/// The space between the label and the tick labels, in points.
fn get_labelpad(artist: AxisArtist) -> PyResult<f64> {
    Ok(artist.labelpad())
}

/// Sets the space between the label and the tick labels, in points.
fn set_labelpad(artist: AxisArtist, pad: f64) -> PyResult<()> {
    artist.set_labelpad(pad);
    Ok(())
}

/// The tick properties of standalone tick marks: `(size, direction,
/// color, width)`.
type MarkStyle = (f64, String, Option<Color>, Option<f64>);

/// Adds tick marks at `locs_angles` (data coordinates, angles in degrees)
/// to `axes`, styled like the major ticks of `axis` unless `style` says
/// otherwise (`ax.add_artist(Ticks(...))`).
fn add_tick_marks(
    axes: Axes,
    axis: String,
    locs_angles: Vec<((f64, f64), f64)>,
    (size, direction, color, width): MarkStyle,
) -> PyResult<RawHandle> {
    let locs_angles = locs_angles
        .into_iter()
        .map(|((x, y), angle)| (Point::new(x, y), angle))
        .collect();
    let marks = TickMarks {
        direction: direction
            .parse()
            .map_err(|_| PyErr::value_error(format!("no tick direction {direction:?}")))?,
        color,
        width,
        ..TickMarks::new(size, axis_named(&axis)?, locs_angles)
    };
    Ok(pltrs::artist::ArtistHandle::raw(&axes.add_tick_marks(marks)).clone())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("axis_artists", axis_artists)?;
    module.add("install", install)?;
    module.add("clear", clear)?;
    module.add("new_fixed_axis", new_fixed_axis)?;
    module.add("new_floating_axis", new_floating_axis)?;
    module.add("assign", assign)?;
    module.add("grid", grid)?;
    module.add("set_axis_direction", set_axis_direction)?;
    module.add("toggle", toggle)?;
    module.add("set_axisline_style", set_axisline_style)?;
    module.add("set_ticks", set_ticks)?;
    module.add("get_ticks", get_ticks)?;
    module.add("set_label", set_label)?;
    module.add("get_label", get_label)?;
    module.add("set_line", set_line)?;
    module.add("get_line_visible", get_line_visible)?;
    module.add("tight_bbox", tight_bbox)?;
    module.add("get_labelpad", get_labelpad)?;
    module.add("set_labelpad", set_labelpad)?;
    module.add("add_tick_marks", add_tick_marks)
}
