//! HarfBuzz's `hb_ot_position`: default advances, `GPOS`, `kerx` or
//! `kern` kerning, `trak` tracking, late zeroing of mark advances,
//! attachment offset propagation and fallback mark positioning.

use crate::aat::{KernContext, KernTable, Trak};
use crate::face::Face;
use crate::layout::apply::{Applier, TableKind};
use crate::layout::buffer::{AttachKind, Buffer};
use crate::scale::Scaler;
use crate::shape::fallback;
use crate::shape::plan::ShapePlan;
use crate::shape::spaces::{self, SpaceWidth};
use crate::types::Direction;

/// Longest chain of attachments followed (HarfBuzz's nesting limit).
const MAX_ATTACHMENT_DEPTH: u8 = 64;

/// Positions the glyphs of a run shaped in `direction` (logical order).
pub(crate) fn position(
    face: &Face<'_>,
    plan: &ShapePlan,
    direction: Direction,
    scaler: &Scaler<'_>,
    spaces: &[(usize, SpaceWidth)],
    buffer: &mut Buffer,
) {
    for (info, pos) in buffer.info.iter().zip(buffer.pos.iter_mut()) {
        // FreeType gives glyphs the font lacks (deleted ones) no advance.
        pos.x_advance = if info.glyph.0 < face.num_glyphs() {
            scaler.advance(face, info.glyph)
        } else {
            0
        };
    }
    let space_glyph = face.glyph_index(' ');
    for &(cluster, width) in spaces {
        let found = buffer
            .info
            .iter()
            .position(|info| info.cluster == cluster && Some(info.glyph) == space_glyph);
        if let Some(index) = found {
            buffer.pos[index].x_advance =
                spaces::advance(width, face, scaler, buffer.pos[index].x_advance);
        }
    }
    let kerning = KernContext {
        scaler,
        kern_mask: plan.kern_mask,
        num_glyphs: face.num_glyphs(),
        forward: direction.is_forward(),
    };
    if let Some(table) = face.gpos().filter(|_| plan.apply_gpos) {
        let mut applier = Applier::new(face, *table, TableKind::Gpos, scaler, direction, buffer);
        for stage in plan.map.stages(TableKind::Gpos) {
            for lookup in &stage.lookups {
                applier.apply_lookup(*lookup);
            }
        }
    } else if let Some(kerx) = KernTable::kerx(face).filter(|_| plan.apply_kerx) {
        kerx.apply(face, buffer, &kerning);
    }
    if let Some(kern) = KernTable::kern(face).filter(|_| plan.apply_kern) {
        kern.apply(face, buffer, &kerning);
    }
    if let Some(trak) = Trak::of(face) {
        trak.apply(buffer, scaler);
    }
    let adjust_offsets = plan.adjust_mark_positioning && direction.is_forward();
    if plan.zero_marks {
        for (info, pos) in buffer.info.iter().zip(buffer.pos.iter_mut()) {
            if info.is_mark() {
                if adjust_offsets {
                    pos.x_offset -= pos.x_advance;
                    pos.y_offset -= pos.y_advance;
                }
                pos.x_advance = 0;
                pos.y_advance = 0;
            }
        }
    }
    for (info, pos) in buffer.info.iter().zip(buffer.pos.iter_mut()) {
        if info.is_default_ignorable() {
            pos.x_advance = 0;
            pos.y_advance = 0;
            pos.x_offset = 0;
        }
    }
    let indices: Vec<usize> = if direction.is_forward() {
        (0..buffer.len()).collect()
    } else {
        (0..buffer.len()).rev().collect()
    };
    for index in indices {
        propagate_attachment(buffer, index, direction, MAX_ATTACHMENT_DEPTH);
    }
    if plan.fallback_mark_positioning {
        fallback::position_marks(face, scaler, direction, adjust_offsets, buffer);
    }
}

