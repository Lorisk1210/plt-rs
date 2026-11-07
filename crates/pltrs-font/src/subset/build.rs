//! Writing the subset font program: the kept glyphs renumbered in their
//! original order, with the tables a TrueType or CFF rasterizer needs.

use std::collections::{BTreeMap, BTreeSet};

use super::Outlines;
use super::cmap::{cmap, search_range};
use super::glyf::{Glyf, renumbered};
use crate::error::{FontError, Result};
use crate::face::Face;
use crate::types::{GlyphId, Tag};

/// Tables copied unchanged from the original font: naming, metrics and
/// the hinting programs the glyph instructions rely on.
const COPIED: [&[u8; 4]; 6] = [b"OS/2", b"cvt ", b"fpgm", b"gasp", b"name", b"prep"];

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

/// The sfnt version of a font with TrueType outlines.
const TRUETYPE: u32 = 0x0001_0000;

/// The sfnt version of a font with CFF outlines (`OTTO`).
const OPENTYPE_CFF: u32 = 0x4F54_544F;

/// The tables of a font file, by tag.
type Tables = BTreeMap<[u8; 4], Vec<u8>>;

/// The sfnt version and the tables of the subset keeping `kept`, its
/// `cmap` mapping the characters of `requested`.
fn tables(
    face: &Face<'_>,
    outlines: &Outlines<'_>,
    kept: &[GlyphId],
    requested: &BTreeSet<GlyphId>,
) -> Result<(u32, Tables)> {
    let count = u16::try_from(kept.len()).map_err(|_| FontError::Malformed {
        table: Tag::new(b"maxp"),
        reason: "too many glyphs",
    })?;
    let renumber: BTreeMap<GlyphId, GlyphId> =
        kept.iter().copied().zip((0..).map(GlyphId)).collect();
    let mut tables = BTreeMap::new();
    let mut head = required(face, *b"head")?.to_vec();
    let version = match outlines {
        Outlines::TrueType(glyf) => {
            let (glyphs, loca) = glyph_tables(glyf, kept, &renumber)?;
            tables.insert(*b"glyf", glyphs);
            tables.insert(*b"loca", loca);
            head = with_u16(&head, 50, 1, *b"head")?;
            TRUETYPE
        }
        Outlines::Cff(cff) => {
            tables.insert(cff.version().tag().0, cff.table(kept)?);
            OPENTYPE_CFF
        }
    };
    let mut hmtx = Vec::with_capacity(kept.len() * 4);
    for &glyph in kept {
        hmtx.extend(face.glyph_advance(glyph).to_be_bytes());
        hmtx.extend(face.left_side_bearing(glyph).to_be_bytes());
    }
    head.get_mut(8..12)
        .ok_or(FontError::Truncated(Tag::new(b"head")))?
        .fill(0);
    let mut post = required(face, *b"post")?
        .get(..32)
        .map(<[u8]>::to_vec)
        .ok_or(FontError::Truncated(Tag::new(b"post")))?;
    post[..4].copy_from_slice(&0x0003_0000u32.to_be_bytes());
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
    tables.insert(*b"cmap", cmap(face, requested, &renumber));
    for tag in COPIED {
        if let Some(data) = face.table(Tag::new(tag)) {
            tables.insert(*tag, data.to_vec());
        }
    }
    Ok((version, tables))
}

/// The subset font file of `kept` (sorted original glyphs), its `cmap`
/// mapping the characters of the `requested` glyphs.
pub(super) fn font_file(
    face: &Face<'_>,
    outlines: &Outlines<'_>,
    kept: &[GlyphId],
    requested: &BTreeSet<GlyphId>,
) -> Result<Vec<u8>> {
    let (version, tables) = tables(face, outlines, kept, requested)?;
    let mut out = Vec::new();
    out.extend(version.to_be_bytes());
    out.extend((tables.len() as u16).to_be_bytes());
    for value in search_range(tables.len(), 16) {
        out.extend((value as u16).to_be_bytes());
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
