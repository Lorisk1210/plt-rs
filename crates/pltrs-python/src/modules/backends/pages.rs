//! The documents of the vector renderers a program makes around a file of
//! its own, outside any save (matplotlib's `RendererSVG(width, height,
//! svgwriter)`, `RendererPS(width, height, pswriter)`,
//! `RendererPgf(figure, fh)` and `RendererPdf(pdffile, ...)`): what the
//! renderer draws goes into its page as pltrs (Rust) draws documents.
//!
//! A PostScript or PGF page hands back what each call wrote, which the
//! renderer writes to its file as matplotlib's does; an SVG page gives its
//! document when it is finished (`finalize`); a PDF renderer draws on the
//! current page of its `PdfFile`, whose graphics state `check_gc` and
//! `finalize` change.

use pltrs::artist::Drawing;
use pltrs::figure::{DrawErrors, pgf_page, ps_page, svg_page};
use pltrs_core::Rgba;
use pltrs_render::{CapStyle, JoinStyle, PgfRenderer, PsRenderer, SvgRenderer};

use super::gc::PyGc;
use super::pdf_pages::PdfDocument;
use super::print::DocumentMetadata;
use crate::binding::prelude::*;

/// The document of a program's vector renderer.
pub(super) enum Page {
    /// An SVG document, until it is finished.
    Svg(Option<SvgRenderer>),
    /// A PostScript page.
    Ps(PsRenderer),
    /// A PGF picture.
    Pgf(PgfRenderer),
}

impl IntoPy for Page {
    /// A native object holding the page.
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        crate::binding::wrap_native(py, self)
    }
}

/// What a call wrote: the text for the renderer's file, and the images
/// to write beside it as `(name, png)` (those of a PGF picture).
type Written = (String, Vec<(String, Bytes)>);

/// Python's error for a page the renderer cannot draw on.
fn finished() -> PyErr {
    PyErr::value_error("the SVG document is finalized")
}

/// The problems met drawing, raised as a draw raises them.
fn drawn(problems: Vec<pltrs::artist::DrawError>) -> PyResult<()> {
    if problems.is_empty() {
        Ok(())
    } else {
        Err(DrawErrors(problems).into())
    }
}

/// The page of an SVG document of `width` × `height` points with
/// `metadata`, its images at `image_dpi` (`RendererSVG`).
fn svg_page_new(
    width: f64,
    height: f64,
    image_dpi: f64,
    metadata: Option<DocumentMetadata>,
) -> PyResult<Page> {
    let metadata = metadata.map(|metadata| metadata.0).unwrap_or_default();
    svg_page(width, height, image_dpi, metadata)
        .map(|page| Page::Svg(Some(page)))
        .map_err(|err| PyErr::value_error(err.to_string()))
}

/// The PostScript page of `width` × `height` inches, its images at
/// `image_dpi` (`RendererPS`).
fn ps_page_new(width: f64, height: f64, image_dpi: f64) -> PyResult<Page> {
    ps_page(width, height, image_dpi)
        .map(Page::Ps)
        .map_err(|err| PyErr::value_error(err.to_string()))
}

/// The PGF picture of `width` × `height` inches at `dpi`, its images
/// named after `image_stem` (`RendererPgf`).
fn pgf_page_new(width: f64, height: f64, dpi: f64, image_stem: Option<String>) -> PyResult<Page> {
    pgf_page(width, height, dpi, image_stem)
        .map(Page::Pgf)
        .map_err(|err| PyErr::value_error(err.to_string()))
}

/// Draws `drawing` onto `page`; what to write to the renderer's file.
fn page_draw(page: Native<Page>, drawing: Native<Drawing>) -> PyResult<Written> {
    let drawing = drawing.borrow();
    match &mut *page.borrow_mut() {
        Page::Svg(svg) => {
            drawn(drawing.draw_onto(svg.as_mut().ok_or_else(finished)?))?;
            Ok((String::new(), Vec::new()))
        }
        Page::Ps(ps) => {
            drawn(drawing.draw_onto(ps))?;
            Ok((ps.take_body(), Vec::new()))
        }
        Page::Pgf(pgf) => {
            drawn(drawing.draw_onto(pgf))?;
            for family in pgf.take_ignored_fonts() {
                pltrs::figure::log_unknown_font(&family);
            }
            let (text, images) = pgf.take_body();
            let images = images
                .into_iter()
                .map(|image| (image.name, Bytes(image.png)))
                .collect();
            Ok((text, images))
        }
    }
}

/// The SVG document of `page`, finished (`RendererSVG.finalize`).
fn page_finish(page: Native<Page>) -> PyResult<String> {
    let Page::Svg(svg) = &mut *page.borrow_mut() else {
        return Err(PyErr::type_error("only SVG documents are finalized"));
    };
    svg.take()
        .ok_or_else(finished)?
        .finish()
        .map_err(|err| PyErr::runtime_error(err.to_string()))
}

/// Runs `change` on the PostScript page `page`; what it wrote.
fn on_ps(page: &Native<Page>, change: impl FnOnce(&mut PsRenderer)) -> PyResult<String> {
    let Page::Ps(ps) = &mut *page.borrow_mut() else {
        return Err(PyErr::type_error("not a PostScript page"));
    };
    change(ps);
    Ok(ps.take_body())
}

