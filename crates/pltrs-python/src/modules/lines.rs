//! `_native.lines`: `Line2D` (matplotlib's `lines`), the line properties
//! plotting functions take as keyword arguments, and the conversions of
//! the style values lines share with patches and collections (line
//! styles, cap and join styles, `markevery`).

use std::str::FromStr;

use pltrs::artist::ArtistHandle;
use pltrs::colors::Color;
use pltrs::lines::{
    DrawStyle, FormatStringError, Line2D, LineProps, LineStyle, MarkEvery, MarkEveryError,
    StepError,
};
use pltrs::markers::{FillStyle, Marker, MarkerStyle, MarkerSymbol};
use pltrs::rc::ColorOrAuto;
use pltrs::transforms::Transform;
use pltrs_render::{CapStyle, JoinStyle};

use super::artist::{ClipSpec, PyPicker};
use super::markers::marker_of_spec;
use super::path::{PathArrays, path_arrays};
use super::patheffects::path_effects;
use super::transforms::{affine, matrix};
use crate::binding::prelude::*;
use crate::binding::{NdArray, dict_items};

mod axline;
mod markevery;

pub(super) use markevery::markevery_error;

artist_handle!(Line2D);

impl From<FormatStringError> for PyErr {
    /// matplotlib's `ValueError` for a bad format string, which quotes it
    /// as Python's `repr` does.
    fn from(err: FormatStringError) -> Self {
        let quoted = Python::with_gil(|py| err.format.as_str().into_py(py)?.repr())
            .unwrap_or_else(|_| format!("{:?}", err.format));
        PyErr::value_error(format!(
            "{quoted} is not a valid format string ({})",
            err.reason
        ))
    }
}

impl From<StepError> for PyErr {
    /// matplotlib's `ValueError` for an unknown draw style.
    fn from(err: StepError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

impl From<MarkEveryError> for PyErr {
    /// matplotlib's `ValueError`s of `_mark_every_path`.
    fn from(err: MarkEveryError) -> Self {
        match err {
            MarkEveryError::NonPositiveDistance(step) => PyErr::value_error(format!(
                "'markevery' step must be positive, but got {step:?}"
            )),
            MarkEveryError::NeedsGeometry => PyErr::value_error(
                "markevery is specified relative to the Axes size, but the line does not \
                 have an Axes as parent",
            ),
            other => PyErr::value_error(other.to_string()),
        }
    }
}

/// Parses a keyword's string value, raising `ValueError` with pltrs (Rust)'s
/// reason.
pub(super) fn parse<T: FromStr<Err: std::fmt::Display>>(object: &Bound<'_>) -> PyResult<T> {
    let text: String = object.extract()?;
    text.parse()
        .map_err(|err: T::Err| PyErr::value_error(err.to_string()))
}

/// A line style as matplotlib takes it: a name or short form (`"--"`),
/// or an `(offset, on-off sequence)` tuple whose sequence may be `None`.
pub(super) struct PyLineStyle(pub(super) LineStyle);

impl FromPy<'_> for PyLineStyle {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        if object.extract::<String>().is_ok() {
            return parse(object).map(Self);
        }
        let invalid = || {
            PyErr::value_error(format!(
                "{} is not a valid value for ls; supported values are '-', '--', '-.', ':', \
                 'None', ' ', '', 'solid', 'dashed', 'dashdot', 'dotted'",
                object.repr().unwrap_or_default()
            ))
        };
        let (offset, pattern): (f64, Option<Vec<f64>>) = object.extract().map_err(|_| invalid())?;
        if pattern
            .as_ref()
            .is_some_and(|pattern| pattern.iter().any(|&v| v < 0.0 || !v.is_finite()))
        {
            return Err(PyErr::value_error(
                "All values in the dash list must be non-negative",
            ));
        }
        if pattern
            .as_ref()
            .is_some_and(|pattern| !pattern.is_empty() && pattern.iter().all(|&v| v == 0.0))
        {
            return Err(PyErr::value_error(
                "At least one value in the dash list must be positive",
            ));
        }
        Ok(Self(LineStyle::Dashes { offset, pattern }))
    }
}

