//! `GPOS` subtables: single and pair adjustment, mark attachment and
//! contextual positioning.

use crate::error::{FontError, Result};
use crate::face::Face;
use crate::layout::apply::{Applier, TableKind};
use crate::layout::buffer::{AttachKind, Buffer, CharProps, GlyphInfo, GlyphPos, gdef_props};
use crate::layout::common::{ClassDef, Coverage};
use crate::layout::context;
use crate::layout::plan::feature_lookups;
use crate::reader::{Array, Reader, peek, tail};
use crate::scale::Scaler;
use crate::types::{GlyphId, Tag};

const GPOS: Tag = Tag::new(b"GPOS");

/// Applies one `GPOS` subtable of lookup type `kind` at the cursor.
/// Malformed subtables simply do not apply.
pub(crate) fn apply(ctx: &mut Applier<'_, '_>, kind: u16, data: &[u8]) -> bool {
    let result = match kind {
        1 => single(ctx, data),
        2 => pair(ctx, data),
        4 => mark_to_base(ctx, data, false),
        5 => mark_to_base(ctx, data, true),
        6 => mark_to_mark(ctx, data),
        7 => context::apply_context(ctx, data, GPOS),
        8 => context::apply_chain_context(ctx, data, GPOS),
        _ => Ok(false),
    };
    result.unwrap_or(false)
}

/// A value record together with the subtable its device offsets are
/// relative to.
#[derive(Clone, Copy)]
struct Value<'a> {
    format: u16,
    record: &'a [u8],
    base: &'a [u8],
}

fn value_size(format: u16) -> usize {
    (format & 0xFF).count_ones() as usize * 2
}

/// HarfBuzz's `Device::get_delta_pixels`.
fn device_pixels(device: &[u8], ppem: u16) -> i64 {
    let (Some(start), Some(end), Some(format)) = (
        peek::<u16>(device, 0),
        peek::<u16>(device, 2),
        peek::<u16>(device, 4),
    ) else {
        return 0;
    };
    if !(1..=3).contains(&format) || ppem < start || ppem > end {
        return 0;
    }
    let s = u32::from(ppem - start);
    let f = u32::from(format);
    let Some(word) = peek::<u16>(device, 6 + 2 * (s >> (4 - f)) as usize) else {
        return 0;
    };
    let bits = u32::from(word) >> (16 - (((s & ((1 << (4 - f)) - 1)) + 1) << f));
    let mask = 0xFFFF >> (16 - (1 << f));
    let delta = i64::from(bits & mask);
    if delta >= i64::from((mask + 1) >> 1) {
        delta - i64::from(mask + 1)
    } else {
        delta
    }
}

impl Value<'_> {
    /// Adds the value to `pos` (horizontal layout: `YAdvance` is ignored).
    fn apply(&self, scaler: &Scaler, pos: &mut GlyphPos) {
        let mut fields = self
            .record
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&pair| i16::from_be_bytes(pair));
        let mut next = |bit: u16| {
            if self.format & bit != 0 {
                fields.next()
            } else {
                None
            }
        };
        if let Some(v) = next(0x0001) {
            pos.x_offset += scaler.x(i32::from(v));
        }
        if let Some(v) = next(0x0002) {
            pos.y_offset += scaler.y(i32::from(v));
        }
        if let Some(v) = next(0x0004) {
            pos.x_advance += scaler.x(i32::from(v));
        }
        next(0x0008);
        let Some(((x_ppem, x_scale), (y_ppem, y_scale))) = scaler.device() else {
            return;
        };
        let delta = |offset: Option<i16>, ppem: u16, scale: i64| -> i32 {
            let Some(offset) = offset.filter(|&o| o != 0) else {
                return 0;
            };
            let Some(device) = self.base.get(usize::from(offset as u16)..) else {
                return 0;
            };
            if ppem == 0 {
                return 0;
            }
            (device_pixels(device, ppem) * scale / i64::from(ppem)) as i32
        };
        pos.x_offset += delta(next(0x0010), x_ppem, x_scale);
        pos.y_offset += delta(next(0x0020), y_ppem, y_scale);
        pos.x_advance += delta(next(0x0040), x_ppem, x_scale);
    }
}

fn current_coverage(ctx: &Applier<'_, '_>, data: &[u8], offset: u16) -> Result<Option<u16>> {
    let coverage = Coverage::at(data, offset, GPOS)?;
    Ok(ctx.cur().and_then(|info| coverage.index(info.glyph)))
}

