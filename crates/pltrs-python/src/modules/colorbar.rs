//! `_native.colorbar`: colorbars (matplotlib's `colorbar`).
//!
//! A colorbar shows the colour mapping of its Python mappable: the
//! mappable's colormap and norm convert to pltrs (Rust)'s (see
//! `colors::mapping`), and the Python `Colorbar` hands them over again
//! when the mappable changes (`update_normal`); a colorbar of an image
//! follows the image pltrs (Rust) draws, its opacity included, and one of
//! a contour set the set's levels, line styles and hatches. pltrs (Rust)
//! lays the colorbar out, in an axes of its own or in the one given, and
//! draws it.

use std::rc::Rc;

use pltrs::artist::{RawHandle, TypedHandle};
use pltrs::axes::Axes;
use pltrs::collections::{Collection, TriMesh};
use pltrs::colorbar::{
    Colorbar, ColorbarAxes, ColorbarError, ColorbarFormat, ColorbarOptions, ColorbarTicks,
    ExtendFrac, Mappable, ParentAnchor, Spacing,
};
use pltrs::colors::{Color, Colormap, Extend, Norm, ScalarMappable};
use pltrs::contour::ContourSet;
use pltrs::figure::Figure;
use pltrs::image::AxesImage;
use pltrs::orientation::Orientation;
use pltrs::patches::{Patch, PathShape};
use pltrs::spines::SpineSide;
use pltrs::ticker::LabelFormat;

use super::colors::mapping::extend;
use super::transforms::anchor;
use crate::binding::dict_value;
use crate::binding::prelude::*;

native_value!(Colorbar);

impl From<ColorbarError> for PyErr {
    /// matplotlib's `ValueError` for colorbars it cannot make; a tick
    /// location across the colorbar has the message of the long axis'
    /// `set_label_position`, which refuses it there.
    fn from(err: ColorbarError) -> Self {
        match err {
            ColorbarError::TickLocation {
                ticklocation,
                orientation,
            } => {
                let supported = match orientation {
                    Orientation::Vertical => "'left', 'right'",
                    Orientation::Horizontal => "'top', 'bottom'",
                };
                PyErr::value_error(format!(
                    "'{}' is not a valid value for position; supported values are {supported}",
                    ticklocation.name()
                ))
            }
            other => PyErr::value_error(other.to_string()),
        }
    }
}

/// The side a location names.
fn side(name: &str) -> PyResult<SpineSide> {
    Ok(match name {
        "left" => SpineSide::Left,
        "right" => SpineSide::Right,
        "top" => SpineSide::Top,
        "bottom" => SpineSide::Bottom,
        _ => {
            return Err(PyErr::value_error(format!(
                "{name:?} is not a valid value for location; supported values are \
                 'left', 'right', 'top', 'bottom'"
            )));
        }
    })
}

/// The orientation a name gives.
fn orientation(name: &str) -> PyResult<Orientation> {
    name.parse()
        .map_err(|err: pltrs::orientation::OrientationError| PyErr::value_error(err.to_string()))
}

/// matplotlib's `extendfrac`: `None`, `'auto'`, a fraction, or one per end.
fn extendfrac(value: &Bound<'_>) -> PyResult<ExtendFrac> {
    if value.is_none() {
        return Ok(ExtendFrac::Default);
    }
    if let Ok(name) = value.extract::<String>() {
        return match name.as_str() {
            "auto" => Ok(ExtendFrac::Auto),
            _ => Err(PyErr::value_error("invalid value for extendfrac")),
        };
    }
    if let Ok(both) = value.extract::<f64>() {
        return Ok(ExtendFrac::Both(both));
    }
    match value.extract::<(f64, f64)>() {
        Ok((lower, upper)) if !lower.is_nan() && !upper.is_nan() => {
            Ok(ExtendFrac::Each(lower, upper))
        }
        _ => Err(PyErr::value_error("invalid value for extendfrac")),
    }
}