/// HarfBuzz's `propagate_attachment_offsets`: adds the offsets of the
/// glyph a glyph is attached to; marks also account for the advances in
/// between (which depend on the direction).
fn propagate_attachment(buffer: &mut Buffer, index: usize, direction: Direction, depth: u8) {
    let pos = buffer.pos[index];
    if pos.attach_chain == 0 {
        return;
    }
    buffer.pos[index].attach_chain = 0;
    let Some(target) = index
        .checked_add_signed(pos.attach_chain as isize)
        .filter(|&t| t < buffer.len())
    else {
        return;
    };
    if depth == 0 {
        return;
    }
    if buffer.pos[target].attach_chain != 0 {
        propagate_attachment(buffer, target, direction, depth - 1);
    }
    let base = buffer.pos[target];
    if pos.attach_kind == AttachKind::Cursive {
        buffer.pos[index].y_offset += base.y_offset;
        return;
    }
    let sum = |range: std::ops::Range<usize>, buffer: &Buffer| {
        buffer.pos[range]
            .iter()
            .fold((0, 0), |(x, y), p| (x + p.x_advance, y + p.y_advance))
    };
    let (dx, dy) = match (target < index, direction.is_forward()) {
        (true, true) => {
            let (x, y) = sum(target..index, buffer);
            (-x, -y)
        }
        (true, false) => sum(target + 1..index + 1, buffer),
        (false, true) => sum(index..target, buffer),
        (false, false) => {
            let (x, y) = sum(index + 1..target + 1, buffer);
            (-x, -y)
        }
    };
    let current = &mut buffer.pos[index];
    current.x_offset += base.x_offset + dx;
    current.y_offset += base.y_offset + dy;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::buffer::{CharProps, GlyphInfo, GlyphPos};
    use crate::types::GlyphId;

    fn buffer(advances: &[i32]) -> Buffer {
        let info = vec![GlyphInfo::new(GlyphId(1), 0, CharProps::default(), 1); advances.len()];
        let mut buffer = Buffer::new(info);
        for (pos, &advance) in buffer.pos.iter_mut().zip(advances) {
            pos.x_advance = advance;
        }
        buffer
    }

    #[test]
    fn mark_offsets_depend_on_direction() {
        let mut ltr = buffer(&[500, 300, 0]);
        ltr.pos[2] = GlyphPos {
            x_offset: 10,
            attach_chain: -2,
            attach_kind: AttachKind::Mark,
            ..GlyphPos::default()
        };
        propagate_attachment(&mut ltr, 2, Direction::LeftToRight, MAX_ATTACHMENT_DEPTH);
        assert_eq!(
            (ltr.pos[2].x_offset, ltr.pos[2].attach_chain),
            (10 - 800, 0)
        );
        let mut rtl = buffer(&[500, 300, 0]);
        rtl.pos[2] = GlyphPos {
            x_offset: 10,
            attach_chain: -2,
            attach_kind: AttachKind::Mark,
            ..GlyphPos::default()
        };
        propagate_attachment(&mut rtl, 2, Direction::RightToLeft, MAX_ATTACHMENT_DEPTH);
        assert_eq!(rtl.pos[2].x_offset, 10 + 300);
    }

    #[test]
    fn cursive_chains_accumulate_vertically() {
        let mut chain = buffer(&[400, 400, 400]);
        chain.pos[0].attach_chain = 1;
        chain.pos[0].attach_kind = AttachKind::Cursive;
        chain.pos[0].y_offset = 5;
        chain.pos[1].attach_chain = 1;
        chain.pos[1].attach_kind = AttachKind::Cursive;
        chain.pos[1].y_offset = 7;
        chain.pos[2].y_offset = 3;
        propagate_attachment(&mut chain, 0, Direction::RightToLeft, MAX_ATTACHMENT_DEPTH);
        assert_eq!(
            (chain.pos[0].y_offset, chain.pos[1].y_offset),
            (5 + 7 + 3, 10)
        );
        assert_eq!(chain.pos[0].x_offset, 0);
    }
}
