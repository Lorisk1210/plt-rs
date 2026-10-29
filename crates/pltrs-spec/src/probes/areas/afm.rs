//! Probe operations for the `afm` area: matplotlib's AFM parser on the
//! bundled core-font metrics and on synthetic files, AFM font lookup by
//! directory and the text measurements of the PDF backend's core fonts
//! (see `spec/reference/probes/test_afm.py`).

use pltrs::rc;
use pltrs_font::{AfmDirectory, AfmFont, AfmMetrics, StrBbox, bundled_metrics};
use pltrs_text::FontRequest;

use super::text_metrics::{font_dict, with_rc};
use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("_afm.AFM.header", header),
    Op::new("_afm.AFM.get_str_bbox_and_descent", str_bbox_and_descent),
    Op::new("_afm.AFM.get_width_char", width_char),
    Op::new("_afm.AFM.get_glyph_name", glyph_name),
    Op::new("_afm.AFM.get_width_from_char_name", width_from_char_name),
    Op::new("_afm.AFM.get_kern_dist_from_name", kern_dist_from_name),
    Op::new("_afm.AFM", parse),
    Op::new("font_manager.findfont", findfont),
    Op::new(
        "font_manager.FontManager._find_fonts_by_props",
        find_fonts_by_props,
    ),
    Op::new(
        "backend_pdf.RendererPdf.get_text_width_height_descent",
        pdf_width_height_descent,
    ),
    Op::new(
        "backend_pdf.RendererPdf._get_font_height_metrics",
        pdf_font_height_metrics,
    ),
];

/// The bundled metrics named by the `file` argument.
fn metrics(args: Args<'_>) -> Result<&'static AfmMetrics, ProbeError> {
    bundled_metrics(args.str("file")?).map_err(|err| ProbeError::BadArgs(err.to_string()))
}

/// A value, or the rejection matplotlib's `KeyError` stands for.
fn found<T: Into<Json>>(value: Option<T>, what: &str) -> Outcome {
    value
        .map(Into::into)
        .ok_or_else(|| ProbeError::Rejected(format!("no {what}")))
}

/// `[left, bottom, width, height, descent]`.
fn bbox_json(bbox: &StrBbox) -> Json {
    Json::from(vec![
        bbox.left,
        bbox.bottom,
        bbox.width,
        bbox.height,
        bbox.descent,
    ])
}

fn header(args: Args<'_>) -> Outcome {
    let afm = metrics(args)?;
    let text = |value: Result<&str, _>| value.map(Json::from);
    let number = |value: Result<f64, _>| value.map(Json::from);
    match args.str("getter")? {
        "get_fontname" => text(afm.font_name()),
        "get_fullname" => text(afm.full_name()),
        "get_familyname" => text(afm.family_name()),
        "get_weight" => text(afm.weight()),
        "get_angle" => number(afm.angle()),
        "get_ascender" => number(afm.ascender()),
        "get_descender" => number(afm.descender()),
        "get_capheight" => number(afm.cap_height()),
        "get_xheight" => number(afm.x_height()),
        "get_underline_thickness" => number(afm.underline_thickness()),
        other => return Err(ProbeError::BadArgs(format!("unknown getter {other}"))),
    }
    .map_err(ProbeError::rejected)
}

fn str_bbox_and_descent(args: Args<'_>) -> Outcome {
    metrics(args)?
        .str_bbox_and_descent(args.str("s")?)
        .map(|bbox| bbox_json(&bbox))
        .map_err(ProbeError::rejected)
}

fn width_char(args: Args<'_>) -> Outcome {
    found(metrics(args)?.width_char(args.i64("code")?), "such code")
}

fn glyph_name(args: Args<'_>) -> Outcome {
    found(metrics(args)?.glyph_name(args.i64("code")?), "such code")
}

fn width_from_char_name(args: Args<'_>) -> Outcome {
    found(
        metrics(args)?.width_from_char_name(args.str("name")?),
        "such glyph",
    )
}

pub(super) fn kern_dist_from_name(args: Args<'_>) -> Outcome {
    Ok(metrics(args)?
        .kern_dist_from_name(args.str("name1")?, args.str("name2")?)
        .into())
}

/// `text.encode("latin-1")`.
pub(super) fn latin1(text: &str) -> Result<Vec<u8>, ProbeError> {
    text.chars()
        .map(|c| u8::try_from(c).map_err(|_| ProbeError::BadArgs(format!("{c:?} is not Latin-1"))))
        .collect()
}

/// `AFM(BytesIO(data.encode("latin-1")))` summarised.
fn parse(args: Args<'_>) -> Outcome {
    let afm = AfmMetrics::parse(&latin1(args.str("data")?)?).map_err(ProbeError::rejected)?;
    Ok(Json::Object(vec![
        ("fontname".into(), afm.font_name().ok().into()),
        ("ascender".into(), afm.ascender().ok().into()),
        (
            "bbox".into(),
            afm.str_bbox_and_descent("AV")
                .ok()
                .as_ref()
                .map_or(Json::Null, bbox_json),
        ),
        ("kern".into(), afm.kern_dist_from_name("A", "V").into()),
    ]))
}

/// The font request of the `key` font dict under the reference
/// configuration.
fn request(args: Args<'_>, key: &str) -> Result<FontRequest, ProbeError> {
    Ok(font_dict(args.get(key))?.request(&rc::params()))
}

/// The `directory` argument.
fn directory(args: Args<'_>) -> Result<AfmDirectory, ProbeError> {
    match args.str("directory")? {
        "pdfcorefonts" => Ok(AfmDirectory::PdfCore),
        "afm" => Ok(AfmDirectory::PostScript),
        other => Err(ProbeError::BadArgs(format!("unknown directory {other}"))),
    }
}

fn findfont(args: Args<'_>) -> Outcome {
    with_rc(args, || {
        let request = request(args, "prop")?;
        AfmFont::find_in(&request.query, &request.generics, directory(args)?)
            .map(|font| font.file.into())
            .map_err(ProbeError::rejected)
    })
}

fn find_fonts_by_props(args: Args<'_>) -> Outcome {
    with_rc(args, || {
        let request = request(args, "prop")?;
        AfmFont::find_all_in(&request.query, &request.generics, directory(args)?)
            .map(|fonts| Json::from(fonts.iter().map(|font| font.file).collect::<Vec<_>>()))
            .map_err(ProbeError::rejected)
    })
}

/// matplotlib's PDF renderer works at 72 dpi.
const PDF_DPI: f64 = 72.0;

fn pdf_width_height_descent(args: Args<'_>) -> Outcome {
    with_rc(args, || {
        let line = request(args, "font")?
            .afm_line_metrics(args.str("s")?, AfmDirectory::PdfCore, PDF_DPI)
            .map_err(ProbeError::rejected)?;
        Ok(Json::from(vec![line.width, line.height, line.descent]))
    })
}

fn pdf_font_height_metrics(args: Args<'_>) -> Outcome {
    with_rc(args, || {
        let heights = request(args, "font")?
            .afm_height_metrics(AfmDirectory::PdfCore, PDF_DPI)
            .map_err(ProbeError::rejected)?;
        Ok(Json::from(vec![
            heights.ascent,
            heights.descent,
            heights.line_gap,
        ]))
    })
}
