//! Probe operations for the `texmanager` area: the LaTeX documents and
//! cache names of usetex, and the readers of TeX's file formats, without
//! TeX (see `spec/reference/probes/test_texmanager.py`).

use std::collections::HashMap;
use std::path::PathBuf;

use plt::rc::RcParams;
use plt_text::tex::{
    FileFinder, PsfontsMap, RawGlyph, RawPage, TexEngine, TexError, TexFiles, TexFonts, TexManager,
    Tfm, Vf, parse_enc, read_dvi, tex_source,
};

use super::text_metrics::with_rc;
use super::type1font::text;
use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError, made, malformed};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("TexManager._get_tex_source", source),
    Op::new("TexManager._get_base_path", base_name),
    Op::new("dviread.PsfontsMap", psfont),
    Op::new("dviread._parse_enc", encoding),
    Op::new("dviread.Tfm", tfm),
    Op::new("dviread.Vf", vf),
    Op::new("dviread.Dvi", dvi),
];

/// The usetex font settings of `rc`.
pub(crate) fn tex_fonts(rc: &RcParams) -> TexFonts {
    TexFonts {
        family: rc.font_family.clone(),
        serif: rc.font_serif.clone(),
        sans_serif: rc.font_sans_serif.clone(),
        cursive: rc.font_cursive.clone(),
        monospace: rc.font_monospace.clone(),
    }
}

fn source(args: Args<'_>) -> Outcome {
    let (tex, fontsize) = (args.str("tex")?, args.f64("fontsize")?);
    with_rc(args, || {
        let rc = plt::rc::params();
        Ok(tex_source(tex, fontsize, &tex_fonts(&rc), &rc.text_latex_preamble).into())
    })
}

fn base_name(args: Args<'_>) -> Outcome {
    let (tex, fontsize, dpi) = (
        args.str("tex")?,
        args.f64("fontsize")?,
        args.opt_f64("dpi")?,
    );
    with_rc(args, || {
        let rc = plt::rc::params();
        let dir = std::env::temp_dir().join(format!("plt-spec-texmanager-{}", std::process::id()));
        let manager = TexManager::new(
            &dir,
            tex_fonts(&rc),
            &rc.text_latex_preamble,
            TexEngine::Latex,
        );
        let path = made(manager.base_path(tex, fontsize, dpi))?;
        let _ = std::fs::remove_dir_all(dir);
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
        Ok(name.unwrap_or_default().into())
    })
}

/// Finds every file by its own name (`test_PsfontsMap`'s
/// `find_tex_file`).
#[derive(Debug)]
struct Identity;

impl FileFinder for Identity {
    fn find(&self, name: &str) -> Result<PathBuf, TexError> {
        Ok(PathBuf::from(name))
    }
}

/// `SlantFont` and `ExtendFont` effects as a dictionary with sorted keys.
pub(crate) fn effects(effects: plt_text::Type1Effects) -> Json {
    let mut entries = Vec::new();
    if let Some(extend) = effects.extend {
        entries.push(("extend".to_owned(), Json::Number(extend)));
    }
    if let Some(slant) = effects.slant {
        entries.push(("slant".to_owned(), Json::Number(slant)));
    }
    Json::Object(entries)
}

/// An optional path as JSON.
fn path(path: Option<PathBuf>) -> Json {
    path.map_or(Json::Null, |p| p.display().to_string().into())
}

fn psfont(args: Args<'_>) -> Outcome {
    let texname = args.str("texname")?;
    let file =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../plt-text/tests/fixtures/tex/test.map");
    let data = std::fs::read(&file).map_err(|e| ProbeError::BadArgs(e.to_string()))?;
    let map = PsfontsMap::parse(&data, &file.display().to_string());
    let font = made(map.get(texname.as_bytes(), &Identity))?;
    Ok(Json::Object(vec![
        ("texname".into(), text(&font.texname)),
        ("psname".into(), text(&font.psname)),
        ("effects".into(), effects(font.effects)),
        ("encoding".into(), path(font.encoding)),
        ("filename".into(), path(font.filename)),
    ]))
}

fn encoding(args: Args<'_>) -> Outcome {
    let text: Vec<u8> = args
        .str("text")?
        .chars()
        .map(|c| u8::try_from(u32::from(c)).map_err(|_| malformed("text")))
        .collect::<Result<_, _>>()?;
    let names = made(parse_enc(&text))?;
    Ok(Json::Array(names.into_iter().map(Json::from).collect()))
}

/// Hexadecimal bytes.
fn unhex(text: &str, what: &str) -> Result<Vec<u8>, ProbeError> {
    if !text.len().is_multiple_of(2) {
        return Err(malformed(what));
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).map_err(|_| malformed(what)))
        .collect()
}

/// Font files held in memory.
#[derive(Debug, Default)]
struct Memory(HashMap<String, Vec<u8>>);

