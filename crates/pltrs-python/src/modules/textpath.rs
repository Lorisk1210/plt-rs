//! `_native.textpath`: the outlines of text (matplotlib's `textpath`), by
//! `pltrs::text::text_path`, `text_path_tex` and `tex_glyphs`.

use std::collections::HashSet;

use pltrs::text::{
    Feature, Language, LanguageRange, set_metrics_cache, tex_glyphs, text_path_shaped,
    text_path_tex,
};
use pltrs_core::{Path, PathCode, Point};
use pltrs_text::{Shaping, TEXT_PATH_FONT_SCALE};

use super::font_manager::FontProps;
use crate::binding::prelude::*;

/// A path as matplotlib's `Path` takes it: `(vertices, codes)`.
type PathArrays = (NdArray, NdArray<u8>);

/// `path` as `(vertices, codes)`, with the codes of a polyline for a path
/// without codes.
fn path_arrays(path: &Path) -> PyResult<PathArrays> {
    let vertices: Vec<f64> = path.vertices().iter().flat_map(|p| [p.x, p.y]).collect();
    let count = path.vertices().len();
    let codes = path.codes().map_or_else(
        || {
            let mut codes = vec![PathCode::LineTo as u8; count];
            if let Some(first) = codes.first_mut() {
                *first = PathCode::MoveTo as u8;
            }
            codes
        },
        |codes| codes.iter().map(|&code| code as u8).collect(),
    );
    Ok((
        NdArray::with_shape(vertices, vec![count, 2])?,
        NdArray::from_vec(codes),
    ))
}

/// A language as `TextToPath.get_text_path` receives it, split as
/// `Text.set_language` splits it: one tag for the whole text, or ranges
/// `(tag, start, end)` of it.
type LanguageSpec = (Option<String>, Vec<(String, usize, usize)>);

/// The shaping of `features` (in `HarfBuzz`'s syntax) and `language`.
fn shaping(features: Option<Vec<String>>, language: LanguageSpec) -> PyResult<Shaping> {
    let features = features
        .unwrap_or_default()
        .iter()
        .map(|feature| {
            feature
                .parse::<Feature>()
                .map_err(|err| PyErr::value_error(err.to_string()))
        })
        .collect::<PyResult<_>>()?;
    let language = match language {
        (Some(tag), _) => Some(Language::from(tag)),
        (None, ranges) if !ranges.is_empty() => Some(
            ranges
                .into_iter()
                .map(|(tag, start, end)| LanguageRange::new(tag.as_str(), start, end))
                .collect(),
        ),
        (None, _) => None,
    };
    Ok(Shaping { features, language })
}

/// `TextToPath.get_text_path(prop, s, ismath, features=..., language=...)`:
/// the outline of `s` at `TextToPath.FONT_SCALE` points, `(vertices,
/// codes)`, typeset by LaTeX when `usetex` is set, as mathtext when `math`
/// is, else with the OpenType `features` and the `language`.
fn get_text_path(
    s: String,
    props: FontProps,
    (usetex, math): (bool, bool),
    features: Option<Vec<String>>,
    language: LanguageSpec,
) -> PyResult<PathArrays> {
    let at = Point::new(0.0, 0.0);
    let size = Some(TEXT_PATH_FONT_SCALE);
    let shaping = shaping(features, language)?;
    // `TextToPath` lays text out by itself, through no renderer's
    // metrics cache (the cache of a renderer drawing meanwhile).
    let installed = set_metrics_cache(None);
    let path = if usetex {
        text_path_tex(at, &s, size, &props.0)
    } else {
        text_path_shaped(at, &s, size, &props.0, &shaping, math)
    };
    set_metrics_cache(installed);
    path_arrays(&path.map_err(|err| PyErr::value_error(err.to_string()))?)
}

/// A glyph of `get_glyphs_tex`: the PostScript name of its font, its index
/// there, its position and its size relative to `FONT_SCALE`.
type TexGlyphFields = (String, u16, f64, f64, f64);

/// The outline of a glyph of `get_glyphs_tex` at `FONT_SCALE` points, by
/// the PostScript name of its font and its index there.
type TexGlyphPath = (String, u16, NdArray, NdArray<u8>);

/// `TextToPath.get_glyphs_tex(prop, s)`: the glyphs of `s` typeset by
/// LaTeX at `FONT_SCALE` points, the outline of each glyph once (as
/// `FT2Font.get_path` gives it after loading the glyph at that size with
/// light hinting), and the rules as `(x, y, height, width)`.
#[expect(
    clippy::type_complexity,
    reason = "the three parts of matplotlib's result"
)]
fn get_glyphs_tex(
    s: String,
    props: FontProps,
) -> PyResult<(
    Vec<TexGlyphFields>,
    Vec<TexGlyphPath>,
    Vec<(f64, f64, f64, f64)>,
)> {
    let (glyphs, boxes) =
        tex_glyphs(&s, &props.0).map_err(|err| PyErr::value_error(err.to_string()))?;
    let mut seen = HashSet::new();
    let mut fields = Vec::with_capacity(glyphs.len());
    let mut paths = Vec::new();
    for glyph in &glyphs {
        let name = glyph
            .file
            .face
            .postscript_name()
            .unwrap_or("UNAVAILABLE")
            .to_owned();
        let index = glyph.index.0;
        if seen.insert((name.clone(), index)) {
            let outline = glyph
                .path_outline()
                .map_err(|err| PyErr::value_error(err.to_string()))?;
            let (vertices, codes) = path_arrays(&outline)?;
            paths.push((name.clone(), index, vertices, codes));
        }
        fields.push((
            name,
            index,
            glyph.x,
            glyph.y,
            glyph.size / TEXT_PATH_FONT_SCALE,
        ));
    }
    let boxes = boxes
        .iter()
        .map(|b| (b.x, b.y, b.height, b.width))
        .collect();
    Ok((fields, paths, boxes))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("get_text_path", get_text_path)?;
    module.add("get_glyphs_tex", get_glyphs_tex)
}
