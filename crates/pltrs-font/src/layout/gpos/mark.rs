//! Anchors and mark attachment: mark-to-base, mark-to-ligature and
//! mark-to-mark positioning.

use super::{GPOS, current_coverage};
use crate::error::Result;
use crate::layout::apply::Applier;
use crate::layout::buffer::{AttachKind, Buffer, GlyphInfo};
use crate::layout::common::Coverage;
use crate::reader::{Array, Reader, peek, tail};

/// HarfBuzz's `roundf`, which rounds halves towards positive infinity.
pub(super) fn round(value: f32) -> i32 {
    (value + 0.5).floor() as i32
}

/// Reads an anchor table and returns its scaled position. Contour-point
/// and device-table refinements need a ppem, which matplotlib's HarfBuzz
/// font (hb-ft) doesn't set, so every format uses its design coordinates.
pub(super) fn anchor(ctx: &Applier<'_, '_>, data: &[u8]) -> Result<(f32, f32)> {
    let mut r = Reader::new(data, GPOS);
    r.skip(2)?;
    let (x, y) = (i32::from(r.i16()?), i32::from(r.i16()?));
    Ok((ctx.scaler.fx(x), ctx.scaler.fy(y)))
}

/// HarfBuzz's `MarkArray::apply`: attaches the current mark to the glyph
/// at `target` using anchor class records found in `anchors` (an array of
/// `class_count` anchor offsets relative to `anchors_base`).
fn attach_mark(
    ctx: &mut Applier<'_, '_>,
    mark_array: &[u8],
    mark_index: u16,
    anchor_offsets: Array<'_, u16>,
    anchors_base: &[u8],
    target: usize,
) -> Result<bool> {
    let mut r = Reader::new(mark_array, GPOS);
    let count = r.u16()?;
    if mark_index >= count {
        return Ok(false);
    }
    r.skip(4 * usize::from(mark_index))?;
    let class = r.u16()?;
    let mark_anchor_offset = r.u16()?;
    let Some(base_offset) = anchor_offsets.get(usize::from(class)).filter(|&o| o != 0) else {
        return Ok(false);
    };
    let idx = ctx.buffer.idx;
    let (mark_x, mark_y) = anchor(
        ctx,
        tail(mark_array, usize::from(mark_anchor_offset), GPOS)?,
    )?;
    let (base_x, base_y) = anchor(ctx, tail(anchors_base, usize::from(base_offset), GPOS)?)?;
    let pos = &mut ctx.buffer.pos[idx];
    pos.x_offset = round(base_x - mark_x);
    pos.y_offset = round(base_y - mark_y);
    pos.attach_kind = AttachKind::Mark;
    pos.attach_chain = target as i32 - idx as i32;
    ctx.buffer.idx += 1;
    Ok(true)
}

/// Offsets of the anchors for one row of a base/component record matrix.
fn anchor_row(data: &[u8], row: usize, class_count: usize) -> Result<Array<'_, u16>> {
    let mut r = Reader::new(data, GPOS);
    let rows = usize::from(r.u16()?);
    if row >= rows {
        return Ok(Array::default());
    }
    r.skip(row * class_count * 2)?;
    r.array(class_count)
}

