//! Vector fields: `quiver`, `quiverkey`, `barbs` and `streamplot`.

use pltrs::axes::{Axes, MappedColors};
use pltrs::collections::Collection;
use pltrs::colors::Color;
use pltrs::numeric::Grid;
use pltrs::quiver::{
    Angles, BarbIncrements, BarbPivot, BarbSizes, BarbsOptions, KeyCoordinates, LabelPosition,
    Pivot, Quiver, QuiverArrowShape, QuiverKey, QuiverKeyOptions, QuiverOptions, QuiverUnits,
    VectorField,
};
use pltrs::streamplot::{
    IntegrationDirection, StreamArrow, StreamColorMap, StreamColors, StreamField, StreamWidths,
    StreamplotError, StreamplotOptions, UnknownArrowStyle,
};
use pltrs::text::Text;
use pltrs::transforms::{Point, Transform};

use super::super::font_manager::FontProps;
use super::super::path::{PathArrays, path_arrays};
use super::mapping::{Mapping, colorizer};
use crate::binding::prelude::*;

artist_handle!(Quiver, QuiverKey, StreamArrow);

impl From<StreamplotError> for PyErr {
    /// matplotlib's `ValueError` for fields and options it cannot trace.
    fn from(err: StreamplotError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

/// An RGBA colour as a 4-tuple.
type Rgba = (f64, f64, f64, f64);

/// The colour of an RGBA tuple.
fn color((r, g, b, a): Rgba) -> Color {
    Color::from([r, g, b, a])
}

/// Values mapped to colours, with their mapping, as `(values, mapping)`.
type Mapped = Option<(NdArray, Mapping)>;

/// The field `u`, `v` at `x`, `y`, its colour values mapped as `mapped`
/// says.
fn vector_field(
    (x, y): (NdArray, NdArray),
    (u, v): (NdArray, NdArray),
    mapped: Mapped,
) -> PyResult<VectorField> {
    let field = VectorField::new(x.into_vec()?, y.into_vec()?, u.into_vec()?, v.into_vec()?);
    Ok(match mapped {
        Some((values, mapping)) => field.with_colors(MappedColors {
            values: values.into_vec()?,
            colorizer: colorizer(mapping),
        }),
        None => field,
    })
}

/// Arrow dimension units by matplotlib's name.
fn quiver_units(name: &str) -> PyResult<QuiverUnits> {
    Ok(match name {
        "width" => QuiverUnits::Width,
        "height" => QuiverUnits::Height,
        "dots" => QuiverUnits::Dots,
        "inches" => QuiverUnits::Inches,
        "x" => QuiverUnits::X,
        "y" => QuiverUnits::Y,
        "xy" => QuiverUnits::Xy,
        _ => {
            return Err(PyErr::value_error(format!(
                "{name:?} is not a valid value for units; supported values are 'width', \
                 'height', 'dots', 'inches', 'x', 'y', 'xy'"
            )));
        }
    })
}

/// The arrow directions: `'uv'`, `'xy'`, or angles in degrees.
pub(super) struct PyAngles(Angles);

impl FromPy<'_> for PyAngles {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        if let Ok(name) = object.extract::<String>() {
            return match name.as_str() {
                "uv" => Ok(Self(Angles::Uv)),
                "xy" => Ok(Self(Angles::Xy)),
                _ => Err(PyErr::value_error(format!(
                    "{name:?} is not a valid value for angles; supported values are 'uv', 'xy'"
                ))),
            };
        }
        let degrees: NdArray = object.extract()?;
        Ok(Self(Angles::Degrees(degrees.into_vec()?)))
    }
}

/// How quiver arrows are sized: `(scale, scale_units, units, angles,
/// width)`.
type Sizing = (Option<f64>, Option<String>, String, PyAngles, Option<f64>);

/// The arrow shape: `(headwidth, headlength, headaxislength, minshaft,
/// minlength)`.
type Shape = (f64, f64, f64, f64, f64);

