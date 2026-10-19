//! Single lines of text laid out with the fonts bundled with plt-rs.

use plt_text::{FontQuery, FontRequest, TextEngine, TextRun, Weight};

use crate::error::UiError;
use crate::geometry::Size;

/// Resolution of UI text at scale factor 1: one point is 96/72 pixels,
/// the desktop convention.
const UI_DPI: f64 = 96.0;

/// A family of the bundled fonts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontFamily {
    /// plt-rs' interface typeface (`Inter`), with `DejaVu Sans` for the
    /// characters it lacks.
    Interface,
    /// matplotlib's proportional text (`DejaVu Sans`).
    #[default]
    SansSerif,
    /// Fixed-width text (`DejaVu Sans Mono`), e.g. for key names.
    Monospace,
}

impl FontFamily {
    /// The family names the font manager resolves, in order of preference.
    fn names(self) -> Vec<String> {
        let names: &[&str] = match self {
            Self::Interface => &["Inter", "DejaVu Sans"],
            Self::SansSerif => &["sans-serif"],
            Self::Monospace => &["monospace"],
        };
        names.iter().map(|&name| name.to_owned()).collect()
    }
}

/// How text is set.
#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    /// Font size in points.
    pub size: f64,
    /// Font weight.
    pub weight: Weight,
    /// Font family.
    pub family: FontFamily,
}

impl TextStyle {
    /// Normal-weight sans-serif text of `size` points.
    pub fn new(size: f64) -> Self {
        Self {
            size,
            weight: Weight::NORMAL,
            family: FontFamily::SansSerif,
        }
    }

    /// The style in bold.
    #[must_use]
    pub fn bold(mut self) -> Self {
        self.weight = Weight::BOLD;
        self
    }

    /// The style in the fixed-width family.
    #[must_use]
    pub fn monospace(mut self) -> Self {
        self.family = FontFamily::Monospace;
        self
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
            families: style.family.names(),
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
    fn monospace_text_has_even_advances() {
        let style = TextStyle::new(10.0).monospace();
        let narrow = TextLayout::new("iiii", &style, 1.0).unwrap();
        let wide = TextLayout::new("MMMM", &style, 1.0).unwrap();
        assert!((narrow.size().width - wide.size().width).abs() < 1e-9);
        let bold = TextLayout::new("Home", &TextStyle::new(10.0).bold(), 1.0).unwrap();
        let normal = TextLayout::new("Home", &TextStyle::new(10.0), 1.0).unwrap();
        assert!(bold.size().width > normal.size().width);
    }

    #[test]
    fn interface_text_is_set_in_inter() {
        let interface = TextStyle {
            family: FontFamily::Interface,
            ..TextStyle::new(10.0)
        };
        let inter = TextLayout::new("Home", &interface, 1.0).unwrap();
        let dejavu = TextLayout::new("Home", &TextStyle::new(10.0), 1.0).unwrap();
        // Inter is the narrower face.
        assert!(inter.size().width < dejavu.size().width);
        assert_eq!(inter.run().glyphs.len(), 4);
    }

    #[test]
    fn empty_text_has_no_width() {
        let layout = TextLayout::new("", &TextStyle::new(10.0), 1.0).unwrap();
        assert_eq!(layout.size().width, 0.0);
        assert!(layout.size().height > 0.0);
        assert_eq!(layout.run().glyphs.len(), 0);
    }
}