/// matplotlib's `format`: a `%`-style or `str.format` template, or a
/// Python formatter (any callable taking `(x, pos)`).
fn format(value: &Bound<'_>) -> PyResult<Option<ColorbarFormat>> {
    if value.is_none() {
        return Ok(None);
    }
    if let Ok(template) = value.extract::<String>() {
        return Ok(Some(ColorbarFormat::Label(LabelFormat::from(template))));
    }
    Ok(Some(ColorbarFormat::Formatter(Rc::new(
        super::archive::tick_formatter(value)?,
    ))))
}

/// matplotlib's `panchor`: `None` for the default (towards the colorbar),
/// `False` to leave the parent's anchor alone, or an anchor.
fn parent_anchor(value: &Bound<'_>) -> PyResult<ParentAnchor> {
    if value.is_none() {
        return Ok(ParentAnchor::Default);
    }
    if value.type_name() == "bool" && !value.is_truthy()? {
        return Ok(ParentAnchor::Unchanged);
    }
    Ok(ParentAnchor::At(anchor(value)?))
}

/// The axes a new colorbar takes space from: one axes, which may give it
/// a cell of its grid (matplotlib's `make_axes_gridspec`), or a list of
/// axes, which only shrink (`make_axes`).
enum Beside {
    One(Axes),
    Many(Vec<Axes>),
}

impl FromPy<'_> for Beside {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        match object.extract::<Axes>() {
            Ok(ax) => Ok(Self::One(ax)),
            Err(_) => Ok(Self::Many(object.extract()?)),
        }
    }
}

/// The options of `Figure.colorbar` given as a dict of matplotlib's
/// keywords (see `colorbar.py`), drawing in `cax` or beside `parents`.
fn options(
    cax: Option<Axes>,
    parents: Option<Beside>,
    spec: &Bound<'_>,
) -> PyResult<ColorbarOptions> {
    let named = |key: &str| -> PyResult<Option<String>> { dict_value(spec, key) };
    Ok(ColorbarOptions {
        axes: match (cax, parents) {
            (Some(cax), _) => ColorbarAxes::Cax(cax),
            (None, Some(Beside::One(parent))) => ColorbarAxes::Parent(parent),
            (None, Some(Beside::Many(parents))) => ColorbarAxes::Parents(parents),
            (None, None) => ColorbarAxes::FromMappable,
        },
        use_gridspec: dict_value(spec, "use_gridspec")?,
        location: named("location")?.as_deref().map(side).transpose()?,
        orientation: named("orientation")?
            .as_deref()
            .map(orientation)
            .transpose()?,
        ticklocation: named("ticklocation")?
            .filter(|name| name != "auto")
            .as_deref()
            .map(side)
            .transpose()?,
        fraction: dict_value(spec, "fraction")?,
        shrink: dict_value(spec, "shrink")?,
        aspect: dict_value(spec, "aspect")?,
        pad: dict_value(spec, "pad")?,
        anchor: dict_value::<Option<Bound<'_>>>(spec, "anchor")?
            .as_ref()
            .map(anchor)
            .transpose()?,
        panchor: parent_anchor(&dict_value(spec, "panchor")?)?,
        extend: named("extend")?.as_deref().map(extend).transpose()?,
        extendfrac: extendfrac(&dict_value(spec, "extendfrac")?)?,
        extendrect: dict_value(spec, "extendrect")?,
        format: format(&dict_value(spec, "format")?)?,
        ticks: dict_value::<Option<Vec<f64>>>(spec, "ticks")?.map(ColorbarTicks::Values),
        drawedges: dict_value(spec, "drawedges")?,
        label: dict_value(spec, "label")?,
        spacing: match named("spacing")?.as_deref() {
            Some("proportional") => Spacing::Proportional,
            _ => Spacing::Uniform,
        },
        boundaries: dict_value(spec, "boundaries")?,
        values: dict_value(spec, "values")?,
    })
}

