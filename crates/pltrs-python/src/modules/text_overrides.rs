//! `_native.text_overrides`: matplotlib's measurement of text
//! (`text._get_text_metrics_with_cache`, through the metrics cache of each
//! renderer, `pltrs::text::MetricsCache`), and the program's replacements
//! of it and of `Text.draw`, handed to pltrs (Rust) so that it measures
//! and draws every text through them (`pltrs::text::set_line_measurer`,
//! `pltrs::text::set_text_painter`).
//!
//! The package installs, once, the stand-ins measuring and drawing
//! through the program's replacements, each with the attribute the
//! program replaces ([`Watched`]): pltrs (Rust) uses a stand-in only while
//! the attribute is not matplotlib's (`monkeypatch.setattr(text,
//! "_get_text_metrics_with_cache", ...)` until the original comes back),
//! so that it only calls Python while there is a replacement. A
//! renderer's metrics cache is installed while pltrs lays out or draws
//! with the renderer (`set_metrics_cache`), which costs no call into
//! Python per text.

use std::rc::Rc;

use pltrs::artist::RawHandle;
use pltrs::fonts::FontProperties;
use pltrs::text::{
    FontHeightMetrics, LineKind, LineMeasurer, LineMetrics, METRICS_CACHE_SIZE, MetricsCache,
    RendererMetrics, TexError, Text, TextError, TextPainter,
    set_metrics_cache as install_metrics_cache,
};

use super::font_manager::FontProps;
use crate::binding::Watched;
use crate::binding::prelude::*;

/// A line's typesetting as matplotlib's `ismath` gives it: `False`,
/// `True` or `"TeX"`.
pub(super) struct IsMath(pub(super) LineKind);

impl FromPy<'_> for IsMath {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        if let Ok(name) = object.extract::<String>() {
            return match name.as_str() {
                "TeX" => Ok(Self(LineKind::Tex)),
                _ => Err(PyErr::value_error(format!(
                    "ismath must be False, True or 'TeX', not {name:?}"
                ))),
            };
        }
        Ok(Self(if object.extract::<bool>()? {
            LineKind::Math
        } else {
            LineKind::Plain
        }))
    }
}

impl IntoPy for IsMath {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        match self.0 {
            LineKind::Plain => false.into_py(py),
            LineKind::Math => true.into_py(py),
            LineKind::Tex => "TeX".into_py(py),
        }
    }
}

/// matplotlib's `get_text_width_height_descent(text, prop, ismath)` at
/// `dpi`, as pltrs (Rust) measures it: `(width, height, descent)` in
/// pixels.
fn measure(text: String, props: FontProps, ismath: IsMath, dpi: f64) -> PyResult<(f64, f64, f64)> {
    let metrics = pltrs::text::measure_line(&text, &props.0, ismath.0, dpi)?;
    Ok((metrics.width, metrics.height, metrics.descent))
}

/// `RendererPdf.get_text_width_height_descent(text, prop, ismath)` at
/// `dpi`: the line measured as the PDF backend measures it.
fn measure_pdf(
    text: String,
    props: FontProps,
    ismath: IsMath,
    dpi: f64,
) -> PyResult<(f64, f64, f64)> {
    let metrics = pltrs::text::measure_line_pdf(&text, &props.0, ismath.0, dpi)?;
    Ok((metrics.width, metrics.height, metrics.descent))
}

/// `RendererPS.get_text_width_height_descent(text, prop, ismath)` at
/// `dpi`: the line measured as the PostScript backend measures it.
fn measure_ps(
    text: String,
    props: FontProps,
    ismath: IsMath,
    dpi: f64,
) -> PyResult<(f64, f64, f64)> {
    let metrics = pltrs::text::measure_line_ps(&text, &props.0, ismath.0, dpi)?;
    Ok((metrics.width, metrics.height, metrics.descent))
}

/// `RendererSVG.get_text_width_height_descent(text, prop, ismath)` at
/// `dpi`: the line measured as the SVG backend measures it.
fn measure_svg(
    text: String,
    props: FontProps,
    ismath: IsMath,
    dpi: f64,
) -> PyResult<(f64, f64, f64)> {
    let metrics = pltrs::text::measure_line_svg(&text, &props.0, ismath.0, dpi)?;
    Ok((metrics.width, metrics.height, metrics.descent))
}

