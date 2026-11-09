//! Printing a figure in a file format (the `print_<format>` methods of
//! matplotlib's canvases, after `print_figure` has resolved its keywords).

use pltrs::colors::Color;
use pltrs::events::EventKind;
use pltrs::figure::{
    BboxInches, Figure, Metadata, PadInches, PageOrientation, Paper, PgfTextMetrics, SaveError,
    SaveFormat, SaveOptions, SvgMetadata,
};
use pltrs::transforms::Bbox;
use pltrs_render::{Timestamp, VectorError};

use super::super::lines::markevery_error;
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

/// The document metadata of `savefig(metadata=...)`: pltrs' defaults
/// with the entries given set (a string) or left out (`None`). A PNG file
/// records every entry as a text chunk; the vector formats record those of
/// PDF's document information dictionary they hold (`Title`, `Author`,
/// `Subject`, `Keywords`, `Creator`, `Producer`, and the dates, given as
/// whole seconds since the Unix epoch).
fn metadata(object: &Bound<'_>) -> PyResult<Metadata> {
    metadata_over(Metadata::default(), object)
}

/// The metadata `base` with the entries of the dict `object` set or left
/// out (see [`metadata`]).
fn metadata_over(base: Metadata, object: &Bound<'_>) -> PyResult<Metadata> {
    let mut metadata = base;
    for (key, value) in dict_items(object)? {
        let key: String = key.extract()?;
        let date = match key.as_str() {
            "CreationDate" => Some(&mut metadata.date),
            "Date" if value.is_none() => Some(&mut metadata.date),
            "ModDate" => Some(&mut metadata.modified),
            _ => None,
        };
        if let Some(date) = date {
            *date = value.extract::<Option<i64>>()?.map(Timestamp);
            continue;
        }
        if svg_entry(&mut metadata.svg, &key, &value)? {
            continue;
        }
        let text: Option<String> = value.extract()?;
        let field = match key.as_str() {
            "Title" => Some(&mut metadata.title),
            "Author" => Some(&mut metadata.author),
            "Subject" => Some(&mut metadata.subject),
            "Keywords" => Some(&mut metadata.keywords),
            "Creator" => Some(&mut metadata.creator),
            "Producer" => Some(&mut metadata.producer),
            _ => None,
        };
        if let Some(field) = field {
            field.clone_from(&text);
        }
        metadata.png = match text {
            Some(text) => metadata.png.with(key, text),
            None => metadata.png.without(&key),
        };
    }
    Ok(metadata)
}

/// Document metadata as native functions take it: a dict of
/// `savefig(metadata=...)` entries (see [`metadata`]).
pub(crate) struct DocumentMetadata(pub(crate) Metadata);

impl FromPy<'_> for DocumentMetadata {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        metadata(object).map(Self)
    }
}

/// A document's whole information, as native functions take it: a dict
/// of the entries of [`DocumentMetadata`], without pltrs' defaults.
pub(crate) struct DocumentInfo(pub(crate) Metadata);

impl FromPy<'_> for DocumentInfo {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        metadata_over(Metadata::none(), object).map(Self)
    }
}

/// Records the SVG entry `key` of `savefig(metadata=...)` in `svg`, if it
/// is one only SVG documents hold or a list of values (`Creator`,
/// `Keywords` and `Date` as lists; `backend_svg` has checked the types
/// and written dates in ISO 8601); `None` leaves a default entry out.
/// Returns whether the entry was SVG's.
fn svg_entry(svg: &mut SvgMetadata, key: &str, value: &Bound<'_>) -> PyResult<bool> {
    let several = match key {
        "Creator" => Some(&mut svg.creators),
        "Keywords" => Some(&mut svg.keywords),
        "Date" => Some(&mut svg.dates),
        "Contributor" => Some(&mut svg.contributors),
        "Publisher" => Some(&mut svg.publishers),
        "Rights" => Some(&mut svg.rights),
        _ => None,
    };
    if let Some(values) = several {
        // A single creator or keyword, or none, is the shared entry.
        let shared = matches!(key, "Creator" | "Keywords");
        if value.is_none() || (shared && value.extract::<String>().is_ok()) {
            return Ok(false);
        }
        *values = match value.extract::<String>() {
            Ok(text) => vec![text],
            Err(_) => value.extract()?,
        };
        return Ok(true);
    }
    let single = match key {
        "Type" => &mut svg.resource_type,
        "Format" => &mut svg.format,
        "Coverage" => &mut svg.coverage,
        "Description" => &mut svg.description,
        "Identifier" => &mut svg.identifier,
        "Language" => &mut svg.language,
        "Relation" => &mut svg.relation,
        "Source" => &mut svg.source,
        _ => return Ok(false),
    };
    *single = value.extract()?;
    Ok(true)
}

