//! Which lines are mathtext, and the mathtext settings of a text (the
//! `mathtext.*` rcParams plus the text's font).

use std::borrow::Cow;

use plt_text::mathtext::{CustomFonts, MathFallback, MathFont, MathStyle, is_math_text};

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
/// the font's math font family, the rest comes from `mathtext.*` in `rc`.
pub(crate) fn math_style(font: &FontProperties, dpi: f64, rc: &RcParams) -> MathStyle {
    MathStyle {
        fontset: font.math_fontfamily,
        default: MathFont::parse(&rc.mathtext_default).unwrap_or_default(),
        fallback: rc
            .mathtext_fallback
            .as_deref()
            .and_then(|name| MathFallback::parse(name).flatten()),
        custom: CustomFonts {
            rm: rc.mathtext_rm.clone(),
            it: rc.mathtext_it.clone(),
            bf: rc.mathtext_bf.clone(),
            bfit: rc.mathtext_bfit.clone(),
            sf: rc.mathtext_sf.clone(),
            tt: rc.mathtext_tt.clone(),
            cal: rc.mathtext_cal.clone(),
        },
        font: font.request(rc),
        dpi,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plt_text::mathtext::Fontset;

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
        rc.mathtext_default = "rm".to_owned();
        let mut font = FontProperties::from_rc(&rc);
        font.math_fontfamily = Fontset::Stix;
        let style = math_style(&font, 72.0, &rc);
        assert_eq!(style.fontset, Fontset::Stix);
        assert_eq!(style.default, MathFont::Rm);
        assert_eq!(style.fallback, None);
        assert_eq!(style.custom.cal, "cursive");
    }
}
