//! Probe operations for the `mpl_ft2font` area: matplotlib's own tests of
//! `FT2Font` (see `spec/reference/probes/test_mpl_ft2font.py`), on the
//! fonts bundled with matplotlib. The sfnt tables are in [`tables`].

mod tables;

use std::collections::BTreeMap;
use std::fmt::Write;

use plt_font::{
    AutohintTarget, Autohinter, Face, FontId, FontSource, GlyphId, Hinting, KerningMode,
    ShapeOptions, Size, TextLayout, bundled_font_data, face_count,
};

use super::text_metrics::{database, load_glyph};
use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError, encode};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("ft2font.FT2Font.attrs", attrs),
    Op::new("ft2font.FT2Font", open_face),
    Op::new("ft2font.FT2Font.get_charmap", get_charmap),
    Op::new("ft2font.FT2Font.get_name_index", get_name_index),
    Op::new("ft2font.FT2Font.get_glyph_name", get_glyph_name),
    Op::new("ft2font.FT2Font.get_sfnt", get_sfnt),
    Op::new("ft2font.FT2Font.get_sfnt_table", get_sfnt_table),
    Op::new("ft2font.FT2Font.get_kerning", get_kerning),
    Op::new("ft2font.FT2Font.set_text", set_text),
    Op::new("ft2font.FT2Font.load_char", load_char),
    Op::new("ft2font.FT2Font.load_glyph", load_glyph),
    Op::new("ft2font.FT2Font._layout.ft_object", layout_fonts),
];

/// The codes whose glyphs `get_charmap` records (see the reference
/// module's `SAMPLE_CODES`).
const SAMPLE_CODES: [u32; 12] = [
    0x2020, 0xB0, 0xA3, 0xA7, 0xB6, 0x221E, 0xA0, 0xA1, 0xA4, 0xA6, 0x1D305, 0x1F0A1,
];

fn rejected(err: impl std::fmt::Display) -> ProbeError {
    ProbeError::rejected(err)
}

fn number(value: impl Into<f64>) -> Json {
    Json::from(value.into())
}

