//! PDF documents object by object (matplotlib's `backend_pdf.PdfFile`,
//! `pdfRepr` and the stream writing of `Stream`), over pltrs-render's
//! [`PdfRenderer`]: the document a `PdfPages` collects pages in, written
//! as the Python program asks.
//!
//! A PDF object arrives as `(kind, value)`, the form `pdfRepr` gives
//! Python values: `("null", None)`, `("bool", b)`, `("int", i)`,
//! `("real", x)`, `("name", s)`, `("bytes", b)` (a literal string),
//! `("text", s)` (text, UTF-16 when not ASCII), `("array", [...])`,
//! `("dict", [(key, object), ...])`, `("ref", id)`, `("seq", [...])` (a
//! box's numbers without brackets) and `("raw", b)` (operators and
//! verbatim content).

use std::path::PathBuf;
use std::sync::Arc;

use pltrs_core::{Point, Rgba};
use pltrs_font::{GlyphId, Type1Font};
use pltrs_render::{
    CapStyle, JoinStyle, PdfFont, PdfMarkerPaint, PdfObject, PdfRenderer, PdfShadingColors,
    PdfTemplatePaint, RgbaImage, VectorError, pdf_text_string,
};
use pltrs_text::tex::DviFont;

use super::super::dviread::tex_files;
use super::super::path::PyPath;
use super::super::transforms::affine;
use super::pdf_pages::PdfDocument;
use super::print::DocumentInfo;
use crate::binding::prelude::*;

/// A PDF object handed over by `pdfRepr` (see the module documentation).
struct PyPdfObject(PdfObject);

impl FromPy<'_> for PyPdfObject {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let (kind, value): (String, Bound<'_>) = object.extract()?;
        let objects = |value: &Bound<'_>| -> PyResult<Vec<PdfObject>> {
            let items: Vec<PyPdfObject> = value.extract()?;
            Ok(items.into_iter().map(|item| item.0).collect())
        };
        Ok(Self(match kind.as_str() {
            "null" => PdfObject::Null,
            "bool" => PdfObject::Bool(value.extract()?),
            "int" => PdfObject::Int(value.extract()?),
            "real" => PdfObject::Real(value.extract()?),
            "name" => PdfObject::Name(value.extract()?),
            "bytes" => PdfObject::Str(value.extract::<Bytes>()?.0),
            "text" => pdf_text_string(&value.extract::<String>()?),
            "array" => PdfObject::Array(objects(&value)?),
            "seq" => PdfObject::Seq(objects(&value)?),
            "dict" => PdfObject::Dict(entries(value.extract()?)),
            "ref" => PdfObject::Ref(value.extract()?),
            "raw" => PdfObject::Raw(value.extract::<Bytes>()?.0),
            other => {
                return Err(PyErr::type_error(format!(
                    "unknown kind of PDF object {other:?}"
                )));
            }
        }))
    }
}

/// The entries of a dictionary.
fn entries(entries: Vec<(String, PyPdfObject)>) -> Vec<(String, PdfObject)> {
    entries
        .into_iter()
        .map(|(key, value)| (key, value.0))
        .collect()
}

/// matplotlib's errors writing PDF: `ValueError` for numbers PDF cannot
/// write, `RuntimeError` otherwise.
fn pdf_error(err: VectorError) -> PyErr {
    match err {
        VectorError::NonFinite(_) => PyErr::value_error("Can only output finite numbers in PDF"),
        other => PyErr::runtime_error(other.to_string()),
    }
}

/// `pdfRepr(obj)`: the object in PDF syntax.
fn pdf_repr(object: PyPdfObject) -> PyResult<Bytes> {
    Ok(Bytes(object.0.repr().map_err(pdf_error)?))
}

/// Runs `write` on the PDF file of `document`.
fn with_file<T>(
    document: &Native<PdfDocument>,
    write: impl FnOnce(&mut PdfRenderer) -> Result<T, VectorError>,
) -> PyResult<T> {
    write(document.borrow_mut().file()?).map_err(pdf_error)
}

/// `reserveObject`: a new object number.
fn pdf_file_reserve(document: Native<PdfDocument>) -> PyResult<u32> {
    with_file(&document, |file| Ok(file.reserve_object()))
}

/// `recordXref(id)`.
fn pdf_file_record_xref(document: Native<PdfDocument>, id: u32) -> PyResult<()> {
    with_file(&document, |file| {
        file.record_xref(id);
        Ok(())
    })
}

