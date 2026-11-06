//! The subset's `cmap` against fontTools' (matplotlib's
//! `get_glyphs_subset`), synthetic fonts whose subsets overflow format 4,
//! and the Last Resort font.

use std::collections::BTreeMap;

use super::*;
use crate::db::{FontDatabase, FontQuery};
use crate::subset::subset;
use crate::types::Tag;

/// `bytes` as lower-case hexadecimal.
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

/// The `cmap` table of the font file `data`.
fn cmap_of(data: &[u8]) -> Vec<u8> {
    let face = Face::parse(data, 0).expect("a valid subset");
    face.table(Tag::new(b"cmap")).expect("a cmap").to_vec()
}

#[test]
fn subsets_map_characters_as_fonttools_does() {
    let db = FontDatabase::with_bundled_fonts();
    let id = db
        .find_font(&FontQuery::families(["DejaVu Sans"]))
        .expect("bundled font");
    let face = db.face(id).expect("parses");
    // matplotlib: font_as_file(get_glyphs_subset(DejaVuSans.ttf, {glyphs
    // of text})) read back with fontTools, `reader['cmap'].hex()`: the
    // (0, 3) and (3, 1) format 4 subtables shared, the (0, 10) format 12
    // one, the (3, 10) one repeating (3, 1) dropped.
    let expected = [
        (
            "ce",
            "00000003000000030000001c0000000a00000044000300010000001c00040028000000060004000100\
             0200630065ffff000000630065ffffffa1ffa00001000000000000000c0000000000280000000000\
             000002000000630000006300000004000000650000006500000005",
        ),
        (
            " ABCabc",
            "00000003000000030000001c0000000a0000004c000300010000001c00040030000000080008000200\
             00002000430063ffff0000002000410061ffffffe3ffc3ffa600010000000000000000000c000000\
             0000340000000000000003000000200000002000000003000000410000004300000004000000610000\
             006300000007",
        ),
        (
            "AB  \u{2192}",
            "00000003000000030000001c0000000a0000004c000300010000001c00040030000000080008000200\
             00002000422192ffff0000002000412192ffffffe3ffc3de7400010000000000000000000c000000\
             0000340000000000000003000000200000002000000003000000410000004200000004000021920000\
             219200000006",
        ),
    ];
    for (text, cmap) in expected {
        let glyphs = text
            .chars()
            .map(|c| face.glyph_index(c).expect("in DejaVu"));
        let kept = subset(&face, glyphs).expect("subsets");
        assert_eq!(hex(&cmap_of(&kept.data)), cmap, "{text:?}");
    }
}

#[test]
fn ranges_split_where_segments_save_bytes() {
    // Consecutive glyphs over a long enough run get a segment of their
    // own (fontTools' `splitRange`); a short run stays in a glyph array.
    let map: BTreeMap<u32, u16> = (10..30)
        .map(|code| {
            (
                code,
                if code < 20 {
                    code as u16 * 3
                } else {
                    code as u16
                },
            )
        })
        .collect();
    assert_eq!(split_range(10, 29, &map), (vec![20], vec![19, 29]));
    let short: BTreeMap<u32, u16> = [(1, 7), (2, 3), (3, 4), (4, 9)].into_iter().collect();
    assert_eq!(split_range(1, 4, &short), (Vec::new(), vec![4]));
    assert_eq!(search_range(39, 2), [64, 5, 14]);
    assert_eq!(search_range(1, 16), [16, 0, 0]);
}

/// A TrueType font of `glyphs` empty glyphs whose `cmap` holds the
/// subtables `subtables` (`(platform, encoding, data)`).
fn synthetic_font(glyphs: u16, subtables: &[(u16, u16, Vec<u8>)]) -> Vec<u8> {
    let be16 =
        |values: &[u16]| -> Vec<u8> { values.iter().flat_map(|v| v.to_be_bytes()).collect() };
    let mut head = be16(&[1, 0, 0, 0, 0, 0, 0x5F0F, 0x3CF5, 0x0008, 1000]);
    head.extend([0; 16]);
    head.extend(be16(&[0, 0, 1000, 1000, 0, 8, 2, 1, 0]));
    let mut hhea = be16(&[1, 0, 800, 0xFF38, 0, 500]);
    hhea.extend([0; 22]);
    hhea.extend(be16(&[glyphs]));
    let maxp = be16(&[1, 0, glyphs, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0]);
    let hmtx: Vec<u8> = (0..glyphs).flat_map(|_| be16(&[500, 0])).collect();
    let loca = vec![0; 4 * (usize::from(glyphs) + 1)];
    let mut post = be16(&[3, 0]);
    post.extend([0; 28]);
    let mut cmap = be16(&[0, subtables.len() as u16]);
    let mut offset = 4 + 8 * subtables.len();
    for (platform, encoding, data) in subtables {
        cmap.extend(be16(&[*platform, *encoding]));
        cmap.extend((offset as u32).to_be_bytes());
        offset += data.len();
    }
    for (.., data) in subtables {
        cmap.extend(data);
    }
    let entries: [(&[u8; 4], &[u8]); 8] = [
        (b"cmap", &cmap),
        (b"glyf", &[]),
        (b"head", &head),
        (b"hhea", &hhea),
        (b"hmtx", &hmtx),
        (b"loca", &loca),
        (b"maxp", &maxp),
        (b"post", &post),
    ];
    let mut font = be16(&[1, 0, entries.len() as u16, 128, 3, 0]);
    let mut offset = 12 + 16 * entries.len();
    for (tag, table) in &entries {
        font.extend_from_slice(*tag);
        font.extend([0; 4]);
        font.extend((offset as u32).to_be_bytes());
        font.extend((table.len() as u32).to_be_bytes());
        offset += table.len().next_multiple_of(4);
    }
    for (_, table) in &entries {
        font.extend_from_slice(table);
        font.resize(font.len().next_multiple_of(4), 0);
    }
    font
}

