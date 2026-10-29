//! The fonts of each fontset (matplotlib's `Fonts` constructors): the
//! font map of `BaKoMa`, STIX, `DejaVu` and custom glyph sets and the sets
//! they fall back on.

use pltrs_font::{FontDatabase, FontId, FontQuery, GenericFamilies};

use super::{GlyphSet, Kind};
use crate::error::TextError;
use crate::mathtext::data::BakomaFile;
use crate::mathtext::font_name::FontName;
use crate::mathtext::fonts::MathFonts;
use crate::mathtext::pattern::parse_pattern;
use crate::mathtext::style::{MathFallback, MathStyle, PatternDefaults};

const STIX_SIZES: [&str; 6] = [
    "STIXGeneral",
    "STIXSizeOneSym",
    "STIXSizeTwoSym",
    "STIXSizeThreeSym",
    "STIXSizeFourSym",
    "STIXSizeFiveSym",
];

/// matplotlib's `findfont` over a database with given generic families.
#[derive(Clone, Copy)]
struct Finder<'a> {
    /// The fonts.
    db: &'a FontDatabase,
    /// `font.serif`, `font.sans-serif`, ... lists.
    generics: &'a GenericFamilies,
    /// What a pattern leaves out ([`MathStyle::pattern_defaults`]).
    defaults: PatternDefaults,
}

impl Finder<'_> {
    fn query(self, query: &FontQuery) -> Result<FontId, TextError> {
        Ok(self.db.find_font_with(query, self.generics)?)
    }

    /// `findfont(FontProperties(pattern))`.
    fn find(self, pattern: &str) -> Result<FontId, TextError> {
        self.query(&parse_pattern(pattern, self.defaults)?)
    }

    fn find_all(self, entries: &[(FontName, &str)]) -> Result<Vec<(FontName, FontId)>, TextError> {
        entries
            .iter()
            .map(|&(name, pattern)| Ok((name, self.find(pattern)?)))
            .collect()
    }

    fn stix_sizes(self) -> Result<Vec<(FontName, FontId)>, TextError> {
        (0u8..)
            .zip(STIX_SIZES)
            .map(|(i, family)| Ok((FontName::Size(i), self.find(family)?)))
            .collect()
    }
}

impl GlyphSet {
    fn new(kind: Kind, text_font: FontId, math_default: FontName) -> Self {
        Self {
            kind,
            fontmap: Vec::new(),
            bakoma_files: Vec::new(),
            text_font,
            fallback: None,
            symbols: None,
            prime: None,
            math_default,
            cmsy10: None,
        }
    }

    /// A set picking glyphs by their Unicode index (matplotlib's
    /// `UnicodeFonts`, which the STIX, `DejaVu` and custom fontsets
    /// extend): it takes `×` and `−` from `cmsy10` where the font in use is
    /// `cmr10`.
    fn unicode(
        kind: Kind,
        db: Finder<'_>,
        text_font: FontId,
        default: FontName,
    ) -> Result<Self, TextError> {
        Ok(Self {
            cmsy10: Some(db.find("cmsy10")?),
            ..Self::new(kind, text_font, default)
        })
    }

    /// The glyph set of `style.fontset`.
    pub(crate) fn for_style(
        db: &FontDatabase,
        style: &MathStyle,
        text_font: FontId,
    ) -> Result<Self, TextError> {
        let finder = Finder {
            db,
            generics: &style.font.generics,
            defaults: style.pattern_defaults,
        };
        let math_default = style.default.into();
        match MathFonts::of(style) {
            MathFonts::Cm => Self::bakoma(finder, text_font, math_default),
            MathFonts::DejaVu { serif } => {
                let stix = Self::stix(finder, text_font, math_default, !serif)?;
                Self::dejavu(finder, text_font, math_default, serif, Some(Box::new(stix)))
            }
            MathFonts::Stix { sans } => Self::stix(finder, text_font, math_default, sans),
            MathFonts::Custom => Self::custom(finder, style, text_font, math_default),
        }
    }

