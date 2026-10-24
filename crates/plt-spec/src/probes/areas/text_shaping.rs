//! Probe operations for the `text_shaping` area: layout and extents of a
//! bundled font under OpenType features and languages (`HarfBuzz`'s feature
//! syntax, BCP 47 languages to language systems, language ranges), and
//! the features and language of `Text`.
//!
//! The arguments are documented in
//! `spec/reference/probes/test_text_shaping.py`.

use plt::artist::ArtistHandle;
use plt::figure::Figure;
use plt::text::{Feature, HAlign, Language, LanguageRange, Text};
use plt_font::{Face, FontChain, Hinting, ShapeOptions, TextLayout};

use super::text_metrics::{bundled_font, database, with_rc};
use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError, encode};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("ft2font.FT2Font._layout", layout),
    Op::new("ft2font.FT2Font.set_text", set_text),
    Op::new("text.Text.get_window_extent", window_extent),
    Op::new("text.Text.get_language", get_language),
];

/// The `features` argument: `HarfBuzz` feature strings (absent or null:
/// none); a string `HarfBuzz` rejects is a rejection.
fn features(args: Args<'_>) -> Result<Vec<Feature>, ProbeError> {
    match args.get("features") {
        None | Some(Json::Null) => Ok(Vec::new()),
        Some(Json::Array(items)) => items
            .iter()
            .map(|item| {
                let text = item
                    .as_str()
                    .ok_or_else(|| ProbeError::BadArgs(format!("bad feature {item}")))?;
                text.parse().map_err(ProbeError::rejected)
            })
            .collect(),
        Some(other) => Err(ProbeError::BadArgs(format!("bad features {other}"))),
    }
}

/// One `[language, start, end]` range; a negative index (which no
/// `LanguageRange` can hold) is outside any non-empty text, as libraqm
/// sees it.
fn language_range(item: &Json, text: &str) -> Result<Option<LanguageRange>, ProbeError> {
    let bad = || ProbeError::BadArgs(format!("bad language range {item}"));
    let [tag, start, end] = item.as_array().ok_or_else(bad)? else {
        return Err(bad());
    };
    let index = |value: &Json| value.as_i64().ok_or_else(bad).map(usize::try_from);
    match (index(start)?, index(end)?) {
        (Ok(start), Ok(end)) => Ok(Some(LanguageRange::new(
            tag.as_str().ok_or_else(bad)?,
            start,
            end,
        ))),
        _ if text.is_empty() => Ok(None),
        _ => Err(ProbeError::rejected(format!(
            "language range {item} outside the text"
        ))),
    }
}

/// The `language` argument: a tag, a list of ranges, or none.
fn language(args: Args<'_>, text: &str) -> Result<Option<Language>, ProbeError> {
    match args.get("language") {
        None | Some(Json::Null) => Ok(None),
        Some(Json::String(tag)) => Ok(Some(Language::from(tag.as_str()))),
        Some(Json::Array(items)) => items
            .iter()
            .filter_map(|item| language_range(item, text).transpose())
            .collect::<Result<Language, _>>()
            .map(Some),
        Some(other) => Err(ProbeError::BadArgs(format!("bad language {other}"))),
    }
}

/// `FT2Font(file).set_size(size, dpi)` then the text laid out unhinted
/// with the probe's features and language.
fn laid_out(args: Args<'_>) -> Result<TextLayout, ProbeError> {
    let file = args.str("file")?;
    let text = args.str("text")?;
    let face: Face<'_> = database()
        .face(bundled_font(file)?)
        .map_err(ProbeError::rejected)?;
    let languages = match language(args, text)? {
        Some(language) => language.ranges(text).map_err(ProbeError::rejected)?,
        None => Vec::new(),
    };
    let options = ShapeOptions {
        features: features(args)?,
        languages,
        ..ShapeOptions::default()
    };
    let size = args.opt_f64("size")?.unwrap_or(12.0);
    let dpi = args.opt_f64("dpi")?.unwrap_or(72.0);
    FontChain::new(vec![face])
        .layout(text, size, dpi, &options, Hinting::None)
        .map_err(ProbeError::rejected)
}

/// `FT2Font._layout(text, NO_HINTING, features=..., language=...)`:
/// glyphs and their positions in 1/64 pixels.
fn layout(args: Args<'_>) -> Outcome {
    let glyphs = laid_out(args)?
        .glyphs
        .iter()
        .map(|glyph| {
            Json::from(vec![
                f64::from(glyph.glyph_id.0),
                f64::from(glyph.x),
                f64::from(glyph.y),
            ])
        })
        .collect();
    Ok(Json::Array(glyphs))
}

/// `FT2Font.set_text(...)` then `get_width_height()` and `get_descent()`.
fn set_text(args: Args<'_>) -> Outcome {
    let laid_out = laid_out(args)?;
    Ok(Json::from(vec![
        f64::from(laid_out.width),
        f64::from(laid_out.height),
        f64::from(laid_out.descent),
    ]))
}

/// A probe's `ha` argument.
fn halign(args: Args<'_>) -> Result<HAlign, ProbeError> {
    match args.opt_str("ha")?.unwrap_or("left") {
        "left" => Ok(HAlign::Left),
        "center" => Ok(HAlign::Center),
        "right" => Ok(HAlign::Right),
        other => Err(ProbeError::BadArgs(format!("bad ha {other:?}"))),
    }
}

/// `Text.get_window_extent()` of a text given features and a language.
fn window_extent(args: Args<'_>) -> Outcome {
    with_rc(args, || {
        let dpi = args.opt_f64("dpi")?.unwrap_or(100.0);
        let fig = Figure::with_size(6.4, 4.8, dpi);
        let content = args.str("text")?;
        let text = fig.text(0.5, 0.5, content);
        text.set_halign(halign(args)?);
        if args.contains("features") {
            text.set_fontfeatures(features(args)?);
        }
        if args.contains("language") {
            text.set_language(language(args, content)?);
        }
        let extent = text
            .window_extent()
            .ok_or_else(|| ProbeError::rejected("the text has no extent"))?;
        Ok(encode::bbox(&extent))
    })
}

/// The JSON of `Text.get_language()`.
fn language_json(language: Option<&Language>) -> Json {
    match language {
        None => Json::Null,
        Some(Language::Tag(tag)) => tag.as_str().into(),
        Some(Language::Ranges(ranges)) => Json::Array(
            ranges
                .iter()
                .map(|range| {
                    Json::Array(vec![
                        range.tag.as_str().into(),
                        Json::Integer(i64::try_from(range.start).unwrap_or(i64::MAX)),
                        Json::Integer(i64::try_from(range.end).unwrap_or(i64::MAX)),
                    ])
                })
                .collect(),
        ),
    }
}

/// `Text.get_language()` of a text made under `text.language`, after an
/// optional `set_language`.
fn get_language(args: Args<'_>) -> Outcome {
    plt::rc::context(|| {
        let rc_language = args.opt_str("rc_language")?.unwrap_or("None");
        plt::rc::set("text.language", rc_language).map_err(ProbeError::rejected)?;
        let fig = Figure::new();
        let text: Text = fig.text(0.0, 0.0, "x");
        if args.opt_bool("set")?.unwrap_or(false) {
            text.set_language(language(args, "x")?);
        }
        Ok(language_json(text.language().as_ref()))
    })
}