impl IntoPy for PyLineStyle {
    /// matplotlib's spelling: the short form of a named style, else the
    /// `(offset, sequence)` tuple.
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        match self.0 {
            LineStyle::Dashes { offset, pattern } => (offset, pattern).into_py(py),
            named => named.to_string().into_py(py),
        }
    }
}

/// A cap style by matplotlib's name.
pub(super) fn cap_style(object: &Bound<'_>) -> PyResult<CapStyle> {
    let name: String = object.str()?;
    CapStyle::from_name(&name)
        .ok_or_else(|| PyErr::value_error(format!("{name:?} is not a valid CapStyle")))
}

/// A join style by matplotlib's name.
pub(super) fn join_style(object: &Bound<'_>) -> PyResult<JoinStyle> {
    let name: String = object.str()?;
    JoinStyle::from_name(&name)
        .ok_or_else(|| PyErr::value_error(format!("{name:?} is not a valid JoinStyle")))
}

/// matplotlib's `markevery`: an integer, an `(start, step)` pair of
/// integers or of floats, a float, a slice, a list of indices or a mask.
struct PyMarkEvery(MarkEvery);

impl FromPy<'_> for PyMarkEvery {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let invalid = || {
            PyErr::value_error(format!(
                "markevery={} is not a recognized value",
                object.repr().unwrap_or_default()
            ))
        };
        if object.type_name() == "slice" {
            let field = |name| object.getattr(name)?.extract::<Option<i64>>();
            return Ok(Self(MarkEvery::Slice {
                start: field("start")?,
                stop: field("stop")?,
                step: field("step")?,
            }));
        }
        if is_integer(object) {
            return Ok(Self(MarkEvery::Every {
                start: 0,
                step: object.extract()?,
            }));
        }
        if object.type_name().starts_with("float") {
            return Ok(Self(MarkEvery::Distance {
                start: 0.0,
                step: object.extract()?,
            }));
        }
        let pair = object.type_name() == "tuple";
        let items: Vec<Bound<'_>> = object.extract().map_err(|_| invalid())?;
        // A pair is a start and a stride only as a tuple; a list of two
        // integers holds indices.
        if pair {
            return start_and_step(object, &items).map(Self);
        }
        if !items.is_empty()
            && items
                .iter()
                .all(|item| item.type_name().starts_with("bool"))
        {
            return Ok(Self(MarkEvery::Mask(
                items.iter().map(Bound::extract).collect::<PyResult<_>>()?,
            )));
        }
        if items.iter().all(is_integer) {
            return Ok(Self(MarkEvery::Indices(
                items.iter().map(Bound::extract).collect::<PyResult<_>>()?,
            )));
        }
        Err(invalid())
    }
}

/// A `markevery` tuple `object` of `items`: `(start, step)` integers, or
/// a start and a float distance step (matplotlib's `_mark_every_path`,
/// whose messages these are).
fn start_and_step(object: &Bound<'_>, items: &[Bound<'_>]) -> PyResult<MarkEvery> {
    let shown = object.repr().unwrap_or_default();
    let [start, step] = items else {
        return Err(PyErr::value_error(format!(
            "`markevery` is a tuple but its len is not 2; markevery={shown}"
        )));
    };
    let is_float = |item: &Bound<'_>| item.type_name().starts_with("float");
    if is_integer(step) {
        if !is_integer(start) {
            return Err(PyErr::value_error(format!(
                "`markevery` is a tuple with len 2 and second element is an int, but the \
                 first element is not an int; markevery={shown}"
            )));
        }
        return Ok(MarkEvery::Every {
            start: start.extract()?,
            step: step.extract()?,
        });
    }
    if !is_float(step) || !(is_integer(start) || is_float(start)) {
        return Err(PyErr::value_error(format!(
            "`markevery` is a tuple with len 2 and second element is a float, but the first \
             element is not a float or an int; markevery={shown}"
        )));
    }
    Ok(MarkEvery::Distance {
        start: start.extract()?,
        step: step.extract()?,
    })
}