/// The drawn artist a colorbar follows: an image, a collection or a
/// triangle mesh (their opacity included), or a contour set (its levels,
/// line styles and hatches).
enum Followed {
    Image(AxesImage),
    Collection(Collection),
    TriMesh(TriMesh),
    Contour(ContourSet),
}

impl Followed {
    /// The artist behind `raw` if it is one a colorbar follows.
    fn of(raw: &RawHandle) -> Option<Self> {
        AxesImage::from_raw_handle(raw)
            .map(Self::Image)
            .or_else(|| Collection::from_raw_handle(raw).map(Self::Collection))
            .or_else(|| TriMesh::from_raw_handle(raw).map(Self::TriMesh))
            .or_else(|| ContourSet::from_raw_handle(raw).map(Self::Contour))
    }
}

/// What a colorbar shows: the mapping through `cmap` and `norm`, which it
/// follows as the artist `followed` when the mappable is one pltrs (Rust)
/// links colorbars to.
struct Shown {
    mapping: ScalarMappable,
    followed: Option<Followed>,
}

impl Shown {
    fn new(cmap: Colormap, norm: Box<dyn Norm>, artist: Option<&RawHandle>) -> Self {
        Self {
            mapping: ScalarMappable::with_norm(cmap, norm),
            followed: artist.and_then(Followed::of),
        }
    }

    fn mappable(&self) -> Mappable<'_> {
        match &self.followed {
            Some(Followed::Image(image)) => Mappable::from(image),
            Some(Followed::Collection(collection)) => Mappable::from(collection),
            Some(Followed::TriMesh(mesh)) => Mappable::from(mesh),
            Some(Followed::Contour(contours)) => Mappable::from(contours),
            None => Mappable::from(&self.mapping),
        }
    }
}

/// A colorbar in `figure` showing the mapping through `cmap` and `norm`
/// (of the drawn `artist`, followed if it is a kind colorbars follow),
/// drawn in `cax` or in a new axes taking space from `parents` (an axes or
/// a list of them), with the options of the dict `spec`.
fn new(
    figure: Figure,
    (cmap, norm, artist): (Colormap, Box<dyn Norm>, Option<RawHandle>),
    cax: Option<Axes>,
    parents: Option<Beside>,
    spec: Owned,
) -> PyResult<Colorbar> {
    let options = Python::with_gil(|py| options(cax, parents, &spec.bind(py)))?;
    let shown = Shown::new(cmap, norm, artist.as_ref());
    Ok(figure.colorbar(shown.mappable(), options)?)
}

/// The axes the colorbar is drawn in.
fn ax(colorbar: Colorbar) -> PyResult<Axes> {
    Ok(colorbar.ax().clone())
}

/// `'vertical'` or `'horizontal'`.
fn get_orientation(colorbar: Colorbar) -> PyResult<&'static str> {
    Ok(match colorbar.orientation() {
        Orientation::Vertical => "vertical",
        Orientation::Horizontal => "horizontal",
    })
}

/// The ends extended beyond the range.
fn get_extend(colorbar: Colorbar) -> PyResult<&'static str> {
    Ok(match colorbar.extend() {
        Extend::Neither => "neither",
        Extend::Both => "both",
        Extend::Min => "min",
        Extend::Max => "max",
    })
}

/// The edges of the colour blocks, in data units.
fn boundaries(colorbar: Colorbar) -> PyResult<NdArray> {
    Ok(NdArray::from_vec(colorbar.boundaries()))
}

/// The value at the middle of each colour block.
fn values(colorbar: Colorbar) -> PyResult<NdArray> {
    Ok(NdArray::from_vec(colorbar.values()))
}

/// The limits of the colorbar's data range.
fn limits(colorbar: Colorbar) -> PyResult<(f64, f64)> {
    Ok((colorbar.vmin(), colorbar.vmax()))
}

/// Draws the levels of the line contours `contours` across the colorbar
/// in their colours and widths; `erase` removes the lines added before
/// (`add_lines(contour_set)`).
fn add_contour_lines(colorbar: Colorbar, contours: ContourSet, erase: bool) -> PyResult<()> {
    colorbar.add_lines(&contours, erase)?;
    Ok(())
}

