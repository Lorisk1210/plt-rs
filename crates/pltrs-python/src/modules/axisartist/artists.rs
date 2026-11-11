//! Axes drawn by axis artists (matplotlib's `axisartist.axislines` and
//! `axis_artist`): the axis artists of an axes, the parts of each (ticks,
//! tick labels, label, line) and tick marks drawn on their own.

use pltrs::artist::RawHandle;
use pltrs::axes::{Axes, AxisName, GridOptions, TickDirection, TickLevel};
use pltrs::axisartist::{
    AxisArtist, AxisArtistAxes, AxislineStyle, AxislineStyleError, GridlinesChoice, LabelBase,
    TickMarks, TickMarksArtist, choose_gridlines,
};
use pltrs::collections::Collection;
use pltrs::colors::Color;
use pltrs::text::{HAlign, Text, VAlign};
use pltrs::transforms::{Point, Transform};

use super::super::axes::removed_by_clear;
use super::super::axes_grid1::side_named;
use super::super::lines::PyLineStyle;
use super::axis_texts::PyTextLook;
use crate::binding::prelude::*;

artist_handle!(AxisArtist);
artist_handle!(TickMarksArtist);

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

/// Clears `axes` and puts its axis artists and gridlines back (`clear`);
/// returns the artists it removed, as the clear of any axes does.
fn clear(axes: Axes) -> PyResult<Vec<RawHandle>> {
    let removed = removed_by_clear(&axes);
    AxisArtistAxes::of(axes).clear();
    Ok(removed)
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

/// The collection of the gridlines of `axes` (matplotlib's
/// `ax.gridlines`), `None` once removed.
fn gridlines(axes: Axes) -> PyResult<Option<Collection>> {
    Ok(AxisArtistAxes::of(axes).gridlines())
}

/// A setting of the gridlines as Python gives it: the name pltrs knows it
/// by (`None` for another value) and Python's `repr` of the value.
type GivenChoice = (Option<String>, String);

/// The choice `given` names, `known` making the value of a name.
fn choice<T>(
    (name, spelled): GivenChoice,
    known: impl FnOnce(&str) -> PyResult<T>,
) -> PyResult<GridlinesChoice<T>> {
    Ok(match name {
        Some(name) => GridlinesChoice::Known(known(&name)?),
        None => GridlinesChoice::Unknown(spelled),
    })
}

/// Chooses the level (`which`) and the axis (`axis`) the gridlines
/// `gridlines` of an axes show, each when given (matplotlib's
/// `GridlinesCollection.set_which` and `set_axis`, which keep any value);
/// `False` for a collection drawing no gridlines of an axes.
fn choose(
    gridlines: Collection,
    which: Option<GivenChoice>,
    axis: Option<GivenChoice>,
) -> PyResult<bool> {
    let which = which
        .map(|given| {
            choice(given, |name| {
                name.parse::<TickLevel>()
                    .map_err(|err| PyErr::value_error(err.to_string()))
            })
        })
        .transpose()?;
    let axis = axis
        .map(|given| {
            choice(given, |name| {
                (name != "both").then(|| axis_named(name)).transpose()
            })
        })
        .transpose()?;
    Ok(choose_gridlines(&gridlines, which, axis))
}

/// Turns the axis artist's texts to the conventions of an axis on
/// `direction` (`set_axis_direction`).
fn set_axis_direction(artist: AxisArtist, direction: String) -> PyResult<()> {
    artist.set_axis_direction(side_named(&direction)?);
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

/// What the tick marks of a level draw with, as their `Ticks` holds it:
/// `(ticksize, tick_direction, visible, color, markeredgewidth, alpha)`,
/// the colour and width `None` where they follow the reference axis'
/// ticks (`"auto"`).
type TicksStyle = (f64, String, bool, Option<Color>, Option<f64>, Option<f64>);

/// Gives the tick marks of `level` (`"major"` or `"minor"`) the style of
/// their `Ticks`, after one of its setters ran.
fn restyle_ticks(
    artist: AxisArtist,
    level: String,
    (size, direction, visible, color, width, alpha): TicksStyle,
) -> PyResult<()> {
    let direction = direction
        .parse::<TickDirection>()
        .map_err(|_| PyErr::value_error(format!("no tick direction {direction:?}")))?;
    ticks(&artist, &level)
        .set_ticksize(size)
        .set_tick_direction(direction)
        .set_visible(visible)
        .set_color(color)
        .set_markeredgewidth(width)
        .set_alpha(alpha);
    Ok(())
}

/// Places the tick marks of `level` (`"major"` or `"minor"`) with
/// `transform`, the transform their object was given (`set_transform`),
/// or as the artist places them (`None`).
fn set_ticks_transform(
    artist: AxisArtist,
    level: String,
    transform: Option<Transform>,
) -> PyResult<()> {
    ticks(&artist, &level).set_transform(transform);
    Ok(())
}

/// Places the text part `part` with `transform`, the transform its object
/// was given (`set_transform`), or as the artist places it (`None`).
fn set_text_transform(
    artist: AxisArtist,
    part: String,
    transform: Option<Transform>,
) -> PyResult<()> {
    label_part(&artist, &part)?.set_transform(transform);
    Ok(())
}

/// Which parts of the axis artist are shown: its major and minor ticks,
/// its major and minor tick labels and its label (what an axes made by
/// pltrs (Rust) hides, `axislines.Axes` hiding the texts of its top and
/// right axis artists).
fn parts_visible(artist: AxisArtist) -> PyResult<(bool, bool, bool, bool, bool)> {
    Ok((
        artist.major_ticks().visible(),
        artist.minor_ticks().visible(),
        artist.major_ticklabels().visible(),
        artist.minor_ticklabels().visible(),
        artist.label().visible(),
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

/// Notes that matplotlib's draw of the text part `part` failed on the
/// first text it draws, once it moved and turned it (see
/// `LabelBase::note_failed_draw`).
fn note_failed_text_draw(artist: AxisArtist, part: String) -> PyResult<()> {
    label_part(&artist, &part)?.note_failed_draw();
    Ok(())
}

/// The angle of the text part `part` as its draws left it (see
/// `LabelBase::drawn_rotation`).
fn drawn_rotation(artist: AxisArtist, part: String) -> PyResult<f64> {
    Ok(label_part(&artist, &part)?.drawn_rotation())
}

/// What a text part draws with, as its `TickLabels` or `AxisLabel`
/// holds it: whether it is shown, its pad in points, its colour (`None`
/// following the reference artist, `"auto"`), its font size, the rest of
/// its typesetting, its rotation, its vertical and horizontal alignment,
/// its axis direction and, for the label, its text (`None` showing the
/// label of the axes' axis, `"__from_axes__"`).
type LabelStyle = (
    (bool, f64, Option<Color>),
    (f64, PyTextLook, f64),
    (String, String, String),
    Option<String>,
);

/// Gives the text part `part` (`"major_ticklabels"`, `"minor_ticklabels"`
/// or `"label"`) the style of its Python text, after one of its setters
/// ran.
fn restyle_label(
    artist: AxisArtist,
    part: String,
    ((visible, pad, color), (size, look, rotation), (valign, halign, direction), text): LabelStyle,
) -> PyResult<()> {
    let label = label_part(&artist, &part)?;
    let (set, turned) = (label.rotation(), label.turned_rotation());
    // The direction first: it resets the alignment and rotation, which
    // the text then sets.
    label
        .set_axis_direction(side_named(&direction)?)
        .set_visible(visible)
        .set_pad(pad)
        .set_color(color)
        .set_size(size)
        .set_look(look.0)
        .set_rotation(rotation)
        .set_va(alignment::<VAlign>(&valign)?)
        .set_ha(alignment::<HAlign>(&halign)?);
    if label.rotation().to_bits() == set.to_bits() {
        // The text kept its angle: the one its draws left stays.
        label.keep_turned_rotation(turned);
    }
    if part == "label" {
        match text {
            Some(text) => artist.label().set_text(text),
            None => artist.label().follow_axis(),
        };
    }
    Ok(())
}

/// The alignment called `name`.
fn alignment<T: std::str::FromStr>(name: &str) -> PyResult<T> {
    name.parse()
        .map_err(|_| PyErr::value_error(format!("{name:?} is not a valid alignment")))
}

/// Draws the box `text`, the object of the text part `part`, has behind
/// each text of the part (`set_bbox`).
fn set_part_bbox(artist: AxisArtist, part: String, text: Text) -> PyResult<()> {
    label_part(&artist, &part)?.set_bbox_patch(text.bbox_patch());
    Ok(())
}

/// Where the label of the axis artist is drawn, as `(x0, y0, x1, y1)` in
/// display pixels (`AxisLabel.get_window_extent`); `None` when it draws
/// nothing.
fn label_extent(artist: AxisArtist) -> PyResult<Option<(f64, f64, f64, f64)>> {
    Ok(artist
        .label()
        .window_extent()
        .map(|b| (b.x0(), b.y0(), b.x1(), b.y1())))
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

/// The tick properties of standalone tick marks: `(size, direction,
/// color, width)`.
type MarkStyle = (f64, String, Option<Color>, Option<f64>);

/// Tick marks at `locs_angles` (data coordinates, angles in degrees),
/// styled like the major ticks of `axis` unless `style` says otherwise.
fn tick_marks(
    axis: &str,
    locs_angles: Vec<((f64, f64), f64)>,
    (size, direction, color, width): MarkStyle,
) -> PyResult<TickMarks> {
    let locs_angles = locs_angles
        .into_iter()
        .map(|((x, y), angle)| (Point::new(x, y), angle))
        .collect();
    Ok(TickMarks {
        direction: direction
            .parse()
            .map_err(|_| PyErr::value_error(format!("no tick direction {direction:?}")))?,
        color,
        width,
        ..TickMarks::new(size, axis_named(axis)?, locs_angles)
    })
}

/// Adds tick marks to `axes` (`ax.add_artist(Ticks(...))`; see
/// [`tick_marks`]).
fn add_tick_marks(
    axes: Axes,
    axis: String,
    locs_angles: Vec<((f64, f64), f64)>,
    style: MarkStyle,
) -> PyResult<TickMarksArtist> {
    Ok(axes.add_tick_marks(tick_marks(&axis, locs_angles, style)?))
}

/// Replaces added tick marks after a property of their `Ticks` changed.
fn set_tick_marks(
    artist: TickMarksArtist,
    axis: String,
    locs_angles: Vec<((f64, f64), f64)>,
    style: MarkStyle,
) -> PyResult<()> {
    artist.set_marks(tick_marks(&axis, locs_angles, style)?);
    Ok(())
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
    module.add("gridlines", gridlines)?;
    module.add("choose_gridlines", choose)?;
    module.add("set_axis_direction", set_axis_direction)?;
    module.add("set_axisline_style", set_axisline_style)?;
    module.add("restyle_ticks", restyle_ticks)?;
    module.add("set_ticks_transform", set_ticks_transform)?;
    module.add("set_text_transform", set_text_transform)?;
    module.add("set_part_bbox", set_part_bbox)?;
    module.add("parts_visible", parts_visible)?;
    module.add("restyle_label", restyle_label)?;
    module.add("label_extent", label_extent)?;
    module.add("note_failed_text_draw", note_failed_text_draw)?;
    module.add("drawn_rotation", drawn_rotation)?;
    module.add("set_line", set_line)?;
    module.add("get_line_visible", get_line_visible)?;
    module.add("tight_bbox", tight_bbox)?;
    module.add("add_tick_marks", add_tick_marks)?;
    module.add("set_tick_marks", set_tick_marks)
}