/// `RendererPgf.get_text_width_height_descent(text, prop, ismath)` at
/// `dpi`: the line typeset by the TeX system of `pgf.texsystem`, as the
/// PGF backend measures it (whatever `ismath`). A text TeX cannot typeset
/// raises matplotlib's `ValueError`, a TeX system that cannot be started
/// or stops on the preamble its `RuntimeError`.
fn measure_pgf(text: String, props: FontProps, dpi: f64) -> PyResult<(f64, f64, f64)> {
    match pltrs::figure::measure_pgf_text(&text, &props.0, dpi) {
        Ok(metrics) => Ok((metrics.width, metrics.height, metrics.descent)),
        Err(err @ TextError::Tex(TexError::Measuring { .. })) => Err(err.into()),
        Err(err) => Err(PyErr::runtime_error(err.to_string())),
    }
}

/// Lines measured by Python: `line(text, props, ismath, dpi)` gives
/// `(width, height, descent)` and `font_height(props, dpi)` the font's
/// `(ascent, descent, line_gap)` or None, while `original` is replaced.
struct PythonMeasurer {
    line: Callback,
    font_height: Callback,
    original: Watched,
}

impl LineMeasurer for PythonMeasurer {
    fn measure(
        &self,
        line: &str,
        font: &FontProperties,
        kind: LineKind,
        dpi: f64,
    ) -> Option<LineMetrics> {
        let (width, height, descent) = self.line.call::<(f64, f64, f64)>((
            line.to_owned(),
            FontProps(font.clone()),
            IsMath(kind),
            dpi,
        ))?;
        Some(LineMetrics {
            width,
            height,
            descent,
        })
    }

    fn font_height(&self, font: &FontProperties, dpi: f64) -> Option<FontHeightMetrics> {
        let (ascent, descent, line_gap) = self
            .font_height
            .call::<Option<(f64, f64, f64)>>((FontProps(font.clone()), dpi))??;
        Some(FontHeightMetrics {
            ascent,
            descent,
            line_gap,
        })
    }

    fn in_force(&self) -> bool {
        !self.original.holds()
    }
}

/// Measures every line pltrs (Rust) lays out on this thread with `line`
/// and `font_height` (see [`PythonMeasurer`]) while `original`,
/// `(module, name, function)`, is replaced, or with pltrs (Rust)'s own
/// measurement always when all are None.
fn set_measurer(
    line: Option<Callback>,
    font_height: Option<Callback>,
    original: Option<Watched>,
) -> PyResult<()> {
    let measurer = match (line, font_height, original) {
        (Some(line), Some(font_height), Some(original)) => Some(Rc::new(PythonMeasurer {
            line,
            font_height,
            original,
        }) as Rc<dyn LineMeasurer>),
        (None, None, None) => None,
        _ => {
            return Err(PyErr::type_error(
                "set_measurer takes both callables and the original, or none",
            ));
        }
    };
    pltrs::text::set_line_measurer(measurer);
    Ok(())
}

/// Texts drawn by Python: `paint(stand_in, dpi)` gives the native handles
/// of the artists to draw in place of the text `stand_in` stands for,
/// while `original` is replaced.
struct PythonPainter {
    paint: Callback,
    original: Watched,
}

impl TextPainter for PythonPainter {
    fn paint(&self, stand_in: &Text, dpi: f64) -> Vec<RawHandle> {
        self.paint
            .call::<Vec<RawHandle>>((stand_in.clone(), dpi))
            .unwrap_or_default()
    }

    fn in_force(&self) -> bool {
        !self.original.holds()
    }
}

/// Draws every text pltrs (Rust) draws on this thread with `paint` (see
/// [`PythonPainter`]) while `original`, `(class, name, function)`, is
/// replaced, or as pltrs (Rust) draws it always when both are None.
fn set_painter(paint: Option<Callback>, original: Option<Watched>) -> PyResult<()> {
    let painter = match (paint, original) {
        (Some(paint), Some(original)) => {
            Some(Rc::new(PythonPainter { paint, original }) as Rc<dyn TextPainter>)
        }
        (None, None) => None,
        _ => {
            return Err(PyErr::type_error(
                "set_painter takes the callable and the original, or neither",
            ));
        }
    };
    pltrs::text::set_text_painter(painter);
    Ok(())
}

