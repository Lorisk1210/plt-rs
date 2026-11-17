//! `_native.font_manager`: finding fonts (matplotlib's `font_manager` and
//! `_fontconfig_pattern`), by `pltrs::fonts`.
//!
//! Font properties cross as a tuple `(families, style, variant, weight,
//! stretch, size, math_fontfamily, file)`, the weight and stretch as a
//! number or a keyword, as `FontProperties` keeps them.

use std::path::PathBuf;

use pltrs::fonts::{
    FontExt, FontListEntry, FontManager, FontProperties, FontStretch, FontWeight,
    FontconfigPattern, parse_math_fontfamily, parse_style, parse_variant,
};
use pltrs_text::tex::TexError;
use pltrs_text::{AfmDirectory, AfmFont, FontSource, TextError};

use crate::binding::prelude::*;

impl From<TextError> for PyErr {
    /// `ValueError` for fonts that cannot be found or read; matplotlib's
    /// `RuntimeError` for a font size `FreeType` refuses; for TeX that is
    /// not installed, fails on the text or lacks a file, the exception
    /// matplotlib's `TexManager` and `dviread` raise (`RuntimeError`,
    /// `FileNotFoundError`).
    fn from(err: TextError) -> Self {
        match err {
            TextError::FontSize(_) => PyErr::runtime_error(err.to_string()),
            TextError::Tex(
                tex @ (TexError::Missing { .. }
                | TexError::Failed { .. }
                | TexError::NotFound { .. }),
            ) => tex.into(),
            err => PyErr::value_error(err.to_string()),
        }
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

/// Font properties crossing as the tuple `FontManager._props` makes of a
/// Python `FontProperties` (see the module documentation), both ways.
pub(super) struct FontProps(pub(super) FontProperties);

impl FromPy<'_> for FontProps {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        properties(object.extract()?).map(Self)
    }
}

/// A weight or stretch as the keyword it was given as, else its number.
fn scale_value<'py>(py: Python<'py>, name: Option<&str>, value: u16) -> PyResult<Bound<'py>> {
    match name {
        Some(name) => name.into_py(py),
        None => i64::from(value).into_py(py),
    }
}

impl IntoPy for FontProps {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        let props = self.0;
        let file = props.file.map(|file| file.display().to_string());
        (
            props.families,
            (props.style.name(), props.variant.name()),
            (
                scale_value(py, props.weight.name(), props.weight.value())?,
                scale_value(py, props.stretch.name(), props.stretch.value())?,
                props.size,
            ),
            (props.math_fontfamily.name(), file),
        )
            .into_py(py)
    }
}

/// The font properties a tuple describes.
fn properties(
    (families, (style, variant), (weight_spec, stretch_spec, size), (math, file)): PropsTuple,
) -> PyResult<FontProperties> {
    let invalid = |err: pltrs::fonts::FontPropertyError| PyErr::value_error(err.to_string());
    let mut props = FontProperties::from_rc(&pltrs::rc::params());
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

/// One font of a font list: `(file, index, name, (style, variant, weight,
/// stretch))`.
type EntryTuple = (String, u32, String, (String, String, String, String));

/// A font-list entry as a tuple, the weight and stretch as their keyword
/// when they have one, else as their number.
fn entry_tuple(entry: FontListEntry) -> EntryTuple {
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
}

/// A `FontEntry` as the Python side gives it: `(file, index, name,
/// (style, variant, weight, stretch))`, the weight and stretch as numbers
/// or keywords.
struct PyFontEntry(FontListEntry);

impl FromPy<'_> for PyFontEntry {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let (file, index, name, (style, variant, weight_spec, stretch_spec)): (
            PathBuf,
            u32,
            String,
            (String, String, Scale, Scale),
        ) = object.extract()?;
        let invalid = |err: pltrs::fonts::FontPropertyError| PyErr::value_error(err.to_string());
        Ok(Self(FontListEntry {
            file: file.display().to_string(),
            index,
            name,
            style: parse_style(&style).map_err(invalid)?,
            variant: parse_variant(&variant).map_err(invalid)?,
            weight: weight(&weight_spec)?,
            stretch: stretch(&stretch_spec)?,
        }))
    }
}

/// `ttflist.append(entry)`: the face the entry names is matched by its
/// name, style, weight and stretch from now on.
fn add_entry(entry: PyFontEntry) -> PyResult<()> {
    Ok(FontManager::global().add_entry(&entry.0)?)
}

/// `ttflist.remove(entry)`: whether the fonts had the entry.
fn remove_entry(entry: PyFontEntry) -> PyResult<bool> {
    Ok(FontManager::global().remove_entry(&entry.0)?)
}

/// Fits the font list to the look figures take: in matplotlib's, the
/// fonts of pltrs' modern look (Inter) are left out of it, as matplotlib
/// ships none. Whether they were.
fn fit_font_list_to_look() -> PyResult<bool> {
    let matplotlib = pltrs::look::appearance().figures == pltrs::look::FigureLook::Matplotlib;
    if matplotlib {
        FontManager::global().leave_out_modern_look_fonts();
    }
    Ok(matplotlib)
}

