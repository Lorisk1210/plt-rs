//! Parsed faces shared by every glyph lookup of one typesetting run.

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use pltrs_font::{
    AutohintTarget, Face, FontDatabase, FontError, FontId, GlyphId, HintedGlyph, Hinter, Hinting,
    HintingInstance, Size,
};

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

    /// The parsed face `id`; parsing it opens its font, as matplotlib's
    /// mathtext does while it parses a formula (`get_font`), which is
    /// noted.
    pub(crate) fn face(&mut self, id: FontId) -> Result<&Face<'e>, FontError> {
        match self.faces.entry(id) {
            Entry::Occupied(entry) => Ok(entry.into_mut()),
            Entry::Vacant(entry) => {
                if let Some(source) = self.db.source(id) {
                    crate::lookups::note_open(source, true);
                }
                Ok(entry.insert(self.db.face(id)?))
            }
        }
    }

    /// The font database the faces come from.
    pub(crate) fn db(&self) -> &'e FontDatabase {
        self.db
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

/// `glyph` of `face`, face `id` of `db`, at `size` as `FT_Load_Glyph`
/// loads it for `hinting`: grid-fitted by the font's bytecode, the
/// autohinter (whose analysis of the face `db` keeps) or the CFF hinter,
/// or `None` when `FreeType` leaves it unhinted.
pub(crate) fn hinted_glyph(
    db: &FontDatabase,
    (id, face): (FontId, &Face<'_>),
    size: &Size,
    glyph: GlyphId,
    hinting: Hinting,
) -> Result<Option<HintedGlyph>, FontError> {
    Ok(match hinting.hinter(face, true) {
        Hinter::Bytecode => Some(HintingInstance::new(face, size).load_glyph(face, glyph)?),
        Hinter::Autohinter => {
            Some(
                db.autohinter(id)?
                    .load_glyph(face, size, glyph, AutohintTarget::Normal)?,
            )
        }
        Hinter::Cff => Some(face.load_cff_hinted(size, glyph)?),
        Hinter::Unhinted => None,
    })
}
