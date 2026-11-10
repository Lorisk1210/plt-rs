//! The TeX font names mathtext uses to pick glyphs.

use std::fmt;

use super::style::MathFont;

/// A font name of matplotlib's mathtext: the user-visible TeX names plus
/// the internal ones for the display font, the STIX size variants and the
/// STIX private-use-area fonts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum FontName {
    /// `rm`: upright.
    Rm,
    /// `it`: italic.
    It,
    /// `bf`: bold.
    Bf,
    /// `bfit`: bold italic.
    Bfit,
    /// `sf`: sans-serif.
    Sf,
    /// `tt`: monospace.
    Tt,
    /// `cal`: calligraphic.
    Cal,
    /// `default`: the surrounding text font.
    Default,
    /// `regular`: synonym of `default`.
    Regular,
    /// `normal`: the math font (italic letters, upright everything else).
    Normal,
    /// `bb`: blackboard bold.
    Bb,
    /// `frak`: fraktur.
    Frak,
    /// `scr`: script.
    Scr,
    /// `ex`: the extension (display) font.
    Ex,
    /// `0` to `5`: the regular font and the STIX size variants.
    Size(u8),
    /// `nonunirm`: STIX glyphs in the private use area, upright.
    NonUniRm,
    /// `nonuniit`: STIX private use area, italic.
    NonUniIt,
    /// `nonunibf`: STIX private use area, bold.
    NonUniBf,
    /// `nonunibfit` (requested by matplotlib but never available).
    NonUniBfit,
}

impl FontName {
    /// Parses a user-visible font name (`\rm`, `\mathbf`, `mathtext.default`):
    /// the name of a [`MathFont`].
    pub(crate) fn parse(name: &str) -> Option<Self> {
        name.parse::<MathFont>().ok().map(Self::from)
    }

    /// Whether selecting this font also changes the font class
    /// (matplotlib's `ParserState.font` setter).
    pub(crate) fn sets_class(self) -> bool {
        matches!(
            self,
            Self::Normal | Self::Rm | Self::It | Self::Bf | Self::Bfit
        )
    }

    /// The private-use-area counterpart of an upright, italic or bold font.
    pub(crate) fn non_unicode(self) -> Option<Self> {
        Some(match self {
            Self::Rm => Self::NonUniRm,
            Self::It => Self::NonUniIt,
            Self::Bf => Self::NonUniBf,
            Self::Bfit => Self::NonUniBfit,
            _ => return None,
        })
    }

    /// The font used for text that is not math (`default` or `regular`).
    pub(crate) fn is_text_font(self) -> bool {
        matches!(self, Self::Default | Self::Regular)
    }
}

/// matplotlib's name of the font (`rm`, `tt`, `0`, `nonunirm`, ...).
impl fmt::Display for FontName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Rm => "rm",
            Self::It => "it",
            Self::Bf => "bf",
            Self::Bfit => "bfit",
            Self::Sf => "sf",
            Self::Tt => "tt",
            Self::Cal => "cal",
            Self::Default => "default",
            Self::Regular => "regular",
            Self::Normal => "normal",
            Self::Bb => "bb",
            Self::Frak => "frak",
            Self::Scr => "scr",
            Self::Ex => "ex",
            Self::Size(size) => return write!(f, "{size}"),
            Self::NonUniRm => "nonunirm",
            Self::NonUniIt => "nonuniit",
            Self::NonUniBf => "nonunibf",
            Self::NonUniBfit => "nonunibfit",
        };
        f.write_str(name)
    }
}

impl From<MathFont> for FontName {
    fn from(font: MathFont) -> Self {
        match font {
            MathFont::Rm => Self::Rm,
            MathFont::It => Self::It,
            MathFont::Bf => Self::Bf,
            MathFont::Bfit => Self::Bfit,
            MathFont::Sf => Self::Sf,
            MathFont::Tt => Self::Tt,
            MathFont::Cal => Self::Cal,
            MathFont::Default => Self::Default,
            MathFont::Regular => Self::Regular,
            MathFont::Normal => Self::Normal,
            MathFont::Bb => Self::Bb,
            MathFont::Frak => Self::Frak,
            MathFont::Scr => Self::Scr,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_user_names_only() {
        assert_eq!(FontName::parse("bfit"), Some(FontName::Bfit));
        assert_eq!(FontName::parse("ex"), None);
        assert_eq!(FontName::parse(""), None);
    }

    #[test]
    fn classes_and_private_use_variants() {
        assert!(FontName::Normal.sets_class());
        assert!(!FontName::Sf.sets_class());
        assert_eq!(FontName::It.non_unicode(), Some(FontName::NonUniIt));
        assert_eq!(FontName::Cal.non_unicode(), None);
        assert!(FontName::Regular.is_text_font());
    }

    #[test]
    fn names_are_matplotlibs() {
        assert_eq!(FontName::Tt.to_string(), "tt");
        assert_eq!(FontName::Size(3).to_string(), "3");
        assert_eq!(FontName::NonUniBf.to_string(), "nonunibf");
    }
}
