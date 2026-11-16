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
pub use output::{MathGlyph, MathLayout, MathRect, RasterExtents, RasterMath};
pub use style::{CustomFonts, Fontset, MathFallback, MathFont, MathStyle, PatternDefaults};

use crate::engine::{FormulaKey, TextEngine};
use crate::error::TextError;
use crate::mathtext_log;
use crate::run::{TextRun, TextSource};

/// The deepest nesting a formula may have (`deep` sizes the stack for it).
pub(crate) const MAX_NESTING: usize = parser::MAX_NESTING;

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
            let shipped = boxes::ship(root, fonts, (0.0, 0.0))?;
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

    /// Typesets `source` for an image, as matplotlib's `Output.to_raster`
    /// does: the box shipped at the origin gives the image's extents, then
    /// it is shipped again with the image's top-left corner at the origin.
    /// Logged messages are kept as [`typeset_math`](Self::typeset_math)
    /// keeps them.
    ///
    /// # Errors
    ///
    /// As [`typeset_math`](Self::typeset_math).
    pub fn typeset_math_raster(
        &self,
        source: &str,
        style: &MathStyle,
    ) -> Result<RasterMath, TextError> {
        let (raster, logged) = self.parse_math(source, style, |root, fonts| {
            let typeset =
                |shipped| MathLayout::new(root.width, root.height, root.depth, shipped, style.dpi);
            let first = typeset(boxes::ship(root, fonts, (0.0, 0.0))?);
            let (extents, (rows, columns), corner) = first.raster_image();
            Ok(RasterMath {
                extents,
                rows,
                columns,
                layout: typeset(boxes::ship(root, fonts, corner)?),
            })
        })?;
        Ok(RasterMath {
            layout: MathLayout {
                logged,
                ..raster.layout
            },
            ..raster
        })
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
    fn parse_math<T: Send>(
        &self,
        source: &str,
        style: &MathStyle,
        then: impl FnOnce(&boxes::List, &mut fonts::Fonts<'_>) -> Result<T, TextError> + Send,
    ) -> Result<(T, Vec<crate::MathNote>), TextError> {
        // As deep as the formula nests: where it has room on the stack.
        let (result, logged) = crate::deep::with_room(crate::deep::formula_need(source), || {
            // The formula's font properties carry its fontset as their
            // math font family, which matplotlib's font manager caches
            // lookups by.
            mathtext_log::collecting(Some(style.fontset), || {
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
            })
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
        let fingerprint = crate::memo::fingerprint(&style.font, (source, style.dpi.to_bits()));
        let matches = |key: &FormulaKey| key.source == source && key.style == *style;
        if let Some(run) = self.formulas.get(fingerprint, matches) {
            return Ok(run);
        }
        // A formula not typeset yet: typeset (and kept) where the stack has
        // room for its nesting.
        crate::deep::with_room(crate::deep::formula_need(source), || {
            self.formulas.get_or_compute(
                fingerprint,
                matches,
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
        })
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

    #[test]
    fn symbols_no_font_has_are_kept_with_what_was_typeset() {
        crate::take_mathtext_log();
        let style = MathStyle::default().with_dpi(72.0);
        let run = TextEngine::global()
            .layout_math("$y\u{1f60e}$", &style)
            .unwrap();
        let Some(TextSource::Math { logged, .. }) = &run.source else {
            panic!("a formula's run names its source");
        };
        assert_eq!(
            logged.as_slice(),
            [crate::MathNote::Message(
                "Font 'rm' does not have a glyph for '\\U0001f60e' [U+1f60e], \
                 substituting with a dummy symbol."
                    .to_owned()
            )]
        );
        assert!(
            crate::take_mathtext_log().is_empty(),
            "typesetting keeps its messages for the caller to log"
        );
    }
}
