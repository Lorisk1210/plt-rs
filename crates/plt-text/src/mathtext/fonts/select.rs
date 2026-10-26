//! Which font file and character code draw a symbol: matplotlib's
//! `_get_glyph` for the `BaKoMa`, Unicode (custom), `DejaVu` and STIX
//! fontsets, with their fallback chains and the STIX virtual fonts, plus
//! plt-rs' `DejaVu` symbols for the custom fontset.

use plt_font::{FontDatabase, FontId, FontQuery, GenericFamilies};

use super::faces::FaceCache;
use super::{GlyphError, MathFonts};
use crate::error::TextError;
use crate::mathtext::data::{
    self, BakomaFile, is_digit, is_greek_capital, is_letter, latex_to_bakoma, unicode_index,
    virtual_font,
};
use crate::mathtext::font_name::FontName;
use crate::mathtext::pattern::parse_pattern;
use crate::mathtext::style::{MathFallback, MathStyle, PatternDefaults};

/// The glyph source for a symbol (before sizing).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Glyph {
    /// The face.
    pub(crate) font: FontId,
    /// Character code in the face's character map.
    pub(crate) code: u32,
    /// Whether the glyph leans (affects script placement).
    pub(crate) slanted: bool,
}

/// The fontset families, each with its own glyph selection rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Bakoma,
    Custom,
    DejaVu,
    Stix { sans: bool },
}

/// One of matplotlib's `Fonts` objects: a font map plus fallbacks.
#[derive(Clone, Debug)]
pub(crate) struct GlyphSet {
    kind: Kind,
    fontmap: Vec<(FontName, FontId)>,
    bakoma_files: Vec<(BakomaFile, FontId)>,
    /// The text font (`default` and `regular`).
    text_font: FontId,
    /// Where symbols missing from this set come from.
    fallback: Option<Box<GlyphSet>>,
    /// The `DejaVu` Sans set a custom set takes its symbols from
    /// ([`MathStyle::dejavu_symbols`]): its big operators always, and what
    /// the custom fonts lack before `fallback`.
    symbols: Option<Box<GlyphSet>>,
    /// `DejaVu` takes `\prime` from Computer Modern.
    prime: Option<Box<GlyphSet>>,
    /// `mathtext.default`, used for STIX's placeholder glyphs.
    math_default: FontName,
    /// `cmsy10`, which supplies `×` and `−` missing from `cmr10`.
    cmsy10: Option<FontId>,
}

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
pub(crate) struct Finder<'a> {
    /// The fonts.
    pub(crate) db: &'a FontDatabase,
    /// `font.serif`, `font.sans-serif`, ... lists.
    pub(crate) generics: &'a GenericFamilies,
    /// What a pattern leaves out ([`MathStyle::pattern_defaults`]).
    pub(crate) defaults: PatternDefaults,
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

/// `\int` and `\oint` lean whatever their font.
fn is_slanted_symbol(symbol: &str) -> bool {
    matches!(symbol, "\\int" | "\\oint")
}

/// The only character of a one-character symbol.
fn single_char(symbol: &str) -> Option<char> {
    let mut chars = symbol.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Some(c),
        _ => None,
    }
}

/// STIX's swapped `⋒`/`⋓` glyphs.
fn stix_glyph_fix(code: u32) -> u32 {
    match code {
        0x22d2 => 0x22d3,
        0x22d3 => 0x22d2,
        other => other,
    }
}