/// Whether `object` is an integer (Python's or numpy's, not a bool).
fn is_integer(object: &Bound<'_>) -> bool {
    let name = object.type_name();
    name == "int" || (name.starts_with("int") || name.starts_with("uint")) && name != "bool"
}

/// `markevery` back as matplotlib's values.
fn markevery_object(py: Python<'_>, every: Option<MarkEvery>) -> PyResult<Bound<'_>> {
    match every {
        None => Ok(Bound::none(py)),
        Some(MarkEvery::Every { start: 0, step }) => step.into_py(py),
        Some(MarkEvery::Every { start, step }) => (start, step).into_py(py),
        Some(MarkEvery::Distance { start, step }) => (start, step).into_py(py),
        Some(MarkEvery::Indices(indices)) => indices.into_py(py),
        Some(MarkEvery::Mask(mask)) => mask.into_py(py),
        Some(MarkEvery::Slice { start, stop, step }) => (start, stop, step).into_py(py),
    }
}

/// A marker as the Python side passes it: the `(kind, value)` spec of a
/// `MarkerStyle` (see `_native.markers`), its fill style, its user
/// transform's matrix and its cap and join styles.
pub(super) struct PyMarker {
    marker: Marker,
    fillstyle: FillStyle,
    transform: Option<NdArray>,
    capstyle: Option<CapStyle>,
    joinstyle: Option<JoinStyle>,
}

impl FromPy<'_> for PyMarker {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let (spec, fillstyle, transform, capstyle, joinstyle): (
            Bound<'_>,
            String,
            Option<NdArray>,
            Option<Bound<'_>>,
            Option<Bound<'_>>,
        ) = object.extract()?;
        Ok(Self {
            marker: marker_of_spec(&spec)?,
            fillstyle: fillstyle.parse()?,
            transform,
            capstyle: capstyle.as_ref().map(cap_style).transpose()?,
            joinstyle: joinstyle.as_ref().map(join_style).transpose()?,
        })
    }
}

impl PyMarker {
    /// The marker shape, without its fill and line styles (what a
    /// sparsity plot takes).
    pub(super) fn marker(&self) -> Marker {
        self.marker.clone()
    }

    /// The marker as a resolved style (what collections of markers take).
    pub(super) fn style(&self) -> PyResult<MarkerStyle> {
        let style = MarkerStyle::with_styles(
            self.marker.clone(),
            self.fillstyle,
            self.capstyle,
            self.joinstyle,
        )?;
        Ok(match &self.transform {
            Some(matrix) => style.transformed(affine(matrix)?),
            None => style,
        })
    }

    /// `props` with the marker's properties.
    fn apply(self, props: LineProps) -> PyResult<LineProps> {
        Ok(props
            .marker(self.marker)
            .fillstyle(self.fillstyle)
            .marker_transform(self.transform.as_ref().map(affine).transpose()?)
            .marker_capstyle(self.capstyle)
            .marker_joinstyle(self.joinstyle))
    }
}

/// The line properties of a dict of `Line2D` keyword arguments, whose
/// aliases the Python side has already resolved (`lw` is `linewidth`)
/// and whose markers it made `MarkerStyle` specs.
#[derive(Clone, Debug, Default)]
pub(super) struct LineKwargs(pub(super) LineProps);

impl FromPy<'_> for LineKwargs {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let mut props = LineProps::default();
        for (key, value) in dict_items(object)? {
            let key: String = key.extract()?;
            props = line_prop(props, &key, &value)?;
        }
        Ok(Self(props))
    }
}

