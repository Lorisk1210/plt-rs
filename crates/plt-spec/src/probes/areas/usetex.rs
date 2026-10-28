//! Probe operations for the `usetex` area: what `TexManager` and `dviread`
//! make of strings `latex` typesets (see
//! `spec/reference/probes/test_usetex.py`). Every operation needs TeX and
//! reports a skip where it is missing.

use std::fmt::Write as _;

use plt::figure::Figure;
use plt::text::check_usetex;
use plt_codec::sha1::sha1;
use plt_text::tex::{TexEngine, TexManager, TexText};
use plt_text::{FontQuery, FontRequest};
use plt_text::{TEXT_PATH_DPI, TEXT_PATH_FONT_SCALE};

use super::texmanager::{effects, tex_fonts};
use super::text_metrics::with_rc;
use super::type1font::hex;
use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError, made};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("TexManager.get_text_width_height_descent", extent),
    Op::new("dviread.Dvi.page", page),
    Op::new("dviread.PsfontsMap.pdftex", pdftex),
    Op::new("dviread.find_tex_file", find_tex_file),
    Op::new("TextToPath.get_glyphs_tex", glyphs_tex),
    Op::new("TexManager.get_grey", grey),
    Op::new("TexManager.make_dvi", make_dvi),
    Op::new("test_usetex.minus_no_descent", minus_no_descent),
];

/// A skip where TeX is not installed.
fn needs_tex() -> Result<(), ProbeError> {
    check_usetex().map_err(|err| ProbeError::Skipped(err.to_string()))
}

/// The TeX typesetter of the current rcParams.
fn manager() -> Result<TexManager, ProbeError> {
    let rc = plt::rc::params();
    let cache_dir = made(plt::config::cache_dir())?;
    Ok(TexManager::new(
        &cache_dir,
        tex_fonts(&rc),
        &rc.text_latex_preamble,
        TexEngine::Latex,
    ))
}

/// The probe's `tex`, `fontsize` and `dpi` arguments.
fn typesetting(args: Args<'_>) -> Result<(&str, f64, f64), ProbeError> {
    Ok((args.str("tex")?, args.f64("fontsize")?, args.f64("dpi")?))
}

fn extent(args: Args<'_>) -> Outcome {
    needs_tex()?;
    let (tex, fontsize, dpi) = typesetting(args)?;
    with_rc(args, || {
        let (w, h, d) = made(manager()?.text_width_height_descent(tex, fontsize, dpi / 72.0))?;
        Ok(Json::Array([w, h, d].map(Json::Number).to_vec()))
    })
}

/// A file's name, as JSON.
fn file_name(path: &std::path::Path) -> Json {
    path.file_name()
        .map_or(Json::Null, |n| n.to_string_lossy().into_owned().into())
}

fn page(args: Args<'_>) -> Outcome {
    needs_tex()?;
    let (tex, fontsize, dpi) = typesetting(args)?;
    with_rc(args, || {
        let manager = manager()?;
        let page = made(manager.page(tex, fontsize, dpi))?;
        let files = manager.files();
        let text = page
            .text
            .iter()
            .map(|t| {
                Ok(Json::Array(vec![
                    Json::Number(t.x),
                    Json::Number(t.y),
                    Json::String(t.font.name()),
                    Json::Number(t.font.size()),
                    Json::Integer(t.glyph),
                    Json::Number(t.width),
                    Json::Integer(made(files.glyph_index(&t.font, t.glyph))?.0.into()),
                    file_name(&made(files.font_path(&t.font))?),
                    effects(made(files.effects(&t.font))?),
                ]))
            })
            .collect::<Result<Vec<_>, ProbeError>>()?;
        let boxes = page
            .boxes
            .iter()
            .map(|b| Json::Array([b.x, b.y, b.height, b.width].map(Json::Number).to_vec()))
            .collect();
        Ok(Json::Object(vec![
            ("text".into(), Json::Array(text)),
            ("boxes".into(), Json::Array(boxes)),
            ("width".into(), Json::Number(page.width)),
            ("height".into(), Json::Number(page.height)),
            ("descent".into(), Json::Number(page.descent)),
        ]))
    })
}

