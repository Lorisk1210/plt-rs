//! Glyphs of OpenType fonts with CFF and CFF2 outlines and of a variable
//! TrueType font at each hinting mode, as matplotlib's `FT2Font` loads
//! them (area `font_formats`): `FreeType`'s CFF hinter, its autohinter, and
//! unhinted outlines, variable fonts at their default instance. The fonts
//! are fixtures of `pltrs-font`'s tests.

use std::path::PathBuf;

use pltrs_font::{
    AutohintTarget, Autohinter, Face, GlyphId, Hinter, Hinting, HintingInstance, Size,
};

use super::text_metrics::PathCodes;
use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("ft2font.FT2Font.load_glyph", load_glyph),
    Op::new("ft2font.FT2Font.metrics", metrics),
];

/// The bytes of the fixture font the `file` argument names.
fn fixture(args: Args<'_>) -> Result<Vec<u8>, ProbeError> {
    let file = args.str("file")?;
    if file.contains(['/', '\\']) {
        return Err(ProbeError::BadArgs(format!("not a fixture: {file:?}")));
    }
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../pltrs-font/tests/fixtures")
        .join(file);
    std::fs::read(&path).map_err(ProbeError::rejected)
}

/// The hinting mode of a `LoadFlags` name.
fn hinting(flags: &str) -> Result<Hinting, ProbeError> {
    Ok(match flags {
        "NO_HINTING" => Hinting::None,
        "DEFAULT" => Hinting::Default,
        "NO_AUTOHINT" => Hinting::Native,
        "FORCE_AUTOHINT" => Hinting::Auto,
        _ => return Err(ProbeError::BadArgs(format!("unsupported flags {flags:?}"))),
    })
}

/// `ft2font.FT2Font.load_glyph`: the glyph's hinted advance (none when
/// unhinted), linear advance and path.
fn load_glyph(args: Args<'_>) -> Outcome {
    let data = fixture(args)?;
    let face = Face::parse(&data, 0).map_err(ProbeError::rejected)?;
    let glyph = u16::try_from(args.usize("glyph")?)
        .map(GlyphId)
        .map_err(ProbeError::rejected)?;
    let size = Size::new(&face, args.f64("size")?, args.f64("dpi")?);
    let mut path = PathCodes::default();
    let hinted = match hinting(args.str("flags")?)?.hinter(&face, true) {
        Hinter::Unhinted => None,
        Hinter::Bytecode => Some(HintingInstance::new(&face, &size).load_glyph(&face, glyph)),
        Hinter::Autohinter => {
            Some(Autohinter::new(&face).load_glyph(&face, &size, glyph, AutohintTarget::Normal))
        }
        Hinter::Cff => Some(face.load_cff_hinted(&size, glyph)),
    }
    .transpose()
    .map_err(ProbeError::rejected)?;
    let advance = if let Some(hinted) = hinted {
        hinted.outline(&mut path);
        f64::from(hinted.advance()).into()
    } else {
        size.outline(&face, glyph, &mut path)
            .map_err(ProbeError::rejected)?;
        Json::Null
    };
    Ok(Json::Object(vec![
        ("advance".into(), advance),
        (
            "linear".into(),
            (size.linear_advance(&face, glyph) as f64).into(),
        ),
        ("path".into(), path.finish()),
    ]))
}

/// `ft2font.FT2Font` metrics and names (`"UNAVAILABLE"` for a face
/// without a PostScript name, as `FT2Font` has it).
fn metrics(args: Args<'_>) -> Outcome {
    let data = fixture(args)?;
    let face = Face::parse(&data, 0).map_err(ProbeError::rejected)?;
    let name = |name: Option<String>| name.map_or(Json::Null, Json::from);
    // `FT2Font.postscript_name` without one.
    let postscript = face
        .postscript_name()
        .unwrap_or_else(|| "UNAVAILABLE".to_owned());
    Ok(Json::Array(vec![
        f64::from(face.num_glyphs()).into(),
        f64::from(face.units_per_em()).into(),
        f64::from(face.ascender()).into(),
        f64::from(face.descender()).into(),
        f64::from(face.height()).into(),
        name(face.family_name()),
        name(face.style_name()),
        postscript.into(),
    ]))
}