/// Whether the bundled font file `file_name` is one of pltrs' modern look
/// (Inter).
fn is_modern_look_font(file_name: String) -> PyResult<bool> {
    Ok(pltrs_text::is_modern_look_font(&file_name))
}

/// The fonts of the TrueType (`"ttf"`) or AFM (`"afm"`) font list, the
/// files of AFM fonts as their paths in the data directory.
fn font_list(fontext: String) -> PyResult<Vec<EntryTuple>> {
    let ext = font_ext(&fontext)?;
    FontManager::global()
        .font_list(ext)
        .into_iter()
        .map(|entry| match ext {
            FontExt::Ttf => Ok(entry_tuple(entry)),
            FontExt::Afm => afm_entry_tuple(entry),
        })
        .collect()
}

/// An entry of the AFM font list as a tuple, its file as its path in the
/// data directory: the AFM fonts are in two of its directories
/// (`fonts/afm`, `fonts/pdfcorefonts`).
fn afm_entry_tuple(mut entry: FontListEntry) -> PyResult<EntryTuple> {
    if let Some(font) = AfmFont::named(&entry.file) {
        let path = pltrs::config::data_dir()
            .map_err(|err| PyErr::runtime_error(err.to_string()))?
            .join("fonts")
            .join(font.relative_path());
        entry.file = path.display().to_string();
    }
    Ok(entry_tuple(entry))
}

/// The best TrueType entry for `props` and its score (10 or more when no
/// family matched), among the fonts under `directory` when one is given,
/// the bundled fonts counting as files of `bundled_dir`: what
/// matplotlib's `_findfont_cached` finds before deciding what to log and
/// whether to fall back. `None` when no font is under `directory`.
fn best_font(
    props: PropsTuple,
    directory: Option<PathBuf>,
    bundled_dir: PathBuf,
) -> PyResult<Option<(EntryTuple, f64)>> {
    let props = properties(props)?;
    let within = |source: &FontSource| {
        directory.as_ref().is_none_or(|directory| match source {
            FontSource::File(path) => path.starts_with(directory),
            FontSource::Bundled(name) => bundled_dir.join(name).starts_with(directory),
        })
    };
    Ok(FontManager::global()
        .best_ttf_entry(&props, &pltrs::rc::params(), within)
        .map(|(entry, score)| (entry_tuple(entry), score)))
}

/// The best AFM entry for `props` and its score (10 or more when no
/// family matched), among the fonts of `directory` (`"pdfcorefonts"` or
/// `"afm"`, matplotlib's two AFM directories; all fonts for `None`):
/// `best_font`'s counterpart for `findfont(fontext="afm")`.
fn best_afm_font(
    props: PropsTuple,
    directory: Option<String>,
) -> PyResult<Option<(EntryTuple, f64)>> {
    let props = properties(props)?;
    let directory = directory
        .map(|name| {
            AfmDirectory::ALL
                .into_iter()
                .find(|directory| directory.name() == name)
                .ok_or_else(|| PyErr::value_error(format!("no AFM font directory {name:?}")))
        })
        .transpose()?;
    FontManager::global()
        .best_afm_entry(&props, &pltrs::rc::params(), directory)
        .map(|(entry, score)| Ok((afm_entry_tuple(entry)?, score)))
        .transpose()
}

/// The family names of the TrueType fonts.
fn font_names() -> PyResult<Vec<String>> {
    Ok(FontManager::global().font_names())
}

/// matplotlib's `score_family(families, family2)`: where `family2` comes
/// in `families` (0.0 at the head, 1.0 when it does not come), the generic
/// families standing for the fonts the current rcParams list for them.
fn score_family(families: Vec<String>, family: String) -> PyResult<f64> {
    Ok(FontManager::score_family(
        &families,
        &family,
        &pltrs::rc::params(),
    ))
}

/// The score of two values `parse` reads, compared by `score`; values it
/// cannot read compare as matplotlib compares them, as strings: 0.0 when
/// equal, else 1.0.
fn score_parsed<T, E>(
    wanted: &str,
    actual: &str,
    parse: impl Fn(&str) -> Result<T, E>,
    score: impl Fn(T, T) -> f64,
) -> f64 {
    match (parse(wanted), parse(actual)) {
        (Ok(wanted), Ok(actual)) => score(wanted, actual),
        _ if wanted == actual => 0.0,
        _ => 1.0,
    }
}

/// matplotlib's `score_style(style1, style2)`.
fn score_style(wanted: String, actual: String) -> PyResult<f64> {
    Ok(score_parsed(
        &wanted,
        &actual,
        parse_style,
        FontManager::score_style,
    ))
}

/// matplotlib's `score_variant(variant1, variant2)`.
fn score_variant(wanted: String, actual: String) -> PyResult<f64> {
    Ok(score_parsed(
        &wanted,
        &actual,
        parse_variant,
        FontManager::score_variant,
    ))
}