fn single(ctx: &mut Applier<'_, '_>, data: &[u8]) -> Result<bool> {
    let mut r = Reader::new(data, GPOS);
    let format = r.u16()?;
    let Some(index) = current_coverage(ctx, data, r.u16()?)? else {
        return Ok(false);
    };
    let value_format = r.u16()?;
    let size = value_size(value_format);
    let record = match format {
        1 => r.bytes(size)?,
        2 => {
            r.skip(2)?;
            r.skip(size * usize::from(index))?;
            r.bytes(size)?
        }
        _ => return Ok(false),
    };
    let idx = ctx.buffer.idx;
    Value {
        format: value_format,
        record,
        base: data,
    }
    .apply(ctx.scaler, &mut ctx.buffer.pos[idx]);
    ctx.buffer.idx += 1;
    Ok(true)
}

/// Applies a pair's two values and moves the cursor like HarfBuzz: onto
/// the second glyph, or past it when the second value is non-empty.
fn apply_pair(
    ctx: &mut Applier<'_, '_>,
    second: usize,
    first_value: Value<'_>,
    second_value: Value<'_>,
) {
    let idx = ctx.buffer.idx;
    first_value.apply(ctx.scaler, &mut ctx.buffer.pos[idx]);
    second_value.apply(ctx.scaler, &mut ctx.buffer.pos[second]);
    ctx.buffer.idx = if second_value.format == 0 {
        second
    } else {
        second + 1
    };
}

fn pair<'d>(ctx: &mut Applier<'_, '_>, data: &'d [u8]) -> Result<bool> {
    let mut r = Reader::new(data, GPOS);
    let format = r.u16()?;
    let Some(index) = current_coverage(ctx, data, r.u16()?)? else {
        return Ok(false);
    };
    let (format1, format2) = (r.u16()?, r.u16()?);
    let (size1, size2) = (value_size(format1), value_size(format2));
    let Some(second) = ctx.next(ctx.buffer.idx) else {
        return Ok(false);
    };
    let second_glyph = ctx.buffer.info[second].glyph;
    let split = |record: &'d [u8]| -> Result<(&'d [u8], &'d [u8])> {
        let first = record.get(..size1).ok_or(FontError::Truncated(GPOS))?;
        let rest = record
            .get(size1..size1 + size2)
            .ok_or(FontError::Truncated(GPOS))?;
        Ok((first, rest))
    };
    let (first, rest) = match format {
        1 => {
            let set_count = usize::from(r.u16()?);
            let sets: Array<'_, u16> = r.array(set_count)?;
            let Some(set_offset) = sets.get(usize::from(index)) else {
                return Ok(false);
            };
            let set = tail(data, usize::from(set_offset), GPOS)?;
            let mut sr = Reader::new(set, GPOS);
            let count = usize::from(sr.u16()?);
            let stride = 2 + size1 + size2;
            let records = sr.bytes(count * stride)?;
            let found = binary_search_records(records, stride, second_glyph);
            let Some(record) = found else {
                return Ok(false);
            };
            split(&record[2..])?
        }
        2 => {
            let class1 =
                ClassDef::at(data, r.u16()?, GPOS)?.class(ctx.buffer.info[ctx.buffer.idx].glyph);
            let class2 = ClassDef::at(data, r.u16()?, GPOS)?.class(second_glyph);
            let (count1, count2) = (r.u16()?, r.u16()?);
            if class1 >= count1 || class2 >= count2 {
                return Ok(false);
            }
            let stride = size1 + size2;
            let at = (usize::from(class1) * usize::from(count2) + usize::from(class2)) * stride;
            r.skip(at)?;
            split(r.bytes(stride)?)?
        }
        _ => return Ok(false),
    };
    apply_pair(
        ctx,
        second,
        Value {
            format: format1,
            record: first,
            base: data,
        },
        Value {
            format: format2,
            record: rest,
            base: data,
        },
    );
    Ok(true)
}

/// Finds the pair value record for `second` in a sorted record array.
fn binary_search_records(records: &[u8], stride: usize, second: GlyphId) -> Option<&[u8]> {
    let (mut low, mut high) = (0, records.len() / stride);
    while low < high {
        let mid = low + (high - low) / 2;
        let record = records.get(mid * stride..(mid + 1) * stride)?;
        let glyph = GlyphId(peek::<u16>(record, 0)?);
        match glyph.cmp(&second) {
            std::cmp::Ordering::Less => low = mid + 1,
            std::cmp::Ordering::Greater => high = mid,
            std::cmp::Ordering::Equal => return Some(record),
        }
    }
    None
}