/// `writeObject(ref, contents)`.
fn pdf_file_write_object(
    document: Native<PdfDocument>,
    id: u32,
    contents: PyPdfObject,
) -> PyResult<()> {
    with_file(&document, |file| file.write_object(id, &contents.0))
}

/// `write(data)`: into the open stream, or the file.
fn pdf_file_write(document: Native<PdfDocument>, data: Bytes) -> PyResult<()> {
    with_file(&document, |file| {
        file.write(&data.0);
        Ok(())
    })
}

/// `output(*objects)`: one line of objects.
fn pdf_file_output(document: Native<PdfDocument>, objects: Vec<PyPdfObject>) -> PyResult<()> {
    let objects: Vec<PdfObject> = objects.into_iter().map(|object| object.0).collect();
    with_file(&document, |file| file.output_objects(&objects))
}

/// The object of the open stream (`currentstream`), if any.
fn pdf_file_open_stream(document: Native<PdfDocument>) -> PyResult<Option<u32>> {
    with_file(&document, |file| Ok(file.open_stream()))
}

/// `beginStream(id, len, extra, png)`, `len` the object of the length or
/// `None` to write it inline.
fn pdf_file_begin_stream(
    document: Native<PdfDocument>,
    id: u32,
    length: Option<u32>,
    extra: Vec<(String, PyPdfObject)>,
    png: Option<PyPdfObject>,
) -> PyResult<()> {
    with_file(&document, |file| {
        file.begin_stream(id, length, entries(extra), png.map(|png| png.0))
    })
}

/// `endStream()`.
fn pdf_file_end_stream(document: Native<PdfDocument>) -> PyResult<()> {
    with_file(&document, PdfRenderer::end_stream)
}

/// `newPage(width, height)`: a page of `width × height` inches.
fn pdf_file_new_page(document: Native<PdfDocument>, width: f64, height: f64) -> PyResult<()> {
    with_file(&document, |file| file.new_page(width, height, 72.0))
}

/// `alphaState((stroke, fill))`: the name of the graphics state.
fn pdf_file_alpha_state(document: Native<PdfDocument>, alphas: (f64, f64)) -> PyResult<String> {
    with_file(&document, |file| Ok(file.alpha_state(alphas)))
}

/// A colour of three or four components.
fn color(components: Option<Vec<f64>>) -> PyResult<Option<Rgba>> {
    components
        .map(|components| match components[..] {
            [r, g, b] => Ok(Rgba::new(r, g, b, 1.0)),
            [r, g, b, a] => Ok(Rgba::new(r, g, b, a)),
            _ => Err(PyErr::value_error("a colour has 3 or 4 components")),
        })
        .transpose()
}

/// A colour's four components.
fn components(color: Option<Rgba>) -> Option<Vec<f64>> {
    color.map(|color| vec![color.r, color.g, color.b, color.a])
}

/// `hatchPattern((edge, face, hatch, linewidth))`: the pattern's name.
fn pdf_file_hatch_pattern(
    document: Native<PdfDocument>,
    (edge, face, hatch, linewidth): (Option<Vec<f64>>, Option<Vec<f64>>, String, f64),
) -> PyResult<String> {
    let (edge, face) = (color(edge)?, color(face)?);
    with_file(&document, |file| {
        Ok(file.hatch_pattern(edge, face, &hatch, linewidth))
    })
}

/// The hatch patterns made so far: `(edge, face, hatch, name)` each.
type HatchEntry = (Option<Vec<f64>>, Option<Vec<f64>>, String, String);

/// `hatchPatterns`: the hatch patterns made so far.
fn pdf_file_hatch_patterns(document: Native<PdfDocument>) -> PyResult<Vec<HatchEntry>> {
    with_file(&document, |file| {
        Ok(file
            .hatch_patterns()
            .into_iter()
            .map(|((edge, face, hatch), name)| (components(edge), components(face), hatch, name))
            .collect())
    })
}

