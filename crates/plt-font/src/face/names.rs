//! The face's names: PostScript glyph names (from `CFF` or `post`) and
//! the family, style and full names FreeType reports (from `name`).

use super::{Face, Outlines};
use crate::tables::name::{NameKey, id};
use crate::tables::os2::Os2;
use crate::tables::post::PostTable;
use crate::types::GlyphId;

impl<'a> Face<'a> {
    /// Whether glyph names are available.
    pub fn has_glyph_names(&self) -> bool {
        match &self.outlines {
            Outlines::Cff(cff) => cff.has_glyph_names(),
            _ => self.post.as_ref().is_some_and(PostTable::has_names),
        }
    }

    /// PostScript name of `glyph` (from `CFF` or `post`).
    pub fn glyph_name(&self, glyph: GlyphId) -> Option<&'a str> {
        match &self.outlines {
            Outlines::Cff(cff) => cff.glyph_name(glyph),
            _ => self.post.as_ref()?.glyph_name(glyph),
        }
    }

    /// Glyph with PostScript name `name` (FreeType's `FT_Get_Name_Index`).
    pub fn glyph_index_by_name(&self, name: &str) -> Option<GlyphId> {
        match &self.outlines {
            Outlines::Cff(_) => (0..self.maxp.num_glyphs)
                .map(GlyphId)
                .find(|&glyph| self.glyph_name(glyph) == Some(name)),
            _ => self.post.as_ref()?.glyph_by_name(name),
        }
    }

    /// Family name as FreeType reports it (`face->family_name`): the WWS or
    /// typographic family when appropriate, else name id 1, as ASCII.
    pub fn family_name(&self) -> Option<String> {
        self.freetype_names(id::WWS_FAMILY, id::TYPOGRAPHIC_FAMILY, id::FAMILY)
    }

    /// Style name as FreeType reports it (`face->style_name`).
    pub fn style_name(&self) -> Option<String> {
        self.freetype_names(id::WWS_SUBFAMILY, id::TYPOGRAPHIC_SUBFAMILY, id::SUBFAMILY)
    }

    /// FreeType's family/style selection: faces flagged as WWS-conformant
    /// prefer the typographic name; others prefer the WWS name, then the
    /// typographic one; both fall back to the legacy name.
    fn freetype_names(&self, wws: u16, typographic: u16, legacy: u16) -> Option<String> {
        let order: &[u16] = if self.os2.as_ref().is_some_and(Os2::is_wws) {
            &[typographic, legacy]
        } else {
            &[wws, typographic, legacy]
        };
        order
            .iter()
            .find_map(|&name_id| self.name.freetype_string(name_id))
    }

    /// Full name (name id 4) as FreeType would decode it.
    pub fn full_name(&self) -> Option<String> {
        self.name.freetype_string(id::FULL_NAME)
    }

    /// PostScript name as FreeType reports it.
    pub fn postscript_name(&self) -> Option<String> {
        self.name.postscript_name()
    }

    /// The `name` record with exactly this key and id, decoded (Latin-1 for
    /// Macintosh records, UTF-16BE otherwise) — matplotlib's `get_sfnt()`.
    pub fn name_string(&self, key: NameKey, name_id: u16) -> Option<String> {
        self.name.decoded(key, name_id)
    }
}
