//! Which font file and character code draw a symbol: matplotlib's
//! `_get_glyph` for the `BaKoMa`, Unicode (custom), `DejaVu` and STIX
//! fontsets, with their fallback chains and the STIX virtual fonts, plus
//! pltrs' `DejaVu` symbols for the custom fontset. The fonts each fontset
//! loads are in `fontsets`.

mod fontsets;

use pltrs_font::FontId;

use super::GlyphError;
use super::faces::FaceCache;
use crate::mathtext::data::{
    self, BakomaFile, is_digit, is_greek_capital, is_letter, latex_to_bakoma, unicode_index,
    virtual_font,
};
use crate::mathtext::font_name::FontName;

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