/// The save options of a dict of `print_figure`'s resolved keywords:
/// `dpi`, `facecolor`, `edgecolor`, `transparent`, `bbox_inches`,
/// `pad_inches`, `orientation`, `papertype`, `jpeg_quality` and
/// `metadata`; absent or `None` ones follow the `savefig.*` rcParams.
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
                "metadata" => options.metadata = Some(value.extract::<DocumentMetadata>()?.0),
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

/// Runs `print` with `on_draw` called once the figure is drawn, before
/// what was drawn is encoded, as matplotlib's `Figure.draw` sends the
/// `draw_event` before its canvas writes the file.
pub(super) fn sending_draw_event<T>(
    figure: &Figure,
    on_draw: Option<Callback>,
    print: impl FnOnce() -> T,
) -> T {
    let connection = on_draw.map(|callback| {
        figure.connect(EventKind::Draw, move |_| {
            // Its exception is raised from the print (see `Callback`).
            let _ = callback.call::<Owned>(());
        })
    });
    let printed = print();
    if let Some(connection) = connection {
        figure.disconnect(connection);
    }
    printed
}

/// The figure as a file of `format` (`png`, `pdf`, `svg`, `eps`, `raw`,
/// ...) with `settings`: the bytes the canvas writes to a path or a file
/// object; `on_draw` is called once it is drawn (see
/// [`sending_draw_event`]). A line whose `markevery` does not fit it
/// raises matplotlib's `ValueError`.
fn print_figure(
    figure: Figure,
    format: String,
    settings: PrintSettings,
    on_draw: Option<Callback>,
) -> PyResult<Bytes> {
    let format: SaveFormat = format.parse()?;
    let options = SaveOptions {
        format: Some(format),
        ..settings.0
    };
    let printed = sending_draw_event(&figure, on_draw, || figure.to_format(format, options));
    raised(&figure, printed).map(Bytes)
}

/// What a print of `figure` gave, its error raised as matplotlib raises
/// it (a line whose `markevery` does not fit it: `ValueError`).
fn raised<T>(figure: &Figure, printed: Result<T, SaveError>) -> PyResult<T> {
    match printed {
        Err(SaveError::Draw(problems)) => {
            Err(markevery_error(figure, &problems)
                .unwrap_or_else(|| SaveError::Draw(problems).into()))
        }
        printed => Ok(printed?),
    }
}

/// The figure as an SVG document with `settings`, and its images as
/// `(name, png)` to write beside the document, named after `image_stem`
/// (the name of the file written to, `""` for a stream without one), with
/// `svg.image_inline` off (matplotlib's `print_svg`); `on_draw` is called
/// once it is drawn.
fn print_svg(
    figure: Figure,
    settings: PrintSettings,
    image_stem: String,
    on_draw: Option<Callback>,
) -> PyResult<(Bytes, Vec<(String, Bytes)>)> {
    let options = SaveOptions {
        format: Some(SaveFormat::Svg),
        ..settings.0
    };
    let printed = sending_draw_event(&figure, on_draw, || {
        figure.to_svg_document(options, &image_stem)
    });
    let (document, images) = raised(&figure, printed)?;
    let images = images
        .into_iter()
        .map(|image| (image.name, Bytes(image.png)))
        .collect();
    Ok((Bytes(document.into_bytes()), images))
}

/// The figure as a PGF picture with `settings`, its text measured by the
/// TeX system as matplotlib's PGF backend measures it (`print_pgf`): the
/// picture and its images as `(name, png)`, named after `image_stem`, to
/// write beside the picture's file; without a stem a figure with images
/// cannot be written. `on_draw` is called once it is drawn.
fn print_pgf(
    figure: Figure,
    settings: PrintSettings,
    image_stem: Option<String>,
    on_draw: Option<Callback>,
) -> PyResult<(String, Vec<(String, Bytes)>)> {
    let options = SaveOptions {
        format: Some(SaveFormat::Pgf),
        pgf_text_metrics: PgfTextMetrics::Latex,
        ..settings.0
    };
    let printed = sending_draw_event(&figure, on_draw, || {
        figure.to_pgf_document(&options, image_stem)
    });
    if let Err(SaveError::Vector(err @ VectorError::StreamedImage)) = &printed {
        return Err(PyErr::value_error(err.to_string()));
    }
    let document = raised(&figure, printed)?;
    let images = document
        .images
        .into_iter()
        .map(|image| (image.name, Bytes(image.png)))
        .collect();
    Ok((document.text, images))
}