/// A format 4 subtable of one segment mapping `first..=last` to the
/// consecutive glyphs from 1.
fn consecutive_format4(first: u16, last: u16) -> Vec<u8> {
    let delta = 1u16.wrapping_sub(first);
    [
        4, 32, 0, 4, 4, 1, 0, last, 0xFFFF, 0, first, 0xFFFF, delta, 1, 0, 0,
    ]
    .iter()
    .flat_map(|v: &u16| v.to_be_bytes())
    .collect()
}

#[test]
fn subsets_beyond_format_4_get_a_format_12_subtable() {
    // 30,000 characters mapped to consecutive glyphs fit one segment; every
    // other glyph of them needs 15,000, past format 4's 16-bit fields.
    let (first, last) = (0x1000u16, 0x1000 + 29_999);
    let font = synthetic_font(30_001, &[(0, 3, consecutive_format4(first, last))]);
    let face = Face::parse(&font, 0).expect("a valid font");
    assert_eq!(
        face.glyph_index_for_code(u32::from(first) + 4),
        Some(GlyphId(5))
    );
    let every_other: Vec<GlyphId> = (1..=30_000).step_by(2).map(GlyphId).collect();
    let kept = subset(&face, every_other.iter().copied()).expect("subsets");
    let parsed = Face::parse(&kept.data, 0).expect("a valid subset");
    let charmaps = parsed.charmaps();
    assert_eq!(charmaps.len(), 1, "only the format 12 subtable");
    assert_eq!(
        (
            charmaps[0].platform_id,
            charmaps[0].encoding_id,
            charmaps[0].format
        ),
        (0, 4, 12)
    );
    for (i, glyph) in every_other.iter().enumerate() {
        let code = u32::from(first) + u32::from(glyph.0) - 1;
        assert_eq!(
            parsed.glyph_index_for_code(code),
            kept.new_id(*glyph),
            "{i}"
        );
        assert_eq!(parsed.glyph_index_for_code(code + 1), None);
    }
    // A few glyphs keep their format 4 subtable.
    let few = subset(&face, [GlyphId(1), GlyphId(3), GlyphId(4)]).expect("subsets");
    let parsed = Face::parse(&few.data, 0).expect("a valid subset");
    assert_eq!(parsed.charmaps().len(), 1);
    assert_eq!(parsed.charmaps()[0].format, 4);
    assert_eq!(
        parsed.glyph_index_for_code(u32::from(first) + 3),
        few.new_id(GlyphId(4))
    );
}

#[test]
fn the_last_resort_font_subsets_to_its_format_13_subtable() {
    let db = FontDatabase::with_bundled_fonts();
    let id = db
        .find_font(&FontQuery::families(["Last Resort High-Efficiency"]))
        .expect("bundled font");
    let mut face = db.face(id).expect("parses");
    face.select_charmap(0).expect("its format 13 subtable");
    let latin = face.glyph_index('A').expect("a Latin block glyph");
    let arrows = face.glyph_index('\u{2192}').expect("an arrows block glyph");
    let kept = subset(&face, [latin, arrows]).expect("subsets");
    let mut parsed = Face::parse(&kept.data, 0).expect("a valid subset");
    let charmaps = parsed.charmaps();
    assert_eq!(charmaps.len(), 1, "the empty (3, 1) subtable is dropped");
    assert_eq!((charmaps[0].platform_id, charmaps[0].format), (0, 13));
    parsed.select_charmap(0).expect("one charmap");
    assert_eq!(parsed.glyph_index('B'), kept.new_id(latin));
    assert_eq!(parsed.glyph_index('\u{2190}'), kept.new_id(arrows));
}