/// Draws lines across the colorbar at the data values `levels`, with the
/// colours `colors` and widths `linewidths` (one per level, or cycled);
/// `erase` removes the lines added before (`add_lines(levels, colors,
/// linewidths)`).
fn add_level_lines(
    colorbar: Colorbar,
    levels: Vec<f64>,
    colors: Vec<Color>,
    linewidths: Vec<f64>,
    erase: bool,
) -> PyResult<()> {
    colorbar.add_level_lines(&levels, colors, linewidths, erase)?;
    Ok(())
}

/// Puts the major ticks at `ticks`.
fn set_ticks(colorbar: Colorbar, ticks: Vec<f64>) -> PyResult<()> {
    colorbar.set_ticks(ticks);
    Ok(())
}

/// The major (or, `minor`, minor) tick locations along the colorbar.
fn get_ticks(colorbar: Colorbar, minor: bool) -> PyResult<NdArray> {
    Ok(NdArray::from_vec(colorbar.long_axis().ticklocs(minor)?))
}

/// Turns the minor ticks on (`on`) or off.
fn set_minorticks(colorbar: Colorbar, on: bool) -> PyResult<()> {
    if on {
        colorbar.minorticks_on();
    } else {
        colorbar.minorticks_off();
    }
    Ok(())
}

/// Shows the mapping through `cmap` and `norm`, of the drawn `artist` if
/// the mappable is one (the mappable changed).
fn update_normal(
    colorbar: Colorbar,
    (cmap, norm, artist): (Colormap, Box<dyn Norm>, Option<RawHandle>),
) -> PyResult<()> {
    colorbar.update_normal(Shown::new(cmap, norm, artist.as_ref()).mappable())?;
    Ok(())
}

/// Removes the colorbar and its axes, giving the space back.
fn remove(colorbar: Colorbar) -> PyResult<()> {
    colorbar.remove();
    Ok(())
}

/// The outline around the colours and the extensions (matplotlib's
/// `outline`).
fn outline(colorbar: Colorbar) -> PyResult<Patch<PathShape>> {
    Ok(colorbar.outline())
}

/// The mesh of colour blocks (matplotlib's `solids`), `None` when the
/// blocks are hatched patches.
fn solids(colorbar: Colorbar) -> PyResult<Option<Collection>> {
    Ok(colorbar.solids())
}

/// The patches filling the extensions, the one at the lower end of the
/// long axis first (matplotlib's `_extend_patches`).
fn extend_patches(colorbar: Colorbar) -> PyResult<Vec<Patch<PathShape>>> {
    Ok(colorbar.extend_patches())
}

/// With `drawedges`, the lines between the colours in the current view, as
/// `(2, 2)` arrays of their ends (matplotlib's `dividers.get_segments()`).
fn divider_segments(colorbar: Colorbar) -> PyResult<Vec<NdArray>> {
    colorbar
        .divider_segments()
        .iter()
        .map(|path| {
            let data = path.vertices().iter().flat_map(|p| [p.x, p.y]).collect();
            NdArray::with_shape(data, vec![path.vertices().len(), 2])
        })
        .collect()
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("new", new)?;
    module.add("ax", ax)?;
    module.add("get_orientation", get_orientation)?;
    module.add("get_extend", get_extend)?;
    module.add("boundaries", boundaries)?;
    module.add("values", values)?;
    module.add("limits", limits)?;
    module.add("add_contour_lines", add_contour_lines)?;
    module.add("add_level_lines", add_level_lines)?;
    module.add("set_ticks", set_ticks)?;
    module.add("get_ticks", get_ticks)?;
    module.add("set_minorticks", set_minorticks)?;
    module.add("update_normal", update_normal)?;
    module.add("remove", remove)?;
    module.add("outline", outline)?;
    module.add("solids", solids)?;
    module.add("extend_patches", extend_patches)?;
    module.add("divider_segments", divider_segments)
}
