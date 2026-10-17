//! What the vector backends need to know about a laid-out [`TextRun`]
//! beyond its glyph positions.
//!
//! matplotlib's backends receive the string and font properties and lay
//! the text out themselves; plt-rs renderers receive the finished layout.
//! The pieces of information the PDF, PostScript and SVG writers use are
//! recovered here: the characters behind each glyph (from the font's
//! character map), whether the run is shaped plain text or typeset
//! mathtext (the backends write the two differently), the kerning between
//! consecutive glyphs, and plain-text positions at another font size.

#![allow(
    clippy::float_cmp,
    reason = "reshaped positions must reproduce the run exactly"
)]

use plt_font::{ShapeOptions, Size, shape_at_size};
use plt_text::{PlacedGlyph, TextRun};

use super::font::FontCache;

/// How a run was produced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RunKind {
    /// Shaped text (`_text_helpers.layout`).
    Plain,
    /// Typeset mathtext (`MathTextParser.parse`).
    Math,
}

/// The letters a Latin compatibility ligature is shaped from (the default
/// `liga` feature turns these sequences into one glyph).
fn ligature_letters(c: char) -> Option<&'static str> {
    Some(match c {
        '\u{fb00}' => "ff",
        '\u{fb01}' => "fi",
        '\u{fb02}' => "fl",
        '\u{fb03}' => "ffi",
        '\u{fb04}' => "ffl",
        '\u{fb05}' => "\u{17f}t",
        '\u{fb06}' => "st",
        _ => return None,
    })
}

/// The characters `glyph` renders, like matplotlib's `LayoutItem.char`:
/// the smallest character code mapped to it (the letters of a ligature),
/// or U+FFFD for glyphs outside the character map.
pub(crate) fn glyph_chars(glyph: &PlacedGlyph, fonts: &mut FontCache) -> String {
    let c = fonts
        .get(glyph.font)
        .ok()
        .and_then(|font| font.char_of(glyph.glyph))
        .unwrap_or(char::REPLACEMENT_CHARACTER);
    ligature_letters(c).map_or_else(|| c.to_string(), str::to_owned)
}

/// The text a run renders, recovered glyph by glyph.
pub(crate) fn source_text(run: &TextRun, fonts: &mut FontCache) -> String {
    run.glyphs.iter().map(|g| glyph_chars(g, fonts)).collect()
}

/// Whether `value` is a whole number.
fn is_integral(value: f64) -> bool {
    value.fract() == 0.0
}

/// Classifies `run`.
///
/// Mathtext runs report extents rounded up to whole pixels, so a run with
/// a fractional width, height or depth is plain text; one with rules is
/// mathtext. Otherwise the run is plain text exactly when shaping its
/// characters again reproduces its glyphs and positions.
pub(crate) fn kind(run: &TextRun, fonts: &mut FontCache) -> RunKind {
    if !(is_integral(run.width) && is_integral(run.height) && is_integral(run.descent)) {
        return RunKind::Plain;
    }
    if !run.rects.is_empty() {
        return RunKind::Math;
    }
    let Some(first) = run.glyphs.first() else {
        return RunKind::Plain;
    };
    let reshaped = shaped_positions(run, fonts, first.size, run.dpi);
    let same = reshaped.is_some_and(|positions| {
        positions
            .iter()
            .zip(&run.glyphs)
            .all(|(&(x, y), g)| x == g.x && y == g.y)
    });
    if same { RunKind::Plain } else { RunKind::Math }
}

/// The consecutive glyphs of `run` that come from the same font.
fn font_groups(glyphs: &[PlacedGlyph]) -> Vec<&[PlacedGlyph]> {
    glyphs
        .chunk_by(|a, b| a.font == b.font && a.size == b.size)
        .collect()
}

