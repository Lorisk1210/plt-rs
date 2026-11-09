//! Glyph metrics for mathtext (matplotlib's `TruetypeFonts` and its
//! subclasses): which face draws each symbol, the symbol's TeX-style box,
//! kerning, sized delimiter variants and the fontset constants.

mod constants;
mod faces;
mod select;

use std::collections::HashMap;
use std::fmt;

use pltrs_font::{FontDatabase, FontError, FontId, GlyphId, KerningMode, Rect, Size};

pub(crate) use constants::FontConstants;
use faces::FaceCache;
use select::GlyphSet;

use super::font_name::FontName;
use super::style::{Fontset, MathStyle};
use crate::error::TextError;

/// Why a glyph could not be provided.
#[derive(Debug)]
pub(crate) enum GlyphError {
    /// The symbol is not a character nor a known TeX symbol.
    UnknownSymbol(String),
    /// A font could not be read.
    Font(FontError),
}

impl fmt::Display for GlyphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSymbol(symbol) => write!(
                f,
                "{symbol:?} is not a valid Unicode character or TeX/Type1 symbol"
            ),
            Self::Font(err) => err.fmt(f),
        }
    }
}

impl From<FontError> for GlyphError {
    fn from(err: FontError) -> Self {
        Self::Font(err)
    }
}

impl From<GlyphError> for TextError {
    fn from(err: GlyphError) -> Self {
        match err {
            GlyphError::UnknownSymbol(_) => Self::Math(err.to_string()),
            GlyphError::Font(err) => err.into(),
        }
    }
}

/// The family of fonts behind a style, with its layout constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MathFonts {
    /// `BaKoMa` Computer Modern.
    Cm,
    /// `DejaVu` Sans or Serif.
    DejaVu {
        /// Serif rather than sans.
        serif: bool,
    },
    /// STIX.
    Stix {
        /// Sans-serif letters.
        sans: bool,
    },
    /// The `mathtext.rm`, ... fonts.
    Custom,
}

impl MathFonts {
    /// The fonts selected by `style`.
    pub(crate) fn of(style: &MathStyle) -> Self {
        match style.fontset {
            Fontset::Cm => Self::Cm,
            Fontset::DejaVuSans => Self::DejaVu { serif: false },
            Fontset::DejaVuSerif => Self::DejaVu { serif: true },
            Fontset::Stix => Self::Stix { sans: false },
            Fontset::StixSans => Self::Stix { sans: true },
            Fontset::Custom => Self::Custom,
        }
    }

    fn constants(self) -> FontConstants {
        match self {
            Self::Cm => FontConstants::computer_modern(),
            Self::DejaVu { serif } => FontConstants::dejavu(serif),
            Self::Stix { sans } => FontConstants::stix(sans),
            Self::Custom => FontConstants::BASE,
        }
    }
}

/// Metrics of a glyph at one size, in pixels (matplotlib's `FontMetrics`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct FontMetrics {
    /// Advance width.
    pub(crate) advance: f64,
    /// Height of the ink box.
    pub(crate) height: f64,
    /// Width of the ink box.
    pub(crate) width: f64,
    /// Left of the ink box.
    pub(crate) xmin: f64,
    /// Right of the ink box.
    pub(crate) xmax: f64,
    /// Bottom of the ink box (y up).
    pub(crate) ymin: f64,
    /// Top of the ink box (y up).
    pub(crate) ymax: f64,
    /// Top of the ink above the baseline (TeX's height).
    pub(crate) iceberg: f64,
    /// Whether scripts treat the glyph as leaning.
    pub(crate) slanted: bool,
}

/// A symbol resolved to a sized glyph (matplotlib's `FontInfo`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FontInfo {
    /// The face.
    pub(crate) font: FontId,
    /// Size in points.
    pub(crate) fontsize: f64,
    /// Character code in the face.
    pub(crate) code: u32,
    /// Glyph index.
    pub(crate) glyph: GlyphId,
    /// Metrics at `fontsize`.
    pub(crate) metrics: FontMetrics,
    /// Vertical offset applied to the glyph (non-zero for `cmex10`).
    pub(crate) offset: f64,
}

