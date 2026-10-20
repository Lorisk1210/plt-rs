//! What a text asks of the fonts: the query, the generic-family lists
//! that resolve it and how its glyphs are rendered.

use plt_font::{FontDatabase, FontId, FontQuery, GenericFamilies, Hinting};

use crate::error::TextError;

/// A font query plus the generic-family lists (`font.sans-serif`, ...)
/// in effect when it is resolved.
#[derive(Clone, Debug, PartialEq)]
pub struct FontRequest {
    /// Families, style, weight, stretch and size.
    pub query: FontQuery,
    /// Preference lists for generic family names.
    pub generics: GenericFamilies,
    /// Whether characters no family covers come from the Last Resort font
    /// rather than the first family's missing-glyph box (matplotlib's
    /// `font.enable_last_resort`).
    pub last_resort: bool,
    /// How glyphs are grid-fitted when the text is laid out and rasterized
    /// (matplotlib's `text.hinting` for the Agg backend; vector output and
    /// text paths are never hinted).
    pub hinting: Hinting,
    /// The keyword the weight was given as (`"bold"`, ...), if it was: the
    /// PGF backend sets only weights named bold in bold, as matplotlib's
    /// does.
    pub weight_keyword: Option<&'static str>,
    /// A face to set the text in instead of matching the query
    /// (matplotlib's `FontProperties(fname=...)`).
    pub file: Option<FontId>,
}

impl FontRequest {
    /// A request resolving generic families with matplotlib's defaults,
    /// unhinted.
    pub fn new(query: FontQuery) -> Self {
        Self {
            query,
            generics: GenericFamilies::default(),
            last_resort: true,
            hinting: Hinting::None,
            weight_keyword: None,
            file: None,
        }
    }

    /// The fonts the text is set in, in fallback order: the requested
    /// file's face, else the best match of each family
    /// (`_find_fonts_by_props`).
    pub fn fonts(&self, db: &FontDatabase) -> Result<Vec<FontId>, TextError> {
        Ok(match self.file {
            Some(face) => vec![face],
            None => db.find_fonts_with(&self.query, &self.generics)?,
        })
    }

    /// The font of the text: the requested file's face, else the best
    /// match (`findfont`).
    pub fn font(&self, db: &FontDatabase) -> Result<FontId, TextError> {
        Ok(match self.file {
            Some(face) => face,
            None => db.find_font_with(&self.query, &self.generics)?,
        })
    }
}
