//! A line of TeX text as backends receive it (the `s` and `prop` of
//! matplotlib's `draw_tex`), its glyphs resolved to Type 1 fonts, and its
//! outline (`TextToPath.get_text_path(prop, s, ismath="TeX")`).

use std::sync::Arc;

use plt_core::{Affine2D, Path};
use plt_font::{GlyphId, Type1Glyph};

use super::dvi::{DviBox, DviFont, DviPage};
use super::error::TexError;
use super::font_file::{TexFontFile, TexHinting};
use super::manager::TexManager;
use crate::engine::FontRequest;

/// The size text paths are typeset at (`TextToPath.FONT_SCALE`).
pub const TEX_FONT_SCALE: f64 = 100.0;

/// The resolution text paths are typeset at (`TextToPath.DPI`).
pub const TEX_PATH_DPI: f64 = 72.0;

/// One line of TeX text: the string given to TeX, the font it is drawn
/// with (only its size matters to TeX; PGF output names the family) and
/// the manager typesetting it.
#[derive(Clone, Debug, PartialEq)]
pub struct TexText {
    /// The TeX string.
    pub tex: String,
    /// The text's font.
    pub font: FontRequest,
    /// Typesets the string.
    pub manager: TexManager,
}

/// A glyph of a typeset page resolved to the Type 1 font that draws it.
#[derive(Clone, Debug)]
pub struct TexGlyph {
    /// The DVI font.
    pub font: Arc<DviFont>,
    /// The Type 1 font drawing it.
    pub file: Arc<TexFontFile>,
    /// The glyph in the Type 1 face (`Text.index`).
    pub index: GlyphId,
    /// The character code in the DVI file (`Text.glyph`).
    pub code: i64,
    /// Position of the glyph origin on the page, y up.
    pub x: f64,
    /// See `x`.
    pub y: f64,
    /// Font size in points (`Text.font_size`).
    pub size: f64,
}

impl TexGlyph {
    /// The glyph loaded at its size and `dpi` as `FT2Font` loads it, in
    /// 26.6 pixels.
    ///
    /// # Errors
    ///
    /// When the glyph cannot be loaded.
    pub fn load(&self, dpi: f64, hinting: TexHinting) -> Result<Type1Glyph, TexError> {
        self.file.load(self.index, self.size, dpi, hinting)
    }

    /// The glyph's outline in pixels relative to its origin, loaded at its
    /// size and `dpi` as `FT2Font.get_path` gives it.
    ///
    /// # Errors
    ///
    /// When the glyph cannot be loaded.
    pub fn outline(&self, dpi: f64, hinting: TexHinting) -> Result<Path, TexError> {
        self.file.outline(self.index, self.size, dpi, hinting)
    }

    /// The outline `TextToPath` makes of the glyph: loaded at
    /// [`TEX_FONT_SCALE`] points and [`TEX_PATH_DPI`] with light hinting.
    /// Scale it by `size / TEX_FONT_SCALE`.
    ///
    /// # Errors
    ///
    /// When the glyph cannot be loaded.
    pub fn path_outline(&self) -> Result<Path, TexError> {
        self.file
            .outline(self.index, TEX_FONT_SCALE, TEX_PATH_DPI, TexHinting::Adobe)
    }
}

impl TexText {
    /// A line of TeX text.
    pub fn new(tex: impl Into<String>, font: FontRequest, manager: TexManager) -> Self {
        Self {
            tex: tex.into(),
            font,
            manager,
        }
    }

    /// The font size in points.
    pub fn size(&self) -> f64 {
        self.font.query.size
    }

    /// The page TeX typesets the text on at `fontsize`, in pixels at `dpi`.
    ///
    /// # Errors
    ///
    /// As [`TexManager::page`].
    pub fn page(&self, fontsize: f64, dpi: f64) -> Result<DviPage, TexError> {
        self.manager.page(&self.tex, fontsize, dpi)
    }

    /// The glyphs of `page` resolved to their Type 1 fonts (each glyph's
    /// `font_path`, `index` and `font_effects`).
    ///
    /// # Errors
    ///
    /// When a font has no Type 1 version or a glyph is not in its
    /// encoding.
    pub fn glyphs(&self, page: &DviPage) -> Result<Vec<TexGlyph>, TexError> {
        let files = self.manager.files();
        page.text
            .iter()
            .map(|text| {
                let file = files.font_file(&text.font)?;
                Ok(TexGlyph {
                    index: file.glyph(&text.font, text.glyph)?,
                    file,
                    font: Arc::clone(&text.font),
                    code: text.glyph,
                    x: text.x,
                    y: text.y,
                    size: text.font.size(),
                })
            })
            .collect()
    }

    /// The text's outline at [`TEX_FONT_SCALE`] points and
    /// [`TEX_PATH_DPI`] (`TextToPath.get_text_path(prop, s,
    /// ismath="TeX")`): every glyph loaded at that size with light hinting
    /// and scaled to its own size, then every rule as a closed rectangle
    /// going right first. Scale it by `size / TEX_FONT_SCALE`.
    ///
    /// # Errors
    ///
    /// When TeX fails or a glyph cannot be resolved or loaded.
    pub fn text_path(&self) -> Result<Path, TexError> {
        let page = self.page(TEX_FONT_SCALE, TEX_PATH_DPI)?;
        let mut parts = Vec::new();
        for glyph in self.glyphs(&page)? {
            let scale = glyph.size / TEX_FONT_SCALE;
            parts.push(
                glyph
                    .path_outline()?
                    .transformed(&Affine2D::scaling(scale, scale).translate(glyph.x, glyph.y)),
            );
        }
        parts.extend(page.boxes.iter().map(DviBox::outline));
        Ok(Path::compound(&parts))
    }
}
