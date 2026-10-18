//! `GSUB` subtables: single, multiple, alternate, ligature, contextual and
//! reverse chaining substitution.

use crate::error::Result;
use crate::layout::apply::Applier;
use crate::layout::buffer::{GlyphInfo, props};
use crate::layout::common::Coverage;
use crate::layout::context;
use crate::reader::{Array, Reader, tail};
use crate::shape::category::GeneralCategory;
use crate::types::{GlyphId, Tag};

const GSUB: Tag = Tag::new(b"GSUB");

/// Lookup type of reverse chaining single substitutions, which are applied
/// back to front.
pub(crate) const REVERSE_CHAIN: u16 = 8;

/// Applies one `GSUB` subtable of lookup type `kind` at the cursor.
/// Malformed subtables simply do not apply.
pub(crate) fn apply(ctx: &mut Applier<'_, '_>, kind: u16, data: &[u8]) -> bool {
    let result = match kind {
        1 => single(ctx, data),
        2 => multiple(ctx, data),
        3 => alternate(ctx, data),
        4 => ligature(ctx, data),
        5 => context::apply_context(ctx, data, GSUB),
        6 => context::apply_chain_context(ctx, data, GSUB),
        REVERSE_CHAIN => reverse_chain(ctx, data),
        _ => Ok(false),
    };
    result.unwrap_or(false)
}

/// Coverage index of the current glyph for a subtable whose coverage
/// offset is at byte 2.
fn covered(ctx: &Applier<'_, '_>, data: &[u8]) -> Result<Option<u16>> {
    let mut r = Reader::new(data, GSUB);
    r.skip(2)?;
    let coverage = Coverage::at(data, r.u16()?, GSUB)?;
    Ok(ctx.cur().and_then(|info| coverage.index(info.glyph)))
}

/// Replaces the current glyph and moves past it.
fn replace(ctx: &mut Applier<'_, '_>, glyph: GlyphId) {
    let gdef = *ctx.face.gdef();
    let idx = ctx.buffer.idx;
    ctx.buffer.info[idx].set_glyph(glyph, &gdef, None);
    ctx.buffer.idx += 1;
}

/// Reads an offset array entry `index` (after a count) at `at` and returns
/// the referenced sub-table.
fn indexed(data: &[u8], at: usize, index: u16) -> Result<Option<&[u8]>> {
    let mut r = Reader::at(data, at, GSUB)?;
    let count = r.u16()?;
    let offsets: Array<'_, u16> = r.array(usize::from(count))?;
    match offsets.get(usize::from(index)) {
        Some(offset) => Ok(Some(tail(data, usize::from(offset), GSUB)?)),
        None => Ok(None),
    }
}

fn single(ctx: &mut Applier<'_, '_>, data: &[u8]) -> Result<bool> {
    let Some(index) = covered(ctx, data)? else {
        return Ok(false);
    };
    let Some(current) = ctx.cur().map(|info| info.glyph) else {
        return Ok(false);
    };
    let mut r = Reader::new(data, GSUB);
    let glyph = match r.u16()? {
        1 => {
            r.skip(2)?;
            GlyphId(current.0.wrapping_add(r.i16()? as u16))
        }
        2 => {
            r.skip(2)?;
            let count = r.u16()?;
            let substitutes: Array<'_, GlyphId> = r.array(usize::from(count))?;
            match substitutes.get(usize::from(index)) {
                Some(glyph) => glyph,
                None => return Ok(false),
            }
        }
        _ => return Ok(false),
    };
    replace(ctx, glyph);
    Ok(true)
}

/// Reads a glyph array (count + ids) at the start of `data`.
fn glyph_array(data: &[u8]) -> Result<Array<'_, GlyphId>> {
    let mut r = Reader::new(data, GSUB);
    let count = r.u16()?;
    r.array(usize::from(count))
}

fn multiple(ctx: &mut Applier<'_, '_>, data: &[u8]) -> Result<bool> {
    let Some(index) = covered(ctx, data)? else {
        return Ok(false);
    };
    let Some(sequence) = indexed(data, 4, index)? else {
        return Ok(false);
    };
    let glyphs = glyph_array(sequence)?;
    if glyphs.len() == 1 {
        replace(ctx, glyphs.get(0).unwrap_or_default());
        return Ok(true);
    }
    if !ctx.buffer.can_grow(glyphs.len().saturating_sub(1)) {
        return Ok(false);
    }
    let gdef = *ctx.face.gdef();
    let idx = ctx.buffer.idx;
    let template = ctx.buffer.info[idx];
    let class = if template.is_ligature() {
        Some(props::BASE)
    } else {
        None
    };
    let expanded = glyphs.iter().enumerate().map(|(component, glyph)| {
        let mut info = template;
        info.set_glyph(glyph, &gdef, class);
        info.multiplied = true;
        // Glyphs attached to a ligature keep their component.
        if template.lig_id == 0 {
            info.set_lig_props_for_mark(0, component as u8);
        }
        info
    });
    let inserted = ctx.buffer.splice(idx, expanded);
    ctx.buffer.idx += inserted;
    Ok(true)
}

fn alternate(ctx: &mut Applier<'_, '_>, data: &[u8]) -> Result<bool> {
    let Some(index) = covered(ctx, data)? else {
        return Ok(false);
    };
    let Some(set) = indexed(data, 4, index)? else {
        return Ok(false);
    };
    // Default features carry the value 1, selecting the first alternate.
    match glyph_array(set)?.get(0) {
        Some(glyph) => {
            replace(ctx, glyph);
            Ok(true)
        }
        None => Ok(false),
    }
}

