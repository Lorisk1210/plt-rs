//! Glyph probes: hinted outlines and advances as matplotlib's `FT2Font`
//! loads them from the bundled fonts.

use std::sync::OnceLock;

use plt_font::{FontDatabase, FontSource, GlyphId, Hinting, HintingInstance, OutlineSink, Size};

use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError};

/// The bundled fonts, loaded once.
fn database() -> &'static FontDatabase {
    static DATABASE: OnceLock<FontDatabase> = OnceLock::new();
    DATABASE.get_or_init(FontDatabase::with_bundled_fonts)
}

/// matplotlib's path of a glyph (`FT2Font.get_path`): codes and vertices
/// in 1/64 pixel, with a `CLOSEPOLY` before every contour but the first
/// and after the last.
#[derive(Default)]
struct PathCodes(Vec<Json>);

impl PathCodes {
    fn push(&mut self, code: f64, x: f32, y: f32) {
        let units = |v: f32| f64::from((v * 64.0).round());
        self.0.push(Json::Array(vec![
            code.into(),
            units(x).into(),
            units(y).into(),
        ]));
    }

    fn finish(mut self) -> Json {
        if !self.0.is_empty() {
            self.push(79.0, 0.0, 0.0);
        }
        Json::Array(self.0)
    }
}

impl OutlineSink for PathCodes {
    fn move_to(&mut self, x: f32, y: f32) {
        if !self.0.is_empty() {
            self.push(79.0, 0.0, 0.0);
        }
        self.push(1.0, x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.push(2.0, x, y);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.push(3.0, x1, y1);
        self.push(3.0, x, y);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.push(4.0, x1, y1);
        self.push(4.0, x2, y2);
        self.push(4.0, x, y);
    }

    fn close(&mut self) {}
}

/// `ft2font.FT2Font.load_glyph`: a glyph of a bundled font hinted with
/// `LoadFlags.DEFAULT` or `LoadFlags.NO_AUTOHINT`.
pub(super) fn load_glyph(args: Args<'_>) -> Outcome {
    let hinting = match args.str("flags")? {
        "DEFAULT" => Hinting::Default,
        "NO_AUTOHINT" => Hinting::Native,
        other => return Err(ProbeError::BadArgs(format!("unsupported flags {other:?}"))),
    };
    let file = args.str("file")?;
    let db = database();
    let font = db
        .entries()
        .iter()
        .map(|entry| entry.font)
        .find(|&font| db.source(font).and_then(FontSource::file_name) == Some(file))
        .ok_or_else(|| ProbeError::BadArgs(format!("unknown font file {file:?}")))?;
    let face = db.face(font).map_err(ProbeError::rejected)?;
    let c = args
        .str("char")?
        .chars()
        .next()
        .ok_or_else(|| ProbeError::BadArgs("empty char".to_owned()))?;
    let glyph = face.glyph_index(c).unwrap_or(GlyphId::NOTDEF);
    if !hinting.uses_bytecode(&face, true) {
        return Err(ProbeError::BadArgs(format!(
            "FreeType autohints {file} under {hinting:?}"
        )));
    }
    let size = Size::new(&face, args.f64("size")?, args.f64("dpi")?);
    let hinted = HintingInstance::new(&face, &size)
        .load_glyph(&face, glyph)
        .map_err(ProbeError::rejected)?;
    let mut path = PathCodes::default();
    hinted.outline(&mut path);
    Ok(Json::Object(vec![
        ("advance".into(), f64::from(hinted.advance()).into()),
        ("path".into(), path.finish()),
    ]))
}
