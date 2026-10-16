//! Single lines of text laid out with matplotlib's bundled fonts.

use plt_text::{FontQuery, FontRequest, TextEngine, TextRun, Weight};

use crate::error::UiError;
use crate::geometry::Size;

/// Resolution of UI text at scale factor 1: one point is 96/72 pixels,
/// the desktop convention.
const UI_DPI: f64 = 96.0;

/// How text is set.
#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    /// Font size in points.
    pub size: f64,
    /// Font weight.
    pub weight: Weight,
}

impl TextStyle {
    /// Normal-weight text of `size` points.
    pub fn new(size: f64) -> Self {
        Self {
            size,
            weight: Weight::NORMAL,
        }
    }
}

/// A line of text laid out for one scale factor: glyph positions and the
/// box they occupy.
///
/// The box's height comes from the font (ascent plus descent), not from
/// the glyphs present, so every line of a style has the same height and
/// baseline.
#[derive(Debug, Clone, PartialEq)]
pub struct TextLayout {
    run: TextRun,
    ascent: f64,
    descent: f64,
}

impl TextLayout {
    /// Lays out `text` (its first line) in `style` for windows with
    /// `scale_factor`, using the fonts bundled with plt-rs so the result
    /// does not depend on the host.
    pub fn new(text: &str, style: &TextStyle, scale_factor: f64) -> Result<Self, UiError> {
        let query = FontQuery {
            size: style.size,
            weight: style.weight,
            ..FontQuery::default()
        };
        let request = FontRequest::new(query);
        let engine = TextEngine::bundled();
        let dpi = UI_DPI * scale_factor;
        let metrics = engine.font_height_metrics(&request, dpi)?;
        Ok(Self {
            run: engine.layout_line(text, &request, dpi)?,
            ascent: metrics.ascent,
            descent: metrics.descent,
        })
    }

    /// The box the text occupies: advance width by line height.
    pub fn size(&self) -> Size {
        Size::new(self.run.width, self.ascent + self.descent)
    }

    /// Distance from the top of the box to the baseline.
    pub fn baseline(&self) -> f64 {
        self.ascent
    }

    /// The laid-out glyphs.
    pub(crate) fn run(&self) -> &TextRun {
        &self.run
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_scales_with_the_scale_factor() {
        let style = TextStyle::new(10.0);
        let normal = TextLayout::new("Home", &style, 1.0).unwrap();
        let hidpi = TextLayout::new("Home", &style, 2.0).unwrap();
        let size = normal.size();
        assert!(size.width > 20.0 && size.height > 10.0, "{size:?}");
        assert!((hidpi.size().width - 2.0 * size.width).abs() < 1.0);
        assert!((hidpi.size().height - 2.0 * size.height).abs() < 1e-9);
        assert!(normal.baseline() > 0.0 && normal.baseline() < size.height);
        assert_eq!(normal.run().glyphs.len(), 4);
    }

    #[test]
    fn line_height_does_not_depend_on_the_glyphs() {
        let style = TextStyle::new(10.0);
        let flat = TextLayout::new("ace", &style, 1.0).unwrap();
        let tall = TextLayout::new("Ågy", &style, 1.0).unwrap();
        assert_eq!(flat.size().height, tall.size().height);
        assert_eq!(flat.baseline(), tall.baseline());
    }

    #[test]
    fn empty_text_has_no_width() {
        let layout = TextLayout::new("", &TextStyle::new(10.0), 1.0).unwrap();
        assert_eq!(layout.size().width, 0.0);
        assert!(layout.size().height > 0.0);
        assert_eq!(layout.run().glyphs.len(), 0);
    }
}
