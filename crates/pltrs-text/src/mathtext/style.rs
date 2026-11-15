//! Configuration of the mathtext typesetter (matplotlib's `mathtext.*`
//! rcParams plus the text's font).

use std::fmt;
use std::str::FromStr;

use pltrs_font::{FontQuery, FontStyle, FontVariant, Hinting, Stretch, Weight};

use crate::TextError;
use crate::engine::FontRequest;

/// The family of math fonts (`mathtext.fontset`, or a text's
/// `math_fontfamily`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Fontset {
    /// `DejaVu Sans`, falling back to STIX sans (matplotlib's default).
    #[default]
    DejaVuSans,
    /// `DejaVu Serif`, falling back to STIX.
    DejaVuSerif,
    /// The `BaKoMa` Computer Modern fonts.
    Cm,
    /// STIX.
    Stix,
    /// STIX with sans-serif letters.
    StixSans,
    /// The fonts named by `mathtext.rm`, `mathtext.it`, ...
    Custom,
}

pltrs_core::archive_enum!(Fontset {
    DejaVuSans,
    DejaVuSerif,
    Cm,
    Stix,
    StixSans,
    Custom,
});

impl FromStr for Fontset {
    type Err = TextError;

    /// Parses a `mathtext.fontset` value.
    fn from_str(name: &str) -> Result<Self, TextError> {
        Ok(match name {
            "dejavusans" => Self::DejaVuSans,
            "dejavuserif" => Self::DejaVuSerif,
            "cm" => Self::Cm,
            "stix" => Self::Stix,
            "stixsans" => Self::StixSans,
            "custom" => Self::Custom,
            _ => return Err(TextError::unknown_name("math fontset", name)),
        })
    }
}

impl Fontset {
    /// The rcParam value naming this fontset.
    pub fn name(self) -> &'static str {
        match self {
            Self::DejaVuSans => "dejavusans",
            Self::DejaVuSerif => "dejavuserif",
            Self::Cm => "cm",
            Self::Stix => "stix",
            Self::StixSans => "stixsans",
            Self::Custom => "custom",
        }
    }
}

impl fmt::Display for Fontset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A TeX font name, as accepted by `mathtext.default`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MathFont {
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
    /// `default`: the font of the surrounding text.
    Default,
    /// `regular`: synonym of `default`.
    Regular,
    /// `normal`: italic letters, upright digits and symbols (the default).
    #[default]
    Normal,
    /// `bb`: blackboard bold.
    Bb,
    /// `frak`: fraktur.
    Frak,
    /// `scr`: script.
    Scr,
}

pltrs_core::archive_enum!(MathFont {
    Rm,
    It,
    Bf,
    Bfit,
    Sf,
    Tt,
    Cal,
    Default,
    Regular,
    Normal,
    Bb,
    Frak,
    Scr,
});

impl MathFont {
    const ALL: [Self; 13] = [
        Self::Rm,
        Self::It,
        Self::Bf,
        Self::Bfit,
        Self::Sf,
        Self::Tt,
        Self::Cal,
        Self::Default,
        Self::Regular,
        Self::Normal,
        Self::Bb,
        Self::Frak,
        Self::Scr,
    ];

    /// The TeX font name (a `mathtext.default` value).
    pub const fn name(self) -> &'static str {
        match self {
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
        }
    }
}

impl fmt::Display for MathFont {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for MathFont {
    type Err = TextError;

    /// Parses a `mathtext.default` value ([`MathFont::name`]).
    fn from_str(name: &str) -> Result<Self, TextError> {
        Self::ALL
            .into_iter()
            .find(|font| font.name() == name)
            .ok_or_else(|| TextError::unknown_name("math font", name))
    }
}

/// The fonts used for symbols missing from the custom or `DejaVu` fonts
/// (`mathtext.fallback`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MathFallback {
    /// Computer Modern (the default).
    #[default]
    Cm,
    /// STIX.
    Stix,
    /// STIX with sans-serif letters.
    StixSans,
}

pltrs_core::archive_enum!(MathFallback { Cm, Stix, StixSans });

impl MathFallback {
    /// The `mathtext.fallback` value naming these fonts.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Cm => "cm",
            Self::Stix => "stix",
            Self::StixSans => "stixsans",
        }
    }
}

impl fmt::Display for MathFallback {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for MathFallback {
    type Err = TextError;

    /// Parses a `mathtext.fallback` value other than `none`
    /// ([`MathFallback::name`], in any case, as matplotlib's
    /// `_validate_mathtext_fallback`).
    fn from_str(name: &str) -> Result<Self, TextError> {
        [Self::Cm, Self::Stix, Self::StixSans]
            .into_iter()
            .find(|fallback| fallback.name().eq_ignore_ascii_case(name))
            .ok_or_else(|| TextError::unknown_name("math fallback", name))
    }
}

/// fontconfig patterns of the custom fontset (`mathtext.rm`, ...).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CustomFonts {
    /// `mathtext.rm`.
    pub rm: String,
    /// `mathtext.it`.
    pub it: String,
    /// `mathtext.bf`.
    pub bf: String,
    /// `mathtext.bfit`.
    pub bfit: String,
    /// `mathtext.sf`.
    pub sf: String,
    /// `mathtext.tt`.
    pub tt: String,
    /// `mathtext.cal`.
    pub cal: String,
}

impl Default for CustomFonts {
    /// matplotlib's defaults.
    fn default() -> Self {
        Self {
            rm: "sans".to_owned(),
            it: "sans:italic".to_owned(),
            bf: "sans:bold".to_owned(),
            bfit: "sans:italic:bold".to_owned(),
            sf: "sans".to_owned(),
            tt: "monospace".to_owned(),
            cal: "cursive".to_owned(),
        }
    }
}

