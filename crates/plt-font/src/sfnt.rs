//! What a face holds as FreeType describes it (matplotlib's `FT2Font`
//! `face_flags`, `max_advance_height`, `get_sfnt` and `get_sfnt_table`):
//! the face flags, the name records, and the `vhea` and `PCLT` tables.

use crate::face::Face;
use crate::tables::head::Hhea;
use crate::tables::name::NameEntry;
use crate::tables::pclt::Pclt;
use crate::types::Tag;

/// The flags of FreeType's `face_flags` (`FT_FACE_FLAG_*`) that describe
/// what an sfnt face holds.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent flags, as FreeType's face_flags bits are"
)]
pub struct FaceFlags {
    /// Outlines (TrueType or CFF), which scale to any size.
    pub scalable: bool,
    /// Every glyph has the same advance (`post.isFixedPitch`).
    pub fixed_width: bool,
    /// An sfnt font (TrueType or OpenType); always set.
    pub sfnt: bool,
    /// Horizontal metrics (`hhea` and `hmtx`); always set.
    pub horizontal: bool,
    /// Vertical metrics (`vhea` and `vmtx`).
    pub vertical: bool,
    /// A legacy `kern` table (FreeType's default build ignores `GPOS`
    /// kerning here).
    pub kerning: bool,
    /// Glyph names (`post` formats 1, 2 and 2.5, or `CFF`).
    pub glyph_names: bool,
}

impl<'a> Face<'a> {
    /// The face's flags as FreeType sets them.
    ///
    /// ```
    /// use plt_font::{Face, bundled_font_data};
    /// let data = bundled_font_data("DejaVuSans.ttf").expect("bundled");
    /// let flags = Face::parse(data, 0)?.face_flags();
    /// assert!(flags.scalable && flags.kerning && flags.glyph_names);
    /// assert!(!flags.vertical && !flags.fixed_width);
    /// # Ok::<(), plt_font::FontError>(())
    /// ```
    pub fn face_flags(&self) -> FaceFlags {
        FaceFlags {
            scalable: self.has_outlines(),
            fixed_width: self.post().is_some_and(|post| post.is_fixed_pitch),
            sfnt: true,
            horizontal: true,
            vertical: self.vhea().is_some(),
            kerning: self.has_legacy_kerning(),
            glyph_names: self.has_glyph_names(),
        }
    }

    /// The `vhea` table when the face has vertical metrics (`vhea` and
    /// `vmtx`, FreeType's `vertical_info`), in the layout of `hhea`: its
    /// ascender, descender and line gap are the vertical typographic ones,
    /// its largest advance the largest advance height.
    pub fn vhea(&self) -> Option<Hhea> {
        self.table(Tag::new(b"vmtx"))?;
        Hhea::parse(self.table(Tag::new(b"vhea"))?).ok()
    }

    /// The `PCLT` table, if the face has a readable one (FreeType hands
    /// out none of version 0).
    pub fn pclt(&self) -> Option<Pclt> {
        Pclt::parse(self.table(Tag::new(b"PCLT"))?)
            .ok()
            .filter(|pclt| pclt.version != 0)
    }

    /// The largest advance height in font units: from `vhea`, or the line
    /// height without vertical metrics (FreeType's `max_advance_height`).
    pub fn max_advance_height(&self) -> i16 {
        self.vhea()
            .map_or_else(|| self.height(), |vhea| vhea.advance_width_max as i16)
    }

    /// Every non-empty name record, in file order (matplotlib's
    /// `FT2Font.get_sfnt`).
    pub fn name_entries(&self) -> Vec<NameEntry<'a>> {
        self.name_table().entries().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundled_font_data;
    use crate::tables::name::NameKey;

    fn face(file: &str) -> Face<'static> {
        Face::parse(bundled_font_data(file).expect("bundled"), 0).expect("valid font")
    }

    #[test]
    fn flags_and_heights_match_freetype() {
        // matplotlib's test_ft2font_cm_attrs.
        let cmtt = face("cmtt10.ttf");
        let flags = cmtt.face_flags();
        assert!(flags.scalable && flags.sfnt && flags.horizontal && flags.glyph_names);
        assert!(!flags.kerning);
        assert_eq!(cmtt.max_advance_height(), 1765);
        assert!(cmtt.vhea().is_none());
    }

    #[test]
    fn pclt_tables_of_bundled_fonts() {
        // matplotlib's test_ft2font_get_sfnt_table.
        let pclt = face("cmtt10.ttf").pclt().expect("cmtt10 has a PCLT table");
        assert_eq!(
            (pclt.pitch, pclt.x_height, pclt.cap_height),
            (1075, 905, 1276)
        );
        assert!(face("DejaVuSans.ttf").pclt().is_none());
    }

    #[test]
    fn name_entries_keep_the_raw_strings() {
        let entries = face("DejaVuSans.ttf").name_entries();
        let family = entries
            .iter()
            .find(|entry| entry.key == NameKey::MAC_ENGLISH && entry.name_id == 1)
            .expect("a Macintosh family name");
        assert_eq!(family.bytes, b"DejaVu Sans");
        assert!(entries.iter().all(|entry| !entry.bytes.is_empty()));
    }
}
