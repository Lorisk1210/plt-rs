//! `_native.annotation`: annotations (matplotlib's `text.Annotation`,
//! `text.OffsetFrom` and `Axes.annotate`), by pltrs (Rust)'s
//! [`Annotation`].
//!
//! An annotation's native object is its [`Text`] handle, so that the text
//! functions of `_native.text` serve it; the functions here recover the
//! annotation from it. Coordinates cross in the tagged form the Python
//! side gives them ([`PyCoords`]), the arrow as a dict of resolved keyword
//! arguments ([`ArrowKwargs`]).

use pltrs::artist::{ArtistHandle, RawHandle};
use pltrs::axes::Axes;
use pltrs::coords::{CoordError, CoordUnit, Coords};
use pltrs::patches::{FancyArrowPatch, Patch};
use pltrs::text::{
    AnnotateOptions, Annotation, ArrowProps, ArrowStart, FancyArrowProps, LegacyArrowProps, Text,
};
use pltrs::transforms::Transform;
use pltrs::units::{AxisValue, DataPoint};
use pltrs_core::Point;

use super::patch_styles::{PyArrowStyle, PyConnectionStyle};
use super::patches::PatchKwargs;
use super::path::{PathArrays, path_arrays};
use crate::binding::prelude::*;
use crate::binding::{dict_items, new_dict};
use crate::modules::dates::Zone;

impl From<CoordError> for PyErr {
    /// `ValueError` with matplotlib's messages where it has one.
    fn from(err: CoordError) -> Self {
        match err {
            CoordError::NoOffset => PyErr::value_error("xycoords cannot be an offset coordinate"),
            other => PyErr::value_error(other.to_string()),
        }
    }
}

/// matplotlib's message for a coordinate name it does not know
/// (`_get_xy_transform`): the name when it is not two words starting with
/// a known box, else the unit.
fn coordinate_name_error(name: &str) -> PyErr {
    let words: Vec<&str> = name.split_whitespace().collect();
    match words[..] {
        ["figure" | "subfigure" | "axes" | "offset", unit] => {
            PyErr::value_error(format!("'{unit}' is not a recognized unit"))
        }
        _ => PyErr::value_error(format!("'{name}' is not a valid coordinate")),
    }
}

/// The unit of an `OffsetFrom`: `"points"` or `"pixels"`.
fn offset_unit(unit: &str) -> PyResult<CoordUnit> {
    match unit {
        "points" => Ok(CoordUnit::Points),
        "pixels" => Ok(CoordUnit::Pixels),
        _ => Err(PyErr::value_error(format!(
            "{unit:?} is not a valid value for unit; supported values are 'points', 'pixels'"
        ))),
    }
}

/// Coordinates as the Python side gives them: a name (`"data"`, `"axes
/// fraction"`, ...), `("blended", x, y)`, `("artist", native)` for the
/// extent of an artist, `("transform", native)` for a transform, or
/// `("offset_from", native, (x, y), unit)` for matplotlib's `OffsetFrom`.
pub(super) struct PyCoords(pub(super) Coords);

impl FromPy<'_> for PyCoords {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        if let Ok(name) = object.extract::<String>() {
            return name
                .parse()
                .map(Self)
                .map_err(|_| coordinate_name_error(&name));
        }
        let tag: String = object
            .call_method("__getitem__", &[0_i64.into_py(object.py())?])?
            .extract()?;
        Ok(Self(match tag.as_str() {
            "blended" => {
                let (_, x, y): (String, PyCoords, PyCoords) = object.extract()?;
                Coords::blended(x.0, y.0)
            }
            "artist" => {
                let (_, artist): (String, RawHandle) = object.extract()?;
                Coords::Artist(artist.id())
            }
            "transform" => {
                let (_, transform): (String, Transform) = object.extract()?;
                Coords::Transform(transform)
            }
            "offset_from" => {
                let (_, artist, (x, y), unit): (String, RawHandle, (f64, f64), String) =
                    object.extract()?;
                Coords::OffsetFrom {
                    artist: artist.id(),
                    reference: Point::new(x, y),
                    unit: offset_unit(&unit)?,
                }
            }
            _ => return Err(PyErr::type_error(format!("unknown coordinates {tag:?}"))),
        }))
    }
}

/// A coordinate of an annotated point as the Python side gives it: a
/// number, a category name, or `("dates", number, zone)`, the zone `None`
/// for a naive date.
struct PyAxisValue(AxisValue);