/// What a glyph lookup is keyed by: font, font class, symbol and size.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GlyphKey<'a> {
    /// TeX font name.
    pub(crate) font: FontName,
    /// Font class (`rm`, `it`, ...), used by the STIX virtual fonts.
    pub(crate) class: FontName,
    /// The symbol (`x`, `\alpha`, ...).
    pub(crate) symbol: &'a str,
    /// Size in points.
    pub(crate) fontsize: f64,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct InfoKey {
    font: FontName,
    class: FontName,
    symbol: String,
    fontsize: u64,
}

/// The fonts of one typesetting run, with matplotlib's per-run caches.
#[derive(Debug)]
pub(crate) struct Fonts<'e> {
    faces: FaceCache<'e>,
    set: GlyphSet,
    constants: FontConstants,
    /// `mathtext.default`, the class used to measure reference glyphs.
    default_class: FontName,
    dpi: f64,
    infos: HashMap<InfoKey, FontInfo>,
    /// The size each face was last set to: `FreeType` kerning uses the
    /// face's current size, which matplotlib only updates when a glyph
    /// lookup misses its cache.
    current_size: HashMap<FontId, f64>,
}

impl<'e> Fonts<'e> {
    /// The fonts of `style`, with `text_font` as the non-math font.
    pub(crate) fn new(
        db: &'e FontDatabase,
        style: &MathStyle,
        text_font: FontId,
    ) -> Result<Self, TextError> {
        Ok(Self {
            faces: FaceCache::new(db),
            set: GlyphSet::for_style(db, style, text_font)?,
            constants: MathFonts::of(style).constants(),
            default_class: style.default.into(),
            dpi: style.dpi,
            infos: HashMap::new(),
            current_size: HashMap::new(),
        })
    }

    /// The fontset's layout constants.
    pub(crate) fn constants(&self) -> &FontConstants {
        &self.constants
    }

    /// The output resolution.
    pub(crate) fn dpi(&self) -> f64 {
        self.dpi
    }

    /// The sized glyph drawing a symbol (cached like matplotlib's
    /// `_get_info`).
    pub(crate) fn info(&mut self, key: GlyphKey<'_>) -> Result<FontInfo, GlyphError> {
        let cache_key = InfoKey {
            font: key.font,
            class: key.class,
            symbol: key.symbol.to_owned(),
            fontsize: key.fontsize.to_bits(),
        };
        if let Some(info) = self.infos.get(&cache_key) {
            return Ok(*info);
        }
        let info = self.load_info(key)?;
        self.current_size.insert(info.font, key.fontsize);
        self.infos.insert(cache_key, info);
        Ok(info)
    }

    fn load_info(&mut self, key: GlyphKey<'_>) -> Result<FontInfo, GlyphError> {
        let glyph = self
            .set
            .glyph(&mut self.faces, key.font, key.class, key.symbol)?;
        let face = self.faces.face(glyph.font)?;
        // matplotlib's mathtext sets the size of every glyph's font.
        let size = Size::checked(face, key.fontsize, self.dpi).map_err(FontError::from)?;
        let glyph_id = face.glyph_index_for_code(glyph.code).unwrap_or(GlyphId(0));
        let cbox = size.glyph_cbox(face, glyph_id)?.unwrap_or(Rect {
            x_min: 0,
            y_min: 0,
            x_max: 0,
            y_max: 0,
        });
        let px = |v: i32| f64::from(v) / 64.0;
        let height = px(cbox.y_max - cbox.y_min);
        let offset = if face.postscript_name().as_deref() == Some("Cmex10") {
            height / 2.0 + key.fontsize / 3.0 * self.dpi / 72.0
        } else {
            0.0
        };
        let metrics = FontMetrics {
            advance: size.linear_advance(face, glyph_id) as f64 / 65536.0,
            height,
            width: px(cbox.x_max - cbox.x_min),
            xmin: px(cbox.x_min),
            xmax: px(cbox.x_max),
            ymin: px(cbox.y_min) + offset,
            ymax: px(cbox.y_max) + offset,
            iceberg: px(cbox.y_max) + offset,
            slanted: glyph.slanted,
        };
        Ok(FontInfo {
            font: glyph.font,
            fontsize: key.fontsize,
            code: glyph.code,
            glyph: glyph_id,
            metrics,
            offset,
        })
    }

