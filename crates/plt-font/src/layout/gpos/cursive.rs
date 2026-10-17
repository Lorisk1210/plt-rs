//! Cursive attachment (`GPOS` lookup type 3): joins the exit anchor of a
//! glyph to the entry anchor of the next one, as HarfBuzz's
//! `CursivePosFormat1` does.

use super::GPOS;
use super::mark::{anchor, round};
use crate::error::Result;
use crate::layout::apply::Applier;
use crate::layout::buffer::{AttachKind, GlyphPos};
use crate::layout::common::{Coverage, lookup_flag};
use crate::reader::{Reader, peek, tail};
use crate::types::Direction;

/// The entry (`exit == false`) or exit anchor offset of the glyph with
/// coverage index `index`; `None` when it has none.
fn anchor_offset(data: &[u8], index: u16, exit: bool) -> Option<u16> {
    let count = peek::<u16>(data, 4)?;
    if index >= count {
        return None;
    }
    let at = 6 + 4 * usize::from(index) + if exit { 2 } else { 0 };
    peek::<u16>(data, at).filter(|&offset| offset != 0)
}

/// HarfBuzz's `reverse_cursive_minor_offset`: when `child` is already
/// attached elsewhere, flips the old chain so that it hangs off the new
/// parent.
fn reverse_minor_offset(pos: &mut [GlyphPos], i: usize, new_parent: usize) {
    let chain = pos[i].attach_chain;
    if chain == 0 || pos[i].attach_kind != AttachKind::Cursive {
        return;
    }
    pos[i].attach_chain = 0;
    let Some(j) = i
        .checked_add_signed(chain as isize)
        .filter(|&j| j < pos.len())
    else {
        return;
    };
    if j == new_parent {
        return;
    }
    reverse_minor_offset(pos, j, new_parent);
    pos[j].y_offset = -pos[i].y_offset;
    pos[j].attach_chain = -chain;
    pos[j].attach_kind = AttachKind::Cursive;
}

/// Applies a cursive attachment subtable at the cursor.
pub(super) fn apply(ctx: &mut Applier<'_, '_>, data: &[u8]) -> Result<bool> {
    let mut r = Reader::new(data, GPOS);
    if r.u16()? != 1 {
        return Ok(false);
    }
    let coverage = Coverage::at(data, r.u16()?, GPOS)?;
    let j = ctx.buffer.idx;
    let Some(entry) = ctx
        .cur()
        .and_then(|info| coverage.index(info.glyph))
        .and_then(|index| anchor_offset(data, index, false))
    else {
        return Ok(false);
    };
    let Some(i) = ctx.prev(j) else {
        return Ok(false);
    };
    let Some(exit) = coverage
        .index(ctx.buffer.info[i].glyph)
        .and_then(|index| anchor_offset(data, index, true))
    else {
        return Ok(false);
    };
    let (exit_x, exit_y) = anchor(ctx, tail(data, usize::from(exit), GPOS)?)?;
    let (entry_x, entry_y) = anchor(ctx, tail(data, usize::from(entry), GPOS)?)?;
    let pos = &mut ctx.buffer.pos;
    match ctx.direction {
        Direction::LeftToRight => {
            pos[i].x_advance = round(exit_x) + pos[i].x_offset;
            let d = round(entry_x) + pos[j].x_offset;
            pos[j].x_advance -= d;
            pos[j].x_offset -= d;
        }
        Direction::RightToLeft => {
            let d = round(exit_x) + pos[i].x_offset;
            pos[i].x_advance -= d;
            pos[i].x_offset -= d;
            pos[j].x_advance = round(entry_x) + pos[j].x_offset;
        }
    }
    // The child aligns itself on its parent; right-to-left lookups attach
    // the earlier glyph to the later one.
    let (mut child, mut parent) = (i, j);
    let mut y_offset = round(entry_y - exit_y);
    if ctx.flags & lookup_flag::RIGHT_TO_LEFT == 0 {
        (child, parent) = (parent, child);
        y_offset = -y_offset;
    }
    reverse_minor_offset(pos, child, parent);
    let chain = parent as i32 - child as i32;
    pos[child].attach_chain = chain;
    pos[child].attach_kind = AttachKind::Cursive;
    pos[child].y_offset = y_offset;
    if pos[parent].attach_chain == -chain {
        pos[parent].attach_chain = 0;
        pos[parent].y_offset = 0;
    }
    ctx.buffer.idx += 1;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attached(chain: i32, y_offset: i32) -> GlyphPos {
        GlyphPos {
            attach_chain: chain,
            attach_kind: AttachKind::Cursive,
            y_offset,
            ..GlyphPos::default()
        }
    }

    #[test]
    fn reversing_a_chain() {
        // 0 -> 1 -> 2; re-parenting 0 onto 3 flips the chain 1 -> 0.
        let mut pos = vec![attached(1, 5), attached(1, 7), GlyphPos::default()];
        pos.push(GlyphPos::default());
        reverse_minor_offset(&mut pos, 0, 3);
        assert_eq!(pos[0].attach_chain, 0);
        assert_eq!((pos[1].attach_chain, pos[1].y_offset), (-1, -5));
        assert_eq!((pos[2].attach_chain, pos[2].y_offset), (-1, -7));
        let mut marks = vec![GlyphPos {
            attach_chain: 1,
            attach_kind: AttachKind::Mark,
            ..GlyphPos::default()
        }];
        reverse_minor_offset(&mut marks, 0, 0);
        assert_eq!(marks[0].attach_chain, 1);
    }

    #[test]
    fn anchor_offsets() {
        let data = [0, 1, 0, 0, 0, 2, 0, 10, 0, 0, 0, 0, 0, 12];
        assert_eq!(anchor_offset(&data, 0, false), Some(10));
        assert_eq!(anchor_offset(&data, 0, true), None);
        assert_eq!(anchor_offset(&data, 1, true), Some(12));
        assert_eq!(anchor_offset(&data, 2, true), None);
    }
}
