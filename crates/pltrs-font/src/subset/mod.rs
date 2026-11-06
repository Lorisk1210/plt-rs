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
//! hinting tables, without layout tables. Its `cmap` maps the characters
//! of the requested glyphs as fontTools' does ([`cmap`]).

mod build;
mod closure;
mod cmap;
mod glyf;
mod gsub;

use std::collections::BTreeSet;

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
    let requested: BTreeSet<GlyphId> = glyphs.into_iter().collect();
    let kept: Vec<GlyphId> = closure::closure(face, &glyf, requested.iter().copied())?
        .into_iter()
        .collect();
    Ok(FontSubset {
        data: build::font_file(face, &glyf, &kept, &requested)?,
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

    /// A `GSUB` whose `ccmp` lookup 0 (chaining context, format 3) applies
    /// lookup 1 (`g → g + 1` for `g < glyphs`) and then itself at every
    /// covered glyph.
    fn self_applying_gsub(glyphs: u16) -> Vec<u8> {
        let mut t = Vec::new();
        let mut put = |values: &[u16]| t.extend(values.iter().flat_map(|v| v.to_be_bytes()));
        let range = [2, 1, 0, glyphs - 1, 0];
        put(&[1, 0, 10, 30, 44]);
        put(&[1, 0x4446, 0x4C54, 8, 4, 0, 0, 0xFFFF, 1, 0]);
        put(&[1, 0x6363, 0x6D70, 8, 0, 1, 0]);
        put(&[2, 6, 44]);
        put(&[6, 0, 1, 8, 3, 0, 1, 20, 0, 2, 0, 1, 0, 0]);
        put(&range);
        put(&[1, 0, 1, 8, 1, 6, 1]);
        put(&range);
        t
    }

    #[test]
    fn self_applying_lookups_close_without_deep_recursion() {
        // Every nested application adds one glyph and resets the memo: the
        // recursion would be 1000 lookups deep.
        let gsub = self_applying_gsub(1000);
        let mut glyphs = std::collections::BTreeSet::from([GlyphId(0)]);
        gsub::close_over_gsub(&gsub, &mut glyphs).expect("valid table");
        assert_eq!(glyphs.len(), 1001);
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