/// `props` with the property `key` set to `value`.
fn line_prop(props: LineProps, key: &str, value: &Bound<'_>) -> PyResult<LineProps> {
    Ok(match key {
        "color" => props.color(value.extract::<Color>()?),
        "linewidth" => props.linewidth(value.extract::<f64>()?),
        "linestyle" => props.linestyle(value.extract::<PyLineStyle>()?.0),
        "drawstyle" => props.drawstyle(drawstyle(value)?),
        "gapcolor" => props.gapcolor(optional_color(value)?),
        "marker" => value.extract::<PyMarker>()?.apply(props)?,
        "fillstyle" => props.fillstyle(parse::<FillStyle>(value)?),
        "markersize" => props.markersize(value.extract::<f64>()?),
        "markeredgewidth" => props.markeredgewidth(value.extract::<f64>()?),
        "markeredgecolor" => props.markeredgecolor(color_or_auto(value)?),
        "markerfacecolor" => props.markerfacecolor(color_or_auto(value)?),
        "markerfacecoloralt" => props.markerfacecoloralt(optional_color(value)?),
        "markevery" => {
            props.markevery(value.extract::<Option<PyMarkEvery>>()?.map(|every| every.0))
        }
        "antialiased" => props.antialiased(value.extract::<bool>()?),
        "pickradius" => props.pickradius(value.extract::<f64>()?),
        "solid_capstyle" => props.solid_capstyle(cap_style(value)?),
        "solid_joinstyle" => props.solid_joinstyle(join_style(value)?),
        "dash_capstyle" => props.dash_capstyle(cap_style(value)?),
        "dash_joinstyle" => props.dash_joinstyle(join_style(value)?),
        "label" => props.label(value.str()?),
        "alpha" => match value.extract::<Option<f64>>()? {
            Some(alpha) => props.alpha(alpha),
            None => props,
        },
        "zorder" => props.zorder(value.extract::<f64>()?),
        "visible" => props.visible(value.extract::<bool>()?),
        "clip_on" => props.clip_on(value.extract::<bool>()?),
        "animated" => props.animated(value.extract::<bool>()?),
        "rasterized" => props.rasterized(value.extract::<bool>()?),
        "gid" => match value.extract::<Option<String>>()? {
            Some(gid) => props.gid(gid),
            None => props,
        },
        "url" => match value.extract::<Option<String>>()? {
            Some(url) => props.url(url),
            None => props,
        },
        "transform" => props.transform(value.extract::<Transform>()?),
        "picker" => match value.extract::<PyPicker>()?.0 {
            Some(picker) => props.picker(picker),
            None => props,
        },
        "path_effects" => props.path_effects(path_effects(value)?),
        "clip_path" => match value.extract::<Option<ClipSpec>>()? {
            Some(clip) => props.clip_path(clip.0),
            None => props,
        },
        _ => {
            return Err(PyErr::new(
                ExceptionKind::AttributeError,
                format!("Line2D.set() got an unexpected keyword argument '{key}'"),
            ));
        }
    })
}

/// A draw style by name, with matplotlib's message for others.
fn drawstyle(value: &Bound<'_>) -> PyResult<DrawStyle> {
    let name: String = value.extract()?;
    name.parse().map_err(|_| {
        PyErr::value_error(format!(
            "{name:?} is not a valid value for drawstyle; supported values are 'default', \
             'steps-mid', 'steps-pre', 'steps-post', 'steps'"
        ))
    })
}

/// A colour, `None` meaning `"none"`.
fn optional_color(value: &Bound<'_>) -> PyResult<Color> {
    Ok(value
        .extract::<Option<Color>>()?
        .unwrap_or_else(Color::none))
}

/// A colour or `"auto"`, `None` meaning `"auto"`.
fn color_or_auto(value: &Bound<'_>) -> PyResult<ColorOrAuto> {
    Ok(match value.extract::<Option<Color>>()? {
        Some(color) => ColorOrAuto::from(color),
        None => ColorOrAuto::Auto,
    })
}

impl LineKwargs {
    /// The properties of the format string `fmt` (`"ro--"`) with these
    /// on top.
    pub(super) fn over_format(self, fmt: Option<&str>) -> PyResult<LineProps> {
        let base = match fmt {
            Some(fmt) => LineProps::fmt(fmt)?,
            None => LineProps::default(),
        };
        Ok(base.overlay(self.0))
    }
}