/// Reads an anchor table and returns its scaled position.
fn anchor(ctx: &Applier<'_, '_>, data: &[u8], glyph: GlyphId) -> Result<(f32, f32)> {
    let mut r = Reader::new(data, GPOS);
    let format = r.u16()?;
    let (x, y) = (i32::from(r.i16()?), i32::from(r.i16()?));
    let (mut fx, mut fy) = (ctx.scaler.fx(x), ctx.scaler.fy(y));
    match format {
        2 if ctx.scaler.device().is_some() => {
            let point = usize::from(r.u16()?);
            if let Some(p) = ctx
                .face
                .load_outline(glyph, ctx.scaler.scaling())
                .ok()
                .and_then(|o| o.points.get(point).copied())
            {
                (fx, fy) = (p.x as f32, p.y as f32);
            }
        }
        3 => {
            if let Some(((x_ppem, x_scale), (y_ppem, y_scale))) = ctx.scaler.device() {
                let (x_device, y_device) = (r.u16()?, r.u16()?);
                let delta = |offset: u16, ppem: u16, scale: i64| {
                    if offset == 0 || ppem == 0 {
                        return 0.0;
                    }
                    data.get(usize::from(offset)..).map_or(0.0, |d| {
                        (device_pixels(d, ppem) * scale / i64::from(ppem)) as f32
                    })
                };
                fx += delta(x_device, x_ppem, x_scale);
                fy += delta(y_device, y_ppem, y_scale);
            }
        }
        _ => {}
    }
    Ok((fx, fy))
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
        ctx.buffer.info[idx].glyph,
    )?;
    let (base_x, base_y) = anchor(
        ctx,
        tail(anchors_base, usize::from(base_offset), GPOS)?,
        ctx.buffer.info[target].glyph,
    )?;
    let pos = &mut ctx.buffer.pos[idx];
    // HarfBuzz's `roundf` rounds halves towards positive infinity.
    pos.x_offset = (base_x - mark_x + 0.5).floor() as i32;
    pos.y_offset = (base_y - mark_y + 0.5).floor() as i32;
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
fn mark_to_base(ctx: &mut Applier<'_, '_>, data: &[u8], ligature: bool) -> Result<bool> {
    let mut r = Reader::new(data, GPOS);
    r.skip(2)?;
    let Some(mark_index) = current_coverage(ctx, data, r.u16()?)? else {
        return Ok(false);
    };
    let base_coverage = Coverage::at(data, r.u16()?, GPOS)?;
    let class_count = usize::from(r.u16()?);
    let mark_array = tail(data, usize::from(r.u16()?), GPOS)?;
    let base_array = tail(data, usize::from(r.u16()?), GPOS)?;
    // The closest preceding glyph that is not a mark.
    let idx = ctx.buffer.idx;
    let Some(target) = (0..idx).rev().find(|&i| {
        let info = &ctx.buffer.info[i];
        !info.is_mark() && !info.chars.default_ignorable
    }) else {
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

fn mark_to_mark(ctx: &mut Applier<'_, '_>, data: &[u8]) -> Result<bool> {
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

/// Kerning between two glyphs from the `GPOS` `kern` feature (Latin
/// script, else the default script), in font units; `None` when the font
/// has no such feature.
pub(crate) fn pair_kerning(face: &Face<'_>, left: GlyphId, right: GlyphId) -> Option<i32> {
    let table = *face.gpos()?;
    let lookups = feature_lookups(&table, Some(Tag::new(b"latn")), Tag::new(b"kern"))?;
    let gdef = face.gdef();
    let infos = [left, right].map(|glyph| {
        let mut info = GlyphInfo::new(glyph, 0, CharProps::default());
        info.props = gdef_props(gdef, glyph);
        info
    });
    let mut buffer = Buffer::new(infos.to_vec());
    let scaler = Scaler::Units;
    let mut applier = Applier::new(face, table, TableKind::Gpos, &scaler, &mut buffer);
    for lookup in lookups {
        applier.apply_lookup(lookup);
    }
    Some(buffer.pos[0].x_advance + buffer.pos[1].x_offset)
}
