//! Glyph names as fontTools gives them (`TTFont.getGlyphOrder`), by which
//! matplotlib's PostScript Type 42 fonts name their glyphs: a `CFF `
//! table's charset names, else the `post` table's, else names made from
//! the Unicode character maps.

use std::borrow::Cow;
use std::collections::HashMap;

use super::{Face, Outlines};
use crate::agl::name_of_code;
use crate::tables::cmap::CharmapInfo;
use crate::tables::post::PostTable;
use crate::types::GlyphId;

/// Whether fontTools reads a subtable as mapping Unicode code points
/// (`isUnicode`: Unicode platform, or Windows symbol, BMP or full
/// repertoire).
const fn is_unicode(info: CharmapInfo) -> bool {
    info.platform_id == 0 || (info.platform_id == 3 && matches!(info.encoding_id, 0 | 1 | 10))
}

impl Face<'_> {
    /// fontTools' names of `glyphs`: the charset names of a `CFF ` table
    /// (`cidNNNNN` in a CID-keyed font), else the `post` table's, else
    /// those `_getGlyphNamesFromCmap` makes.
    pub fn fonttools_glyph_names(&self, glyphs: &[GlyphId]) -> Vec<String> {
        let fallback = |glyph: GlyphId| format!("glyph{:05}", glyph.0);
        if let Outlines::Cff(cff) = &self.outlines
            && cff.fonttools_name(GlyphId(0)).is_some()
        {
            return glyphs
                .iter()
                .map(|&glyph| {
                    cff.fonttools_name(glyph)
                        .map_or_else(|| fallback(glyph), Cow::into_owned)
                })
                .collect();
        }
        if let Some(post) = self.post.as_ref().filter(|post| PostTable::has_names(post)) {
            return glyphs
                .iter()
                .map(|&glyph| {
                    post.glyph_name(glyph)
                        .map_or_else(|| fallback(glyph), str::to_owned)
                })
                .collect();
        }
        let names = self.cmap_glyph_names();
        glyphs
            .iter()
            .map(|&glyph| {
                names
                    .get(usize::from(glyph.0))
                    .map_or_else(|| fallback(glyph), |name| name.clone().into_owned())
            })
            .collect()
    }

    /// The names `_getGlyphNamesFromCmap` gives every glyph: `.notdef`, the
    /// name of the lowest code point of a Unicode subtable mapping to the
    /// glyph (`.altN` added to the name's later uses), or `glyphNNNNN`.
    fn cmap_glyph_names(&self) -> Vec<Cow<'static, str>> {
        let count = usize::from(self.num_glyphs());
        let mut lowest: Vec<Option<u32>> = vec![None; count];
        for (info, _, subtable) in self.cmap_subtables() {
            if !is_unicode(info) {
                continue;
            }
            subtable.for_each(&mut |code, glyph| {
                if let Some(slot) = lowest.get_mut(usize::from(glyph.0)) {
                    *slot = Some(slot.map_or(code, |lowest| lowest.min(code)));
                }
            });
        }
        let mut uses: HashMap<Cow<'static, str>, usize> = HashMap::new();
        lowest
            .into_iter()
            .enumerate()
            .map(|(glyph, code)| match code {
                _ if glyph == 0 => Cow::Borrowed(".notdef"),
                Some(code) => {
                    let name = name_of_code(code);
                    let used = uses.entry(name.clone()).or_default();
                    *used += 1;
                    if *used > 1 {
                        Cow::Owned(format!("{name}.alt{}", *used - 1))
                    } else {
                        name
                    }
                }
                None => Cow::Owned(format!("glyph{glyph:05}")),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(file: &str) -> Vec<u8> {
        std::fs::read(format!(
            "{}/tests/fixtures/{file}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    }

    /// `data` with its `post` table made version 3.0, which has no names.
    fn without_post_names(mut data: Vec<u8>) -> Vec<u8> {
        let tables = usize::from(u16::from_be_bytes([data[4], data[5]]));
        let record = (0..tables)
            .map(|i| 12 + 16 * i)
            .find(|&at| &data[at..at + 4] == b"post")
            .unwrap();
        let at = u32::from_be_bytes(data[record + 8..record + 12].try_into().unwrap()) as usize;
        data[at..at + 4].copy_from_slice(&0x0003_0000u32.to_be_bytes());
        data
    }

    #[test]
    fn names_as_fonttools_gives_them() {
        // A name-keyed CFF font: its charset's names.
        let data = fixture("synthetic-cff.otf");
        let face = Face::parse(&data, 0).unwrap();
        let a = face.glyph_index('A').unwrap();
        assert_eq!(
            face.fonttools_glyph_names(&[GlyphId(0), a]),
            [".notdef", "A"]
        );
        // A CFF2 font: its `post` names, or without them names from the
        // cmap (U+00B4 is "acute").
        for data in [
            fixture("synthetic-var.otf"),
            without_post_names(fixture("synthetic-var.otf")),
        ] {
            let face = Face::parse(&data, 0).unwrap();
            let glyphs: Vec<GlyphId> = (0..7).map(GlyphId).collect();
            assert_eq!(
                face.fonttools_glyph_names(&glyphs),
                [".notdef", "space", "A", "O", "H", "acute", "Aacute"]
            );
        }
        let face_data = without_post_names(fixture("synthetic-var.otf"));
        let face = Face::parse(&face_data, 0).unwrap();
        assert!(!face.has_glyph_names());
        assert_eq!(face.fonttools_glyph_names(&[GlyphId(9)]), ["glyph00009"]);
    }
}
