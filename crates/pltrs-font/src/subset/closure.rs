//! The glyphs a subset keeps (fontTools' `Subsetter._closure_glyphs` with
//! matplotlib's options): the requested glyphs, `.notdef` (and the next
//! three glyphs of a TrueType font), the `MATH` variants and assembly
//! parts of those, every glyph `GSUB` can substitute, and the components
//! of composite glyphs or implied `seac`s.

use std::collections::BTreeSet;

use super::Outlines;
use super::glyf::components;
use super::gsub::close_over_gsub;
use crate::error::Result;
use crate::face::Face;
use crate::layout::common::Coverage;
use crate::reader::{Array, Reader, tail};
use crate::types::{GlyphId, Tag};

const MATH: Tag = Tag::new(b"MATH");

/// How many leading glyphs of a TrueType font are always kept
/// (`.notdef` plus fontTools' "recommended glyphs"); of a CFF font,
/// fontTools keeps `.notdef` alone.
const RECOMMENDED_GLYPHS: u16 = 4;

/// The variant glyphs and assembly parts of one `MathGlyphConstruction`.
fn construction_glyphs(data: &[u8]) -> Result<Vec<GlyphId>> {
    let mut r = Reader::new(data, MATH);
    let assembly = r.u16()?;
    let count = r.u16()?;
    let mut glyphs: Vec<GlyphId> = r
        .array::<u32>(usize::from(count))?
        .iter()
        .map(|record| GlyphId((record >> 16) as u16))
        .collect();
    if assembly != 0 {
        let mut r = Reader::at(data, usize::from(assembly), MATH)?;
        r.skip(4)?;
        let parts = r.u16()?;
        for _ in 0..parts {
            glyphs.push(r.glyph()?);
            r.skip(8)?;
        }
    }
    Ok(glyphs)
}

/// Adds the vertical and horizontal variants (and assembly parts) of the
/// glyphs in `glyphs`, one pass (`MathVariants.closure_glyphs`).
fn close_over_math(math: &[u8], glyphs: &mut BTreeSet<GlyphId>) -> Result<()> {
    let mut r = Reader::new(math, MATH);
    r.skip(8)?;
    let variants_offset = r.u16()?;
    if variants_offset == 0 {
        return Ok(());
    }
    let variants = tail(math, usize::from(variants_offset), MATH)?;
    let mut r = Reader::new(variants, MATH);
    r.skip(2)?;
    let vertical = r.u16()?;
    let horizontal = r.u16()?;
    let vertical_count = r.u16()?;
    let horizontal_count = r.u16()?;
    let vertical_offsets: Array<'_, u16> = r.array(usize::from(vertical_count))?;
    let horizontal_offsets: Array<'_, u16> = r.array(usize::from(horizontal_count))?;
    let mut added = Vec::new();
    for (coverage, offsets) in [
        (vertical, vertical_offsets),
        (horizontal, horizontal_offsets),
    ] {
        if coverage == 0 {
            continue;
        }
        for (index, glyph) in Coverage::at(variants, coverage, MATH)?.entries() {
            let construction = offsets.get(usize::from(index)).filter(|&o| o != 0);
            if let (true, Some(offset)) = (glyphs.contains(&glyph), construction) {
                added.extend(construction_glyphs(tail(
                    variants,
                    usize::from(offset),
                    MATH,
                )?)?);
            }
        }
    }
    glyphs.extend(added);
    Ok(())
}

/// Adds the glyphs `parts` gives each glyph, recursively: the components
/// of composite glyphs, or the base and accent of implied `seac`s.
fn close_over_parts(
    glyphs: &mut BTreeSet<GlyphId>,
    parts: impl Fn(GlyphId) -> Result<Vec<GlyphId>>,
) -> Result<()> {
    let mut pending: Vec<GlyphId> = glyphs.iter().copied().collect();
    while !pending.is_empty() {
        let mut found = Vec::new();
        for glyph in pending {
            found.extend(parts(glyph)?);
        }
        pending = found.into_iter().filter(|&g| glyphs.insert(g)).collect();
    }
    Ok(())
}

/// The glyphs a subset of `face` holding `requested` keeps, in glyph
/// order; glyphs the face does not have are left out.
pub(super) fn closure(
    face: &Face<'_>,
    outlines: &Outlines<'_>,
    requested: impl IntoIterator<Item = GlyphId>,
) -> Result<BTreeSet<GlyphId>> {
    let count = face.num_glyphs();
    let mut glyphs: BTreeSet<GlyphId> = requested.into_iter().filter(|g| g.0 < count).collect();
    let leading = match outlines {
        Outlines::TrueType(_) => RECOMMENDED_GLYPHS,
        Outlines::Cff(_) => 1,
    };
    glyphs.extend((0..leading.min(count)).map(GlyphId));
    if let Some(math) = face.table(MATH) {
        close_over_math(math, &mut glyphs)?;
    }
    if let Some(gsub) = face.table(Tag::new(b"GSUB")) {
        close_over_gsub(gsub, &mut glyphs)?;
    }
    glyphs.retain(|g| g.0 < count);
    match outlines {
        Outlines::TrueType(glyf) => {
            close_over_parts(&mut glyphs, |glyph| components(glyf.record(glyph)?))?;
        }
        Outlines::Cff(cff) => close_over_parts(&mut glyphs, |glyph| cff.components(glyph))?,
    }
    Ok(glyphs)
}