/// Glyphs `cmr10` lacks and the `cmsy10` codes that draw them.
fn cmr10_substitution(code: u32) -> Option<u32> {
    match code {
        0x00d7 => Some(0x00a3),
        0x2212 => Some(0x00a1),
        _ => None,
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

    fn is_stix(&self) -> bool {
        matches!(self.kind, Kind::Stix { .. })
    }

    fn in_fontmap(&self, name: FontName) -> bool {
        self.fontmap.iter().any(|(font, _)| *font == name)
    }

    /// matplotlib's `_get_font`: the face for a TeX font name.
    fn font(&self, name: FontName) -> Option<FontId> {
        if name.is_text_font() {
            return Some(self.text_font);
        }
        self.fontmap
            .iter()
            .find(|(font, _)| *font == name)
            .map(|(_, id)| *id)
    }

    fn bakoma_file(&self, file: BakomaFile) -> Option<FontId> {
        self.bakoma_files
            .iter()
            .find(|(f, _)| *f == file)
            .map(|(_, id)| *id)
    }

    /// Picks the face and character code drawing `symbol` in `font`.
    pub(crate) fn glyph(
        &self,
        faces: &mut FaceCache<'_>,
        font: FontName,
        class: FontName,
        symbol: &str,
    ) -> Result<Glyph, GlyphError> {
        match self.kind {
            Kind::Bakoma => self.bakoma_glyph(faces, font, class, symbol),
            Kind::DejaVu => self.dejavu_glyph(faces, font, class, symbol),
            Kind::Custom => self.custom_glyph(faces, font, class, symbol),
            Kind::Stix { .. } => self.unicode_glyph(faces, font, class, symbol),
        }
    }

    /// Whether this set's display font (`DejaVu` Sans or Serif Display,
    /// which holds the big operators) has a glyph for `code`.
    fn has_display_glyph(&self, faces: &mut FaceCache<'_>, code: u32) -> Result<bool, GlyphError> {
        match self.font(FontName::Ex) {
            Some(display) => Ok(faces.char_index(display, code)?.0 != 0),
            None => Ok(false),
        }
    }

    fn fallback_glyph(
        &self,
        faces: &mut FaceCache<'_>,
        font: FontName,
        class: FontName,
        symbol: &str,
    ) -> Result<Glyph, GlyphError> {
        match &self.fallback {
            Some(fallback) => fallback.glyph(faces, font, class, symbol),
            None => Err(GlyphError::UnknownSymbol(symbol.to_owned())),
        }
    }

    fn bakoma_glyph(
        &self,
        faces: &mut FaceCache<'_>,
        font: FontName,
        class: FontName,
        symbol: &str,
    ) -> Result<Glyph, GlyphError> {
        let mut font = font;
        let mut candidate = None;
        if let Some((file, code)) = latex_to_bakoma(symbol).filter(|_| self.in_fontmap(font)) {
            let slanted = file == BakomaFile::Cmmi10 || is_slanted_symbol(symbol);
            candidate = self.bakoma_file(file).map(|id| (id, code, slanted));
        } else if let Some(c) = single_char(symbol) {
            let mut slanted = matches!(font, FontName::It | FontName::Normal);
            if font == FontName::Normal && is_digit(c) {
                // Digits come from the roman font, as in LaTeX.
                font = FontName::Rm;
                slanted = false;
            }
            candidate = self.font(font).map(|id| (id, u32::from(c), slanted));
        }
        if let Some((id, code, slanted)) = candidate
            && faces.char_index(id, code)?.0 != 0
        {
            return Ok(Glyph {
                font: id,
                code,
                slanted,
            });
        }
        self.fallback_glyph(faces, font, class, symbol)
    }

    fn dejavu_glyph(
        &self,
        faces: &mut FaceCache<'_>,
        font: FontName,
        class: FontName,
        symbol: &str,
    ) -> Result<Glyph, GlyphError> {
        if symbol == "\\prime"
            && let Some(prime) = &self.prime
        {
            return prime.glyph(faces, font, class, symbol);
        }
        let code =
            unicode_index(symbol).ok_or_else(|| GlyphError::UnknownSymbol(symbol.to_owned()))?;
        if self.has_display_glyph(faces, code)? {
            return self.unicode_glyph(faces, FontName::Ex, class, symbol);
        }
        self.unicode_glyph(faces, font, class, symbol)
    }

    /// The custom fontset's glyph: with `DejaVu` symbols, the big operators
    /// come from them as in the `DejaVu` fontsets; everything else as
    /// matplotlib's `UnicodeFonts` picks it.
    fn custom_glyph(
        &self,
        faces: &mut FaceCache<'_>,
        font: FontName,
        class: FontName,
        symbol: &str,
    ) -> Result<Glyph, GlyphError> {
        if let Some(symbols) = &self.symbols
            && let Some(code) = unicode_index(symbol)
            && symbols.has_display_glyph(faces, code)?
        {
            return symbols.glyph(faces, font, class, symbol);
        }
        self.unicode_glyph(faces, font, class, symbol)
    }

    /// matplotlib's `UnicodeFonts._get_glyph`.
    fn unicode_glyph(
        &self,
        faces: &mut FaceCache<'_>,
        font: FontName,
        class: FontName,
        symbol: &str,
    ) -> Result<Glyph, GlyphError> {
        let index = unicode_index(symbol);
        let (font, code) = self.map_virtual_font(font, class, index.unwrap_or(u32::from('?')));
        if index.is_some() {
            let font_name = if font == FontName::Normal && code < 0x10000 {
                if !is_letter(code) || is_greek_capital(code) {
                    FontName::Rm
                } else {
                    FontName::It
                }
            } else {
                font
            };
            let slanted = font_name == FontName::It || is_slanted_symbol(symbol);
            if let Some(mut id) = self.font(font_name) {
                let mut code = code;
                if let Some(substitute) = cmr10_substitution(code)
                    && let Some(cmsy10) = self.cmsy10
                    && faces.family_name(id)?.as_deref() == Some("cmr10")
                {
                    id = cmsy10;
                    code = substitute;
                }
                if faces.char_index(id, code)?.0 != 0 {
                    return Ok(Glyph {
                        font: id,
                        code,
                        slanted,
                    });
                }
            }
        }
        let regular = matches!(font, FontName::It | FontName::Regular | FontName::Normal);
        // The `DejaVu` symbols of a custom set come before its fallback.
        if let Some(next) = self.symbols.as_ref().or(self.fallback.as_ref()) {
            let font = if regular && next.is_stix() {
                FontName::Rm
            } else {
                font
            };
            return next.glyph(faces, font, class, symbol);
        }
        if regular && self.is_stix() {
            return self.glyph(faces, FontName::Rm, class, symbol);
        }
        // A placeholder (the currency sign) for lack of anything better.
        let id = self
            .font(FontName::Rm)
            .ok_or_else(|| GlyphError::UnknownSymbol(symbol.to_owned()))?;
        Ok(Glyph {
            font: id,
            code: 0xa4,
            slanted: false,
        })
    }

    /// STIX's virtual fonts: alphabets stored at other code points or in
    /// other faces. Other fontsets map nothing.
    fn map_virtual_font(&self, font: FontName, class: FontName, code: u32) -> (FontName, u32) {
        let Kind::Stix { sans } = self.kind else {
            return (font, code);
        };
        let (mut font, mut code) = (font, code);
        let mut mapping = virtual_font(font);
        let mut sans_conversion = false;
        if sans && mapping.is_none() && !font.is_text_font() {
            mapping = virtual_font(FontName::Sf);
            sans_conversion = true;
        }
        if let Some(ranges) = mapping.and_then(|mapping| mapping.ranges(class)) {
            match ranges.iter().find(|r| r.start <= code && code <= r.end) {
                Some(range) => {
                    code = code - range.start + range.target;
                    font = range.font;
                }
                None if !sans_conversion => {
                    // A dummy character in the default font.
                    code = 1;
                    font = self.math_default;
                }
                None => {}
            }
        }
        if matches!(font, FontName::Rm | FontName::It | FontName::Normal) {
            code = stix_glyph_fix(code);
        }
        if (0xe000..=0xf8ff).contains(&code)
            && let Some(private) = font.non_unicode()
        {
            font = private;
        }
        (font, code)
    }

    /// Variants of `symbol` from smallest to largest (for auto-sized
    /// delimiters, radicals and wide accents).
    pub(crate) fn sized_alternatives(
        &self,
        faces: &mut FaceCache<'_>,
        font: FontName,
        symbol: &str,
    ) -> Result<Vec<(FontName, String)>, GlyphError> {
        let single = || vec![(font, symbol.to_owned())];
        match self.kind {
            Kind::Bakoma => Ok(data::bakoma_size_alternatives(symbol).map_or_else(
                single,
                |alts| {
                    alts.iter()
                        .map(|&(font, symbol)| (font, symbol.to_owned()))
                        .collect()
                },
            )),
            Kind::Stix { .. } => self.stix_alternatives(faces, font, symbol),
            Kind::Custom | Kind::DejaVu => match &self.fallback {
                Some(fallback) => fallback.sized_alternatives(faces, font, symbol),
                None => Ok(single()),
            },
        }
    }

    fn stix_alternatives(
        &self,
        faces: &mut FaceCache<'_>,
        font: FontName,
        symbol: &str,
    ) -> Result<Vec<(FontName, String)>, GlyphError> {
        let symbol = match symbol {
            "\\{" => "{",
            "\\}" => "}",
            "\\[" => "[",
            "\\]" => "]",
            "<" => "\u{27e8}",
            ">" => "\u{27e9}",
            other => other,
        };
        let Some(c) = unicode_index(symbol).and_then(char::from_u32) else {
            return Ok(vec![(font, symbol.to_owned())]);
        };
        let mut alternatives = Vec::new();
        for size in 0..6 {
            if let Some(id) = self.font(FontName::Size(size))
                && faces.char_index(id, u32::from(c))?.0 != 0
            {
                alternatives.push((FontName::Size(size), c.to_string()));
            }
        }
        if symbol == "\\__sqrt__" {
            // The largest STIX radical is disconnected from its stem.
            alternatives.pop();
        }
        Ok(alternatives)
    }
}
