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

pub use boxes::MathNode;
pub use output::{MathGlyph, MathLayout, MathRect, RasterExtents};
pub use style::{CustomFonts, Fontset, MathFallback, MathFont, MathStyle, PatternDefaults};

use crate::engine::{FormulaKey, TextEngine};
use crate::error::TextError;
use crate::mathtext_log;
use crate::run::{TextRun, TextSource};

/// Whether matplotlib treats `text` as math: it contains an even, non-zero
/// number of dollar signs not escaped as `\$`.
pub fn is_math_text(text: &str) -> bool {
    let dollars = text.matches('$').count() - text.matches("\\$").count();
    dollars > 0 && dollars.is_multiple_of(2)
}

/// matplotlib's `get_unicode_index`: the code point of a single character,
/// or of a TeX symbol name (with or without its backslash); `None` for
/// names mathtext does not know.
///
/// ```
/// use pltrs_text::mathtext::unicode_index;
/// assert_eq!(unicode_index("a"), Some(0x61));
/// assert_eq!(unicode_index(r"\alpha"), Some(0x3B1));
/// assert_eq!(unicode_index(r"\foo"), None);
/// ```
pub fn unicode_index(symbol: &str) -> Option<u32> {
    data::unicode_index(symbol)
}

/// matplotlib's `Parser._accent_map`: each narrow accent command (without
/// its backslash) and the symbol it draws, sorted by command.
pub fn accent_map() -> &'static [(&'static str, &'static str)] {
    data::accent_map()
}

/// matplotlib's `Parser._wide_accents`: the accents that stretch over
/// their body, sorted.
pub fn wide_accents() -> &'static [&'static str] {
    data::wide_accents()
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
    /// The messages matplotlib's mathtext logs typesetting it are kept in
    /// the layout's `logged`, for the caller to log where matplotlib's
    /// parser parses the formula.
    pub fn typeset_math(&self, source: &str, style: &MathStyle) -> Result<MathLayout, TextError> {
        let (layout, logged) = self.parse_math(source, style, |root, fonts| {
            let shipped = boxes::ship(root, fonts)?;
            Ok(MathLayout::new(
                root.width,
                root.height,
                root.depth,
                shipped,
                style.dpi,
            ))
        })?;
        Ok(MathLayout { logged, ..layout })
    }

    /// The box tree `source` is typeset into, before it is shipped out
    /// (matplotlib's `_mathtext.Parser().parse(s, fonts, fontsize, dpi)`,
    /// which logs what it logs typesetting every time).
    ///
    /// ```
    /// use pltrs_text::TextEngine;
    /// use pltrs_text::mathtext::{MathNode, MathStyle};
    ///
    /// let tree = TextEngine::global().math_box_tree(r"$x$", &MathStyle::default())?;
    /// assert!(matches!(tree, MathNode::List { class: "Hlist", .. }));
    /// # Ok::<(), pltrs_text::TextError>(())
    /// ```
    pub fn math_box_tree(&self, source: &str, style: &MathStyle) -> Result<MathNode, TextError> {
        let (tree, logged) = self.parse_math(source, style, |root, _| Ok(MathNode::from(root)))?;
        crate::log_mathtext(&logged);
        Ok(tree)
    }

    /// Parses `source` into its root list and hands it, with the fonts it
    /// was typeset with, to `then`: what `then` gives, with the messages
    /// matplotlib's mathtext logs typesetting it. A parse that fails logs
    /// what it noted before failing, as matplotlib's does.
    fn parse_math<T>(
        &self,
        source: &str,
        style: &MathStyle,
        then: impl FnOnce(&boxes::List, &mut fonts::Fonts<'_>) -> Result<T, TextError>,
    ) -> Result<(T, Vec<String>), TextError> {
        let (result, logged) = mathtext_log::collecting(|| {
            let db = self.database();
            let text_font = style.font.font(db)?;
            let mut fonts = fonts::Fonts::new(db, style, text_font)?;
            let root = parser::parse(
                source,
                &mut fonts,
                style.font.query.size,
                style.default.into(),
            )?;
            then(&root, &mut fonts)
        });
        match result {
            Ok(value) => Ok((value, logged)),
            Err(err) => {
                crate::log_mathtext(&logged);
                Err(err)
            }
        }
    }

    /// Lays out `source` as matplotlib's `MathTextParser("path").parse`
    /// does: width, height and depth rounded up to whole pixels and glyphs
    /// relative to the baseline, y up.
    ///
    /// ```
    /// use pltrs_text::TextEngine;
    /// use pltrs_text::mathtext::{Fontset, MathStyle};
    ///
    /// let style = MathStyle::default().with_fontset(Fontset::Cm).with_dpi(100.0);
    /// let run = TextEngine::global().layout_math(r"$\frac{a}{b}$", &style)?;
    /// assert_eq!(run.glyphs.len(), 2);
    /// assert_eq!(run.rects.len(), 1);
    /// # Ok::<(), pltrs_text::TextError>(())
    /// ```
    pub fn layout_math(&self, source: &str, style: &MathStyle) -> Result<TextRun, TextError> {
        self.formulas.get_or_compute(
            crate::memo::fingerprint(&style.font, (source, style.dpi.to_bits())),
            |key| key.source == source && key.style == *style,
            || FormulaKey {
                source: source.to_owned(),
                style: style.clone(),
            },
            || {
                let layout = self.typeset_math(source, style)?;
                let mut run = layout.to_text_run();
                run.source = Some(TextSource::Math {
                    text: source.to_owned(),
                    style: style.clone(),
                    logged: layout.logged,
                });
                Ok(run)
            },
        )
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