impl FromPy<'_> for PyAxisValue {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        if let Ok(name) = object.extract::<String>() {
            return Ok(Self(AxisValue::Category(name)));
        }
        if let Ok((tag, number, zone)) = object.extract::<(String, f64, Option<Zone>)>()
            && tag == "dates"
        {
            return Ok(Self(AxisValue::Date {
                number,
                tz: zone.map(|Zone(tz)| tz),
            }));
        }
        Ok(Self(AxisValue::Number(object.extract()?)))
    }
}

/// A point of `annotate`: its two coordinates.
fn data_point((x, y): (PyAxisValue, PyAxisValue)) -> DataPoint {
    DataPoint { x: x.0, y: y.0 }
}

/// The arrow of an annotation from matplotlib's `arrowprops`, aliases
/// resolved and styles given as their descriptions by the Python side: a
/// styled arrow when it has an `arrowstyle`, else the legacy arrow.
pub(super) struct ArrowKwargs(pub(super) ArrowProps);

/// What clips an arrow at its ends, from `patchA`/`patchB`: `None` for
/// nothing, else an artist's native object.
fn clipping_artist(value: &Bound<'_>) -> PyResult<Option<pltrs::artist::ArtistId>> {
    if value.is_none() {
        return Ok(None);
    }
    Ok(Some(value.extract::<RawHandle>()?.id()))
}

impl FromPy<'_> for ArrowKwargs {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let py = object.py();
        let mut items = Vec::new();
        let mut arrowstyle = None;
        for (key, value) in dict_items(object)? {
            let name: String = key.extract()?;
            if name == "arrowstyle" {
                arrowstyle = Some(value.extract::<PyArrowStyle>()?.0);
            } else {
                items.push((name, value));
            }
        }

        let mut fancy = arrowstyle.map(FancyArrowProps::new);
        let mut legacy = LegacyArrowProps::default();
        let rest = new_dict(py)?;
        for (name, value) in items {
            match (fancy.as_mut(), name.as_str()) {
                (fancy, "connectionstyle") => {
                    let style = value.extract::<PyConnectionStyle>()?.0;
                    match fancy {
                        Some(fancy) => fancy.connectionstyle = style,
                        None => legacy.connectionstyle = style,
                    }
                }
                (Some(fancy), "relpos") => fancy.relpos = value.extract()?,
                (Some(fancy), "shrinkA") => fancy.shrink_a = value.extract()?,
                (Some(fancy), "shrinkB") => fancy.shrink_b = value.extract()?,
                (Some(fancy), "mutation_scale") => fancy.mutation_scale = value.extract()?,
                (Some(fancy), "mutation_aspect") => fancy.mutation_aspect = value.extract()?,
                (Some(fancy), "patchA") => {
                    fancy.patch_a =
                        clipping_artist(&value)?.map_or(ArrowStart::Unclipped, ArrowStart::Artist);
                }
                (Some(fancy), "patchB") => fancy.patch_b = clipping_artist(&value)?,
                (None, "width") => legacy.width = value.extract()?,
                (None, "headwidth") => legacy.headwidth = value.extract()?,
                (None, "headlength") => legacy.headlength = value.extract()?,
                (None, "shrink") => legacy.shrink = value.extract()?,
                _ => rest.set_item(name, value)?,
            }
        }
        let patch = rest.extract::<PatchKwargs>()?.0;
        Ok(Self(if let Some(mut fancy) = fancy {
            fancy.patch = patch;
            ArrowProps::Fancy(fancy)
        } else {
            legacy.patch = patch;
            ArrowProps::Legacy(legacy)
        }))
    }
}

/// The annotation whose text `text` is.
fn annotation(text: &Text) -> PyResult<Annotation> {
    text.raw()
        .downcast::<Annotation>()
        .ok_or_else(|| PyErr::type_error("the text is not an annotation"))
}

/// The arguments of `annotate` and `Annotation` besides the axes and the
/// text: `(xy, xytext, xycoords, textcoords, arrowprops,
/// annotation_clip)`.
type AnnotateArgs = (
    (PyAxisValue, PyAxisValue),
    Option<(PyAxisValue, PyAxisValue)>,
    PyCoords,
    Option<PyCoords>,
    Option<ArrowKwargs>,
    Option<bool>,
);

/// The annotated point and the options `args` give.
fn annotate_options(args: AnnotateArgs) -> (DataPoint, AnnotateOptions) {
    let (xy, xytext, xycoords, textcoords, arrowprops, annotation_clip) = args;
    let options = AnnotateOptions {
        xytext: xytext.map(data_point),
        xycoords: xycoords.0,
        textcoords: textcoords.map(|coords| coords.0),
        arrowprops: arrowprops.map(|arrow| arrow.0),
        annotation_clip,
        clip_on: false,
    };
    (data_point(xy), options)
}