/// Shapes the characters of a plain-text run again at `points` and `dpi`
/// (font group by font group, as matplotlib's fallback does) and returns
/// the glyph origins in pixels, or `None` when shaping does not reproduce
/// the run's glyphs.
pub(crate) fn shaped_positions(
    run: &TextRun,
    fonts: &mut FontCache,
    points: f64,
    dpi: f64,
) -> Option<Vec<(f64, f64)>> {
    let mut positions = Vec::with_capacity(run.glyphs.len());
    let mut pen = 0i32;
    for group in font_groups(&run.glyphs) {
        let text: String = group.iter().map(|g| glyph_chars(g, fonts)).collect();
        if text.contains(char::REPLACEMENT_CHARACTER) {
            return None;
        }
        let font = fonts.get(group[0].font).ok()?;
        let face = font.face();
        let size = Size::new(face, points, dpi);
        let shaped = shape_at_size(face, &text, &size, &ShapeOptions::default());
        if shaped.len() != group.len()
            || shaped.iter().zip(group).any(|(s, g)| s.glyph_id != g.glyph)
        {
            return None;
        }
        for glyph in shaped {
            positions.push((
                f64::from(pen + glyph.x_offset) / 64.0,
                f64::from(glyph.y_offset) / 64.0,
            ));
            pen += glyph.x_advance;
        }
    }
    Some(positions)
}

/// matplotlib's `LayoutItem.prev_kern` for every glyph of a plain run: how
/// far the glyph sits from where the previous glyph's unhinted advance
/// ends (zero for the first glyph).
pub(crate) fn prev_kerns(run: &TextRun, fonts: &mut FontCache) -> Vec<f64> {
    let mut kerns = Vec::with_capacity(run.glyphs.len());
    let mut previous: Option<&PlacedGlyph> = None;
    for glyph in &run.glyphs {
        let kern = previous.map_or(0.0, |prev| {
            let advance = fonts.get(prev.font).map_or(0.0, |font| {
                font.linear_advance(prev.glyph, prev.size, run.dpi)
            });
            glyph.x - prev.x - advance
        });
        kerns.push(kern);
        previous = Some(glyph);
    }
    kerns
}

/// The font size mathtext was typeset at: the largest glyph size (scripts
/// and fractions only shrink glyphs).
pub(crate) fn base_size(run: &TextRun) -> Option<f64> {
    run.glyphs.iter().map(|g| g.size).reduce(f64::max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use plt_text::{FontQuery, FontRequest};

    fn plain(text: &str, size: f64) -> TextRun {
        FontCache::default()
            .engine()
            .layout_line(
                text,
                &FontRequest::new(FontQuery {
                    size,
                    ..FontQuery::families(["DejaVu Sans"])
                }),
                72.0,
            )
            .expect("bundled font")
    }

    #[test]
    fn plain_runs_are_recognised_and_kerned() {
        let mut fonts = FontCache::default();
        let run = plain("AVA To", 10.0);
        assert_eq!(kind(&run, &mut fonts), RunKind::Plain);
        assert_eq!(source_text(&run, &mut fonts), "AVA To");
        // matplotlib: prev_kern of 'V' after 'A' at 10 pt is -0.6376953125.
        let kerns = prev_kerns(&run, &mut fonts);
        assert_eq!(kerns[0], 0.0);
        assert_eq!(kerns[1], -0.637_695_312_5);
        assert_eq!(kerns[3], 0.002_929_687_5);
    }

    #[test]
    fn ligatures_map_back_to_their_letters() {
        let mut fonts = FontCache::default();
        let run = plain("fit", 10.0);
        assert_eq!(run.glyphs.len(), 2, "fi is one glyph");
        assert_eq!(glyph_chars(&run.glyphs[0], &mut fonts), "fi");
        assert_eq!(source_text(&run, &mut fonts), "fit");
    }

    #[test]
    fn positions_at_another_size() {
        let mut fonts = FontCache::default();
        let run = plain("1.0", 10.0);
        let at_100 = shaped_positions(&run, &mut fonts, 100.0, 72.0).expect("reshapes");
        // matplotlib's SVG output places '.' at 63.625 and '0' at 95.40625.
        assert_eq!(at_100, [(0.0, 0.0), (63.625, 0.0), (95.406_25, 0.0)]);
    }

    #[test]
    fn rules_and_rounded_extents_mean_mathtext() {
        let mut fonts = FontCache::default();
        let mut run = plain("x", 10.0);
        run.width = run.width.ceil();
        run.height = run.height.ceil();
        run.descent = run.descent.ceil();
        run.glyphs[0].y = 0.25;
        assert_eq!(kind(&run, &mut fonts), RunKind::Math);
        run.glyphs.clear();
        assert_eq!(kind(&run, &mut fonts), RunKind::Plain);
        run.rects
            .push(plt_core::Bbox::from_bounds(0.0, 0.0, 1.0, 1.0));
        assert_eq!(kind(&run, &mut fonts), RunKind::Math);
        assert_eq!(base_size(&run), None);
    }
}
