//! Parsed faces shared by every glyph lookup of one typesetting run.

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use plt_font::{Face, FontDatabase, FontError, FontId, GlyphId};

/// Lazily parsed faces of a font database.
#[derive(Debug)]
pub(crate) struct FaceCache<'e> {
    db: &'e FontDatabase,
    faces: HashMap<FontId, Face<'e>>,
}

impl<'e> FaceCache<'e> {
    /// An empty cache over `db`.
    pub(crate) fn new(db: &'e FontDatabase) -> Self {
        Self {
            db,
            faces: HashMap::new(),
        }
    }

    /// The parsed face `id`.
    pub(crate) fn face(&mut self, id: FontId) -> Result<&Face<'e>, FontError> {
        match self.faces.entry(id) {
            Entry::Occupied(entry) => Ok(entry.into_mut()),
            Entry::Vacant(entry) => Ok(entry.insert(self.db.face(id)?)),
        }
    }

    /// `FT_Get_Char_Index`: the glyph for `code`, or glyph 0.
    pub(crate) fn char_index(&mut self, id: FontId, code: u32) -> Result<GlyphId, FontError> {
        Ok(self
            .face(id)?
            .glyph_index_for_code(code)
            .unwrap_or(GlyphId(0)))
    }

    /// The family name `FreeType` reports.
    pub(crate) fn family_name(&mut self, id: FontId) -> Result<Option<String>, FontError> {
        Ok(self.face(id)?.family_name())
    }
}