native_value!(Rc<MetricsCache>);

/// A new, empty metrics cache for a renderer measuring as `metrics`
/// says: `"raster"` (Agg's), `"pdf"`, `"ps"` or `"svg"` (a vector
/// backend's renderer, see [`RendererMetrics`]).
fn new_metrics_cache(metrics: String) -> PyResult<Rc<MetricsCache>> {
    let metrics = match metrics.as_str() {
        "raster" => RendererMetrics::Raster,
        "pdf" => RendererMetrics::Pdf,
        "ps" => RendererMetrics::PostScript,
        "svg" => RendererMetrics::Svg,
        other => {
            return Err(PyErr::value_error(format!(
                "no renderer measures text as {other:?}"
            )));
        }
    };
    Ok(Rc::new(MetricsCache::measuring_as(metrics)))
}

/// Calling a renderer's metrics cache with `(text, prop, ismath, dpi)`:
/// the cached `(width, height, descent)`, else those `measure()` gives
/// (the renderer's `get_text_width_height_descent`), cached.
fn metrics_cache_call(
    cache: Rc<MetricsCache>,
    text: String,
    props: FontProps,
    ismath: IsMath,
    dpi: f64,
    measure: Callback,
) -> PyResult<(f64, f64, f64)> {
    let metrics = cache.get_or_measure(&text, &props.0, ismath.0, dpi, || {
        // A failing measurement raises its own exception when this call
        // returns; the error only stops the caching.
        let (width, height, descent) = measure
            .call::<(f64, f64, f64)>(())
            .ok_or_else(|| TextError::Font("the renderer could not measure the text".into()))?;
        Ok(LineMetrics {
            width,
            height,
            descent,
        })
    })?;
    Ok((metrics.width, metrics.height, metrics.descent))
}

/// A metrics cache's `cache_info()`: `(hits, misses, maxsize, currsize)`.
fn metrics_cache_info(cache: Rc<MetricsCache>) -> PyResult<(u64, u64, usize, usize)> {
    let info = cache.info();
    Ok((info.hits, info.misses, METRICS_CACHE_SIZE, info.size))
}

/// A metrics cache's `cache_clear()`.
fn metrics_cache_clear(cache: Rc<MetricsCache>) -> PyResult<()> {
    cache.clear();
    Ok(())
}

/// Installs `cache` (or none) as the cache of the lines pltrs measures on
/// this thread; returns the cache installed before.
fn set_metrics_cache(cache: Option<Rc<MetricsCache>>) -> PyResult<Option<Rc<MetricsCache>>> {
    Ok(install_metrics_cache(cache))
}

/// Has what matplotlib measures with the renderer it is handed (every
/// line of text, the pads its code converts with the renderer) measured at
/// `dpi` (`None`: at the figure's resolution), as a renderer of that
/// resolution measures it (`pltrs::artist::set_measured_at`); returns the
/// resolution set before.
fn set_measured_at(dpi: Option<f64>) -> PyResult<Option<f64>> {
    Ok(pltrs::artist::set_measured_at(dpi))
}