/// Everything that determines how mathtext is typeset.
#[derive(Clone, Debug, PartialEq)]
pub struct MathStyle {
    /// The math fonts.
    pub fontset: Fontset,
    /// The font of math outside any font command (`mathtext.default`).
    pub default: MathFont,
    /// Where missing symbols come from (`mathtext.fallback`).
    pub fallback: Option<MathFallback>,
    /// The fonts of [`Fontset::Custom`].
    pub custom: CustomFonts,
    /// Whether [`Fontset::Custom`] takes its symbols from `DejaVu` Sans as
    /// [`Fontset::DejaVuSans`] does: big operators from `DejaVu` Sans
    /// Display, and the symbols its fonts lack from `DejaVu` Sans before
    /// the fallback (pltrs' `pltrs.mathtext.dejavu_symbols`; matplotlib
    /// has no such setting).
    pub dejavu_symbols: bool,
    /// The text font: used for text outside `$...$` and for `\mathdefault`;
    /// its size is the size of the math. Its generic families also resolve
    /// the custom fontset's patterns.
    pub font: FontRequest,
    /// What the fontset's font patterns leave out.
    pub pattern_defaults: PatternDefaults,
    /// Output resolution in dots per inch.
    pub dpi: f64,
    /// The hinting glyphs are loaded with, which their boxes take: none
    /// for paths and vector output (matplotlib's `MathTextParser("path")`,
    /// whose fonts load glyphs with `NO_HINTING`), `text.hinting` for an
    /// image (`MathTextParser("agg")`, its `load_glyph_flags`).
    pub hinting: Hinting,
}

impl Default for MathStyle {
    /// matplotlib's defaults: `DejaVu Sans` math at 10 pt and 72 dpi.
    fn default() -> Self {
        Self {
            fontset: Fontset::default(),
            default: MathFont::default(),
            fallback: Some(MathFallback::default()),
            custom: CustomFonts::default(),
            dejavu_symbols: false,
            font: FontRequest::new(FontQuery::default()),
            pattern_defaults: PatternDefaults::default(),
            dpi: 72.0,
            hinting: Hinting::None,
        }
    }
}

/// What matplotlib's `FontProperties(pattern)` takes from the `font.*`
/// rcParams for the properties a font pattern leaves out: `"DejaVu Sans"`
/// names no style, so `font.style: italic` slants upright math.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PatternDefaults {
    /// `font.style`.
    pub style: FontStyle,
    /// `font.variant`.
    pub variant: FontVariant,
    /// `font.weight`.
    pub weight: Weight,
    /// The name `font.weight` was given as, if it was one (the font
    /// manager's messages name it).
    pub weight_keyword: Option<&'static str>,
    /// `font.stretch`.
    pub stretch: Stretch,
    /// The name `font.stretch` was given as, if it was one.
    pub stretch_keyword: Option<&'static str>,
}

impl PatternDefaults {
    /// The style, variant, weight and stretch `request` asks for.
    pub fn of(request: &FontRequest) -> Self {
        let query = &request.query;
        Self {
            style: query.style,
            variant: query.variant,
            weight: query.weight,
            weight_keyword: request.weight_keyword,
            stretch: query.stretch,
            stretch_keyword: request.stretch_keyword,
        }
    }
}

impl Default for PatternDefaults {
    /// matplotlib's defaults: normal style, variant, weight and stretch.
    fn default() -> Self {
        Self::of(&FontRequest::new(FontQuery::default()))
    }
}

impl MathStyle {
    /// The style with another fontset.
    #[must_use]
    pub fn with_fontset(mut self, fontset: Fontset) -> Self {
        self.fontset = fontset;
        self
    }

    /// The style with the font size set to `points`.
    #[must_use]
    pub fn with_size(mut self, points: f64) -> Self {
        self.font.query.size = points;
        self
    }

    /// The style at another resolution.
    #[must_use]
    pub fn with_dpi(mut self, dpi: f64) -> Self {
        self.dpi = dpi;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rc_values_round_trip() {
        for fontset in [
            Fontset::DejaVuSans,
            Fontset::DejaVuSerif,
            Fontset::Cm,
            Fontset::Stix,
            Fontset::StixSans,
            Fontset::Custom,
        ] {
            assert_eq!(fontset.to_string().parse(), Ok(fontset));
        }
        assert!("comic".parse::<Fontset>().is_err());
        for font in MathFont::ALL {
            assert_eq!(font.to_string().parse(), Ok(font));
        }
        assert_eq!("frak".parse(), Ok(MathFont::Frak));
        assert!("ex".parse::<MathFont>().is_err());
        assert_eq!("STIX".parse(), Ok(MathFallback::Stix));
        assert_eq!(MathFallback::StixSans.to_string(), "stixsans");
        assert!("None".parse::<MathFallback>().is_err());
        assert!("x".parse::<MathFallback>().is_err());
    }

    #[test]
    fn defaults_follow_matplotlib() {
        let style = MathStyle::default()
            .with_fontset(Fontset::Cm)
            .with_size(12.0)
            .with_dpi(100.0);
        assert_eq!(style.fontset, Fontset::Cm);
        assert_eq!(style.default, MathFont::Normal);
        assert_eq!(style.fallback, Some(MathFallback::Cm));
        assert!(!style.dejavu_symbols);
        assert_eq!((style.font.query.size, style.dpi), (12.0, 100.0));
        assert_eq!(style.custom.cal, "cursive");
    }
}