fn ligature(ctx: &mut Applier<'_, '_>, data: &[u8]) -> Result<bool> {
    let Some(index) = covered(ctx, data)? else {
        return Ok(false);
    };
    let Some(set) = indexed(data, 4, index)? else {
        return Ok(false);
    };
    let mut r = Reader::new(set, GSUB);
    let count = r.u16()?;
    let offsets: Array<'_, u16> = r.array(usize::from(count))?;
    for offset in offsets.iter() {
        let mut lig = Reader::at(set, usize::from(offset), GSUB)?;
        let glyph = lig.glyph()?;
        let component_count = usize::from(lig.u16()?);
        let components: Array<'_, GlyphId> = lig.array(component_count.saturating_sub(1))?;
        let matcher = |i: usize, info: &GlyphInfo| components.get(i - 1) == Some(info.glyph);
        if let Some(positions) = ctx.match_input(components.len(), &matcher) {
            ligate(ctx, &positions, glyph);
            return Ok(true);
        }
    }
    Ok(false)
}

/// HarfBuzz's `ligate_input`: replaces the first matched glyph by the
/// ligature, deletes the other components and renumbers the ligature
/// components of marks caught in between or attached to the components.
fn ligate(ctx: &mut Applier<'_, '_>, positions: &[usize], glyph: GlyphId) {
    let info = &ctx.buffer.info;
    let first = info[positions[0]];
    let rest_are_marks = positions[1..].iter().all(|&p| info[p].is_mark());
    let is_base_ligature = first.is_base() && rest_are_marks;
    let is_mark_ligature = first.is_mark() && rest_are_marks;
    let is_ligature = !is_base_ligature && !is_mark_ligature;
    let total_components: u32 = positions
        .iter()
        .map(|&p| u32::from(info[p].lig_num_comps()))
        .sum();
    let lig_id = if is_ligature {
        ctx.buffer.allocate_lig_id()
    } else {
        0
    };
    let last = *positions.last().unwrap_or(&positions[0]);
    ctx.buffer.merge_clusters(positions[0], last + 1);

    let gdef = *ctx.face.gdef();
    let start = positions[0];
    let head = &mut ctx.buffer.info[start];
    let mut last_lig_id = head.lig_id;
    let mut last_num_components = u32::from(head.lig_num_comps());
    let mut components_so_far = last_num_components;
    head.set_glyph(glyph, &gdef, is_ligature.then_some(props::LIGATURE));
    head.multiplied = false;
    if is_ligature {
        head.set_lig_props_for_ligature(lig_id, total_components as u8);
        // A ligature starting with a nonspacing mark is no mark any more.
        if head.chars.is_nonspacing_mark() {
            head.chars.category = GeneralCategory::OtherLetter;
            head.chars.combining_class = 0;
        }
    }

    let renumber = |info: &mut GlyphInfo, so_far: u32, last_count: u32| {
        let this = match u32::from(info.lig_comp()) {
            0 => last_count,
            comp => comp,
        };
        let new_comp = so_far - last_count + this.min(last_count);
        info.set_lig_props_for_mark(lig_id, new_comp as u8);
    };
    for window in positions.windows(2) {
        if is_ligature {
            for between in window[0] + 1..window[1] {
                renumber(
                    &mut ctx.buffer.info[between],
                    components_so_far,
                    last_num_components,
                );
            }
        }
        let component = ctx.buffer.info[window[1]];
        last_lig_id = component.lig_id;
        last_num_components = u32::from(component.lig_num_comps());
        components_so_far += last_num_components;
    }
    if !is_mark_ligature && last_lig_id != 0 {
        for following in &mut ctx.buffer.info[last + 1..] {
            if following.lig_id != last_lig_id || following.lig_comp() == 0 {
                break;
            }
            renumber(following, components_so_far, last_num_components);
        }
    }
    for &position in positions[1..].iter().rev() {
        ctx.buffer.remove(position);
    }
    ctx.buffer.idx = last + 1 - (positions.len() - 1);
}

fn reverse_chain(ctx: &mut Applier<'_, '_>, data: &[u8]) -> Result<bool> {
    let Some(index) = covered(ctx, data)? else {
        return Ok(false);
    };
    let mut r = Reader::new(data, GSUB);
    r.skip(4)?;
    let backtrack_count = r.u16()?;
    let backtrack: Array<'_, u16> = r.array(usize::from(backtrack_count))?;
    let lookahead_count = r.u16()?;
    let lookahead: Array<'_, u16> = r.array(usize::from(lookahead_count))?;
    let glyph_count = r.u16()?;
    let substitutes: Array<'_, GlyphId> = r.array(usize::from(glyph_count))?;
    let backtrack_matcher = |i: usize, info: &GlyphInfo| covers(data, backtrack, i, info.glyph);
    let lookahead_matcher = |i: usize, info: &GlyphInfo| covers(data, lookahead, i, info.glyph);
    let idx = ctx.buffer.idx;
    if !ctx.match_backtrack(idx, backtrack.len(), &backtrack_matcher)
        || !ctx.match_lookahead(idx, lookahead.len(), &lookahead_matcher)
    {
        return Ok(false);
    }
    let Some(glyph) = substitutes.get(usize::from(index)) else {
        return Ok(false);
    };
    let gdef = *ctx.face.gdef();
    ctx.buffer.info[idx].set_glyph(glyph, &gdef, None);
    Ok(true)
}

/// Whether coverage table `offsets[index]` (relative to `data`) covers
/// `glyph`.
fn covers(data: &[u8], offsets: Array<'_, u16>, index: usize, glyph: GlyphId) -> bool {
    offsets
        .get(index)
        .and_then(|offset| Coverage::at(data, offset, GSUB).ok())
        .is_some_and(|coverage| coverage.contains(glyph))
}
