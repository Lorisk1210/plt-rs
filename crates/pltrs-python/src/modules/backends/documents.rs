//! The helpers of matplotlib's PostScript and SVG backends that work on
//! documents rather than figures: `pstoeps`, `gs_distill` and
//! `xpdf_distill` rewriting a PostScript file in place, and the streaming
//! `XMLWriter` of SVG documents.

use std::path::PathBuf;

use pltrs::figure::{DistillOptions, ProgramError, distill_file};
use pltrs::rc::Distiller;
use pltrs::transforms::Bbox;
use pltrs_render::{Paper, SVG_PROLOG, XmlWriter, pstoeps};

use crate::binding::prelude::*;

impl From<ProgramError> for PyErr {
    /// `OSError` for a program that cannot start or a file that cannot be
    /// read or written, `TypeError` for a page the size of the figure
    /// without its bounding box (matplotlib's `bbox[2]` of `None`),
    /// `RuntimeError` for a program that is missing (which the Python side
    /// checks first, as matplotlib does) or fails.
    fn from(err: ProgramError) -> Self {
        match err {
            ProgramError::Start { .. } | ProgramError::Io { .. } => {
                PyErr::new(ExceptionKind::OSError, err.to_string())
            }
            ProgramError::MissingBbox => {
                PyErr::type_error("'NoneType' object is not subscriptable")
            }
            ProgramError::NotFound(_) | ProgramError::Failed { .. } => {
                PyErr::runtime_error(err.to_string())
            }
        }
    }
}

/// A bounding box `(llx, lly, urx, ury)` in points.
type Extents = (f64, f64, f64, f64);

/// `pstoeps(tmpfile, bbox, rotated)`: rewrites the PostScript file at
/// `path` as Encapsulated PostScript, declaring `bbox` if given.
fn pstoeps_file(path: PathBuf, bbox: Option<Extents>, rotated: bool) -> PyResult<()> {
    let io = |err: std::io::Error| PyErr::new(ExceptionKind::OSError, err.to_string());
    let document = std::fs::read(&path).map_err(io)?;
    let bbox = bbox.map(|(x0, y0, x1, y1)| Bbox::from_extents(x0, y0, x1, y1));
    std::fs::write(&path, pstoeps(&document, bbox.as_ref(), rotated)).map_err(io)
}

/// `gs_distill` (`distiller` `"ghostscript"`) and `xpdf_distill`
/// (`"xpdf"`) of the file at `path`, on the paper `ptype`.
fn distill(
    path: PathBuf,
    distiller: String,
    eps: bool,
    ptype: String,
    bbox: Option<Extents>,
    rotated: bool,
) -> PyResult<()> {
    let distiller = match distiller.as_str() {
        "ghostscript" => Distiller::Ghostscript,
        "xpdf" => Distiller::Xpdf,
        _ => return Err(PyErr::value_error(format!("no distiller {distiller:?}"))),
    };
    let paper: Paper = ptype
        .parse()
        .map_err(|err: pltrs_render::PageSetupError| PyErr::value_error(err.to_string()))?;
    let options = DistillOptions {
        eps,
        paper,
        bbox: bbox.map(|(x0, y0, x1, y1)| Bbox::from_extents(x0, y0, x1, y1)),
        rotated,
    };
    Ok(distill_file(&path, distiller, &options)?)
}

/// The XML declaration and doctype of SVG documents (`svgProlog`).
fn svg_prolog() -> PyResult<&'static str> {
    Ok(SVG_PROLOG)
}

/// A streaming XML writer Python reaches by reference.
struct XmlWriterCell(XmlWriter);

impl IntoPy for XmlWriterCell {
    /// A native object holding the writer.
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        crate::binding::wrap_native(py, self)
    }
}

/// A new XML writer (`XMLWriter(file)`; Python writes what it takes to
/// the file).
fn xml_new() -> PyResult<XmlWriterCell> {
    Ok(XmlWriterCell(XmlWriter::new("")))
}

/// Opens an element with its non-empty attributes; its depth.
fn xml_start(
    writer: Native<XmlWriterCell>,
    tag: String,
    attributes: Vec<(String, String)>,
) -> PyResult<usize> {
    let attributes: Vec<(&str, String)> = attributes
        .iter()
        .map(|(name, value)| (name.as_str(), value.clone()))
        .collect();
    Ok(writer.borrow_mut().0.start(&tag, &attributes))
}

/// Writes a comment.
fn xml_comment(writer: Native<XmlWriterCell>, text: String) -> PyResult<()> {
    writer.borrow_mut().0.comment(&text);
    Ok(())
}

/// Adds character data to the open element.
fn xml_data(writer: Native<XmlWriterCell>, text: String) -> PyResult<()> {
    writer.borrow_mut().0.data(&text);
    Ok(())
}

/// Closes the open element, its end tag indented with `indent`.
fn xml_end(writer: Native<XmlWriterCell>, indent: bool) -> PyResult<()> {
    writer.borrow_mut().0.end(indent);
    Ok(())
}

/// Closes the elements down to and including depth `id`.
fn xml_close(writer: Native<XmlWriterCell>, id: usize) -> PyResult<()> {
    writer.borrow_mut().0.close(id);
    Ok(())
}

/// The open elements, outermost first.
fn xml_tags(writer: Native<XmlWriterCell>) -> PyResult<Vec<String>> {
    Ok(writer.borrow().0.open_tags().to_vec())
}

/// Whether `tag` names the innermost open element.
fn xml_closes(writer: Native<XmlWriterCell>, tag: String) -> PyResult<bool> {
    Ok(writer.borrow().0.closes(&tag))
}

/// The text written since the last call.
fn xml_take(writer: Native<XmlWriterCell>) -> PyResult<String> {
    Ok(writer.borrow_mut().0.take())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("pstoeps", pstoeps_file)?;
    module.add("distill", distill)?;
    module.add("svg_prolog", svg_prolog)?;
    module.add("xml_new", xml_new)?;
    module.add("xml_start", xml_start)?;
    module.add("xml_comment", xml_comment)?;
    module.add("xml_data", xml_data)?;
    module.add("xml_end", xml_end)?;
    module.add("xml_close", xml_close)?;
    module.add("xml_tags", xml_tags)?;
    module.add("xml_closes", xml_closes)?;
    module.add("xml_take", xml_take)
}