    /// Kerning between two adjacent symbols (zero unless they share font
    /// name and size), from the first glyph's face at its current size.
    pub(crate) fn kern(
        &mut self,
        left: GlyphKey<'_>,
        right: GlyphKey<'_>,
    ) -> Result<f64, GlyphError> {
        #[expect(clippy::float_cmp, reason = "matplotlib kerns only identical sizes")]
        if left.font != right.font || left.fontsize != right.fontsize {
            return Ok(0.0);
        }
        let first = self.info(left)?;
        let second = self.info(right)?;
        let points = self
            .current_size
            .get(&first.font)
            .copied()
            .unwrap_or(first.fontsize);
        let face = self.faces.face(first.font)?;
        let size = Size::new(face, points, self.dpi);
        let kerning = size.kerning(face, first.glyph, second.glyph, KerningMode::Default);
        Ok(kerning as f64 / 64.0)
    }

    fn measure(
        &mut self,
        font: FontName,
        symbol: &str,
        fontsize: f64,
    ) -> Result<FontMetrics, GlyphError> {
        let class = self.default_class;
        Ok(self
            .info(GlyphKey {
                font,
                class,
                symbol,
                fontsize,
            })?
            .metrics)
    }

    /// Height of the math axis (where fraction bars sit).
    pub(crate) fn axis_height(&mut self, font: FontName, fontsize: f64) -> Result<f64, GlyphError> {
        if let Some(axis) = self.constants.axis_height {
            return Ok(axis * fontsize * self.dpi / 72.0);
        }
        // Aligned with the middle of the minus sign.
        let minus = self.measure(font, "\u{2212}", fontsize)?;
        Ok(f64::midpoint(minus.ymax, minus.ymin))
    }

    /// Width of a quad (1 em).
    pub(crate) fn quad(&mut self, font: FontName, fontsize: f64) -> Result<f64, GlyphError> {
        match self.constants.quad {
            Some(quad) => Ok(quad * fontsize * self.dpi / 72.0),
            None => Ok(self.measure(font, "m", fontsize)?.advance),
        }
    }

    /// The x-height.
    pub(crate) fn x_height(&mut self, font: FontName, fontsize: f64) -> Result<f64, GlyphError> {
        match self.constants.x_height {
            Some(x_height) => Ok(x_height * fontsize * self.dpi / 72.0),
            None => Ok(self.measure(font, "x", fontsize)?.iceberg),
        }
    }

    /// Thickness of rules (fraction bars, radicals, overlines).
    pub(crate) fn underline_thickness(&self, fontsize: f64) -> f64 {
        ((0.75 / 12.0) * fontsize * self.dpi) / 72.0
    }

    /// Size variants of a symbol, smallest first.
    pub(crate) fn sized_alternatives(
        &mut self,
        font: FontName,
        symbol: &str,
    ) -> Result<Vec<(FontName, String)>, GlyphError> {
        self.set.sized_alternatives(&mut self.faces, font, symbol)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mathtext::style::{MathFallback, MathFont};
    use pltrs_font::FontQuery;

    fn fonts(db: &FontDatabase, fontset: Fontset) -> Fonts<'_> {
        let style = MathStyle::default().with_fontset(fontset).with_dpi(100.0);
        let text = db.find_font(&style.font.query).expect("text font");
        Fonts::new(db, &style, text).expect("fonts")
    }