/// `addGouraudTriangles(points, colors)`: `points` `(n, 3, 2)` and
/// `colors` `(n, 3, 1)` (grey) or `(n, 3, 4)` (RGBA); the shading's name
/// and object.
fn pdf_file_gouraud(
    document: Native<PdfDocument>,
    points: NdArray,
    colors: NdArray,
) -> PyResult<(String, u32)> {
    let triangles: Vec<[Point; 3]> = points
        .data
        .as_chunks::<6>()
        .0
        .iter()
        .map(|t| {
            [
                Point::new(t[0], t[1]),
                Point::new(t[2], t[3]),
                Point::new(t[4], t[5]),
            ]
        })
        .collect();
    let colors = match colors.shape.last() {
        Some(1) => PdfShadingColors::Gray(colors.data.as_chunks::<3>().0.to_vec()),
        Some(4) => PdfShadingColors::Rgb(
            colors
                .data
                .as_chunks::<12>()
                .0
                .iter()
                .map(|c| {
                    [
                        Rgba::new(c[0], c[1], c[2], c[3]),
                        Rgba::new(c[4], c[5], c[6], c[7]),
                        Rgba::new(c[8], c[9], c[10], c[11]),
                    ]
                })
                .collect(),
        ),
        _ => {
            return Err(PyErr::value_error(
                "Gouraud triangles have grey or RGBA colours",
            ));
        }
    };
    with_file(&document, |file| {
        Ok(file.add_gouraud_triangles(triangles, colors))
    })
}

/// `imageObject(image)`: the name of a new image of the `(M, N, 4)` byte
/// pixels `image`, bottom row first.
fn pdf_file_image(document: Native<PdfDocument>, image: NdArray<u8>) -> PyResult<String> {
    let [height, width, 4] = image.shape[..] else {
        return Err(PyErr::value_error("an image has shape (M, N, 4)"));
    };
    let rows: Vec<u8> = image
        .data
        .chunks_exact((width * 4).max(1))
        .rev()
        .flatten()
        .copied()
        .collect();
    let image =
        RgbaImage::new(width, height, rows).map_err(|err| PyErr::value_error(err.to_string()))?;
    with_file(&document, |file| Ok(file.image_object(&image)))
}

/// A join style by matplotlib's name.
fn join_style(name: &str) -> PyResult<JoinStyle> {
    JoinStyle::from_name(name)
        .ok_or_else(|| PyErr::value_error(format!("{name:?} is not a valid JoinStyle")))
}

/// A cap style by matplotlib's name.
fn cap_style(name: &str) -> PyResult<CapStyle> {
    CapStyle::from_name(name)
        .ok_or_else(|| PyErr::value_error(format!("{name:?} is not a valid CapStyle")))
}

/// `markerObject(path, trans, fill, stroke, lw, joinstyle, capstyle)`:
/// the marker form's name; `trans` a 3×3 matrix.
#[expect(
    clippy::too_many_arguments,
    reason = "matplotlib's signature of markerObject"
)]
fn pdf_file_marker(
    document: Native<PdfDocument>,
    path: PyPath,
    trans: NdArray,
    fill: bool,
    stroke: bool,
    linewidth: f64,
    join: String,
    cap: String,
) -> PyResult<String> {
    let paint = PdfMarkerPaint {
        fill,
        stroke,
        join: join_style(&join)?,
        cap: cap_style(&cap)?,
        linewidth,
    };
    let transform = affine(&trans)?;
    with_file(&document, |file| {
        Ok(file.marker_object(&path.path, transform, paint))
    })
}

/// `pathCollectionObject(gc, path, trans, padding, filled, stroked)` with
/// the join and cap styles of `gc`: the template's name.
#[expect(
    clippy::too_many_arguments,
    reason = "matplotlib's signature of pathCollectionObject"
)]
fn pdf_file_path_collection(
    document: Native<PdfDocument>,
    path: PyPath,
    trans: NdArray,
    padding: f64,
    filled: bool,
    stroked: bool,
    join: String,
    cap: String,
) -> PyResult<String> {
    let paint = PdfTemplatePaint {
        join: join_style(&join)?,
        cap: cap_style(&cap)?,
        padding,
        filled,
        stroked,
    };
    let transform = affine(&trans)?;
    with_file(&document, |file| {
        Ok(file.path_collection_object(&path.path, transform, paint))
    })
}

/// `writeExtGSTates()`.
fn pdf_file_write_ext_gstates(document: Native<PdfDocument>) -> PyResult<()> {
    with_file(&document, PdfRenderer::write_ext_gstates)
}

/// `writeHatches()`.
fn pdf_file_write_hatches(document: Native<PdfDocument>) -> PyResult<()> {
    with_file(&document, PdfRenderer::write_hatches)
}

/// `writeGouraudTriangles()`.
fn pdf_file_write_gouraud(document: Native<PdfDocument>) -> PyResult<()> {
    with_file(&document, PdfRenderer::write_gouraud_triangles)
}

/// `writeImages()`.
fn pdf_file_write_images(document: Native<PdfDocument>) -> PyResult<()> {
    with_file(&document, PdfRenderer::write_images)
}

