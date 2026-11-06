//! The face's character maps: which one is active and how characters map
//! to glyphs through it.

use super::Face;
use crate::error::{FontError, Result};
use crate::tables::cmap::CharmapInfo;
use crate::types::GlyphId;

impl Face<'_> {
    /// All usable character maps, in file order.
    pub fn charmaps(&self) -> Vec<CharmapInfo> {
        self.cmap.infos().collect()
    }

    /// The active character map.
    pub fn charmap(&self) -> Option<CharmapInfo> {
        self.charmap
            .and_then(|index| self.cmap.get(index))
            .map(|(info, _)| info)
    }

    /// The character map `FT_Select_Charmap(face, FT_ENCODING_UNICODE)`
    /// activates, as an index into [`Self::charmaps`].
    pub(crate) fn freetype_unicode_charmap(&self) -> Option<usize> {
        self.cmap.freetype_unicode_index()
    }

    /// The character map HarfBuzz's own font functions use, as an index
    /// into [`Self::charmaps`].
    pub(crate) fn harfbuzz_charmap(&self) -> Option<usize> {
        self.cmap.harfbuzz_index()
    }

    /// This face with character map `index` active; `None` maps no
    /// character.
    pub(crate) fn with_charmap(&self, index: Option<usize>) -> Self {
        Self {
            charmap: index,
            ..self.clone()
        }
    }

    /// Activates character map `index` (an index into [`Self::charmaps`]),
    /// like FreeType's `FT_Set_Charmap`.
    pub fn select_charmap(&mut self, index: usize) -> Result<()> {
        let count = self.cmap.infos().count();
        if index >= count {
            return Err(FontError::CharmapOutOfRange { index, count });
        }
        self.charmap = Some(index);
        Ok(())
    }

    /// Glyph for a character code in the active character map. Symbol
    /// fonts also map `U+0000..=U+00FF` to `U+F000..=U+F0FF` like
    /// HarfBuzz does.
    pub fn glyph_index_for_code(&self, code: u32) -> Option<GlyphId> {
        let (info, subtable) = self.cmap.get(self.charmap?)?;
        subtable.glyph(code).or_else(|| {
            (info.is_symbol() && code <= 0xFF)
                .then(|| subtable.glyph(0xF000 + code))
                .flatten()
        })
    }

    /// Glyph for `c`, or `None` when the font cannot display it.
    pub fn glyph_index(&self, c: char) -> Option<GlyphId> {
        self.glyph_index_for_code(u32::from(c))
    }

    /// Every usable character map in file order, with its language, for
    /// copying them (into a subset).
    pub(crate) fn cmap_subtables(
        &self,
    ) -> impl Iterator<Item = (CharmapInfo, u32, crate::tables::cmap::Subtable<'_>)> + '_ {
        self.cmap.subtables()
    }

    /// Every `(character code, glyph)` pair of the active character map.
    pub fn mappings(&self) -> Vec<(u32, GlyphId)> {
        let mut out = Vec::new();
        if let Some((_, subtable)) = self.charmap.and_then(|index| self.cmap.get(index)) {
            subtable.for_each(&mut |code, glyph| out.push((code, glyph)));
        }
        out
    }
}
