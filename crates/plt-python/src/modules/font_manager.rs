//! `_native.font_manager`: finding fonts (matplotlib's `font_manager` and
//! `_fontconfig_pattern`), by `plt::fonts`.
//!
//! Font properties cross as a tuple `(families, style, variant, weight,
//! stretch, size, math_fontfamily, file)`, the weight and stretch as a
//! number or a keyword, as `FontProperties` keeps them.

use std::path::PathBuf;

use plt::fonts::{
    FontExt, FontListEntry, FontManager, FontProperties, FontStretch, FontWeight,
    FontconfigPattern, parse_math_fontfamily, parse_style, parse_variant,
};
use plt_text::TextError;

use crate::binding::prelude::*;

impl From<TextError> for PyErr {
    /// `ValueError` for fonts that cannot be found or read.
    fn from(err: TextError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

/// A weight or stretch given as a number or a keyword.
enum Scale {
    Number(i64),
    Keyword(String),
}

impl FromPy<'_> for Scale {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        match object.extract::<String>() {
            Ok(keyword) => Ok(Self::Keyword(keyword)),
            Err(_) => Ok(Self::Number(object.extract()?)),
        }
    }
}

/// The weight of a number or keyword.
fn weight(scale: &Scale) -> PyResult<FontWeight> {
    let invalid = || PyErr::value_error("weight is invalid");
    match scale {
        Scale::Number(value) => FontWeight::from_number(*value).map_err(|_| invalid()),
        Scale::Keyword(name) => FontWeight::from_name(name).ok_or_else(invalid),
    }
}

/// The stretch of a number or keyword.
fn stretch(scale: &Scale) -> PyResult<FontStretch> {
    let invalid = || PyErr::value_error("stretch is invalid");
    match scale {
        Scale::Number(value) => FontStretch::from_number(*value).map_err(|_| invalid()),
        Scale::Keyword(name) => FontStretch::from_name(name).ok_or_else(invalid),
    }
}

/// `FontProperties` as the Python class keeps them.
type PropsTuple = (
    Vec<String>,
    (String, String),
    (Scale, Scale, f64),
    (String, Option<PathBuf>),
);

/// The font properties a tuple describes.
fn properties(
    (families, (style, variant), (weight_spec, stretch_spec, size), (math, file)): PropsTuple,
) -> PyResult<FontProperties> {
    let invalid = |err: plt::fonts::FontPropertyError| PyErr::value_error(err.to_string());
    let mut props = FontProperties::from_rc(&plt::rc::params());
    props.families = families;
    props.style = parse_style(&style).map_err(invalid)?;
    props.variant = parse_variant(&variant).map_err(invalid)?;
    props.weight = weight(&weight_spec)?;
    props.stretch = stretch(&stretch_spec)?;
    props.size = size;
    props.math_fontfamily = parse_math_fontfamily(&math).map_err(invalid)?;
    props.file = file;
    Ok(props)
}

/// matplotlib's `fontext`: `"ttf"` or `"afm"` (and their synonyms).
fn font_ext(name: &str) -> PyResult<FontExt> {
    match name {
        "ttf" | "otf" | "ttc" => Ok(FontExt::Ttf),
        "afm" => Ok(FontExt::Afm),
        _ => Err(PyErr::value_error(format!(
            "unknown font extension {name:?}"
        ))),
    }
}

/// `findfont(prop, fontext, fallback_to_default)`: the file of the font
/// that best matches the properties, and its face index.
fn find_font(props: PropsTuple, fontext: String, fallback: bool) -> PyResult<String> {
    let props = properties(props)?;
    Ok(FontManager::global().find_font(
        &props,
        &plt::rc::params(),
        font_ext(&fontext)?,
        fallback,
    )?)
}

/// `_find_fonts_by_props(prop, fallback_to_default)`: the best file of
/// each family that matches, in family order.
fn find_fonts(props: PropsTuple, fallback: bool) -> PyResult<Vec<String>> {
    let props = properties(props)?;
    Ok(FontManager::global().find_fonts(&props, &plt::rc::params(), fallback)?)
}

/// One font of a font list: `(file, index, name, (style, variant, weight,
/// stretch))`.
type EntryTuple = (String, u32, String, (String, String, String, String));

/// The fonts of the TrueType (`"ttf"`) or AFM (`"afm"`) font list, the
/// weight and stretch as their keyword when they have one, else as their
/// number.
fn font_list(fontext: String) -> PyResult<Vec<EntryTuple>> {
    let entries = FontManager::global().font_list(font_ext(&fontext)?);
    Ok(entries
        .into_iter()
        .map(|entry: FontListEntry| {
            let weight = entry
                .weight
                .name()
                .map_or_else(|| entry.weight.value().to_string(), str::to_owned);
            let stretch = entry
                .stretch
                .name()
                .map_or_else(|| entry.stretch.value().to_string(), str::to_owned);
            (
                entry.file,
                entry.index,
                entry.name,
                (
                    entry.style.name().to_owned(),
                    entry.variant.name().to_owned(),
                    weight,
                    stretch,
                ),
            )
        })
        .collect())
}

/// The family names of the TrueType fonts.
fn font_names() -> PyResult<Vec<String>> {
    Ok(FontManager::global().font_names())
}

/// Adds the font file at `path` to the fonts text is drawn with.
fn add_font(path: PathBuf) -> PyResult<()> {
    Ok(FontManager::global().add_font(path)?)
}

/// matplotlib's `score_weight(weight1, weight2)`: 0 for equal weights, up
/// to 1 for the most different.
fn score_weight(wanted: Scale, actual: Scale) -> PyResult<f64> {
    Ok(FontManager::score_weight(
        weight(&wanted)?,
        weight(&actual)?,
    ))
}

/// `parse_fontconfig_pattern(pattern)`: the pattern's properties, each
/// with its values, in order.
fn parse_pattern(pattern: String) -> PyResult<Vec<(String, Vec<String>)>> {
    let parsed = FontconfigPattern::parse(&pattern).map_err(|err| {
        PyErr::value_error(format!("Could not parse font string: {pattern:?}\n{err}"))
    })?;
    Ok(parsed.properties().to_vec())
}

/// A family name escaped for a fontconfig pattern.
fn escape_family(family: String) -> PyResult<String> {
    Ok(plt_text::fontconfig::escape_family(&family))
}

/// A property value escaped for a fontconfig pattern.
fn escape_value(value: String) -> PyResult<String> {
    Ok(plt_text::fontconfig::escape_value(&value))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("find_font", find_font)?;
    module.add("find_fonts", find_fonts)?;
    module.add("font_list", font_list)?;
    module.add("font_names", font_names)?;
    module.add("add_font", add_font)?;
    module.add("score_weight", score_weight)?;
    module.add("parse_pattern", parse_pattern)?;
    module.add("escape_family", escape_family)?;
    module.add("escape_value", escape_value)
}