/// `Axes.annotate(text, xy, xytext, xycoords, textcoords, arrowprops,
/// annotation_clip)`: the annotation's text handle.
fn axes_annotate(axes: Axes, text: String, args: AnnotateArgs) -> PyResult<Text> {
    let (xy, options) = annotate_options(args);
    let note = axes.annotate(text, xy, options)?;
    Ok((*note).clone())
}

/// `Annotation(text, xy, xytext, xycoords, textcoords, arrowprops,
/// annotation_clip)`: the text handle of an annotation outside any
/// figure.
fn new(text: String, args: AnnotateArgs) -> PyResult<Text> {
    let (xy, options) = annotate_options(args);
    let note = Annotation::loose(text, xy, options)?;
    Ok((*note).clone())
}

/// `Annotation.xy`: the annotated point.
fn get_xy(text: Text) -> PyResult<(f64, f64)> {
    let xy = annotation(&text)?.xy();
    Ok((xy.x, xy.y))
}

/// Moves the annotated point.
fn set_xy(text: Text, x: f64, y: f64) -> PyResult<()> {
    annotation(&text)?.set_xy(Point::new(x, y));
    Ok(())
}

/// Sets the coordinates of the annotated point (`xycoords`).
fn set_xycoords(text: Text, coords: PyCoords) -> PyResult<()> {
    annotation(&text)?.set_xycoords(coords.0)?;
    Ok(())
}

/// Sets the coordinates of the text (`set_anncoords`).
fn set_textcoords(text: Text, coords: PyCoords) -> PyResult<()> {
    annotation(&text)?.set_textcoords(coords.0);
    Ok(())
}

/// `get_annotation_clip()`.
fn get_annotation_clip(text: Text) -> PyResult<Option<bool>> {
    Ok(annotation(&text)?.annotation_clip())
}

/// `set_annotation_clip(b)`.
fn set_annotation_clip(text: Text, clip: Option<bool>) -> PyResult<()> {
    annotation(&text)?.set_annotation_clip(clip);
    Ok(())
}

/// Whether the annotation draws an arrow to its point.
fn has_arrow(text: Text) -> PyResult<bool> {
    Ok(annotation(&text)?.has_arrow())
}

/// The arrow as a loose fancy arrow patch of its look, `None` without an
/// arrow (matplotlib's `arrow_patch`; see `Annotation::arrow_patch`).
fn arrow_patch(text: Text) -> PyResult<Option<Patch<FancyArrowPatch>>> {
    Ok(annotation(&text)?.arrow_patch())
}

/// Gives the arrow the look of `patch`, a patch `arrow_patch` made and the
/// program changed.
fn restyle_arrow(text: Text, patch: Patch<FancyArrowPatch>) -> PyResult<()> {
    annotation(&text)?.restyle_arrow(&patch);
    Ok(())
}

/// The display box `(x0, y0, x1, y1)` of the arrow as drawn, `None`
/// without an arrow or while the annotation is hidden.
fn arrow_extent(text: Text) -> PyResult<Option<(f64, f64, f64, f64)>> {
    Ok(annotation(&text)?
        .arrow_extent()
        .map(|bbox| (bbox.x0(), bbox.y0(), bbox.x1(), bbox.y1())))
}

/// The display box `(x0, y0, x1, y1)` of the text alone, without the
/// arrow (`Text.get_window_extent(annotation)`).
fn text_extent(text: Text) -> PyResult<Option<(f64, f64, f64, f64)>> {
    Ok(annotation(&text)?
        .text_extent()
        .map(|bbox| (bbox.x0(), bbox.y0(), bbox.x1(), bbox.y1())))
}

/// The outline of the arrow as drawn, in display pixels, as the arrays of
/// a path; `None` without an arrow or while the annotation is hidden.
fn arrow_display_path(text: Text) -> PyResult<Option<PathArrays>> {
    annotation(&text)?
        .arrow_display_path()
        .map(|path| path_arrays(&path, false))
        .transpose()
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("axes_annotate", axes_annotate)?;
    module.add("new", new)?;
    module.add("get_xy", get_xy)?;
    module.add("set_xy", set_xy)?;
    module.add("set_xycoords", set_xycoords)?;
    module.add("set_textcoords", set_textcoords)?;
    module.add("get_annotation_clip", get_annotation_clip)?;
    module.add("set_annotation_clip", set_annotation_clip)?;
    module.add("has_arrow", has_arrow)?;
    module.add("arrow_patch", arrow_patch)?;
    module.add("restyle_arrow", restyle_arrow)?;
    module.add("arrow_extent", arrow_extent)?;
    module.add("arrow_display_path", arrow_display_path)?;
    module.add("text_extent", text_extent)
}