    fn key(font: FontName, symbol: &str) -> GlyphKey<'_> {
        GlyphKey {
            font,
            class: FontName::Rm,
            symbol,
            fontsize: 10.0,
        }
    }

    #[test]
    fn dejavu_letters_are_italic() {
        let db = FontDatabase::with_bundled_fonts();
        let mut fonts = fonts(&db, Fontset::DejaVuSans);
        let x = fonts.info(key(FontName::Normal, "x")).expect("x");
        assert!(x.metrics.slanted);
        assert!(x.metrics.advance > 0.0 && x.metrics.width > 0.0);
        let two = fonts.info(key(FontName::Normal, "2")).expect("2");
        assert!(!two.metrics.slanted);
        assert!(matches!(
            fonts.info(key(FontName::Normal, "\\[")),
            Err(GlyphError::UnknownSymbol(_))
        ));
    }

    #[test]
    fn cmex_glyphs_are_offset() {
        let db = FontDatabase::with_bundled_fonts();
        let mut fonts = fonts(&db, Fontset::Cm);
        let sum = fonts.info(key(FontName::Rm, "\\sum")).expect("sum");
        assert!(sum.offset > 0.0);
        assert_eq!(sum.code, 0x58);
        let alts = fonts
            .sized_alternatives(FontName::Normal, "(")
            .expect("parens");
        assert_eq!(alts.len(), 5);
    }

    #[test]
    fn cmr10_text_takes_its_minus_from_cmsy10() {
        // matplotlib's test_mathtext_cmr10_minus_sign: cmr10 has no minus
        // sign, so the STIX fallback of Computer Modern takes it from
        // cmsy10, as every Unicode fontset does.
        let db = FontDatabase::with_bundled_fonts();
        let cmsy10 = db
            .find_font(&FontQuery::families(["cmsy10"]))
            .expect("cmsy10");
        for fontset in [Fontset::Cm, Fontset::Stix, Fontset::DejaVuSans] {
            let style = MathStyle::default().with_fontset(fontset).with_dpi(100.0);
            let cmr10 = db
                .find_font(&FontQuery::families(["cmr10"]))
                .expect("cmr10");
            let mut fonts = Fonts::new(&db, &style, cmr10).expect("fonts");
            let minus = fonts
                .info(key(FontName::Default, "\u{2212}"))
                .expect("minus");
            assert_eq!((minus.font, minus.code), (cmsy10, 0xa1), "{fontset:?}");
        }
    }

    #[test]
    fn dejavu_symbols_draw_the_operators_of_the_custom_fontset() {
        let db = FontDatabase::with_bundled_fonts();
        let family = |name: &str| db.find_font(&FontQuery::families([name])).expect(name);
        let mut style = MathStyle::default()
            .with_fontset(Fontset::Custom)
            .with_dpi(100.0);
        style.custom.rm = "Inter".to_owned();
        style.custom.it = "Inter:italic".to_owned();
        style.fallback = Some(MathFallback::StixSans);
        let text = db.find_font(&style.font.query).expect("text font");
        let font_of = |style: &MathStyle, font, symbol| {
            let mut fonts = Fonts::new(&db, style, text).expect("fonts");
            fonts.info(key(font, symbol)).expect(symbol).font
        };
        // matplotlib's custom fontset: Inter's text-size operators, STIX
        // for what Inter lacks.
        assert_eq!(font_of(&style, FontName::Normal, "\\sum"), family("Inter"));
        assert_eq!(
            font_of(&style, FontName::Normal, "\\nabla"),
            family("STIXGeneral")
        );
        style.dejavu_symbols = true;
        let display = family("DejaVu Sans Display");
        assert_eq!(font_of(&style, FontName::Normal, "\\sum"), display);
        assert_eq!(font_of(&style, FontName::Bf, "\\int"), display);
        assert_eq!(
            font_of(&style, FontName::Normal, "\\nabla"),
            family("DejaVu Sans")
        );
        assert_eq!(font_of(&style, FontName::Normal, "2"), family("Inter"));
        // What DejaVu lacks still comes from the fallback.
        assert_eq!(font_of(&style, FontName::Bb, "R"), family("STIXGeneral"));
    }

    #[test]
    fn constants_and_measures() {
        let db = FontDatabase::with_bundled_fonts();
        let mut fonts = fonts(&db, Fontset::Custom);
        assert_eq!(fonts.constants().axis_height, None);
        assert!(fonts.x_height(FontName::Normal, 10.0).expect("x") > 0.0);
        assert!(fonts.quad(FontName::Normal, 10.0).expect("m") > 0.0);
        assert!(fonts.axis_height(FontName::Normal, 10.0).expect("minus") > 0.0);
        assert_eq!(fonts.underline_thickness(12.0), 0.75 * 100.0 / 72.0);
        assert_eq!(MathFont::default(), MathFont::Normal);
    }
}
