//! Probe operations for the `mpl_font_manager` area: font matching ported
//! from matplotlib's `tests/test_font_manager.py` (see
//! `spec/reference/probes/test_mpl_font_manager.py`): weight scoring and
//! names, `findfont`, `FontProperties`, the bundled font names, character
//! maps and the alternative family names of a face.

use pltrs::fonts::{FontManager, FontProperties, FontWeight};
use pltrs::rc::RcParams;
use pltrs_font::{Face, NameKey, alternative_family_names, bundled_font_data};

use super::text_metrics::{findfont, font_properties_op, weight};
use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("FontManager.score_weight", score_weight),
    Op::new("font_manager._normalize_weight", normalize_weight),
    Op::new("font_manager.findfont", findfont),
    Op::new("font_manager.FontProperties", font_properties_op),
    Op::new("font_manager.FontManager.get_font_names", font_names),
    Op::new("font_manager.get_font", get_font),
    Op::new("ft2font.FT2Font.get_charmap", charmap),
    Op::new("font_manager._get_font_alt_names", alt_names),
];

/// A weight argument: a keyword or a number.
fn weight_arg(args: Args<'_>, key: &str) -> Result<FontWeight, ProbeError> {
    weight(args.json(key)?)
}

fn score_weight(args: Args<'_>) -> Outcome {
    let (w1, w2) = (weight_arg(args, "weight1")?, weight_arg(args, "weight2")?);
    Ok(FontManager::score_weight(w1, w2).into())
}

fn normalize_weight(args: Args<'_>) -> Outcome {
    Ok(f64::from(weight_arg(args, "weight")?.value()).into())
}

/// `get_font_names` of the bundled fonts, without Inter, which pltrs
/// bundles beyond matplotlib's fonts for its modern look.
fn font_names(_: Args<'_>) -> Outcome {
    let names = FontManager::bundled().font_names();
    Ok(Json::from(
        names
            .into_iter()
            .filter(|name| !name.starts_with("Inter"))
            .collect::<Vec<_>>(),
    ))
}

/// `get_font(path)`: opening a font file by path.
fn get_font(args: Args<'_>) -> Outcome {
    let path = std::path::Path::new(args.str("path")?);
    FontProperties::from_file(path, &RcParams::matplotlib()).map_err(ProbeError::rejected)?;
    Ok(path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .into())
}

/// `FT2Font(file).get_charmap()`: the codes of the selected character map
/// that map to a glyph, as `FreeType`'s `FT_Get_First_Char` and
/// `FT_Get_Next_Char` enumerate them (glyph 0 is skipped).
fn charmap(args: Args<'_>) -> Outcome {
    let file = args.str("file")?;
    let data = bundled_font_data(file)
        .ok_or_else(|| ProbeError::BadArgs(format!("{file} is not bundled")))?;
    let face = Face::parse(data, 0).map_err(ProbeError::rejected)?;
    let mappings: Vec<(u32, u16)> = face
        .mappings()
        .into_iter()
        .map(|(code, glyph)| (code, glyph.0))
        .filter(|&(_, glyph)| glyph != 0)
        .collect();
    let codes = args
        .json("codes")?
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs("codes is not a list".into()))?;
    let glyphs = codes
        .iter()
        .map(|code| {
            let code = code
                .as_i64()
                .and_then(|code| u32::try_from(code).ok())
                .ok_or_else(|| ProbeError::BadArgs(format!("bad code {code}")))?;
            mappings
                .iter()
                .find(|&&(mapped, _)| mapped == code)
                .map(|&(_, glyph)| f64::from(glyph))
                .ok_or_else(|| ProbeError::rejected(format!("code {code} is not mapped")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json::Object(vec![
        ("size".into(), (mappings.len() as f64).into()),
        ("glyphs".into(), glyphs.into()),
    ]))
}

/// One `[platform_id, encoding_id, language_id, name_id, text]` record.
fn name_record(record: &Json) -> Result<(NameKey, u16, String), ProbeError> {
    let bad = || ProbeError::BadArgs(format!("bad name record {record}"));
    let items = record.as_array().ok_or_else(bad)?;
    let [platform, encoding, language, name_id, text] = items else {
        return Err(bad());
    };
    let number = |value: &Json| {
        value
            .as_f64()
            .and_then(|v| u16::try_from(v as i64).ok())
            .ok_or_else(bad)
    };
    let key = NameKey {
        platform_id: number(platform)?,
        encoding_id: number(encoding)?,
        language_id: number(language)?,
    };
    let text = text.as_str().ok_or_else(bad)?.to_owned();
    Ok((key, number(name_id)?, text))
}

/// `_get_font_alt_names` of a face whose `name` table holds the `sfnt`
/// records (none for `null`).
fn alt_names(args: Args<'_>) -> Outcome {
    let records = match args.get("sfnt") {
        None => Vec::new(),
        Some(sfnt) => sfnt
            .as_array()
            .ok_or_else(|| ProbeError::BadArgs("sfnt is not a list".into()))?
            .iter()
            .map(name_record)
            .collect::<Result<_, _>>()?,
    };
    let lookup = |key: NameKey, name_id: u16| {
        records
            .iter()
            .find(|(k, id, _)| *k == key && *id == name_id)
            .map(|(_, _, text)| text.clone())
    };
    Ok(Json::Array(
        alternative_family_names(args.str("primary")?, lookup)
            .into_iter()
            .map(|(name, weight)| Json::Array(vec![name.into(), f64::from(weight).into()]))
            .collect(),
    ))
}
