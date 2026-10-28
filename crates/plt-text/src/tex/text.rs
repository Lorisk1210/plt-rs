//! A line of TeX text as backends receive it (the `s` and `prop` of
//! matplotlib's `draw_tex`), its glyphs resolved to Type 1 fonts, and its
//! outline (`TextToPath.get_text_path(prop, s, ismath="TeX")`).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use plt_core::{Affine2D, Path, Point};
use plt_font::{GlyphId, Type1Effects, Type1Face, Type1Glyph};

use super::dvi::{DviFont, DviPage};
use super::error::{TexError, TexFile};
use super::manager::TexManager;
use crate::engine::{FontRequest, PathSink};

/// The size text paths are typeset at (`TextToPath.FONT_SCALE`).
pub const TEX_FONT_SCALE: f64 = 100.0;

/// The resolution text paths are typeset at (`TextToPath.DPI`).
pub const TEX_PATH_DPI: f64 = 72.0;

/// How many glyph outlines are kept before the cache starts afresh.
const OUTLINE_CACHE_LIMIT: usize = 8192;

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
    /// Its Type 1 file.
    pub path: PathBuf,
    /// The Type 1 face.
    pub face: Arc<Type1Face>,
    /// The glyph in the face (`Text.index`).
    pub index: GlyphId,
    /// The character code in the DVI file (`Text.glyph`).
    pub code: i64,
    /// Position of the glyph origin on the page, y up.
    pub x: f64,
    /// See `x`.
    pub y: f64,
    /// Font size in points (`Text.font_size`).
    pub size: f64,
    /// The `SlantFont` and `ExtendFont` effects of the font map.
    pub effects: Type1Effects,
}

/// How a Type 1 glyph is loaded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TexHinting {
    /// Unhinted (`LoadFlags.NO_HINTING`).
    None,
    /// Hinted by FreeType's Adobe engine (`TARGET_LIGHT`, `DEFAULT`).
    Adobe,
}

/// A loaded glyph outline's identity in the cache.
type OutlineKey = (PathBuf, u16, u64, u64, TexHinting);

fn outline_cache() -> &'static Mutex<HashMap<OutlineKey, Path>> {
    static CACHE: OnceLock<Mutex<HashMap<OutlineKey, Path>>> = OnceLock::new();
    CACHE.get_or_init(Mutex::default)
}

impl TexGlyph {
    /// The glyph's outline in pixels relative to its origin, loaded at its
    /// size and `dpi` as `FT2Font.get_path` gives it (from the cache when
    /// possible).
    ///
    /// # Errors
    ///
    /// When the glyph cannot be loaded.
    pub fn outline(&self, dpi: f64, hinting: TexHinting) -> Result<Path, TexError> {
        glyph_outline(&self.path, &self.face, self.index, self.size, dpi, hinting)
    }
}

/// Glyph `index` of `face` loaded at `points` and `dpi` as `FT2Font`
/// loads it after `set_size` (`load_glyph` with `hinting`), in 26.6
/// pixels.
///
/// # Errors
///
/// When the glyph cannot be loaded.
pub fn load_glyph(
    face: &Type1Face,
    index: GlyphId,
    points: f64,
    dpi: f64,
    hinting: TexHinting,
) -> Result<Type1Glyph, TexError> {
    let size = face.size(points, dpi);
    match hinting {
        TexHinting::None => face
            .load_glyph(index)
            .map(|glyph| glyph.scaled(size.x_scale(), size.y_scale())),
        TexHinting::Adobe => face.load_glyph_hinted(index, &size),
    }
    .map_err(|err| TexError::malformed(TexFile::Type1, err.to_string()))
}

/// The outline of glyph `index` of `face` (read from `path`) at `points`
/// and `dpi`, in pixels (`FT2Font.set_size`, `load_glyph`, `get_path`).
///
/// # Errors
///
/// When the glyph cannot be loaded.
pub fn glyph_outline(
    path: &std::path::Path,
    face: &Type1Face,
    index: GlyphId,
    points: f64,
    dpi: f64,
    hinting: TexHinting,
) -> Result<Path, TexError> {
    let key = (
        path.to_path_buf(),
        index.0,
        points.to_bits(),
        dpi.to_bits(),
        hinting,
    );
    if let Some(outline) = outline_cache()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&key)
    {
        return Ok(outline.clone());
    }
    let glyph = load_glyph(face, index, points, dpi, hinting)?;
    let mut sink = PathSink::default();
    glyph.outline(&mut sink);
    // From FreeType's 26.6 units to pixels.
    let outline = sink
        .finish()
        .transformed(&Affine2D::scaling(1.0 / 64.0, 1.0 / 64.0));
    let mut cache = outline_cache()
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if cache.len() >= OUTLINE_CACHE_LIMIT {
        cache.clear();
    }
    cache.insert(key, outline.clone());
    Ok(outline)
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
                Ok(TexGlyph {
                    path: files.font_path(&text.font)?,
                    face: files.face(&text.font)?,
                    index: files.glyph_index(&text.font, text.glyph)?,
                    effects: files.effects(&text.font)?,
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
            let outline = glyph_outline(
                &glyph.path,
                &glyph.face,
                glyph.index,
                TEX_FONT_SCALE,
                TEX_PATH_DPI,
                TexHinting::Adobe,
            )?;
            let scale = glyph.size / TEX_FONT_SCALE;
            parts.push(
                outline.transformed(&Affine2D::scaling(scale, scale).translate(glyph.x, glyph.y)),
            );
        }
        parts.extend(page.boxes.iter().map(|b| {
            Path::compound_from_polygons(&[[
                Point::new(b.x, b.y),
                Point::new(b.x + b.width, b.y),
                Point::new(b.x + b.width, b.y + b.height),
                Point::new(b.x, b.y + b.height),
                Point::new(b.x, b.y),
            ]])
        }));
        Ok(Path::compound(&parts))
    }
}