/// A line through `(x, y)` styled by the keyword arguments, outside any
/// figure (`Line2D(xdata, ydata, **kwargs)`), for an axes or a figure to
/// adopt (`_native.artist.adopt_into_axes`).
fn new(x: NdArray, y: NdArray, kwargs: LineKwargs) -> PyResult<Line2D> {
    Ok(Line2D::loose(x.into_vec()?, y.into_vec()?, kwargs.0))
}

/// The x data.
fn get_xdata(line: Line2D) -> PyResult<NdArray> {
    Ok(NdArray::from_vec(line.xdata()))
}

/// The y data.
fn get_ydata(line: Line2D) -> PyResult<NdArray> {
    Ok(NdArray::from_vec(line.ydata()))
}

/// Replaces both data arrays.
fn set_data(line: Line2D, x: NdArray, y: NdArray) -> PyResult<()> {
    line.set_data(x.into_vec()?, y.into_vec()?);
    Ok(())
}

/// Sets the properties of a dict of keyword arguments (`Line2D.set`).
fn update(line: Line2D, kwargs: LineKwargs) -> PyResult<()> {
    line.update_props(kwargs.0);
    Ok(())
}

/// The property `name` of the line, as matplotlib's `get_<name>` returns
/// it: colours as given (`markeredgecolor` and `markerfacecolor`
/// resolved as matplotlib resolves `"auto"`), styles by name.
fn get(line: Line2D, name: String) -> PyResult<Owned> {
    Python::with_gil(|py| {
        let value = match name.as_str() {
            "color" => line.color().into_py(py)?,
            "linewidth" => line.linewidth().into_py(py)?,
            "linestyle" => match line.linestyle() {
                LineStyle::Dashes { .. } => "--".into_py(py)?,
                named => named.to_string().into_py(py)?,
            },
            "dash_pattern" => PyLineStyle(line.linestyle()).into_py(py)?,
            "drawstyle" => line.drawstyle().to_string().into_py(py)?,
            "gapcolor" => gap_color(line.gapcolor()).into_py(py)?,
            "marker" => marker_object(py, line.marker())?,
            "fillstyle" => line.fillstyle().to_string().into_py(py)?,
            "markersize" => line.markersize().into_py(py)?,
            "markeredgewidth" => line.markeredgewidth().into_py(py)?,
            "markeredgecolor" => line.effective_markeredgecolor().into_py(py)?,
            "markerfacecolor" => line.effective_markerfacecolor().into_py(py)?,
            "markerfacecoloralt" => line.markerfacecoloralt().into_py(py)?,
            "markeredgecolor_setting" => color_setting(line.markeredgecolor()).into_py(py)?,
            "markerfacecolor_setting" => color_setting(line.markerfacecolor()).into_py(py)?,
            "markevery" => markevery_object(py, line.markevery())?,
            "antialiased" => line.antialiased().into_py(py)?,
            "pickradius" => line.pickradius().into_py(py)?,
            "solid_capstyle" => line.solid_capstyle().name().into_py(py)?,
            "solid_joinstyle" => line.solid_joinstyle().name().into_py(py)?,
            "dash_capstyle" => line.dash_capstyle().name().into_py(py)?,
            "dash_joinstyle" => line.dash_joinstyle().name().into_py(py)?,
            "marker_transform" => line
                .marker_transform()
                .map(matrix)
                .transpose()?
                .into_py(py)?,
            _ => {
                return Err(PyErr::new(
                    ExceptionKind::AttributeError,
                    format!("'Line2D' object has no property '{name}'"),
                ));
            }
        };
        Ok(value.unbind())
    })
}

/// A marker colour as set: `"auto"` or the colour.
fn color_setting(color: ColorOrAuto) -> Color {
    match color {
        ColorOrAuto::Auto => Color::from("auto"),
        ColorOrAuto::Color(color) => color,
    }
}

/// The gap colour, `None` for `"none"` (matplotlib's default).
fn gap_color(color: Color) -> Option<Color> {
    (!color.is_none()).then_some(color)
}