/// The join style named `name` (or by matplotlib's codes 0 to 2).
fn join(name: &Bound<'_>) -> PyResult<JoinStyle> {
    let named = match name.extract::<i64>() {
        Ok(0) => "miter".to_owned(),
        Ok(1) => "round".to_owned(),
        Ok(2) => "bevel".to_owned(),
        _ => name.extract::<String>()?,
    };
    JoinStyle::from_name(&named)
        .ok_or_else(|| PyErr::new(ExceptionKind::KeyError, format!("{named:?}")))
}

/// The cap style named `name` (or by matplotlib's codes 0 to 2).
fn cap(name: &Bound<'_>) -> PyResult<CapStyle> {
    let named = match name.extract::<i64>() {
        Ok(0) => "butt".to_owned(),
        Ok(1) => "round".to_owned(),
        Ok(2) => "projecting".to_owned(),
        _ => name.extract::<String>()?,
    };
    CapStyle::from_name(&named)
        .ok_or_else(|| PyErr::new(ExceptionKind::KeyError, format!("{named:?}")))
}

/// `set_color(r, g, b, store)`; what it wrote.
fn ps_set_color(page: Native<Page>, rgb: (f64, f64, f64), store: bool) -> PyResult<String> {
    let (r, g, b) = rgb;
    on_ps(&page, |ps| ps.set_color([r, g, b], store))
}

/// `set_linewidth(linewidth, store)`; what it wrote.
fn ps_set_linewidth(page: Native<Page>, width: f64, store: bool) -> PyResult<String> {
    on_ps(&page, |ps| ps.set_linewidth(width, store))
}

/// `set_linejoin(linejoin, store)`; what it wrote.
fn ps_set_linejoin(page: Native<Page>, linejoin: Owned, store: bool) -> PyResult<String> {
    let join = Python::with_gil(|py| join(&linejoin.bind(py)))?;
    on_ps(&page, |ps| ps.set_linejoin(join, store))
}

/// `set_linecap(linecap, store)`; what it wrote.
fn ps_set_linecap(page: Native<Page>, linecap: Owned, store: bool) -> PyResult<String> {
    let cap = Python::with_gil(|py| cap(&linecap.bind(py)))?;
    on_ps(&page, |ps| ps.set_linecap(cap, store))
}

/// `set_linedash(offset, seq, store)`; what it wrote.
fn ps_set_linedash(
    page: Native<Page>,
    offset: f64,
    seq: Option<Vec<f64>>,
    store: bool,
) -> PyResult<String> {
    on_ps(&page, |ps| ps.set_linedash(offset, seq.as_deref(), store))
}

/// `set_font(fontname, fontsize, store)`; what it wrote.
fn ps_set_font(page: Native<Page>, name: String, size: f64, store: bool) -> PyResult<String> {
    on_ps(&page, |ps| ps.set_font(&name, size, store))
}

/// `create_hatch(hatch, linewidth)`: the pattern's name, and what
/// defining it wrote.
fn ps_create_hatch(
    page: Native<Page>,
    hatch: String,
    linewidth: f64,
) -> PyResult<(String, String)> {
    let mut name = String::new();
    let written = on_ps(&page, |ps| name = ps.create_hatch(&hatch, linewidth))?;
    Ok((name, written))
}

/// Draws `drawing` on the current page of the PDF document `document`
/// (the drawing methods of a `RendererPdf` around a program's `PdfFile`).
fn pdf_file_draw(document: Native<PdfDocument>, drawing: Native<Drawing>) -> PyResult<()> {
    let problems = drawing.borrow().draw_onto(document.borrow_mut().file()?);
    drawn(problems)
}

/// `check_gc(gc, fillcolor)`: the graphics state of the current page of
/// `document` changed into the one `gc` and the fill need.
fn pdf_file_check_gc(
    document: Native<PdfDocument>,
    gc: PyGc,
    fill: Option<(f64, f64, f64, f64)>,
) -> PyResult<()> {
    let fill = fill.map(|(r, g, b, a)| Rgba::new(r, g, b, a));
    document.borrow_mut().file()?.check_gc(&gc.0, fill);
    Ok(())
}

/// `RendererPdf.finalize`: every graphics state the current page of
/// `document` saved, restored.
fn pdf_file_restore_gc(document: Native<PdfDocument>) -> PyResult<()> {
    document
        .borrow_mut()
        .file()?
        .restore_gc()
        .map_err(|err| PyErr::runtime_error(err.to_string()))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("svg_page_new", svg_page_new)?;
    module.add("ps_page_new", ps_page_new)?;
    module.add("pgf_page_new", pgf_page_new)?;
    module.add("page_draw", page_draw)?;
    module.add("page_finish", page_finish)?;
    module.add("ps_set_color", ps_set_color)?;
    module.add("ps_set_linewidth", ps_set_linewidth)?;
    module.add("ps_set_linejoin", ps_set_linejoin)?;
    module.add("ps_set_linecap", ps_set_linecap)?;
    module.add("ps_set_linedash", ps_set_linedash)?;
    module.add("ps_set_font", ps_set_font)?;
    module.add("ps_create_hatch", ps_create_hatch)?;
    module.add("pdf_file_draw", pdf_file_draw)?;
    module.add("pdf_file_check_gc", pdf_file_check_gc)?;
    module.add("pdf_file_restore_gc", pdf_file_restore_gc)
}
