//! The `FT2Font` operations of the `type1font` area: a `.pfb` file as
//! `FreeType`'s Type 1 driver loads it.

use pltrs_font::{GlyphId, OutlineSink, Type1Face};

use super::{effects, font};
use crate::json::Json;
use crate::probes::areas::text_metrics::PathCodes;
use crate::probes::{Args, Outcome, ProbeError, made};

/// The face of the fixture named by `file`, transformed by the `effects`
/// argument when it is given.
fn face(args: Args<'_>) -> Result<Type1Face, ProbeError> {
    let font = font(args)?;
    let font = match args.get("effects") {
        Some(json) => made(font.transform(effects(json)?))?,
        None => font,
    };
    made(Type1Face::new(&font))
}

fn integer(value: impl Into<i64>) -> Json {
    Json::Integer(value.into())
}

/// `FT2Font` attributes of the face.
pub(super) fn attrs(args: Args<'_>) -> Outcome {
    let face = face(args)?;
    let bbox = face.bbox();
    let name = |name: Option<&str>| name.unwrap_or("UNAVAILABLE").into();
    Ok(Json::Object(vec![
        ("postscript_name".into(), name(face.postscript_name())),
        ("family_name".into(), name(face.family_name())),
        ("style_name".into(), face.style_name().into()),
        ("num_glyphs".into(), integer(face.num_glyphs())),
        ("fixed_width".into(), Json::Bool(face.is_fixed_pitch())),
        (
            "style_flags".into(),
            Json::Object(vec![
                ("italic".into(), Json::Bool(face.is_italic())),
                ("bold".into(), Json::Bool(face.is_bold())),
            ]),
        ),
        ("units_per_EM".into(), integer(face.units_per_em())),
        (
            "underline_position".into(),
            integer(face.underline_position()),
        ),
        (
            "underline_thickness".into(),
            integer(face.underline_thickness()),
        ),
        ("ascender".into(), integer(face.ascender())),
        ("descender".into(), integer(face.descender())),
        ("height".into(), integer(face.height())),
        (
            "max_advance_width".into(),
            integer(face.max_advance_width()),
        ),
        ("max_advance_height".into(), integer(face.height())),
        (
            "bbox".into(),
            Json::Array(
                [bbox.x_min, bbox.y_min, bbox.x_max, bbox.y_max]
                    .into_iter()
                    .map(integer)
                    .collect(),
            ),
        ),
    ]))
}

/// An outline in font units (or 26.6 pixels) handed to [`PathCodes`],
/// which records 64ths of its pixels.
struct FontUnits(PathCodes);

impl OutlineSink for FontUnits {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to(x / 64.0, y / 64.0);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to(x / 64.0, y / 64.0);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.quad_to(x1 / 64.0, y1 / 64.0, x / 64.0, y / 64.0);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.0.curve_to(
            x1 / 64.0,
            y1 / 64.0,
            x2 / 64.0,
            y2 / 64.0,
            x / 64.0,
            y / 64.0,
        );
    }

    fn close(&mut self) {
        self.0.close();
    }
}

/// A glyph loaded with `LoadFlags.NO_SCALE`.
pub(super) fn load_glyph(args: Args<'_>) -> Outcome {
    let face = face(args)?;
    let index = u16::try_from(args.i64("glyph")?)
        .map_err(|_| ProbeError::BadArgs("bad glyph index".into()))?;
    let glyph = made(face.load_glyph(GlyphId(index)))?;
    let mut path = FontUnits(PathCodes::default());
    glyph.outline(&mut path);
    let bbox = glyph
        .control_box()
        .map_or([0; 4], |b| [b.x_min, b.y_min, b.x_max, b.y_max]);
    Ok(Json::Object(vec![
        (
            "name".into(),
            face.glyph_name(GlyphId(index))
                .map_or(Json::Null, Json::from),
        ),
        ("advance".into(), integer(glyph.advance())),
        (
            "bbox".into(),
            Json::Array(bbox.into_iter().map(integer).collect()),
        ),
        ("path".into(), path.0.finish()),
    ]))
}

/// A glyph after `set_size(size, dpi)`, hinted (`TARGET_LIGHT` or
/// `DEFAULT`) or not (`NO_HINTING`), in 26.6 pixels.
pub(super) fn load_sized(args: Args<'_>) -> Outcome {
    let face = face(args)?;
    let index = u16::try_from(args.i64("glyph")?)
        .map_err(|_| ProbeError::BadArgs("bad glyph index".into()))?;
    let size = face.size(args.f64("size")?, args.f64("dpi")?);
    let glyph = match args.str("flags")? {
        "TARGET_LIGHT" | "DEFAULT" => made(face.load_glyph_hinted(GlyphId(index), &size))?,
        "NO_HINTING" => {
            made(face.load_glyph(GlyphId(index)))?.scaled(size.x_scale(), size.y_scale())
        }
        other => return Err(ProbeError::BadArgs(format!("unsupported flags {other:?}"))),
    };
    let mut path = FontUnits(PathCodes::default());
    glyph.outline(&mut path);
    Ok(Json::Object(vec![
        ("advance".into(), integer(glyph.advance())),
        ("path".into(), path.0.finish()),
    ]))
}

/// `FT_Get_Name_Index`: 0 for unknown names.
pub(super) fn name_index(args: Args<'_>) -> Outcome {
    let face = face(args)?;
    let index = face
        .glyph_index(args.str("name")?)
        .map_or(0, |glyph| glyph.0);
    Ok(integer(index))
}

pub(super) fn encoding_vector(args: Args<'_>) -> Outcome {
    let face = face(args)?;
    Ok(Json::Array(
        face.encoding_vector()
            .iter()
            .map(|glyph| integer(glyph.0))
            .collect(),
    ))
}