/// `writeMarkers()`.
fn pdf_file_write_markers(document: Native<PdfDocument>) -> PyResult<()> {
    with_file(&document, PdfRenderer::write_markers)
}

/// `writePathCollectionTemplates()`.
fn pdf_file_write_templates(document: Native<PdfDocument>) -> PyResult<()> {
    with_file(&document, PdfRenderer::write_path_collection_templates)
}

/// `writeFonts()`.
fn pdf_file_write_fonts(document: Native<PdfDocument>) -> PyResult<()> {
    with_file(&document, PdfRenderer::write_fonts)
}

/// `writeInfoDict()`: the object of the information dictionary.
fn pdf_file_write_info_dict(document: Native<PdfDocument>) -> PyResult<u32> {
    with_file(&document, PdfRenderer::write_info_dict)
}

/// The first object reserved but not written, if any.
fn pdf_file_unwritten(document: Native<PdfDocument>) -> PyResult<Option<usize>> {
    with_file(&document, |file| Ok(file.unwritten_object()))
}

/// `writeXref()`.
fn pdf_file_write_xref(document: Native<PdfDocument>) -> PyResult<()> {
    with_file(&document, PdfRenderer::write_xref)
}

/// `writeTrailer()`.
fn pdf_file_write_trailer(document: Native<PdfDocument>) -> PyResult<()> {
    with_file(&document, PdfRenderer::write_trailer)
}

/// `finalize()`.
fn pdf_file_finalize(document: Native<PdfDocument>) -> PyResult<()> {
    with_file(&document, PdfRenderer::finalize)
}

/// `close()`: the file written, the document closed.
fn pdf_file_close(document: Native<PdfDocument>) -> PyResult<Bytes> {
    let file = document.borrow_mut().take()?;
    Ok(Bytes(file.close().map_err(pdf_error)?))
}

/// One entry of the document information: its key, and its text or its
/// date in seconds since the epoch.
type InfoEntry = (String, Option<String>, Option<i64>);

/// The document information the file will write, in matplotlib's order.
fn pdf_file_info(document: Native<PdfDocument>) -> PyResult<Vec<InfoEntry>> {
    with_file(&document, |file| {
        let info = file.metadata_mut();
        let text = |key: &str, value: &Option<String>| {
            value
                .clone()
                .map(|value| (key.to_owned(), Some(value), None))
        };
        let date = |key: &str, value: Option<pltrs_render::Timestamp>| {
            value.map(|date| (key.to_owned(), None, Some(date.0)))
        };
        Ok([
            text("Creator", &info.creator),
            text("Producer", &info.producer),
            date("CreationDate", info.date),
            text("Title", &info.title),
            text("Author", &info.author),
            text("Subject", &info.subject),
            text("Keywords", &info.keywords),
            date("ModDate", info.modified),
        ]
        .into_iter()
        .flatten()
        .collect())
    })
}

/// Sets the document information to the entries of the dict `info` (as
/// `savefig(metadata=...)` gives them, without pltrs' defaults).
fn pdf_file_set_info(document: Native<PdfDocument>, info: DocumentInfo) -> PyResult<()> {
    with_file(&document, |file| {
        *file.metadata_mut() = info.0;
        Ok(())
    })
}

/// `fontName` for face `index` of the TrueType file `path`: the name of
/// subset `subset`.
fn pdf_file_font_name(
    document: Native<PdfDocument>,
    path: PathBuf,
    index: u32,
    subset: usize,
) -> PyResult<String> {
    with_file(&document, |file| file.font_name(&path, index, subset))
}

/// `fontName` for the PDF core font whose metrics file is named `file`.
fn pdf_file_core_font_name(document: Native<PdfDocument>, name: String) -> PyResult<String> {
    with_file(&document, |file| file.core_font_name(&name))
}

/// One font named: its file (a bare name for a bundled font), face,
/// subset, whether it is a core font, and its name.
type FontNameEntry = (String, u32, usize, bool, String);

/// `fontNames`: the fonts named so far.
fn pdf_file_font_names(document: Native<PdfDocument>) -> PyResult<Vec<FontNameEntry>> {
    with_file(&document, |file| {
        Ok(file
            .font_names()
            .into_iter()
            .map(|(font, name)| match font {
                PdfFont::TrueType {
                    file,
                    index,
                    subset,
                } => (file.display().to_string(), index, subset, false, name),
                PdfFont::Core(file) => (file.to_owned(), 0, 0, true, name),
            })
            .collect())
    })
}

