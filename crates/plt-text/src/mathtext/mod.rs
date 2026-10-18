//! matplotlib's mathtext: TeX-like math typesetting, ported faithfully so
//! that extents and glyph positions agree with matplotlib.
//!
//! A string mixes plain text and math between unescaped `$` signs, e.g.
//! `"area $\pi r^2$"`. The pipeline follows matplotlib's:
//!
//! * the **parser** (a recursive-descent port of the pyparsing grammar and
//!   its actions) builds a TeX box model while parsing;
//! * the **fonts** layer picks the face and glyph for each symbol according
//!   to the fontset ([`Fontset`]: `DejaVu`, Computer Modern, STIX, custom),
//!   with fallbacks, STIX virtual fonts and sized delimiters;
//! * the **box model** packs horizontal and vertical lists with glue and
//!   kerning and ships them out to absolute glyph and rule positions.
//!
//! [`layout`] (or [`TextEngine::layout_math`]) returns a [`TextRun`] like
//! matplotlib's `MathTextParser("path").parse`; [`TextEngine::typeset_math`]
//! returns the unrounded [`MathLayout`].

mod boxes;
mod data;
mod font_name;
mod fonts;
mod output;
mod parser;
mod pattern;
mod style;

pub use output::{MathGlyph, MathLayout, MathRect, RasterExtents};
pub use style::{CustomFonts, Fontset, MathFallback, MathFont, MathStyle};

use crate::engine::TextEngine;
use crate::error::TextError;
use crate::run::{TextRun, TextSource};

/// Whether matplotlib treats `text` as math: it contains an even, non-zero
/// number of dollar signs not escaped as `\$`.
pub fn is_math_text(text: &str) -> bool {
    let dollars = text.matches('$').count() - text.matches("\\$").count();
    dollars > 0 && dollars.is_multiple_of(2)
}

/// Lays out mathtext with the shared [`TextEngine`]; see
/// [`TextEngine::layout_math`].
pub fn layout(source: &str, style: &MathStyle) -> Result<TextRun, TextError> {
    TextEngine::global().layout_math(source, style)
}

impl TextEngine {
    /// Typesets `source` (plain text with `$...$` math) without rounding.
    ///
    /// Malformed math yields [`TextError::Math`] with matplotlib's message.
    pub fn typeset_math(&self, source: &str, style: &MathStyle) -> Result<MathLayout, TextError> {
        let db = self.database();
        let text_font = db.find_font_with(&style.font.query, &style.font.generics)?;
        let mut fonts = fonts::Fonts::new(db, style, text_font)?;
        let root = parser::parse(
            source,
            &mut fonts,
            style.font.query.size,
            style.default.into(),
        )?;
        let shipped = boxes::ship(&root, &mut fonts)?;
        Ok(MathLayout::new(
            root.width,
            root.height,
            root.depth,
            shipped,
            style.dpi,
        ))
    }

    /// Lays out `source` as matplotlib's `MathTextParser("path").parse`
    /// does: width, height and depth rounded up to whole pixels and glyphs
    /// relative to the baseline, y up.
    ///
    /// ```
    /// use plt_text::TextEngine;
    /// use plt_text::mathtext::{Fontset, MathStyle};
    ///
    /// let style = MathStyle::default().with_fontset(Fontset::Cm).with_dpi(100.0);
    /// let run = TextEngine::global().layout_math(r"$\frac{a}{b}$", &style)?;
    /// assert_eq!(run.glyphs.len(), 2);
    /// assert_eq!(run.rects.len(), 1);
    /// # Ok::<(), plt_text::TextError>(())
    /// ```
    pub fn layout_math(&self, source: &str, style: &MathStyle) -> Result<TextRun, TextError> {
        let mut run = self.typeset_math(source, style)?.to_text_run();
        run.source = Some(TextSource::Math {
            text: source.to_owned(),
            style: style.clone(),
        });
        Ok(run)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn math_text_needs_paired_unescaped_dollars() {
        assert!(is_math_text("$x$"));
        assert!(is_math_text("a $b$ c $d$"));
        assert!(!is_math_text("x"));
        assert!(!is_math_text("$x"));
        assert!(!is_math_text(r"price \$5"));
        assert!(is_math_text(r"$\$5$"));
        assert!(!is_math_text("$a$ $"));
    }
}
