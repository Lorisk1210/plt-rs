//! Font manager probes: the font lists, `findfont`,
//! `_find_fonts_by_props`, `FontProperties` and fontconfig patterns.

use pltrs::fonts::{
    FontExt, FontListEntry, FontManager, FontProperties, FontStretch, FontWeight, FontconfigPattern,
};
use pltrs::rc;

use super::args::{font_dict, font_properties, with_rc};
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError};

/// `fontext`: `ttf` (default) or `afm`.
fn font_ext(args: Args<'_>) -> Result<FontExt, ProbeError> {
    match args.opt_str("fontext")? {
        None | Some("ttf") => Ok(FontExt::Ttf),
        Some("afm") => Ok(FontExt::Afm),
        Some(other) => Err(ProbeError::BadArgs(format!("unknown fontext {other:?}"))),
    }
}

/// A weight as matplotlib stores it: the keyword or the number.
fn weight_json(weight: FontWeight) -> Json {
    weight
        .name()
        .map_or_else(|| Json::from(f64::from(weight.value())), Json::from)
}

/// A stretch as matplotlib stores it: the keyword or the number.
fn stretch_json(stretch: FontStretch) -> Json {
    stretch
        .name()
        .map_or_else(|| Json::from(f64::from(stretch.value())), Json::from)
}

fn entry_json(entry: &FontListEntry) -> Json {
    Json::Array(vec![
        entry.file.as_str().into(),
        Json::from(f64::from(entry.index)),
        entry.name.as_str().into(),
        entry.style.name().into(),
        entry.variant.name().into(),
        weight_json(entry.weight),
        stretch_json(entry.stretch),
        "scalable".into(),
    ])
}

/// `font_manager.font_list`: the bundled font database, without Inter,
/// which pltrs bundles beyond matplotlib's fonts for its modern look.
pub(super) fn font_list(args: Args<'_>) -> Outcome {
    let entries = FontManager::bundled().font_list(font_ext(args)?);
    Ok(Json::Array(
        entries
            .iter()
            .filter(|entry| !entry.file.starts_with("Inter-"))
            .map(entry_json)
            .collect(),
    ))
}

/// The `prop` argument: a font dict, `null`, or a fontconfig pattern.
fn prop(args: Args<'_>) -> Result<FontProperties, ProbeError> {
    match args.get("prop") {
        Some(Json::String(pattern)) => {
            FontProperties::from_pattern(pattern, &rc::params()).map_err(ProbeError::rejected)
        }
        other => font_dict(other),
    }
}

fn fallback(args: Args<'_>) -> Result<bool, ProbeError> {
    Ok(args.opt_bool("fallback_to_default")?.unwrap_or(true))
}

/// `font_manager.findfont`.
pub(crate) fn findfont(args: Args<'_>) -> Outcome {
    with_rc(args, || {
        let props = prop(args)?;
        FontManager::bundled()
            .find_font(&props, &rc::params(), font_ext(args)?, fallback(args)?)
            .map(Json::from)
            .map_err(ProbeError::rejected)
    })
}

/// `font_manager.FontManager._find_fonts_by_props`.
pub(super) fn find_fonts_by_props(args: Args<'_>) -> Outcome {
    with_rc(args, || {
        let props = prop(args)?;
        FontManager::bundled()
            .find_fonts(&props, &rc::params(), fallback(args)?)
            .map(Json::from)
            .map_err(ProbeError::rejected)
    })
}

/// `FontProperties` read back through its getters: family, style,
/// variant, weight, stretch, size in points, math font family and
/// fontconfig pattern.
pub(crate) fn properties_json(props: &FontProperties) -> Json {
    Json::Object(vec![
        ("family".into(), Json::from(props.families.clone())),
        ("style".into(), props.style.name().into()),
        ("variant".into(), props.variant.name().into()),
        ("weight".into(), weight_json(props.weight)),
        ("stretch".into(), stretch_json(props.stretch)),
        ("size".into(), props.size.into()),
        (
            "math_fontfamily".into(),
            props.math_fontfamily.name().into(),
        ),
        ("pattern".into(), props.fontconfig_pattern().into()),
    ])
}

/// `font_manager.FontProperties(**kw)` and its getters.
pub(crate) fn font_properties_op(args: Args<'_>) -> Outcome {
    with_rc(args, || {
        let props = font_properties(args.json("kw")?)?;
        Ok(properties_json(&props))
    })
}

/// `_fontconfig_pattern.parse_fontconfig_pattern`.
pub(crate) fn parse_fontconfig_pattern(args: Args<'_>) -> Outcome {
    let parsed = FontconfigPattern::parse(args.str("pattern")?).map_err(ProbeError::rejected)?;
    Ok(Json::Object(
        parsed
            .properties()
            .iter()
            .map(|(key, values)| (key.clone(), Json::from(values.clone())))
            .collect(),
    ))
}
