//! Probe operations for the `mathtext` area: matplotlib's
//! `MathTextParser("path").parse` on the expressions of its own mathtext
//! tests, in every fontset, compared glyph by glyph and rule by rule.
//!
//! The reference configuration and the result encoding are documented in
//! `spec/reference/probes/test_mathtext.py`. Fonts come from matplotlib's
//! bundled fonts only, as in the reference.

use plt::fonts::{FontProperties, FontSize};
use plt::rc::RcParams;
use plt::text::math_style;
use plt_text::TextEngine;
use plt_text::mathtext::MathLayout;

use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError, encode};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[Op::new("mathtext.MathTextParser.parse", parse)];

/// The reference rcParams (`matplotlib.testing.setup()` plus
/// `text.hinting = none`) with `mathtext.fontset` set.
fn reference_rc(fontset: &str) -> Result<RcParams, ProbeError> {
    let mut rc = RcParams::matplotlib();
    for (key, value) in [
        ("font.family", "DejaVu Sans"),
        ("text.hinting", "none"),
        ("mathtext.fontset", fontset),
    ] {
        rc.set(key, value).map_err(ProbeError::rejected)?;
    }
    Ok(rc)
}

/// `MathTextParser("path").parse(s, dpi, FontProperties(size=size))` with
/// the probe's fontset, encoded as the reference does.
fn parse(args: Args<'_>) -> Outcome {
    let rc = reference_rc(args.str("fontset")?)?;
    let mut font = FontProperties::from_rc(&rc);
    font.set_size(FontSize::Points(args.f64("size")?), rc.font_size);
    let style = math_style(&font, args.f64("dpi")?, &rc);
    let engine = TextEngine::bundled();
    let layout = engine
        .typeset_math(args.str("s")?, &style)
        .map_err(ProbeError::rejected)?;
    Ok(encode_layout(engine, &layout))
}

/// `{"box", "extent", "fonts", "glyphs", "rects"}` of a typeset expression:
/// the unrounded box, the `VectorParse` extents, the font files in order of
/// first use and the glyphs and rules as `VectorParse` places them.
fn encode_layout(engine: &TextEngine, layout: &MathLayout) -> Json {
    let run = layout.to_text_run();
    let mut fonts: Vec<&str> = Vec::new();
    let glyphs = run
        .glyphs
        .iter()
        .zip(&layout.glyphs)
        .map(|(placed, glyph)| {
            let file = engine
                .database()
                .source(glyph.font)
                .and_then(|source| source.file_name())
                .unwrap_or_default();
            let index = fonts.iter().position(|f| *f == file).unwrap_or_else(|| {
                fonts.push(file);
                fonts.len() - 1
            });
            Json::Array(vec![
                Json::Integer(i64::try_from(index).unwrap_or(i64::MAX)),
                glyph.size.into(),
                Json::Integer(glyph.code.into()),
                placed.x.into(),
                placed.y.into(),
            ])
        })
        .collect();
    // `TextRun` rects are corner boxes; read the exact sizes from the
    // layout and apply `VectorParse`'s flip to upward y.
    let top = layout.height.ceil();
    let rects = layout
        .rects
        .iter()
        .map(|r| Json::from(vec![r.x, top - (r.y + r.height), r.width, r.height]))
        .collect();
    encode::object([
        (
            "box",
            Json::from(vec![layout.width, layout.height, layout.depth]),
        ),
        (
            "extent",
            Json::from(vec![run.width, run.height, run.descent]),
        ),
        ("fonts", Json::from(fonts)),
        ("glyphs", Json::Array(glyphs)),
        ("rects", Json::Array(rects)),
    ])
}