/// The pixels of the figure printed with `settings` for a raster file:
/// straight-alpha RGBA rows from the top, as a `height × width × 4`
/// array, for the formats the program's image writer encodes
/// (`FigureCanvasAgg._print_pil`); `on_draw` is called once it is drawn.
fn print_rgba(
    figure: Figure,
    settings: PrintSettings,
    on_draw: Option<Callback>,
) -> PyResult<NdArray<u8>> {
    let canvas = sending_draw_event(&figure, on_draw, || figure.render_raster_with(settings.0))?;
    let image = canvas.into_image();
    let shape = vec![image.height(), image.width(), 4];
    NdArray::with_shape(image.into_data(), shape)
}

/// The creator pltrs (Rust) records in the documents it writes
/// (`pltrs v0.1.0`).
fn default_creator() -> PyResult<Option<String>> {
    Ok(Metadata::default().creator)
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("print_figure", print_figure)?;
    module.add("print_pgf", print_pgf)?;
    module.add("print_svg", print_svg)?;
    module.add("print_rgba", print_rgba)?;
    module.add("default_creator", default_creator)
}

#[cfg(test)]
mod tests {
    use crate::binding::testing::{eval, extension, with_python};
    use crate::binding::{Bound, IntoPy, PyResult};

    /// `_native.backends.print_figure` of a 2 × 1 inch figure at 50 dpi
    /// in `format`, with the settings dict `settings` (Python source).
    fn print<'py>(native: &Bound<'py>, format: &str, settings: &str) -> PyResult<Bound<'py>> {
        print_calling(native, format, settings, Bound::none(native.py()))
    }

    /// [`print`] calling `on_draw` once the figure is drawn.
    fn print_calling<'py>(
        native: &Bound<'py>,
        format: &str,
        settings: &str,
        on_draw: Bound<'py>,
    ) -> PyResult<Bound<'py>> {
        let py = native.py();
        let number = |v: f64| v.into_py(py).unwrap();
        let figure = native
            .getattr("figure")?
            .getattr("new")?
            .call(&[number(2.0), number(1.0), number(50.0)], &[])?;
        native.getattr("backends")?.getattr("print_figure")?.call(
            &[figure, format.into_py(py)?, eval(py, "", settings), on_draw],
            &[],
        )
    }

    #[test]
    fn the_draw_callback_runs_once_per_print_and_its_exception_raises() {
        with_python(|py| {
            let native = extension(py);
            let calls = eval(py, "", "[]");
            let on_draw = calls.getattr("append").unwrap();
            let on_draw = eval(py, "import functools", "functools.partial")
                .call(&[on_draw, 1.into_py(py).unwrap()], &[])
                .unwrap();
            for format in ["png", "pdf", "svg"] {
                print_calling(&native, format, "{}", on_draw.clone()).unwrap();
            }
            assert_eq!(calls.repr().unwrap(), "[1, 1, 1]");
            let failing = eval(py, "", "lambda: 1 / 0");
            let err = print_calling(&native, "png", "{}", failing).unwrap_err();
            assert!(err.message(py).contains("division by zero"));
        });
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
    fn metadata_is_written_where_the_format_holds_it() {
        with_python(|py| {
            let native = extension(py);
            let settings = "{'metadata': {'Title': 'Waves', 'Creator': None, \
                            'CreationDate': None}}";
            let pdf = print(&native, "pdf", settings).unwrap();
            let pdf = pdf.repr().unwrap();
            assert!(pdf.contains("/Title (Waves)") && !pdf.contains("/Creator"));
            assert!(!pdf.contains("/CreationDate"));
            let png = print(&native, "png", "{'metadata': {'Author': 'me'}}").unwrap();
            assert!(png.repr().unwrap().contains("Author"));
            let err = print(&native, "jpeg", "{'metadata': {}}").unwrap_err();
            assert!(err.message(py).contains("metadata not supported"));
            let dated = "{'metadata': {'ModDate': 1}}";
            let pdf = print(&native, "pdf", dated).unwrap().repr().unwrap();
            assert!(pdf.contains("/ModDate (D:19700101000001Z)"));
            let svg = "{'metadata': {'Creator': ['a', 'b'], 'Format': None}}";
            let svg = print(&native, "svg", svg).unwrap().repr().unwrap();
            assert!(svg.contains("<dc:title>b</dc:title>") && !svg.contains("dc:format"));
        });
    }

    #[test]
    fn the_printed_pixels_come_as_an_array() {
        use pltrs::figure::Figure;

        let figure = Figure::with_size(2.0, 1.0, 50.0);
        let settings = super::PrintSettings(pltrs::figure::SaveOptions {
            dpi: Some(10.0),
            ..pltrs::figure::SaveOptions::default()
        });
        let pixels = super::print_rgba(figure, settings, None).unwrap();
        assert_eq!(pixels.shape, [10, 20, 4]);
        assert_eq!(pixels.data[..4], [255, 255, 255, 255], "a white figure");
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
