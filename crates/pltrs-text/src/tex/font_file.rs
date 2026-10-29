//! The Type 1 font a TeX font is drawn with (what matplotlib's
//! `DviFont.resolve_path`, `effects` and `_index_dvi_to_freetype` find),
//! and its glyphs loaded as `FT2Font` loads them.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use pltrs_core::{Affine2D, Path};
use pltrs_font::{GlyphId, Type1Effects, Type1Face, Type1Glyph};

use super::dvi::DviFont;
use super::error::{TexError, TexFile};
use crate::engine::PathSink;

/// How many glyph outlines a font keeps before it starts afresh.
const OUTLINE_CACHE_LIMIT: usize = 8192;

/// How a Type 1 glyph is loaded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TexHinting {
    /// Unhinted (`LoadFlags.NO_HINTING`).
    None,
    /// Hinted by FreeType's Adobe engine (`TARGET_LIGHT`, `DEFAULT`).
    Adobe,
}

/// A loaded glyph outline's identity in its font's cache: the glyph, the
/// bits of its size and resolution, and its hinting.
type OutlineKey = (u16, u64, u64, TexHinting);

/// The Type 1 font that draws a TeX font: its file and face, the effects
/// the font map gives it, the glyph of every character code of DVI files,
/// and the glyph outlines loaded so far.
#[derive(Debug)]
pub struct TexFontFile {
    /// The Type 1 file.
    pub path: PathBuf,
    /// Its face, as FreeType opens it.
    pub face: Arc<Type1Face>,
    /// The `SlantFont` and `ExtendFont` effects of the font map.
    pub effects: Type1Effects,
    /// The glyph of each character code.
    glyphs: Vec<GlyphId>,
    outlines: Mutex<HashMap<OutlineKey, Path>>,
}

impl TexFontFile {
    /// The font at `path` opened as `face`, with the `effects` of the font
    /// map and the glyph of each character code.
    pub(crate) fn new(
        path: PathBuf,
        face: Arc<Type1Face>,
        effects: Type1Effects,
        glyphs: Vec<GlyphId>,
    ) -> Self {
        Self {
            path,
            face,
            effects,
            glyphs,
            outlines: Mutex::default(),
        }
    }

    /// The glyph that character `code` of `font` shows
    /// (`_index_dvi_to_freetype`). Negative codes count from the end, as
    /// Python's indexing does.
    ///
    /// # Errors
    ///
    /// When `code` is beyond the font's encoding.
    pub fn glyph(&self, font: &DviFont, code: i64) -> Result<GlyphId, TexError> {
        let len = self.glyphs.len() as i64;
        let index = if code < 0 { code + len } else { code };
        usize::try_from(index)
            .ok()
            .and_then(|index| self.glyphs.get(index).copied())
            .ok_or_else(|| {
                TexError::malformed(
                    TexFile::Encoding,
                    format!("character {code} is beyond the encoding of {}", font.name()),
                )
            })
    }

    /// Glyph `index` loaded at `points` and `dpi` as `FT2Font` loads it
    /// after `set_size` (`load_glyph` with `hinting`), in 26.6 pixels.
    ///
    /// # Errors
    ///
    /// When the glyph cannot be loaded.
    pub fn load(
        &self,
        index: GlyphId,
        points: f64,
        dpi: f64,
        hinting: TexHinting,
    ) -> Result<Type1Glyph, TexError> {
        let size = self.face.size(points, dpi);
        match hinting {
            TexHinting::None => self
                .face
                .load_glyph(index)
                .map(|glyph| glyph.scaled(size.x_scale(), size.y_scale())),
            TexHinting::Adobe => self.face.load_glyph_hinted(index, &size),
        }
        .map_err(|err| TexError::malformed(TexFile::Type1, err.to_string()))
    }

    /// The outline of glyph `index` at `points` and `dpi`, in pixels
    /// (`FT2Font.set_size`, `load_glyph`, `get_path`), from the cache
    /// when possible.
    ///
    /// # Errors
    ///
    /// When the glyph cannot be loaded.
    pub fn outline(
        &self,
        index: GlyphId,
        points: f64,
        dpi: f64,
        hinting: TexHinting,
    ) -> Result<Path, TexError> {
        let key = (index.0, points.to_bits(), dpi.to_bits(), hinting);
        let outlines = || self.outlines.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(outline) = outlines().get(&key) {
            return Ok(outline.clone());
        }
        let mut sink = PathSink::default();
        self.load(index, points, dpi, hinting)?.outline(&mut sink);
        // From FreeType's 26.6 units to pixels.
        let outline = sink
            .finish()
            .transformed(&Affine2D::scaling(1.0 / 64.0, 1.0 / 64.0));
        let mut cache = outlines();
        if cache.len() >= OUTLINE_CACHE_LIMIT {
            cache.clear();
        }
        cache.insert(key, outline.clone());
        Ok(outline)
    }
}