/// `dviFontName(dvifont)`: the name the TeX font is registered under.
fn pdf_file_dvi_font_name(document: Native<PdfDocument>, font: Arc<DviFont>) -> PyResult<String> {
    let files = tex_files()?;
    with_file(&document, |file| Ok(file.dvi_font_name(font, files)))
}

/// `embedTTF(filename, subset_index, charmap)` for face `index` of the
/// file `path`, `charmap` as `(code, glyph index)` pairs: the font
/// dictionary's object.
fn pdf_file_embed_ttf(
    document: Native<PdfDocument>,
    path: PathBuf,
    index: u32,
    subset: usize,
    charmap: Vec<(u32, u16)>,
) -> PyResult<u32> {
    let charmap = charmap
        .into_iter()
        .map(|(code, glyph)| (code, GlyphId(glyph)))
        .collect();
    with_file(&document, |file| {
        file.embed_ttf(&path, index, subset, charmap)
    })
}

/// Whether glyphs of face `index` of the font file `path` were drawn in
/// the document (what matplotlib's character tracker holds).
fn pdf_file_tracks_font(
    document: Native<PdfDocument>,
    path: PathBuf,
    index: u32,
) -> PyResult<bool> {
    with_file(&document, |file| file.tracks_font(&path, index))
}

/// `createType1Descriptor(t1font)`: the descriptor's object.
fn pdf_file_type1_descriptor(
    document: Native<PdfDocument>,
    font: Native<Type1Font>,
) -> PyResult<u32> {
    let font = font.borrow();
    with_file(&document, |file| file.create_type1_descriptor(&font))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("pdf_repr", pdf_repr)?;
    module.add("pdf_file_reserve", pdf_file_reserve)?;
    module.add("pdf_file_record_xref", pdf_file_record_xref)?;
    module.add("pdf_file_write_object", pdf_file_write_object)?;
    module.add("pdf_file_write", pdf_file_write)?;
    module.add("pdf_file_output", pdf_file_output)?;
    module.add("pdf_file_open_stream", pdf_file_open_stream)?;
    module.add("pdf_file_begin_stream", pdf_file_begin_stream)?;
    module.add("pdf_file_end_stream", pdf_file_end_stream)?;
    module.add("pdf_file_new_page", pdf_file_new_page)?;
    module.add("pdf_file_alpha_state", pdf_file_alpha_state)?;
    module.add("pdf_file_hatch_pattern", pdf_file_hatch_pattern)?;
    module.add("pdf_file_hatch_patterns", pdf_file_hatch_patterns)?;
    module.add("pdf_file_gouraud", pdf_file_gouraud)?;
    module.add("pdf_file_image", pdf_file_image)?;
    module.add("pdf_file_marker", pdf_file_marker)?;
    module.add("pdf_file_path_collection", pdf_file_path_collection)?;
    module.add("pdf_file_write_ext_gstates", pdf_file_write_ext_gstates)?;
    module.add("pdf_file_write_hatches", pdf_file_write_hatches)?;
    module.add("pdf_file_write_gouraud", pdf_file_write_gouraud)?;
    module.add("pdf_file_write_images", pdf_file_write_images)?;
    module.add("pdf_file_write_markers", pdf_file_write_markers)?;
    module.add("pdf_file_write_templates", pdf_file_write_templates)?;
    module.add("pdf_file_write_fonts", pdf_file_write_fonts)?;
    module.add("pdf_file_write_info_dict", pdf_file_write_info_dict)?;
    module.add("pdf_file_unwritten", pdf_file_unwritten)?;
    module.add("pdf_file_write_xref", pdf_file_write_xref)?;
    module.add("pdf_file_write_trailer", pdf_file_write_trailer)?;
    module.add("pdf_file_finalize", pdf_file_finalize)?;
    module.add("pdf_file_close", pdf_file_close)?;
    module.add("pdf_file_info", pdf_file_info)?;
    module.add("pdf_file_set_info", pdf_file_set_info)?;
    module.add("pdf_file_font_name", pdf_file_font_name)?;
    module.add("pdf_file_core_font_name", pdf_file_core_font_name)?;
    module.add("pdf_file_font_names", pdf_file_font_names)?;
    module.add("pdf_file_dvi_font_name", pdf_file_dvi_font_name)?;
    module.add("pdf_file_embed_ttf", pdf_file_embed_ttf)?;
    module.add("pdf_file_tracks_font", pdf_file_tracks_font)?;
    module.add("pdf_file_type1_descriptor", pdf_file_type1_descriptor)
}
