//! The character maps the autohinter reads: FreeType's Unicode character
//! map for `FT_Get_Char_Index` / `FT_Get_Next_Char`, and the face as
//! HarfBuzz's own character map sees it, for shaping.

use crate::face::Face;
use crate::types::GlyphId;

/// FreeType's Unicode character map of a face
/// (`FT_Select_Charmap(face, FT_ENCODING_UNICODE)`): code points mapped to
/// existing glyphs, sorted.
#[derive(Clone, Debug)]
pub(crate) struct UnicodeMap {
    entries: Vec<(u32, GlyphId)>,
}

impl UnicodeMap {
    /// The map, `None` when the face has no Unicode character map (and
    /// FreeType's selection fails).
    pub(crate) fn new(face: &Face<'_>) -> Option<Self> {
        let face = face.with_charmap(Some(face.freetype_unicode_charmap()?));
        let count = face.num_glyphs();
        let mut entries: Vec<(u32, GlyphId)> = face
            .mappings()
            .into_iter()
            .filter(|&(_, glyph)| glyph.0 != 0 && glyph.0 < count)
            .collect();
        entries.sort_by_key(|&(code, _)| code);
        entries.dedup_by_key(|&mut (code, _)| code);
        Some(Self { entries })
    }

    /// `FT_Get_Char_Index`: the glyph of `code`, `None` for none.
    pub(crate) fn glyph(&self, code: u32) -> Option<GlyphId> {
        self.entries
            .binary_search_by_key(&code, |&(c, _)| c)
            .ok()
            .map(|index| self.entries[index].1)
    }

    /// The glyphs of the code points `first..=last` in code order, as
    /// `FT_Get_Char_Index` then `FT_Get_Next_Char` walk them.
    pub(crate) fn range(&self, first: u32, last: u32) -> impl Iterator<Item = GlyphId> + '_ {
        let start = self.entries.partition_point(|&(code, _)| code < first);
        self.entries[start..]
            .iter()
            .take_while(move |&&(code, _)| code <= last)
            .map(|&(_, glyph)| glyph)
    }
}

/// `face` with the character map HarfBuzz's font functions use, which may
/// differ from the one FreeType (or matplotlib) selected.
pub(crate) fn harfbuzz_face<'f>(face: &Face<'f>) -> Face<'f> {
    face.with_charmap(face.harfbuzz_charmap())
}
