//! Writing the subset font program: the kept glyphs renumbered in their
//! original order, with the tables a TrueType rasterizer needs.

use std::collections::BTreeMap;

use super::glyf::{Glyf, renumbered};
use crate::error::{FontError, Result};
use crate::face::Face;
use crate::types::{GlyphId, Tag};

/// Tables copied unchanged from the original font: naming, metrics and
/// the hinting programs the glyph instructions rely on.
const COPIED: [&[u8; 4]; 6] = [b"OS/2", b"cvt ", b"fpgm", b"gasp", b"name", b"prep"];

/// The table directory's search fields for `count` tables.
fn search_fields(count: u16) -> [u16; 3] {
    let mut power = 1u16;
    let mut log = 0u16;
    while power * 2 <= count {
        power *= 2;
        log += 1;
    }
    [power * 16, log, count * 16 - power * 16]
}

/// The OpenType table checksum.
fn checksum(data: &[u8]) -> u32 {
    data.chunks(4).fold(0u32, |sum, chunk| {
        let mut word = [0u8; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        sum.wrapping_add(u32::from_be_bytes(word))
    })
}

/// A table of the original face, required.
fn required<'a>(face: &Face<'a>, tag: [u8; 4]) -> Result<&'a [u8]> {
    let tag = Tag::new(&tag);
    face.table(tag).ok_or(FontError::MissingTable(tag))
}

/// `data` with the big-endian `u16` at `at` replaced.
fn with_u16(data: &[u8], at: usize, value: u16, tag: [u8; 4]) -> Result<Vec<u8>> {
    let mut out = data.to_vec();
    out.get_mut(at..at + 2)
        .ok_or(FontError::Truncated(Tag::new(&tag)))?
        .copy_from_slice(&value.to_be_bytes());
    Ok(out)
}

/// `glyf` and long-format `loca` of the kept glyphs; the outline of glyph
/// 0 is dropped as fontTools does by default.
fn glyph_tables(
    glyf: &Glyf<'_>,
    kept: &[GlyphId],
    renumber: &BTreeMap<GlyphId, GlyphId>,
) -> Result<(Vec<u8>, Vec<u8>)> {
    let mut glyphs = Vec::new();
    let mut loca = Vec::with_capacity((kept.len() + 1) * 4);
    for &glyph in kept {
        loca.extend((glyphs.len() as u32).to_be_bytes());
        if glyph.0 == 0 {
            continue;
        }
        let record = renumbered(glyf.record(glyph)?, |g| {
            renumber.get(&g).copied().unwrap_or_default()
        })?;
        glyphs.extend(record);
        glyphs.resize(glyphs.len().next_multiple_of(4), 0);
    }
    loca.extend((glyphs.len() as u32).to_be_bytes());
    Ok((glyphs, loca))
}

/// The `cmap` of the subset: a Windows Unicode BMP (format 4) subtable
/// mapping the original character codes of the kept glyphs.
fn cmap(face: &Face<'_>, renumber: &BTreeMap<GlyphId, GlyphId>) -> Vec<u8> {
    let mut mappings: Vec<(u16, u16)> = face
        .mappings()
        .into_iter()
        .filter_map(|(code, glyph)| {
            let code = u16::try_from(code).ok().filter(|&c| c != 0xFFFF)?;
            Some((code, renumber.get(&glyph)?.0))
        })
        .collect();
    mappings.sort_unstable();
    mappings.dedup_by_key(|m| m.0);
    // One segment per code, then the closing 0xFFFF segment.
    let segments: Vec<(u16, u16)> = mappings.into_iter().chain([(0xFFFF, 0)]).collect();
    let count = segments.len() as u16;
    let [range, selector, shift] = search_fields(count);
    let mut table = Vec::new();
    for value in [
        4,
        16 + 8 * count,
        0,
        count * 2,
        range / 8,
        selector,
        shift / 8,
    ] {
        table.extend(value.to_be_bytes());
    }
    table.extend(segments.iter().flat_map(|&(code, _)| code.to_be_bytes()));
    table.extend(0u16.to_be_bytes());
    table.extend(segments.iter().flat_map(|&(code, _)| code.to_be_bytes()));
    table.extend(
        segments
            .iter()
            .flat_map(|&(code, glyph)| glyph.wrapping_sub(code).to_be_bytes()),
    );
    table.extend(segments.iter().flat_map(|_| 0u16.to_be_bytes()));
    let mut out = Vec::new();
    for value in [0u16, 1, 3, 1] {
        out.extend(value.to_be_bytes());
    }
    out.extend(12u32.to_be_bytes());
    out.extend(table);
    out
}