/// Arrows for the vectors `u`, `v` at `x`, `y` (matplotlib's `quiver`,
/// whose positional forms the Python side parsed), sized as `sizing`
/// says, shaped as `shape`, placed on them by their `"tail"`, `"middle"`
/// or `"tip"`, in the face colour `color` unless values are mapped, with
/// the positions in `transform` (`None`: the data coordinates).
pub(super) fn quiver(
    axes: Axes,
    positions: (NdArray, NdArray),
    components: (NdArray, NdArray),
    mapped: Mapped,
    (scale, scale_units, units, angles, width): Sizing,
    (head_width, head_length, head_axis_length, min_shaft, min_length): Shape,
    (pivot, rgba): (String, Rgba),
    transform: Option<Transform>,
) -> PyResult<Quiver> {
    let options = QuiverOptions {
        scale,
        scale_units: scale_units.as_deref().map(quiver_units).transpose()?,
        units: quiver_units(&units)?,
        angles: angles.0,
        width,
        shape: QuiverArrowShape {
            head_width,
            head_length,
            head_axis_length,
            min_shaft,
            min_length,
        },
        pivot: match pivot.as_str() {
            "middle" => Pivot::Middle,
            "tip" => Pivot::Tip,
            _ => Pivot::Tail,
        },
        color: color(rgba),
        transform,
        ..QuiverOptions::default()
    };
    Ok(axes.quiver(vector_field(positions, components, mapped)?, options)?)
}

/// Replaces the components of the arrows (matplotlib's `set_UVC`), one
/// pair per arrow or one for all.
pub(super) fn quiver_set_uvc(quiver: Quiver, u: NdArray, v: NdArray) -> PyResult<()> {
    quiver.set_uvc(&u.into_vec()?, &v.into_vec()?)?;
    Ok(())
}

/// The key's arrow: `(angle, coordinates, color, zorder)`.
type KeyArrow = (f64, String, Option<Rgba>, Option<f64>);

/// The key's label: `(labelsep, labelpos, labelcolor, font)`.
type KeyLabel = (f64, String, Option<Rgba>, Option<FontProps>);

/// A key of `quiver` on `axes`: an arrow of `length` at `position`
/// labelled `label` (matplotlib's `quiverkey`).
pub(super) fn quiverkey(
    axes: Axes,
    quiver: Quiver,
    position: (f64, f64),
    length: f64,
    label: String,
    (angle, coordinates, key_color, zorder): KeyArrow,
    (labelsep, labelpos, labelcolor, font): KeyLabel,
) -> PyResult<QuiverKey> {
    let options = QuiverKeyOptions {
        angle,
        coordinates: match coordinates.as_str() {
            "figure" => KeyCoordinates::Figure,
            "data" => KeyCoordinates::Data,
            "inches" => KeyCoordinates::Inches,
            _ => KeyCoordinates::Axes,
        },
        color: key_color.map(color),
        labelsep,
        labelpos: match labelpos.as_str() {
            "S" => LabelPosition::South,
            "E" => LabelPosition::East,
            "W" => LabelPosition::West,
            _ => LabelPosition::North,
        },
        labelcolor: labelcolor.map(color),
        font: font.map(|font| font.0),
        zorder,
    };
    Ok(axes.quiverkey(&quiver, position, length, label, options))
}

/// The label of a quiver key.
pub(super) fn quiverkey_text(key: QuiverKey) -> PyResult<Text> {
    Ok(key.text())
}

/// The outline of a quiver key's arrow in the current layout, as
/// `(vertices, codes)` pairs (matplotlib's `QuiverKey.vector.get_paths()`).
pub(super) fn quiverkey_paths(key: QuiverKey) -> PyResult<Vec<PathArrays>> {
    key.vector_paths()
        .iter()
        .map(|path| path_arrays(path, false))
        .collect()
}

/// The barb sizes: `(spacing, height, width, emptybarb)`.
type Sizes = (f64, f64, f64, f64);

/// The barb increments: `(half, full, flag)`.
type Increments = (f64, f64, f64);

/// The barb colours: `(barbcolor, flagcolor)`.
type BarbColors = (Option<Rgba>, Option<Rgba>);

/// The barb outlines: `(sizes, fill_empty, increments, rounding, flip)`.
type BarbShape = (Sizes, bool, Increments, bool, Vec<bool>);