/// The bytes of the bundled font named by `file`.
fn font_data(args: Args<'_>) -> Result<&'static [u8], ProbeError> {
    let file = args.str("file")?;
    bundled_font_data(file).ok_or_else(|| ProbeError::BadArgs(format!("unknown font {file:?}")))
}

/// Face 0 of the bundled font named by `file`.
fn face(args: Args<'_>) -> Result<Face<'static>, ProbeError> {
    Face::parse(font_data(args)?, 0).map_err(rejected)
}

/// The face's size for `set_size(size, dpi)`.
fn size(args: Args<'_>, face: &Face<'_>) -> Result<Size, ProbeError> {
    Ok(Size::new(face, args.f64("size")?, args.f64("dpi")?))
}

/// The glyph of the first character of argument `key`.
fn glyph_of(face: &Face<'_>, args: Args<'_>, key: &str) -> Result<Option<GlyphId>, ProbeError> {
    let c = args
        .str(key)?
        .chars()
        .next()
        .ok_or_else(|| ProbeError::BadArgs(format!("empty {key}")))?;
    Ok(face.glyph_index(c))
}

fn attrs(args: Args<'_>) -> Outcome {
    let data = font_data(args)?;
    let face = Face::parse(data, 0).map_err(rejected)?;
    let flags = face.face_flags();
    let style = face.style_flags();
    let (position, thickness) = face.underline_metrics().unwrap_or_default();
    let bbox = face.bbox();
    let text = |value: Option<String>| value.map_or(Json::Null, Json::from);
    Ok(encode::object([
        ("postscript_name", text(face.postscript_name())),
        ("family_name", text(face.family_name())),
        ("style_name", text(face.style_name())),
        ("num_faces", number(face_count(data).map_err(rejected)?)),
        ("num_glyphs", number(face.num_glyphs())),
        ("num_charmaps", number(face.charmaps().len() as f64)),
        (
            "face_flags",
            encode::object([
                ("scalable", flags.scalable.into()),
                ("sfnt", flags.sfnt.into()),
                ("horizontal", flags.horizontal.into()),
                ("kerning", flags.kerning.into()),
                ("glyph_names", flags.glyph_names.into()),
            ]),
        ),
        (
            "style_flags",
            encode::object([("italic", style.italic.into()), ("bold", style.bold.into())]),
        ),
        ("scalable", flags.scalable.into()),
        ("units_per_EM", number(face.units_per_em())),
        // FreeType reports the centre of the underline.
        ("underline_position", number(position - thickness / 2)),
        ("underline_thickness", number(thickness)),
        ("ascender", number(face.ascender())),
        ("descender", number(face.descender())),
        ("height", number(face.height())),
        ("max_advance_width", number(face.hhea().advance_width_max)),
        ("max_advance_height", number(face.max_advance_height())),
        (
            "bbox",
            Json::from(vec![
                f64::from(bbox.x_min),
                f64::from(bbox.y_min),
                f64::from(bbox.x_max),
                f64::from(bbox.y_max),
            ]),
        ),
    ]))
}

fn open_face(args: Args<'_>) -> Outcome {
    let data = font_data(args)?;
    let index = u32::try_from(args.i64("face_index")?).map_err(rejected)?;
    let face = Face::parse(data, index).map_err(rejected)?;
    Ok(encode::object([
        ("face_index", number(face.index())),
        ("num_faces", number(face_count(data).map_err(rejected)?)),
    ]))
}

/// Activates the character map of argument `charmap`: an index
/// (`set_charmap`), or the first Unicode or Apple Roman map
/// (`select_charmap` of `unic` or `armn`).
fn select_charmap(face: &mut Face<'_>, charmap: &Json) -> Result<(), ProbeError> {
    match charmap {
        // A freshly opened face has FreeType's Unicode map active.
        Json::String(name) if name == "unic" => Ok(()),
        Json::String(name) if name == "armn" => {
            let index = face
                .charmaps()
                .iter()
                .position(|info| (info.platform_id, info.encoding_id) == (1, 0))
                .ok_or_else(|| ProbeError::rejected("no Apple Roman character map"))?;
            face.select_charmap(index).map_err(rejected)
        }
        other => {
            let index = other
                .as_i64()
                .ok_or_else(|| ProbeError::BadArgs(format!("bad charmap {other}")))?;
            let index = usize::try_from(index).map_err(rejected)?;
            face.select_charmap(index).map_err(rejected)
        }
    }
}

fn get_charmap(args: Args<'_>) -> Outcome {
    let mut face = face(args)?;
    select_charmap(&mut face, args.json("charmap")?)?;
    let cmap: BTreeMap<u32, u16> = face
        .mappings()
        .into_iter()
        .filter(|&(_, glyph)| glyph != GlyphId::NOTDEF)
        .map(|(code, glyph)| (code, glyph.0))
        .collect();
    let checksum: u64 = cmap
        .iter()
        .map(|(&code, &glyph)| u64::from(code) * 65537 + u64::from(glyph))
        .sum();
    let sample = (1..128)
        .chain(SAMPLE_CODES)
        .map(|code| cmap.get(&code).map_or(Json::Null, |&glyph| number(glyph)))
        .collect();
    Ok(encode::object([
        ("size", number(cmap.len() as f64)),
        ("checksum", number(checksum as f64)),
        ("sample", Json::Array(sample)),
    ]))
}

fn get_name_index(args: Args<'_>) -> Outcome {
    let face = face(args)?;
    let glyph = face
        .glyph_index_by_name(args.str("name")?)
        .unwrap_or(GlyphId::NOTDEF);
    Ok(number(glyph.0))
}

fn get_glyph_name(args: Args<'_>) -> Outcome {
    let face = face(args)?;
    let index = u16::try_from(args.i64("index")?).map_err(rejected)?;
    face.glyph_name(GlyphId(index))
        .map(Json::from)
        .ok_or_else(|| ProbeError::rejected("the glyph has no name"))
}

fn get_sfnt(args: Args<'_>) -> Outcome {
    let face = face(args)?;
    // A dictionary keeps the last record of a key.
    let records: BTreeMap<(u16, u16, u16, u16), &[u8]> = face
        .name_entries()
        .into_iter()
        .map(|entry| {
            let key = entry.key;
            (
                (
                    key.platform_id,
                    key.encoding_id,
                    key.language_id,
                    entry.name_id,
                ),
                entry.bytes,
            )
        })
        .collect();
    Ok(Json::Array(
        records
            .into_iter()
            .map(|((platform, encoding, language, name_id), bytes)| {
                let hex = bytes.iter().fold(String::new(), |mut hex, b| {
                    write!(hex, "{b:02x}").expect("writing to a String cannot fail");
                    hex
                });
                Json::Array(vec![
                    number(platform),
                    number(encoding),
                    number(language),
                    number(name_id),
                    Json::from(hex),
                ])
            })
            .collect(),
    ))
}

fn get_sfnt_table(args: Args<'_>) -> Outcome {
    Ok(tables::sfnt_table(&face(args)?, args.str("table")?))
}

fn get_kerning(args: Args<'_>) -> Outcome {
    let face = face(args)?;
    let mode = match args.str("mode")? {
        "UNSCALED" => KerningMode::Unscaled,
        "UNFITTED" => KerningMode::Unfitted,
        "DEFAULT" => KerningMode::Default,
        other => {
            return Err(ProbeError::BadArgs(format!(
                "unknown kerning mode {other:?}"
            )));
        }
    };
    let left = glyph_of(&face, args, "left")?.unwrap_or(GlyphId::NOTDEF);
    let right = glyph_of(&face, args, "right")?.unwrap_or(GlyphId::NOTDEF);
    Ok(number(
        size(args, &face)?.kerning(&face, left, right, mode) as f64
    ))
}

/// `set_text(text)` with matplotlib's default `FORCE_AUTOHINT`.
fn laid_out(args: Args<'_>) -> Result<TextLayout, ProbeError> {
    let face = face(args)?;
    plt_font::FontChain::new(vec![face])
        .layout(
            args.str("text")?,
            args.f64("size")?,
            args.f64("dpi")?,
            &ShapeOptions::default(),
            Hinting::Auto,
        )
        .map_err(rejected)
}

fn set_text(args: Args<'_>) -> Outcome {
    let layout = laid_out(args)?;
    let xys = layout
        .glyphs
        .iter()
        .map(|glyph| Json::from(vec![f64::from(glyph.x), f64::from(glyph.y)]))
        .collect();
    Ok(encode::object([
        ("xys", Json::Array(xys)),
        (
            "width_height",
            Json::from(vec![f64::from(layout.width), f64::from(layout.height)]),
        ),
        ("num_glyphs", number(layout.glyphs.len() as f64)),
        ("descent", number(layout.descent)),
        (
            "bitmap_offset",
            Json::from(vec![f64::from(layout.x_min), 0.0]),
        ),
    ]))
}

/// The metrics of `load_char(ord(char))`, autohinted as by default.
fn load_char(args: Args<'_>) -> Outcome {
    let face = face(args)?;
    let glyph = glyph_of(&face, args, "char")?
        .ok_or_else(|| ProbeError::rejected("failed to find glyph to load"))?;
    let size = size(args, &face)?;
    let hinted = Autohinter::new(&face)
        .load_glyph(&face, &size, glyph, AutohintTarget::Normal)
        .map_err(rejected)?;
    let metrics = hinted.metrics();
    let bbox = hinted.control_box().unwrap_or_default();
    Ok(encode::object([
        ("width", number(metrics.width)),
        ("height", number(metrics.height)),
        ("horiBearingX", number(metrics.hori_bearing_x)),
        ("horiBearingY", number(metrics.hori_bearing_y)),
        ("horiAdvance", number(metrics.hori_advance)),
        (
            "linearHoriAdvance",
            number(size.linear_advance(&face, glyph) as f64),
        ),
        (
            "bbox",
            Json::from(vec![
                f64::from(bbox.x_min),
                f64::from(bbox.y_min),
                f64::from(bbox.x_max),
                f64::from(bbox.y_max),
            ]),
        ),
    ]))
}

/// The bundled font named `file`.
fn font_id(file: &str) -> Result<FontId, ProbeError> {
    let db = database();
    db.entries()
        .iter()
        .map(|entry| entry.font)
        .find(|&font| db.source(font).and_then(FontSource::file_name) == Some(file))
        .ok_or_else(|| ProbeError::BadArgs(format!("unknown font file {file:?}")))
}

fn layout_fonts(args: Args<'_>) -> Outcome {
    let db = database();
    let fonts = args
        .json("files")?
        .as_array()
        .unwrap_or(&[])
        .iter()
        .map(|file| {
            file.as_str()
                .ok_or_else(|| ProbeError::BadArgs(format!("bad file {file}")))
                .and_then(font_id)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let chain = db.font_chain(&fonts, true).map_err(rejected)?;
    let mut items = Vec::new();
    for line in args.str("text")?.split('\n') {
        let layout = chain
            .layout(line, 12.0, 72.0, &ShapeOptions::default(), Hinting::None)
            .map_err(rejected)?;
        for glyph in &layout.glyphs {
            let c = line[glyph.cluster..].chars().next().unwrap_or_default();
            let file = chain
                .font_id(glyph.face)
                .and_then(|id| db.source(id))
                .and_then(FontSource::file_name)
                .ok_or_else(|| ProbeError::rejected("a glyph from outside the database"))?;
            items.push(Json::Array(vec![
                Json::from(c.to_string()),
                Json::from(file),
            ]));
        }
    }
    Ok(Json::Array(items))
}
