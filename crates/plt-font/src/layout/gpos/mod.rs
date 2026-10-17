//! `GPOS` subtables: single and pair adjustment, cursive and mark
//! attachment, contextual positioning.

mod cursive;
mod mark;

use crate::error::{FontError, Result};
use crate::face::Face;
use crate::layout::apply::{Applier, TableKind};
use crate::layout::buffer::{Buffer, CharProps, GlyphInfo, GlyphPos, Mask, gdef_props};
use crate::layout::common::{ClassDef, Coverage};
use crate::layout::context;
use crate::layout::feature::feature_lookups;
use crate::layout::map::LookupMap;
use crate::reader::{Array, Reader, peek, tail};
use crate::scale::Scaler;
use crate::types::{Direction, GlyphId, Tag};
use mark::{mark_to_base, mark_to_mark};

const GPOS: Tag = Tag::new(b"GPOS");

/// Applies one `GPOS` subtable of lookup type `kind` at the cursor.
/// Malformed subtables simply do not apply.
pub(crate) fn apply(ctx: &mut Applier<'_, '_>, kind: u16, data: &[u8]) -> bool {
    let result = match kind {
        1 => single(ctx, data),
        2 => pair(ctx, data),
        3 => cursive::apply(ctx, data),
        4 => mark_to_base(ctx, data, false),
        5 => mark_to_base(ctx, data, true),
        6 => mark_to_mark(ctx, data),
        7 => context::apply_context(ctx, data, GPOS),
        8 => context::apply_chain_context(ctx, data, GPOS),
        _ => Ok(false),
    };
    result.unwrap_or(false)
}

/// A value record. Its device tables never apply: matplotlib's HarfBuzz
/// font (hb-ft) has no ppem, so hinting deltas are zero.
#[derive(Clone, Copy)]
struct Value<'a> {
    format: u16,
    record: &'a [u8],
}

fn value_size(format: u16) -> usize {
    (format & 0xFF).count_ones() as usize * 2
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
    }
}

/// Coverage index of the current glyph in the coverage table at `offset`.
pub(super) fn current_coverage(
    ctx: &Applier<'_, '_>,
    data: &[u8],
    offset: u16,
) -> Result<Option<u16>> {
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
        },
        Value {
            format: format2,
            record: rest,
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

/// Kerning between two glyphs from the `GPOS` `kern` feature (Latin
/// script, else the default script), in font units; `None` when the font
/// has no such feature.
pub(crate) fn pair_kerning(face: &Face<'_>, left: GlyphId, right: GlyphId) -> Option<i32> {
    let table = *face.gpos()?;
    let lookups = feature_lookups(&table, Some(Tag::new(b"latn")), Tag::new(b"kern"))?;
    let gdef = face.gdef();
    let infos = [left, right].map(|glyph| {
        let mut info = GlyphInfo::new(glyph, 0, CharProps::default(), Mask::MAX);
        info.props = gdef_props(gdef, glyph);
        info
    });
    let mut buffer = Buffer::new(infos.to_vec());
    let scaler = Scaler::Units;
    let mut applier = Applier::new(
        face,
        table,
        TableKind::Gpos,
        &scaler,
        Direction::LeftToRight,
        &mut buffer,
    );
    for lookup in lookups {
        applier.apply_lookup(LookupMap::global(lookup));
    }
    Some(buffer.pos[0].x_advance + buffer.pos[1].x_offset)
}
