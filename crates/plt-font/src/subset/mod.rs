//! TrueType subsetting for embedding fonts in documents, keeping the glyphs
//! matplotlib's Type 42 embedding keeps (fontTools' `Subsetter` with
//! `glyph_names` and `recommended_glyphs`).
//!
//! The kept glyphs are the requested ones closed over the first four
//! glyphs, the `MATH` variants, the `GSUB` substitutions of the features
//! fontTools keeps and composite components ([`closure`]); they are
//! renumbered in their original order, so new glyph ids match fontTools'.
//! The font program itself ([`build`]) holds `glyf`, `loca`, `hmtx`,
//! `cmap` and the header tables plus the original naming, `OS/2` and
//! hinting tables, without layout tables.

mod build;
mod closure;
mod glyf;
mod gsub;

use crate::error::Result;
use crate::face::Face;
use crate::types::GlyphId;

/// A font reduced to some of its glyphs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontSubset {
    /// The subset as a TrueType font file.
    pub data: Vec<u8>,
    /// The original glyph behind every new glyph id, ascending.
    pub glyphs: Vec<GlyphId>,
}

impl FontSubset {
    /// The new id of original glyph `glyph`, if the subset keeps it.
    pub fn new_id(&self, glyph: GlyphId) -> Option<GlyphId> {
        let index = self.glyphs.binary_search(&glyph).ok()?;
        u16::try_from(index).ok().map(GlyphId)
    }
}

/// The subset of the TrueType `face` keeping `glyphs` and what they need.
///
/// # Errors
///
/// When the face has no TrueType outlines (`glyf`) or a table it needs is
/// missing or malformed.
pub fn subset(face: &Face<'_>, glyphs: impl IntoIterator<Item = GlyphId>) -> Result<FontSubset> {
    let glyf = glyf::Glyf::new(face)?;
    let kept: Vec<GlyphId> = closure::closure(face, &glyf, glyphs)?.into_iter().collect();
    Ok(FontSubset {
        data: build::font_file(face, &glyf, &kept)?,
        glyphs: kept,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::FontDatabase;
    use crate::outline::OutlineRecorder;
    use crate::types::Tag;

    fn dejavu() -> (FontDatabase, crate::db::FontId) {
        let db = FontDatabase::with_bundled_fonts();
        let id = db
            .find_font(&crate::db::FontQuery::families(["DejaVu Sans"]))
            .expect("bundled font");
        (db, id)
    }

    #[test]
    fn keeps_recommended_glyphs_and_renumbers() {
        let (db, id) = dejavu();
        let face = db.face(id).unwrap();
        let (c, e) = (
            face.glyph_index('c').unwrap(),
            face.glyph_index('e').unwrap(),
        );
        let kept = subset(&face, [e, c]).unwrap();
        // matplotlib's subset of "ce": .notdef, .null, nonmarkingreturn,
        // space, c, e.
        assert_eq!(kept.glyphs.len(), 6);
        assert_eq!(kept.new_id(c), Some(GlyphId(4)));
        assert_eq!(kept.new_id(e), Some(GlyphId(5)));
        let parsed = Face::parse(&kept.data, 0).unwrap();
        assert_eq!(parsed.num_glyphs(), 6);
        assert_eq!(parsed.glyph_advance(GlyphId(4)), face.glyph_advance(c));
        let (mut original, mut copy) = (OutlineRecorder::default(), OutlineRecorder::default());
        face.outline(c, &mut original).unwrap();
        parsed.outline(GlyphId(4), &mut copy).unwrap();
        assert_eq!(original, copy);
        assert_eq!(parsed.glyph_index('e'), Some(GlyphId(5)));
        assert!(parsed.table(Tag::new(b"GSUB")).is_none());
    }

    #[test]
    fn closes_over_ligatures_and_components() {
        let (db, id) = dejavu();
        let face = db.face(id).unwrap();
        let f = face.glyph_index('f').unwrap();
        let i = face.glyph_index('i').unwrap();
        let fi = face.glyph_index('\u{fb01}').unwrap();
        let letters = subset(&face, [f, i]).unwrap();
        assert!(letters.new_id(fi).is_some(), "the fi ligature comes along");
        let e_acute = face.glyph_index('\u{e9}').unwrap();
        let with_components = subset(&face, [e_acute]).unwrap();
        assert!(with_components.glyphs.len() > 5, "é is a composite glyph");
    }
}
