//! Printing a figure in a file format (the `print_<format>` methods of
//! matplotlib's canvases, after `print_figure` has resolved its keywords).

use pltrs::colors::Color;
use pltrs::figure::{
    BboxInches, Figure, PadInches, PageOrientation, Paper, SaveFormat, SaveOptions,
};
use pltrs::transforms::Bbox;

use crate::binding::dict_items;
use crate::binding::prelude::*;

/// Parses a keyword's string value, raising `ValueError` with pltrs
/// (Rust)'s reason.
fn parse<T: std::str::FromStr<Err: std::fmt::Display>>(object: &Bound<'_>) -> PyResult<T> {
    let text: String = object.extract()?;
    text.parse()
        .map_err(|err: T::Err| PyErr::value_error(err.to_string()))
}

/// The part of the figure saved: `"tight"` or `(x0, y0, x1, y1)` inches.
fn bbox_inches(object: &Bound<'_>) -> PyResult<BboxInches> {
    if let Ok(text) = object.extract::<String>() {
        return match text.as_str() {
            "tight" => Ok(BboxInches::Tight),
            other => Err(PyErr::value_error(format!(
                "bbox_inches must be 'tight' or a Bbox, not {other:?}"
            ))),
        };
    }
    let (x0, y0, x1, y1): (f64, f64, f64, f64) = object.extract()?;
    Ok(BboxInches::Inches(Bbox::from_extents(x0, y0, x1, y1)))
}

/// The padding around a tight box: inches, or `"layout"`.
fn pad_inches(object: &Bound<'_>) -> PyResult<PadInches> {
    match object.extract::<String>() {
        Ok(text) if text == "layout" => Ok(PadInches::Layout),
        Ok(text) => Err(PyErr::value_error(format!(
            "pad_inches must be a number or 'layout', not {text:?}"
        ))),
        Err(_) => Ok(PadInches::Inches(object.extract()?)),
    }
}

/// The save options of a dict of `print_figure`'s resolved keywords:
/// `dpi`, `facecolor`, `edgecolor`, `transparent`, `bbox_inches`,
/// `pad_inches`, `orientation`, `papertype` and `jpeg_quality`; absent
/// or `None` ones follow the `savefig.*` rcParams.
pub(super) struct PrintSettings(pub(super) SaveOptions);

impl FromPy<'_> for PrintSettings {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let mut options = SaveOptions::default();
        for (key, value) in dict_items(object)? {
            if value.is_none() {
                continue;
            }
            let key: String = key.extract()?;
            match key.as_str() {
                "dpi" => options.dpi = Some(value.extract()?),
                "facecolor" => options.facecolor = Some(value.extract::<Color>()?),
                "edgecolor" => options.edgecolor = Some(value.extract::<Color>()?),
                "transparent" => options.transparent = Some(value.extract()?),
                "bbox_inches" => options.bbox_inches = Some(bbox_inches(&value)?),
                "pad_inches" => options.pad_inches = Some(pad_inches(&value)?),
                "orientation" => options.orientation = Some(parse::<PageOrientation>(&value)?),
                "papertype" => options.papertype = Some(parse::<Paper>(&value)?),
                "jpeg_quality" => options.jpeg_quality = Some(value.extract()?),
                _ => {
                    return Err(PyErr::type_error(format!(
                        "print_figure() got an unexpected setting '{key}'"
                    )));
                }
            }
        }
        Ok(Self(options))
    }
}

/// The figure as a file of `format` (`png`, `pdf`, `svg`, `eps`, `raw`,
/// ...) with `settings`: the bytes the canvas writes to a path or a file
/// object.
fn print_figure(figure: Figure, format: String, settings: PrintSettings) -> PyResult<Bytes> {
    let format: SaveFormat = format.parse()?;
    let options = SaveOptions {
        format: Some(format),
        ..settings.0
    };
    Ok(Bytes(figure.to_format(format, options)?))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("print_figure", print_figure)
}

#[cfg(test)]
mod tests {
    use crate::binding::testing::{eval, extension, with_python};
    use crate::binding::{Bound, IntoPy, PyResult};

    /// `_native.backends.print_figure` of a 2 × 1 inch figure at 50 dpi
    /// in `format`, with the settings dict `settings` (Python source).
    fn print<'py>(native: &Bound<'py>, format: &str, settings: &str) -> PyResult<Bound<'py>> {
        let py = native.py();
        let number = |v: f64| v.into_py(py).unwrap();
        let figure = native
            .getattr("figure")?
            .getattr("new")?
            .call(&[number(2.0), number(1.0), number(50.0)], &[])?;
        native
            .getattr("backends")?
            .getattr("print_figure")?
            .call(&[figure, format.into_py(py)?, eval(py, "", settings)], &[])
    }

    #[test]
    fn a_figure_prints_with_resolved_settings() {
        with_python(|py| {
            let native = extension(py);
            let png = print(&native, "png", "{'dpi': 20.0, 'facecolor': 'red'}").unwrap();
            assert!(png.repr().unwrap().starts_with("b'\\x89PNG"));
            let raw = print(&native, "rgba", "{'dpi': 10.0, 'bbox_inches': None}").unwrap();
            let length = raw.call_method("__len__", &[]).unwrap();
            assert_eq!(length.extract::<usize>().unwrap(), 20 * 10 * 4);
        });
    }

    #[test]
    fn unknown_settings_and_bad_values_raise() {
        with_python(|py| {
            let native = extension(py);
            for (settings, error) in [
                ("{'frobnicate': 1}", "TypeError"),
                ("{'bbox_inches': 'loose'}", "ValueError"),
                ("{'pad_inches': 'wide'}", "ValueError"),
                ("{'orientation': 'sideways'}", "ValueError"),
            ] {
                let err = print(&native, "png", settings).unwrap_err();
                assert_eq!(err.class_name(py), error, "{settings}");
            }
        });
    }
}