/// The tables of the subset, by tag.
fn tables(
    face: &Face<'_>,
    glyf: &Glyf<'_>,
    kept: &[GlyphId],
) -> Result<BTreeMap<[u8; 4], Vec<u8>>> {
    let count = u16::try_from(kept.len()).map_err(|_| FontError::Malformed {
        table: Tag::new(b"maxp"),
        reason: "too many glyphs",
    })?;
    let renumber: BTreeMap<GlyphId, GlyphId> =
        kept.iter().copied().zip((0..).map(GlyphId)).collect();
    let (glyphs, loca) = glyph_tables(glyf, kept, &renumber)?;
    let mut hmtx = Vec::with_capacity(kept.len() * 4);
    for &glyph in kept {
        hmtx.extend(face.glyph_advance(glyph).to_be_bytes());
        hmtx.extend(face.left_side_bearing(glyph).to_be_bytes());
    }
    let mut head = with_u16(required(face, *b"head")?, 50, 1, *b"head")?;
    head.get_mut(8..12)
        .ok_or(FontError::Truncated(Tag::new(b"head")))?
        .fill(0);
    let mut post = required(face, *b"post")?
        .get(..32)
        .map(<[u8]>::to_vec)
        .ok_or(FontError::Truncated(Tag::new(b"post")))?;
    post[..4].copy_from_slice(&0x0003_0000u32.to_be_bytes());
    let mut tables = BTreeMap::new();
    tables.insert(*b"glyf", glyphs);
    tables.insert(*b"loca", loca);
    tables.insert(*b"hmtx", hmtx);
    tables.insert(*b"head", head);
    tables.insert(
        *b"hhea",
        with_u16(required(face, *b"hhea")?, 34, count, *b"hhea")?,
    );
    tables.insert(
        *b"maxp",
        with_u16(required(face, *b"maxp")?, 4, count, *b"maxp")?,
    );
    tables.insert(*b"post", post);
    tables.insert(*b"cmap", cmap(face, &renumber));
    for tag in COPIED {
        if let Some(data) = face.table(Tag::new(tag)) {
            tables.insert(*tag, data.to_vec());
        }
    }
    Ok(tables)
}

/// The subset font file of `kept` (sorted original glyphs).
pub(super) fn font_file(face: &Face<'_>, glyf: &Glyf<'_>, kept: &[GlyphId]) -> Result<Vec<u8>> {
    let tables = tables(face, glyf, kept)?;
    let count = tables.len() as u16;
    let mut out = Vec::new();
    out.extend(0x0001_0000u32.to_be_bytes());
    out.extend(count.to_be_bytes());
    for value in search_fields(count) {
        out.extend(value.to_be_bytes());
    }
    let mut offset = 12 + 16 * tables.len();
    let mut head_at = None;
    for (tag, data) in &tables {
        if tag == b"head" {
            head_at = Some(offset);
        }
        out.extend(tag);
        out.extend(checksum(data).to_be_bytes());
        out.extend((offset as u32).to_be_bytes());
        out.extend((data.len() as u32).to_be_bytes());
        offset += data.len().next_multiple_of(4);
    }
    for data in tables.values() {
        out.extend(data);
        out.resize(out.len().next_multiple_of(4), 0);
    }
    let head_at = head_at.expect("the subset always has a head table");
    let adjustment = 0xB1B0_AFBAu32.wrapping_sub(checksum(&out));
    out[head_at + 8..head_at + 12].copy_from_slice(&adjustment.to_be_bytes());
    Ok(out)
}