/// matplotlib's `score_stretch(stretch1, stretch2)`: a string that is
/// neither a keyword nor a number counts as `normal`, as in matplotlib.
fn score_stretch(wanted: Scale, actual: Scale) -> PyResult<f64> {
    let value = |scale: &Scale| match scale {
        Scale::Keyword(text) => Ok(text.parse().unwrap_or(FontStretch::NORMAL)),
        Scale::Number(_) => stretch(scale),
    };
    Ok(FontManager::score_stretch(value(&wanted)?, value(&actual)?))
}

/// matplotlib's `score_size(size1, size2)` of sizes in points, `actual`
/// `None` for a scalable font.
fn score_size(wanted: f64, actual: Option<f64>) -> PyResult<f64> {
    Ok(FontManager::score_size(wanted, actual))
}

/// `parse_fontconfig_pattern(pattern)`: the pattern's properties, each
/// with its values, in order.
fn parse_pattern(pattern: String) -> PyResult<Vec<(String, Vec<String>)>> {
    // matplotlib raises pyparsing's explanation of the failure.
    let parsed = FontconfigPattern::try_parse(&pattern)
        .map_err(|err| PyErr::value_error(format!("\n{}", err.explanation())))?;
    Ok(parsed.properties().to_vec())
}

/// A family name escaped for a fontconfig pattern.
fn escape_family(family: String) -> PyResult<String> {
    Ok(pltrs_text::fontconfig::escape_family(&family))
}

/// A property value escaped for a fontconfig pattern.
fn escape_value(value: String) -> PyResult<String> {
    Ok(pltrs_text::fontconfig::escape_value(&value))
}

/// `_get_font_alt_names(font, primary)` over a face's decoded name
/// records `((platform, encoding, language, name id), string)`: the other
/// family names it advertises, each with the weight of its subfamily.
fn alternative_family_names(
    primary: String,
    records: Vec<((u16, u16, u16, u16), String)>,
) -> PyResult<Vec<(String, u16)>> {
    Ok(pltrs_font::alternative_family_names(
        &primary,
        |key, name_id| {
            records
                .iter()
                .find(|((platform, encoding, language, id), _)| {
                    (*platform, *encoding, *language, *id)
                        == (key.platform_id, key.encoding_id, key.language_id, name_id)
                })
                .map(|(_, name)| name.clone())
        },
    ))
}

/// The Adobe glyph name of the character `c` (`uniXXXX` when it has none),
/// as AFM metrics name glyphs (matplotlib's `uni2type1` lookups).
fn type1_name(c: String) -> PyResult<String> {
    let mut chars = c.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Ok(pltrs_font::type1_name(c).into_owned()),
        _ => Err(PyErr::type_error("type1_name() expects a single character")),
    }
}

/// matplotlib's `uni2type1`: every character with an Adobe glyph name, by
/// code point.
fn uni2type1() -> PyResult<Vec<(u32, &'static str)>> {
    Ok(pltrs_font::uni2type1().to_vec())
}

/// matplotlib's `get_data_path`: the data directory holding the fonts
/// pltrs bundles as files, written the first time it is asked for.
fn data_path() -> PyResult<String> {
    pltrs::config::data_dir()
        .map(|dir| dir.display().to_string())
        .map_err(|err| PyErr::runtime_error(err.to_string()))
}

/// Where the data directory of [`data_path`] is, without writing its
/// files (what importing the package asks for).
fn data_location() -> PyResult<String> {
    pltrs::config::data_dir_location()
        .map(|dir| dir.display().to_string())
        .map_err(|err| PyErr::runtime_error(err.to_string()))
}

/// The factor of the base size the size keyword `name` stands for in
/// `font_manager.font_scalings` (the package registers its lookup as
/// `font_scaling`), if the program added it there.
fn font_scaling(name: &str) -> Option<f64> {
    Python::with_gil(|py| {
        let lookup = crate::binding::lookup(py, "font_scaling").ok()?;
        let scale = lookup.call(&[name.into_py(py).ok()?], &[]).ok()?;
        scale.extract::<Option<f64>>().ok()?
    })
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    pltrs::fonts::set_size_keywords(font_scaling);
    module.add("data_path", data_path)?;
    module.add("data_location", data_location)?;
    module.add("type1_name", type1_name)?;
    module.add("uni2type1", uni2type1)?;
    module.add("alternative_family_names", alternative_family_names)?;
    module.add("best_font", best_font)?;
    module.add("best_afm_font", best_afm_font)?;
    module.add("font_list", font_list)?;
    module.add("fit_font_list_to_look", fit_font_list_to_look)?;
    module.add("is_modern_look_font", is_modern_look_font)?;
    module.add("font_names", font_names)?;
    module.add("add_entry", add_entry)?;
    module.add("remove_entry", remove_entry)?;
    module.add("score_family", score_family)?;
    module.add("score_style", score_style)?;
    module.add("score_variant", score_variant)?;
    module.add("score_stretch", score_stretch)?;
    module.add("score_size", score_size)?;
    module.add("parse_pattern", parse_pattern)?;
    module.add("escape_family", escape_family)?;
    module.add("escape_value", escape_value)
}