/// Mark-to-base (`ligature == false`) and mark-to-ligature attachment.
pub(super) fn mark_to_base(ctx: &mut Applier<'_, '_>, data: &[u8], ligature: bool) -> Result<bool> {
    let mut r = Reader::new(data, GPOS);
    r.skip(2)?;
    let Some(mark_index) = current_coverage(ctx, data, r.u16()?)? else {
        return Ok(false);
    };
    let base_coverage = Coverage::at(data, r.u16()?, GPOS)?;
    let class_count = usize::from(r.u16()?);
    let mark_array = tail(data, usize::from(r.u16()?), GPOS)?;
    let base_array = tail(data, usize::from(r.u16()?), GPOS)?;
    // The closest preceding glyph that is not a mark; mark-to-base only
    // attaches to the first glyph of a multiple substitution sequence.
    let idx = ctx.buffer.idx;
    let accept = |buffer: &Buffer, i: usize| {
        ligature || accepts_base(buffer, i) || base_coverage.contains(buffer.info[i].glyph)
    };
    let Some(target) = ctx.find_base(idx, &accept) else {
        return Ok(false);
    };
    let Some(base_index) = base_coverage.index(ctx.buffer.info[target].glyph) else {
        return Ok(false);
    };
    if !ligature {
        let row = anchor_row(base_array, usize::from(base_index), class_count)?;
        return attach_mark(ctx, mark_array, mark_index, row, base_array, target);
    }
    let mut lr = Reader::new(base_array, GPOS);
    let attach_count = usize::from(lr.u16()?);
    let attaches: Array<'_, u16> = lr.array(attach_count)?;
    let Some(attach_offset) = attaches.get(usize::from(base_index)) else {
        return Ok(false);
    };
    let attach = tail(base_array, usize::from(attach_offset), GPOS)?;
    let component_count = usize::from(peek::<u16>(attach, 0).unwrap_or(0));
    if component_count == 0 {
        return Ok(false);
    }
    let (lig, mark) = (ctx.buffer.info[target], ctx.buffer.info[idx]);
    let component = if lig.lig_id != 0 && lig.lig_id == mark.lig_id && mark.lig_comp() > 0 {
        component_count.min(usize::from(mark.lig_comp())) - 1
    } else {
        component_count - 1
    };
    let row = anchor_row(attach, component, class_count)?;
    attach_mark(ctx, mark_array, mark_index, row, attach, target)
}

/// HarfBuzz's `MarkBasePosFormat1::accept`: rejects the second and later
/// glyphs of a multiple substitution sequence, unless a mark precedes them.
fn accepts_base(buffer: &Buffer, index: usize) -> bool {
    let info = &buffer.info[index];
    if !info.multiplied || info.lig_comp() == 0 || index == 0 {
        return true;
    }
    let previous = &buffer.info[index - 1];
    previous.is_mark()
        || !previous.multiplied
        || info.lig_id != previous.lig_id
        || info.lig_comp() != previous.lig_comp() + 1
}

/// Whether two marks may attach to each other (HarfBuzz's ligature
/// component checks in `MarkMarkPosFormat1::apply`).
fn marks_compatible(mark1: &GlyphInfo, mark2: &GlyphInfo) -> bool {
    let (id1, id2) = (mark1.lig_id, mark2.lig_id);
    let (comp1, comp2) = (mark1.lig_comp(), mark2.lig_comp());
    if id1 == id2 {
        id1 == 0 || comp1 == comp2
    } else {
        (id1 > 0 && comp1 == 0) || (id2 > 0 && comp2 == 0)
    }
}

pub(super) fn mark_to_mark(ctx: &mut Applier<'_, '_>, data: &[u8]) -> Result<bool> {
    let mut r = Reader::new(data, GPOS);
    r.skip(2)?;
    let Some(mark1_index) = current_coverage(ctx, data, r.u16()?)? else {
        return Ok(false);
    };
    let mark2_coverage = Coverage::at(data, r.u16()?, GPOS)?;
    let class_count = usize::from(r.u16()?);
    let mark1_array = tail(data, usize::from(r.u16()?), GPOS)?;
    let mark2_array = tail(data, usize::from(r.u16()?), GPOS)?;
    let idx = ctx.buffer.idx;
    let Some(previous) = ctx.prev_ignoring_classes(idx) else {
        return Ok(false);
    };
    let (mark1, mark2) = (ctx.buffer.info[idx], ctx.buffer.info[previous]);
    if !mark2.is_mark() || !marks_compatible(&mark1, &mark2) {
        return Ok(false);
    }
    let Some(mark2_index) = mark2_coverage.index(mark2.glyph) else {
        return Ok(false);
    };
    let row = anchor_row(mark2_array, usize::from(mark2_index), class_count)?;
    attach_mark(ctx, mark1_array, mark1_index, row, mark2_array, previous)
}