impl FileFinder for Memory {
    fn find(&self, name: &str) -> Result<PathBuf, TexError> {
        if self.0.contains_key(name) {
            Ok(PathBuf::from(name))
        } else {
            Err(TexError::NotFound {
                name: name.to_owned(),
            })
        }
    }

    fn read(&self, name: &str) -> Result<Vec<u8>, TexError> {
        self.0.get(name).cloned().ok_or_else(|| TexError::NotFound {
            name: name.to_owned(),
        })
    }
}

/// The files of the `tfm` and `vf` arguments.
fn files(args: Args<'_>) -> Result<TexFiles, ProbeError> {
    let mut memory = Memory::default();
    for (key, suffix) in [("tfm", "tfm"), ("vf", "vf")] {
        for (name, data) in args.get(key).and_then(Json::as_object).unwrap_or(&[]) {
            let hex = data.as_str().ok_or_else(|| malformed(key))?;
            memory
                .0
                .insert(format!("{name}.{suffix}"), unhex(hex, key)?);
        }
    }
    Ok(TexFiles::new(memory))
}

/// A DVI unit count as JSON.
fn int(value: i128) -> Json {
    i64::try_from(value).map_or(Json::Number(value as f64), Json::Integer)
}

fn tfm(args: Args<'_>) -> Outcome {
    let tfm = made(Tfm::parse(&unhex(args.str("data")?, "data")?))?;
    let metrics = tfm
        .codes()
        .into_iter()
        .filter_map(|code| tfm.metrics(code).map(|m| (code, m)))
        .map(|(code, m)| {
            Json::Array(vec![
                Json::Integer(code),
                Json::Integer(m.width.into()),
                Json::Integer(m.height.into()),
                Json::Integer(m.depth.into()),
            ])
        })
        .collect();
    Ok(Json::Object(vec![
        ("checksum".into(), Json::Integer(tfm.checksum().into())),
        (
            "design_size".into(),
            Json::Integer(tfm.design_size().into()),
        ),
        ("metrics".into(), Json::Array(metrics)),
    ]))
}

/// A glyph of a virtual font's packet: `[x, y, texname, scale, code]`.
fn packet_glyph(glyph: &RawGlyph) -> Json {
    Json::Array(vec![
        int(glyph.x),
        int(glyph.y),
        text(glyph.font.texname()),
        int(glyph.font.scale()),
        int(glyph.glyph),
    ])
}

fn vf(args: Args<'_>) -> Outcome {
    let files = files(args)?;
    let font = made(Vf::parse(&unhex(args.str("data")?, "data")?, &files))?;
    let packets = font
        .codes()
        .into_iter()
        .filter_map(|code| font.packet(code).map(|p| (code, p)))
        .map(|(code, packet)| {
            let rules = packet
                .rules()
                .iter()
                .map(|r| Json::Array(vec![int(r.x), int(r.y), int(r.height), int(r.width)]))
                .collect();
            Json::Array(vec![
                Json::Integer(code),
                int(packet.width()),
                Json::Array(packet.glyphs().iter().map(packet_glyph).collect()),
                Json::Array(rules),
            ])
        })
        .collect();
    Ok(Json::Object(vec![("packets".into(), Json::Array(packets))]))
}

/// A page in TeX's units (`Dvi(..., None)`).
fn raw_page(page: &RawPage) -> Outcome {
    let text = page
        .glyphs()
        .iter()
        .map(|t| {
            Json::Array(vec![
                int(t.x),
                int(t.y),
                text(t.font.texname()),
                Json::Number(t.font.size()),
                int(t.glyph),
                int(t.width),
            ])
        })
        .collect();
    let boxes = page
        .rules()
        .iter()
        .map(|r| Json::Array(vec![int(r.x), int(r.y), int(r.height), int(r.width)]))
        .collect();
    let (width, height, descent) = made(page.extent())?;
    Ok(Json::Object(vec![
        ("text".into(), Json::Array(text)),
        ("boxes".into(), Json::Array(boxes)),
        ("width".into(), int(width)),
        ("height".into(), int(height)),
        ("descent".into(), int(descent)),
    ]))
}

/// A page at `dpi`.
fn scaled_page(page: &RawPage, dpi: f64) -> Outcome {
    let page = made(page.at(dpi))?;
    let text = page
        .text
        .iter()
        .map(|t| {
            Json::Array(vec![
                Json::Number(t.x),
                Json::Number(t.y),
                text(t.font.texname()),
                Json::Number(t.font.size()),
                Json::Integer(t.glyph),
                Json::Number(t.width),
            ])
        })
        .collect();
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
}

fn dvi(args: Args<'_>) -> Outcome {
    let files = files(args)?;
    let pages = made(read_dvi(&unhex(args.str("data")?, "data")?, &files))?;
    let dpi = args.opt_f64("dpi")?;
    pages
        .iter()
        .map(|page| match dpi {
            None => raw_page(page),
            Some(dpi) => scaled_page(page, dpi),
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Json::Array)
}
