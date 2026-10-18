//! The glyphs a subset keeps (fontTools' `Subsetter._closure_glyphs` with
//! matplotlib's options): the requested glyphs, the first four glyphs, the
//! `MATH` variants and assembly parts of those, every glyph `GSUB` can
//! substitute, and the components of composite glyphs.

use std::collections::BTreeSet;

use super::glyf::{Glyf, components};
use super::gsub::close_over_gsub;
use crate::error::Result;
use crate::face::Face;
use crate::layout::common::Coverage;
use crate::reader::{Array, Reader, tail};
use crate::types::{GlyphId, Tag};

const MATH: Tag = Tag::new(b"MATH");

/// How many leading glyphs are always kept (`.notdef` plus fontTools'
/// "recommended glyphs").
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

/// Adds the components of composite glyphs, recursively.
fn close_over_glyf(glyf: &Glyf<'_>, glyphs: &mut BTreeSet<GlyphId>) -> Result<()> {
    let mut pending: Vec<GlyphId> = glyphs.iter().copied().collect();
    while !pending.is_empty() {
        let mut found = Vec::new();
        for glyph in pending {
            found.extend(components(glyf.record(glyph)?)?);
        }
        pending = found.into_iter().filter(|&g| glyphs.insert(g)).collect();
    }
    Ok(())
}

/// The glyphs a subset of `face` holding `requested` keeps, in glyph
/// order; glyphs the face does not have are left out.
pub(super) fn closure(
    face: &Face<'_>,
    glyf: &Glyf<'_>,
    requested: impl IntoIterator<Item = GlyphId>,
) -> Result<BTreeSet<GlyphId>> {
    let count = face.num_glyphs();
    let mut glyphs: BTreeSet<GlyphId> = requested.into_iter().filter(|g| g.0 < count).collect();
    glyphs.extend((0..RECOMMENDED_GLYPHS.min(count)).map(GlyphId));
    if let Some(math) = face.table(MATH) {
        close_over_math(math, &mut glyphs)?;
    }
    if let Some(gsub) = face.table(Tag::new(b"GSUB")) {
        close_over_gsub(gsub, &mut glyphs)?;
    }
    glyphs.retain(|g| g.0 < count);
    close_over_glyf(glyf, &mut glyphs)?;
    Ok(glyphs)
}
