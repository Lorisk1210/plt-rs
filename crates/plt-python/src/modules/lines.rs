//! `_native.lines`: `Line2D` (matplotlib's `lines`), and the line
//! properties plotting functions take as keyword arguments.

use plt::colors::Color;
use plt::lines::{FormatStringError, Line2D, LineProps, LineStyle};
use plt::markers::Marker;

use crate::binding::prelude::*;
use crate::binding::{NdArray, dict_items};

artist_handle!(Line2D);

impl From<FormatStringError> for PyErr {
    /// matplotlib's `ValueError` for a bad format string.
    fn from(err: FormatStringError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

/// Parses a keyword's string value, raising `ValueError` with plt-rs's
/// reason.
fn parse<T: std::str::FromStr<Err: std::fmt::Display>>(object: &Bound<'_>) -> PyResult<T> {
    let text: String = object.extract()?;
    text.parse()
        .map_err(|err: T::Err| PyErr::value_error(err.to_string()))
}

/// The line properties of a dict of `Line2D` keyword arguments, whose
/// aliases the Python side has already resolved (`lw` is `linewidth`).
#[derive(Clone, Debug, Default)]
pub(super) struct LineKwargs(pub(super) LineProps);

impl FromPy<'_> for LineKwargs {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let mut props = LineProps::default();
        for (key, value) in dict_items(object)? {
            let key: String = key.extract()?;
            props = match key.as_str() {
                "color" => props.color(value.extract::<Color>()?),
                "linewidth" => props.linewidth(value.extract::<f64>()?),
                "linestyle" => props.linestyle(parse::<LineStyle>(&value)?),
                "marker" => props.marker(parse::<Marker>(&value)?),
                "markersize" => props.markersize(value.extract::<f64>()?),
                "label" => props.label(value.str()?),
                "alpha" => props.alpha(value.extract::<f64>()?),
                "zorder" => props.zorder(value.extract::<f64>()?),
                "visible" => props.visible(value.extract::<bool>()?),
                _ => {
                    return Err(PyErr::new(
                        ExceptionKind::AttributeError,
                        format!("Line2D.set() got an unexpected keyword argument '{key}'"),
                    ));
                }
            };
        }
        Ok(Self(props))
    }
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

/// The line colour, as it was given.
fn get_color(line: Line2D) -> PyResult<Color> {
    Ok(line.color())
}

/// Sets the line colour.
fn set_color(line: Line2D, color: Color) -> PyResult<()> {
    line.set_color(color);
    Ok(())
}

/// The line width in points.
fn get_linewidth(line: Line2D) -> PyResult<f64> {
    Ok(line.linewidth())
}

/// Sets the line width in points.
fn set_linewidth(line: Line2D, width: f64) -> PyResult<()> {
    line.set_linewidth(width);
    Ok(())
}

/// The line style, as matplotlib spells it (`"--"`).
fn get_linestyle(line: Line2D) -> PyResult<String> {
    Ok(line.linestyle().to_string())
}

/// Sets the line style from its name or short form.
fn set_linestyle(line: Line2D, style: String) -> PyResult<()> {
    let style: LineStyle =
        style
            .parse()
            .map_err(|err: <LineStyle as std::str::FromStr>::Err| {
                PyErr::value_error(err.to_string())
            })?;
    line.set_linestyle(style);
    Ok(())
}

/// The marker size in points.
fn get_markersize(line: Line2D) -> PyResult<f64> {
    Ok(line.markersize())
}

/// Sets the marker size in points.
fn set_markersize(line: Line2D, size: f64) -> PyResult<()> {
    line.set_markersize(size);
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("get_xdata", get_xdata)?;
    module.add("get_ydata", get_ydata)?;
    module.add("set_data", set_data)?;
    module.add("get_color", get_color)?;
    module.add("set_color", set_color)?;
    module.add("get_linewidth", get_linewidth)?;
    module.add("set_linewidth", set_linewidth)?;
    module.add("get_linestyle", get_linestyle)?;
    module.add("set_linestyle", set_linestyle)?;
    module.add("get_markersize", get_markersize)?;
    module.add("set_markersize", set_markersize)
}
