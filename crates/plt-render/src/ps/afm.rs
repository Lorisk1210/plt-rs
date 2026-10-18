//! Text in AFM fonts (`ps.useafm`): matplotlib's `draw_text` places every
//! character by its AFM width and kerning and shows it by glyph name in
//! the standard PostScript font, which the document references by name
//! without embedding it.

use std::borrow::Cow;

use plt_core::Point;
use plt_font::{AfmError, type1_name};
use plt_text::{AfmDirectory, AfmMetrics, FontRequest};

use super::PsRenderer;
use crate::gc::GraphicsContext;
use crate::vector::VectorError;

/// The glyph code matplotlib measures characters the font lacks with.
const QUESTION_MARK: i64 = 0x3f;

/// Every character of `text` as `(x, glyph name)`, `x` in points for a
/// font of `size` points: AFM widths and kerning accumulated as
/// matplotlib's PostScript backend does, characters the font lacks shown
/// as `question` with the width of its `?`.
fn place_glyphs(
    metrics: &AfmMetrics,
    text: &str,
    size: f64,
) -> Result<Vec<(f64, Cow<'static, str>)>, AfmError> {
    let scale = 0.001 * size;
    let mut x = 0.0;
    let mut last = Cow::Borrowed("");
    let mut placed = Vec::with_capacity(text.len());
    for c in text.chars() {
        let name = type1_name(c);
        let (name, width) = if let Some(width) = metrics.width_from_char_name(&name) {
            (name, width)
        } else {
            let width = metrics
                .width_char(QUESTION_MARK)
                .ok_or_else(|| AfmError::MissingGlyph("?".to_owned()))?;
            (Cow::Borrowed("question"), width)
        };
        x += metrics.kern_dist_from_name(&last, &name) * scale;
        placed.push((x, name.clone()));
        x += width * scale;
        last = name;
    }
    Ok(placed)
}

impl PsRenderer {
    /// `draw_text` with `ps.useafm` for text in `font`: the glyphs of its
    /// AFM font of `directory`, placed by the font's metrics, in one block
    /// (nothing for empty text).
    pub(super) fn afm_text(
        &mut self,
        gc: &GraphicsContext,
        text: &str,
        font: &FontRequest,
        directory: AfmDirectory,
        origin: Point,
        angle: f64,
    ) {
        let size = font.query.size;
        let placed = font.afm_metrics(directory).and_then(|metrics| {
            let name = metrics.font_name()?;
            Ok((name, place_glyphs(metrics, text, size)?))
        });
        let Some((name, placed)) = self.problems.record(placed.map_err(VectorError::from)) else {
            return;
        };
        let ps_name: String = name
            .chars()
            .map(|c| if c.is_ascii() { c } else { '?' })
            .collect();
        let rgb = gc.stroke_color();
        self.set_color([rgb.r, rgb.g, rgb.b], true);
        if placed.is_empty() {
            return;
        }
        let glyphs: Vec<(f64, f64, &str)> = placed
            .iter()
            .map(|(x, glyph)| (*x, 0.0, glyph.as_ref()))
            .collect();
        self.show_glyphs(gc, &ps_name, size, origin, angle, &glyphs);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plt_font::bundled_metrics;

    #[test]
    fn glyphs_are_kerned_and_unknown_characters_shown_as_question_marks() {
        let helvetica = bundled_metrics("Helvetica.afm").expect("bundled");
        let placed = place_glyphs(helvetica, "AV\u{2192}", 10.0).expect("known");
        // matplotlib: A at 0, V kerned by -70/1000 em after A's 667.
        assert_eq!(placed[0], (0.0, Cow::Borrowed("A")));
        assert_eq!(placed[1].1, "V");
        assert!((placed[1].0 - 5.97).abs() < 1e-12);
        assert_eq!(placed[2].1, "question");
        assert_eq!(place_glyphs(helvetica, "", 10.0), Ok(Vec::new()));
        let dingbats = bundled_metrics("ZapfDingbats.afm").expect("bundled");
        let shown = place_glyphs(dingbats, "a", 10.0).expect("measured by code 63");
        assert_eq!(shown[0].1, "question");
    }
}
