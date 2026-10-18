//! What the vector backends need to know about a laid-out [`TextRun`]
//! beyond what it carries: the characters behind each glyph as the font
//! trackers record them, the kerning between consecutive glyphs and the
//! run at the resolution matplotlib's PDF and PostScript backends lay text
//! out at.

use std::borrow::Cow;

use plt_text::{PlacedGlyph, TextError, TextRun, TextSource};

use super::font::FontCache;

/// The resolution of the vector backends' coordinates: one unit per
/// point.
pub(crate) const POINTS_DPI: f64 = 72.0;

/// The characters `glyph` renders, U+FFFD when unknown.
pub(crate) fn glyph_chars<'a>(run: &'a TextRun, glyph: &PlacedGlyph) -> Cow<'a, str> {
    run.glyph_text(glyph).unwrap_or(Cow::Borrowed("\u{fffd}"))
}

/// `run` laid out in points (at 72 dpi), as matplotlib's PDF and
/// PostScript backends lay out the text they receive.
pub(crate) fn in_points<'a>(
    run: &'a TextRun,
    source: &TextSource,
    fonts: &FontCache,
) -> Result<Cow<'a, TextRun>, TextError> {
    if run.dpi == POINTS_DPI {
        Ok(Cow::Borrowed(run))
    } else {
        source
            .layout(fonts.engine(), source.size(), POINTS_DPI)
            .map(Cow::Owned)
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use plt_text::{FontQuery, FontRequest};

    fn plain(text: &str, size: f64, dpi: f64) -> TextRun {
        FontCache::default()
            .engine()
            .layout_line(
                text,
                &FontRequest::new(FontQuery {
                    size,
                    ..FontQuery::families(["DejaVu Sans"])
                }),
                dpi,
            )
            .expect("bundled font")
    }

    #[test]
    fn plain_runs_are_kerned() {
        let mut fonts = FontCache::default();
        let run = plain("AVA To", 10.0, 72.0);
        assert_eq!(glyph_chars(&run, &run.glyphs[1]), "V");
        // matplotlib: prev_kern of 'V' after 'A' at 10 pt is -0.6376953125.
        let kerns = prev_kerns(&run, &mut fonts);
        assert_eq!(kerns[0], 0.0);
        assert_eq!(kerns[1], -0.637_695_312_5);
        assert_eq!(kerns[3], 0.002_929_687_5);
    }

    #[test]
    fn runs_are_laid_out_again_in_points() {
        let fonts = FontCache::default();
        let at_72 = plain("1.0", 10.0, 72.0);
        let at_100 = plain("1.0", 10.0, 100.0);
        let source = at_100.source.clone().expect("laid out by the engine");
        let points = in_points(&at_100, &source, &fonts).expect("bundled font");
        assert_eq!(points.glyphs, at_72.glyphs);
        assert!(matches!(
            in_points(&at_72, &source, &fonts),
            Ok(Cow::Borrowed(_))
        ));
    }
}
