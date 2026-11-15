//! Which lines are mathtext, and the mathtext settings of a text (the
//! `mathtext.*` rcParams plus the text's font).

use std::borrow::Cow;

use pltrs_text::Hinting;
use pltrs_text::mathtext::{CustomFonts, MathStyle, PatternDefaults, is_math_text};

use crate::fonts::FontProperties;
use crate::rc::RcParams;

/// One line of a text, prepared for layout (matplotlib's
/// `Text._preprocess_math`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Line<'a> {
    /// Plain text, laid out as is.
    Plain(Cow<'a, str>),
    /// Text with `$...$` math, typeset by mathtext.
    Math(&'a str),
}

impl<'a> Line<'a> {
    /// A line is math when math parsing is on and it holds an even,
    /// non-zero number of unescaped dollar signs; otherwise it is plain text
    /// whose escaped dollars (`\$`) are shown as `$` when math parsing is on.
    pub(crate) fn classify(line: &'a str, parse_math: bool) -> Self {
        if !parse_math {
            Self::Plain(Cow::Borrowed(line))
        } else if is_math_text(line) {
            Self::Math(line)
        } else if line.contains("\\$") {
            Self::Plain(Cow::Owned(line.replace("\\$", "$")))
        } else {
            Self::Plain(Cow::Borrowed(line))
        }
    }
}

/// The mathtext configuration for text in `font` at `dpi`: the fontset is
/// the font's math font family, the rest comes from `mathtext.*` in `rc`
/// (what matplotlib's `MathTextParser("path").parse(s, dpi, prop)`
/// typesets with: glyphs loaded unhinted).
pub fn math_style(font: &FontProperties, dpi: f64, rc: &RcParams) -> MathStyle {
    MathStyle {
        fontset: font.math_fontfamily,
        default: rc.mathtext_default,
        fallback: rc.mathtext_fallback,
        custom: CustomFonts {
            rm: rc.mathtext_rm.clone(),
            it: rc.mathtext_it.clone(),
            bf: rc.mathtext_bf.clone(),
            bfit: rc.mathtext_bfit.clone(),
            sf: rc.mathtext_sf.clone(),
            tt: rc.mathtext_tt.clone(),
            cal: rc.mathtext_cal.clone(),
        },
        dejavu_symbols: rc.pltrs_mathtext_dejavu_symbols,
        font: font.drawn_request(),
        pattern_defaults: PatternDefaults::of(&FontProperties::from_rc(rc).request(rc)),
        dpi,
        hinting: Hinting::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs_text::mathtext::{Fontset, MathFont};

    #[test]
    fn preprocessing_follows_matplotlib() {
        assert_eq!(Line::classify("$x$", true), Line::Math("$x$"));
        assert_eq!(Line::classify("$x$", false), Line::Plain("$x$".into()));
        assert_eq!(Line::classify(r"\$5", true), Line::Plain("$5".into()));
        assert_eq!(Line::classify(r"\$5", false), Line::Plain(r"\$5".into()));
        assert_eq!(Line::classify("$", true), Line::Plain("$".into()));
    }

    #[test]
    fn style_from_rc_and_font() {
        let mut rc = RcParams::matplotlib();
        rc.mathtext_fallback = None;
        rc.mathtext_default = MathFont::Rm;
        let mut font = FontProperties::from_rc(&rc);
        font.math_fontfamily = Fontset::Stix;
        let style = math_style(&font, 72.0, &rc);
        assert_eq!(style.fontset, Fontset::Stix);
        assert_eq!(style.default, MathFont::Rm);
        assert_eq!(style.fallback, None);
        assert_eq!(style.custom.cal, "cursive");
        assert!(!style.dejavu_symbols);
        rc.set("pltrs.mathtext.dejavu_symbols", "True")
            .expect("valid");
        assert!(math_style(&font, 72.0, &rc).dejavu_symbols);
    }

    #[test]
    fn font_patterns_default_to_the_rc_font_not_the_text_font() {
        let mut rc = RcParams::matplotlib();
        rc.set("font.style", "italic").expect("valid");
        rc.set("font.weight", "bold").expect("valid");
        let upright = FontProperties::from_rc(&RcParams::matplotlib());
        let defaults = math_style(&upright, 72.0, &rc).pattern_defaults;
        assert_eq!(defaults.style, pltrs_text::FontStyle::Italic);
        assert_eq!(defaults.weight, pltrs_text::Weight::BOLD);
    }
}