/// Wind barbs for the vectors `u`, `v` at `x`, `y` (matplotlib's
/// `barbs`): staffs of `length` points placed by their `"tip"`,
/// `"middle"` or an offset along the staff, coloured as `colors` says.
pub(super) fn barbs(
    axes: Axes,
    positions: (NdArray, NdArray),
    components: (NdArray, NdArray),
    mapped: Mapped,
    (pivot, length): (Owned, f64),
    (barbcolor, flagcolor): BarbColors,
    (sizes, fill_empty, increments, rounding, flip): BarbShape,
    transform: Option<Transform>,
) -> PyResult<Collection> {
    let pivot = Python::with_gil(|py| {
        let pivot = pivot.bind(py);
        Ok::<_, PyErr>(match pivot.extract::<String>() {
            Ok(name) if name == "middle" => BarbPivot::Middle,
            Ok(_) => BarbPivot::Tip,
            Err(_) => BarbPivot::Offset(pivot.extract()?),
        })
    })?;
    let (spacing, height, width, emptybarb) = sizes;
    let (half, full, flag) = increments;
    let options = BarbsOptions {
        pivot,
        length,
        barbcolor: barbcolor.map(color),
        flagcolor: flagcolor.map(color),
        sizes: BarbSizes {
            spacing,
            height,
            width,
            emptybarb,
        },
        fill_empty,
        increments: BarbIncrements { half, full, flag },
        rounding,
        flip,
        transform,
        ..BarbsOptions::default()
    };
    Ok(axes.barbs(vector_field(positions, components, mapped)?, options)?)
}

/// A 2-D float array as a grid.
fn grid(array: &NdArray) -> PyResult<Grid<f64>> {
    match array.shape[..] {
        [rows, cols] => Grid::from_vec(rows, cols, array.data.clone())
            .map_err(|err| PyErr::value_error(err.to_string())),
        _ => Err(PyErr::value_error(format!(
            "expected a 2-D array, got shape {:?}",
            array.shape
        ))),
    }
}

/// The streamline colours: one RGBA colour, or values on the grid mapped
/// as `Mapping` says; `None` for the next colour of the line cycle.
type StreamColorSpec = (Option<Rgba>, Mapped);

/// The streamline widths: one width, or widths on the grid; `None` for
/// `lines.linewidth`.
type StreamWidthSpec = (Option<f64>, Option<NdArray>);

/// The traced streamlines: `(density, minlength, maxlength, start
/// points, direction, broken_streamlines, max step scale, max error
/// scale)`.
type Tracing = (
    (f64, f64),
    f64,
    f64,
    Option<NdArray>,
    String,
    bool,
    (f64, f64),
);

/// The arrows: `(arrowsize, arrowstyle, num_arrows)`.
type StreamArrows = (f64, String, usize);

/// Streamlines of the field `u`, `v` on the grid of `x` (one per column)
/// and `y` (one per row) (matplotlib's `streamplot`, whose meshes the
/// Python side reduced): coloured, widened, traced and given arrows as
/// the specs say, at `zorder`, with the grid in `transform`. Returns the
/// line collection and the arrows.
pub(super) fn streamplot(
    axes: Axes,
    (x, y): (NdArray, NdArray),
    (u, v): (NdArray, NdArray),
    (single, mapped): StreamColorSpec,
    (uniform, varying): StreamWidthSpec,
    tracing: Tracing,
    (arrowsize, arrowstyle, num_arrows): StreamArrows,
    (zorder, transform): (Option<f64>, Option<Transform>),
) -> PyResult<(Collection, Vec<StreamArrow>)> {
    let (density, minlength, maxlength, start_points, direction, broken, scales) = tracing;
    let field = StreamField::new(x.into_vec()?, y.into_vec()?, grid(&u)?, grid(&v)?);
    let color = match (single, mapped) {
        (_, Some((values, mapping))) => Some(StreamColors::Mapped(Box::new(StreamColorMap {
            values: grid(&values)?,
            colorizer: colorizer(mapping),
        }))),
        (Some(rgba), None) => Some(StreamColors::Single(color(rgba))),
        (None, None) => None,
    };
    let linewidth = match (uniform, varying) {
        (_, Some(widths)) => Some(StreamWidths::Varying(grid(&widths)?)),
        (Some(width), None) => Some(StreamWidths::Uniform(width)),
        (None, None) => None,
    };
    let start_points = start_points.map(|points| {
        points
            .data
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&[x, y]| Point::new(x, y))
            .collect()
    });
    let options = StreamplotOptions {
        density,
        linewidth,
        color,
        arrowsize,
        arrowstyle: arrowstyle
            .parse()
            .map_err(|err: UnknownArrowStyle| PyErr::value_error(err.to_string()))?,
        minlength,
        maxlength,
        start_points,
        zorder,
        integration_direction: match direction.as_str() {
            "forward" => IntegrationDirection::Forward,
            "backward" => IntegrationDirection::Backward,
            _ => IntegrationDirection::Both,
        },
        broken_streamlines: broken,
        integration_max_step_scale: scales.0,
        integration_max_error_scale: scales.1,
        num_arrows,
        transform,
    };
    let set = axes.streamplot(&field, options)?;
    Ok((set.lines, set.arrows))
}