/// Forgets the lines, formulas and glyph outlines pltrs (Rust) laid out
/// or made for any figure so far, keeping the fonts loaded
/// (`pltrs::text::clear_text_caches`): what the benchmark harness does
/// between runs, as it clears matplotlib's mathtext cache.
fn text_engine_clear_caches() -> PyResult<()> {
    pltrs::text::clear_text_caches();
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("new_metrics_cache", new_metrics_cache)?;
    module.add("metrics_cache_call", metrics_cache_call)?;
    module.add("metrics_cache_info", metrics_cache_info)?;
    module.add("metrics_cache_clear", metrics_cache_clear)?;
    module.add("set_metrics_cache", set_metrics_cache)?;
    module.add("set_measured_at", set_measured_at)?;
    module.add("measure", measure)?;
    module.add("measure_pdf", measure_pdf)?;
    module.add("measure_ps", measure_ps)?;
    module.add("measure_svg", measure_svg)?;
    module.add("measure_pgf", measure_pgf)?;
    module.add("set_measurer", set_measurer)?;
    module.add("set_painter", set_painter)?;
    module.add("text_engine_clear_caches", text_engine_clear_caches)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::binding::testing::{eval, extension, import, with_python};
    use crate::binding::{Bound, PyResult};
    use pltrs::artist::ArtistHandle;

    /// Calls `_native.<module>.<name>(*args)`.
    fn call<'py>(
        native: &Bound<'py>,
        module: &str,
        name: &str,
        args: &[Bound<'py>],
    ) -> PyResult<Bound<'py>> {
        native.getattr(module)?.getattr(name)?.call(args, &[])
    }

    /// The width of the extent of a text "abcd" in a figure at 100 dpi.
    fn text_width(native: &Bound<'_>) -> f64 {
        let py = native.py();
        let size = ["6.4", "4.8", "100.0"].map(|v| eval(py, "", v));
        let figure = call(native, "figure", "new", &size).unwrap();
        let position = ["0.5", "0.5", "'abcd'"].map(|v| eval(py, "", v));
        let [x, y, s] = position;
        let text: pltrs::text::Text = call(native, "text", "figure_text", &[figure, x, y, s])
            .unwrap()
            .extract()
            .unwrap();
        text.raw().extent_as_is().expect("measured").width()
    }

    #[test]
    fn lines_are_measured_by_the_measurer_while_it_is_set() {
        with_python(|py| {
            let native = extension(py);
            let props = eval(
                py,
                "",
                "(['DejaVu Sans'], ('normal', 'normal'), ('normal', 'normal', 10.0), \
                 ('dejavusans', None))",
            );
            let measure = |ismath: &str| {
                let args = [
                    eval(py, "", "'lp'"),
                    props.clone(),
                    eval(py, "", ismath),
                    eval(py, "", "100.0"),
                ];
                call(&native, "text_overrides", "measure", &args)
                    .and_then(|m| m.extract::<(f64, f64, f64)>())
            };
            let own = measure("False").unwrap();
            assert!(own.0 > 0.0 && own.2 > 0.0, "{own:?}");
            assert!(measure("'math'").is_err());
            let measured = text_width(&native);
            let line = eval(
                py,
                "",
                "lambda s, prop, ismath, dpi: (10.0 * len(s), prop[2][2], 0.0)",
            );
            let tables = eval(py, "", "lambda prop, dpi: None");
            // The module whose `measure` is watched: matplotlib's `len`
            // until the program replaces it.
            let original = eval(
                py,
                "import types\nmodule = types.SimpleNamespace(measure=len)",
                "(module, 'measure', len)",
            );
            let module = original
                .call_method("__getitem__", &[eval(py, "", "0")])
                .unwrap();
            let replace = |value: &str| {
                let setattr = import(py, "builtins").getattr("setattr").unwrap();
                let args = [
                    module.clone(),
                    eval(py, "", "'measure'"),
                    eval(py, "", value),
                ];
                setattr.call(&args, &[]).unwrap();
            };
            call(
                &native,
                "text_overrides",
                "set_measurer",
                &[line, tables.clone(), original],
            )
            .unwrap();
            let unreplaced = text_width(&native);
            replace("abs");
            let replaced = text_width(&native);
            let again = measure("False");
            replace("len");
            let restored = text_width(&native);
            let none = eval(py, "", "None");
            let half = call(
                &native,
                "text_overrides",
                "set_measurer",
                &[tables, none.clone(), none.clone()],
            );
            call(
                &native,
                "text_overrides",
                "set_measurer",
                &[none.clone(), none.clone(), none],
            )
            .unwrap();
            assert_eq!(unreplaced, measured, "in force while replaced only");
            assert_eq!(replaced, 40.0);
            assert_eq!(again.unwrap(), own, "the original ignores the measurer");
            assert_eq!(restored, measured);
            assert!(half.is_err(), "all or none");
            assert_eq!(text_width(&native), measured);
        });
    }
}