    fn bakoma(db: Finder<'_>, text_font: FontId, default: FontName) -> Result<Self, TextError> {
        let mut set = Self::new(Kind::Bakoma, text_font, default);
        set.fontmap = db.find_all(&[
            (FontName::Normal, "cmmi10"),
            (FontName::Cal, "cmsy10"),
            (FontName::Rm, "cmr10"),
            (FontName::Tt, "cmtt10"),
            (FontName::It, "cmti10"),
            (FontName::Bf, "cmb10"),
            (FontName::Sf, "cmss10"),
            (FontName::Ex, "cmex10"),
        ])?;
        set.bakoma_files = [
            BakomaFile::Cmex10,
            BakomaFile::Cmmi10,
            BakomaFile::Cmr10,
            BakomaFile::Cmsy10,
            BakomaFile::Cmtt10,
        ]
        .into_iter()
        .map(|file| Ok((file, db.find(file.family())?)))
        .collect::<Result<_, TextError>>()?;
        set.fallback = Some(Box::new(Self::stix(db, text_font, default, false)?));
        Ok(set)
    }

    fn stix(
        db: Finder<'_>,
        text_font: FontId,
        default: FontName,
        sans: bool,
    ) -> Result<Self, TextError> {
        let mut set = Self::unicode(Kind::Stix { sans }, db, text_font, default)?;
        set.fontmap = db.find_all(&[
            (FontName::Rm, "STIXGeneral"),
            (FontName::It, "STIXGeneral:italic"),
            (FontName::Bf, "STIXGeneral:weight=bold"),
            (FontName::Bfit, "STIXGeneral:italic:bold"),
            (FontName::NonUniRm, "STIXNonUnicode"),
            (FontName::NonUniIt, "STIXNonUnicode:italic"),
            (FontName::NonUniBf, "STIXNonUnicode:weight=bold"),
        ])?;
        set.fontmap.extend(db.stix_sizes()?);
        Ok(set)
    }

    /// The `DejaVu` Sans or Serif set, falling back to `fallback` (STIX in
    /// the `DejaVu` fontsets).
    fn dejavu(
        db: Finder<'_>,
        text_font: FontId,
        default: FontName,
        serif: bool,
        fallback: Option<Box<Self>>,
    ) -> Result<Self, TextError> {
        let mut set = Self::unicode(Kind::DejaVu, db, text_font, default)?;
        let family = if serif { "DejaVu Serif" } else { "DejaVu Sans" };
        let patterns = [
            (FontName::Rm, family.to_owned()),
            (FontName::It, format!("{family}:italic")),
            (FontName::Bf, format!("{family}:weight=bold")),
            (FontName::Bfit, format!("{family}:italic:bold")),
            (FontName::Sf, "DejaVu Sans".to_owned()),
            (FontName::Tt, "DejaVu Sans Mono".to_owned()),
            (FontName::Ex, format!("{family} Display")),
            (FontName::Size(0), family.to_owned()),
        ];
        let entries: Vec<(FontName, &str)> = patterns
            .iter()
            .map(|(name, pattern)| (*name, pattern.as_str()))
            .collect();
        set.fontmap = db.find_all(&entries)?;
        set.fontmap.extend(db.stix_sizes()?.into_iter().skip(1));
        set.fallback = fallback;
        set.prime = Some(Box::new(Self::bakoma(db, text_font, default)?));
        Ok(set)
    }

    fn custom(
        db: Finder<'_>,
        style: &MathStyle,
        text_font: FontId,
        default: FontName,
    ) -> Result<Self, TextError> {
        let mut set = Self::unicode(Kind::Custom, db, text_font, default)?;
        let fonts = &style.custom;
        set.fallback = match style.fallback {
            Some(MathFallback::Cm) => Some(Box::new(Self::bakoma(db, text_font, default)?)),
            Some(MathFallback::Stix) => Some(Box::new(Self::stix(db, text_font, default, false)?)),
            Some(MathFallback::StixSans) => {
                Some(Box::new(Self::stix(db, text_font, default, true)?))
            }
            None => None,
        };
        if style.dejavu_symbols {
            let dejavu = Self::dejavu(db, text_font, default, false, set.fallback.clone())?;
            set.symbols = Some(Box::new(dejavu));
        }
        set.fontmap = db.find_all(&[
            (FontName::Cal, &fonts.cal),
            (FontName::Rm, &fonts.rm),
            (FontName::Tt, &fonts.tt),
            (FontName::It, &fonts.it),
            (FontName::Bf, &fonts.bf),
            (FontName::Sf, &fonts.sf),
            (FontName::Bfit, &fonts.bfit),
        ])?;
        set.fontmap
            .push((FontName::Ex, db.query(&FontQuery::families(["cmex10"]))?));
        if set
            .fallback
            .as_ref()
            .is_some_and(|fallback| fallback.is_stix())
        {
            set.fontmap.extend(db.stix_sizes()?);
        }
        Ok(set)
    }
}
