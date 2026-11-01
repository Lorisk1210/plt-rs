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
use pltrs_text::{AfmDirectory, FontSource, TextError};

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

/// Checks that text can be set in the font file `path` (or the bundled
/// font so named), as matplotlib finds out when it opens it:
/// `FileNotFoundError` for a file that does not exist, `OSError` for one
/// that cannot be read, `RuntimeError` for one that holds no font.
pub(super) fn check_font_file(path: &std::path::Path) -> PyResult<()> {
    match pltrs_text::TextEngine::global().database().file_faces(path) {
        Ok(_) => Ok(()),
        Err(pltrs_font::FontError::Io { path, kind }) => {
            let (exception, errno) = if kind == std::io::ErrorKind::NotFound {
                (
                    ExceptionKind::FileNotFoundError,
                    "[Errno 2] No such file or directory",
                )
            } else {
                (ExceptionKind::OSError, "cannot read the font file")
            };
            Err(PyErr::new(
                exception,
                format!("{errno}: '{}'", path.display()),
            ))
        }
        Err(err) => Err(PyErr::runtime_error(format!(
            "In FT2Font: Can not load face ({err})"
        ))),
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

/// `_find_fonts_by_props(prop, fallback_to_default)`: the best file of
/// each family that matches, in family order.
fn find_fonts(props: PropsTuple, fallback: bool) -> PyResult<Vec<String>> {
    let props = properties(props)?;
    Ok(FontManager::global().find_fonts(&props, &pltrs::rc::params(), fallback)?)
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

/// The fonts of the TrueType (`"ttf"`) or AFM (`"afm"`) font list.
fn font_list(fontext: String) -> PyResult<Vec<EntryTuple>> {
    let entries = FontManager::global().font_list(font_ext(&fontext)?);
    Ok(entries.into_iter().map(entry_tuple).collect())
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
    let directory = match directory.as_deref() {
        None => None,
        Some("pdfcorefonts") => Some(AfmDirectory::PdfCore),
        Some("afm") => Some(AfmDirectory::PostScript),
        Some(other) => {
            return Err(PyErr::value_error(format!(
                "no AFM font directory {other:?}"
            )));
        }
    };
    Ok(FontManager::global()
        .best_afm_entry(&props, &pltrs::rc::params(), directory)
        .map(|(entry, score)| (entry_tuple(entry), score)))
}

/// The family names of the TrueType fonts.
fn font_names() -> PyResult<Vec<String>> {
    Ok(FontManager::global().font_names())
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

/// matplotlib's `get_data_path`: the data directory holding the fonts
/// pltrs bundles as files.
fn data_path() -> PyResult<String> {
    pltrs::config::data_dir()
        .map(|dir| dir.display().to_string())
        .map_err(|err| PyErr::runtime_error(err.to_string()))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("data_path", data_path)?;
    module.add("type1_name", type1_name)?;
    module.add("alternative_family_names", alternative_family_names)?;
    module.add("find_fonts", find_fonts)?;
    module.add("best_font", best_font)?;
    module.add("best_afm_font", best_afm_font)?;
    module.add("font_list", font_list)?;
    module.add("font_names", font_names)?;
    module.add("add_entry", add_entry)?;
    module.add("remove_entry", remove_entry)?;
    module.add("score_weight", score_weight)?;
    module.add("parse_pattern", parse_pattern)?;
    module.add("escape_family", escape_family)?;
    module.add("escape_value", escape_value)
}