fn pdftex(args: Args<'_>) -> Outcome {
    needs_tex()?;
    let texname = args.str("texname")?;
    let font = made(manager()?.files().psfont(texname.as_bytes()))?;
    let name = |path: Option<std::path::PathBuf>| path.as_deref().map_or(Json::Null, file_name);
    Ok(Json::Object(vec![
        (
            "psname".into(),
            String::from_utf8_lossy(&font.psname).into_owned().into(),
        ),
        ("effects".into(), effects(font.effects)),
        ("encoding".into(), name(font.encoding)),
        ("filename".into(), name(font.filename)),
    ]))
}

fn find_tex_file(args: Args<'_>) -> Outcome {
    needs_tex()?;
    let path = made(manager()?.files().find(args.str("name")?))?;
    Ok(file_name(&path))
}

/// `urllib.parse.quote` with its default safe characters.
fn url_quote(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"_.-~/".contains(&byte) {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

fn glyphs_tex(args: Args<'_>) -> Outcome {
    needs_tex()?;
    let tex = args.str("tex")?;
    with_rc(args, || {
        let font = FontRequest::new(FontQuery::families(["DejaVu Sans"]));
        let text = TexText::new(tex, font, manager()?);
        let page = made(text.page(TEXT_PATH_FONT_SCALE, TEXT_PATH_DPI))?;
        let glyphs = made(text.glyphs(&page))?
            .iter()
            .map(|g| {
                let name = g.file.face.postscript_name().unwrap_or_default();
                Json::Array(vec![
                    url_quote(&format!("{name}-{:x}", g.index.0)).into(),
                    Json::Number(g.x),
                    Json::Number(g.y),
                    Json::Number(g.size / TEXT_PATH_FONT_SCALE),
                ])
            })
            .collect();
        Ok(Json::Object(vec![
            ("glyphs".into(), Json::Array(glyphs)),
            ("rects".into(), Json::Integer(page.boxes.len() as i64)),
        ]))
    })
}

fn grey(args: Args<'_>) -> Outcome {
    needs_tex()?;
    let (tex, fontsize, dpi) = typesetting(args)?;
    with_rc(args, || {
        let grey = made(manager()?.grey(tex, fontsize, dpi))?;
        let sum: i64 = grey.data.iter().map(|&v| i64::from(v)).sum();
        Ok(Json::Array(vec![
            Json::Integer(grey.height.into()),
            Json::Integer(grey.width.into()),
            Json::Integer(sum),
            hex(&sha1(&grey.data)).into(),
        ]))
    })
}

fn make_dvi(args: Args<'_>) -> Outcome {
    needs_tex()?;
    let tex = args.str("tex")?;
    with_rc(args, || {
        made(manager()?.make_dvi(tex, 10.0))?;
        Ok(Json::Bool(true))
    })
}

/// The number of pixel rows of `fig` holding anything not white (in red).
fn inked_rows(fig: &Figure) -> Result<i64, ProbeError> {
    let canvas = made(fig.render_raster())?;
    let pixmap = canvas.pixmap();
    let rows = (0..pixmap.height())
        .filter(|&y| (0..pixmap.width()).any(|x| pixmap.pixel(x, y).is_some_and(|p| p.r != 255)))
        .count();
    Ok(rows as i64)
}

fn minus_no_descent(args: Args<'_>) -> Outcome {
    needs_tex()?;
    let fontsize = args.f64("fontsize")?;
    with_rc(args, || {
        made(plt::rc::set("font.size", &fontsize.to_string()))?;
        let mut heights = Vec::new();
        for values in [&[1][..], &[-1], &[-1, 1]] {
            let fig = Figure::new();
            for value in values {
                fig.text(0.5, 0.5, format!("${value}$")).set_usetex(true);
            }
            heights.push(Json::Integer(inked_rows(&fig)?));
        }
        Ok(Json::Array(heights))
    })
}