/// A marker as matplotlib spells it: a named marker by its key (`"o"`,
/// an integer code), `(sides, style, angle)` for polygons, the vertices,
/// `("path", (vertices, codes))` for a path, or the mathtext.
fn marker_object(py: Python<'_>, marker: Marker) -> PyResult<Bound<'_>> {
    match marker {
        Marker::Symbol(MarkerSymbol::Nothing) => "None".into_py(py),
        Marker::Symbol(symbol) => match MarkerSymbol::all().find(|(_, s)| *s == symbol) {
            Some((pltrs::markers::MarkerKey::Char(key), _)) => key.into_py(py),
            Some((pltrs::markers::MarkerKey::Code(code), _)) => code.into_py(py),
            None => "None".into_py(py),
        },
        Marker::Polygon {
            sides,
            style,
            angle,
        } => {
            let code: i64 = match style {
                pltrs::markers::PolygonStyle::Regular => 0,
                pltrs::markers::PolygonStyle::Star => 1,
                pltrs::markers::PolygonStyle::Asterisk => 2,
            };
            (sides, code, angle).into_py(py)
        }
        Marker::Vertices(vertices) => {
            let shape = vec![vertices.len(), 2];
            super::transforms::points_array(&vertices, shape)?.into_py(py)
        }
        Marker::Path(path) => ("path", path_arrays(&path, false)?).into_py(py),
        Marker::Math(text) => text.into_py(py),
    }
}

/// The path the line draws (its data through the draw style), as
/// `(vertices, codes)`.
fn get_path(line: Line2D) -> PyResult<PathArrays> {
    path_arrays(&line.path()?, false)
}

/// How many pieces each segment of the line is split into before a
/// curved projection bends it (matplotlib's
/// `get_path()._interpolation_steps`).
fn get_interpolation_steps(line: Line2D) -> PyResult<usize> {
    Ok(line.interpolation_steps())
}

/// Draws each segment of the line as `steps` segments, which a curved
/// projection bends.
fn set_interpolation_steps(line: Line2D, steps: usize) -> PyResult<()> {
    line.set_interpolation_steps(steps);
    Ok(())
}

/// The indices of `points` (an `(N, 2)` array of display coordinates) that
/// get markers with `every` (matplotlib's `_mark_every_path`); a distance
/// is measured against `diagonal`, the axes' diagonal in pixels (`None`
/// outside an axes).
fn markevery_indices(
    every: PyMarkEvery,
    points: NdArray,
    diagonal: Option<f64>,
) -> PyResult<Vec<usize>> {
    Ok(every
        .0
        .select(&super::transforms::points(&points)?, diagonal)?)
}

/// Whether the line is dashed (matplotlib's `is_dashed`).
fn is_dashed(line: Line2D) -> PyResult<bool> {
    Ok(!matches!(
        line.linestyle(),
        LineStyle::Solid | LineStyle::None
    ))
}

/// The indices of the points that get markers.
fn marker_indices(line: Line2D) -> PyResult<Vec<usize>> {
    line.marker_indices()
        .map_err(|err| PyErr::value_error(err.to_string()))
}

/// The resolved RGBA colour of the line.
fn get_rgba(line: Line2D) -> PyResult<(f64, f64, f64, f64)> {
    let rgba = line.rgba()?;
    Ok((rgba.r, rgba.g, rgba.b, rgba.a))
}

/// Whether the line is in its figure.
fn is_attached(line: Line2D) -> PyResult<bool> {
    Ok(line.raw().is_attached())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("new", new)?;
    module.add("get_xdata", get_xdata)?;
    module.add("get_ydata", get_ydata)?;
    module.add("set_data", set_data)?;
    module.add("update", update)?;
    module.add("get", get)?;
    module.add("get_path", get_path)?;
    module.add("get_interpolation_steps", get_interpolation_steps)?;
    module.add("set_interpolation_steps", set_interpolation_steps)?;
    module.add("is_dashed", is_dashed)?;
    module.add("markevery_indices", markevery_indices)?;
    module.add("marker_indices", marker_indices)?;
    module.add("get_rgba", get_rgba)?;
    module.add("is_attached", is_attached)?;
    axline::register(module)
}
