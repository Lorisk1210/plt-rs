//! Insertion subtables (type 5): insert glyph lists before or after the
//! marked and the current glyph (HarfBuzz's `InsertionSubtable`).

use crate::aat::buffer::AatBuffer;
use crate::aat::state::{DONT_ADVANCE, Entry, Flavor, Machine, StateTable};
use crate::reader::peek;
use crate::types::GlyphId;

/// Makes the current glyph the marked one.
const SET_MARK: u16 = 0x8000;
/// Inserts before the current glyph rather than after it.
const CURRENT_INSERT_BEFORE: u16 = 0x0800;
/// Inserts before the marked glyph rather than after it.
const MARKED_INSERT_BEFORE: u16 = 0x0400;
/// The number of glyphs inserted at the current glyph.
const CURRENT_INSERT_COUNT: u16 = 0x03E0;
/// The number of glyphs inserted at the marked glyph.
const MARKED_INSERT_COUNT: u16 = 0x001F;
/// The entry data meaning "no insertion".
const NONE: u16 = 0xFFFF;

/// The running state of an insertion subtable.
#[derive(Debug)]
pub(super) struct Insertion<'a> {
    table: StateTable<'a>,
    /// Offset (from the state table) of the inserted glyphs.
    glyphs: usize,
    mark: usize,
}

impl<'a> Insertion<'a> {
    /// The subtable whose state table is `table`.
    pub(super) fn new(table: StateTable<'a>, flavor: Flavor) -> Option<Self> {
        let glyphs = flavor.read_field(table.data(), table.header_len())?;
        (glyphs != 0).then_some(Self {
            table,
            glyphs,
            mark: 0,
        })
    }

    /// The `count` glyphs listed from index `start` (none when the list
    /// runs past the table).
    fn list(&self, start: u16, count: usize) -> Vec<GlyphId> {
        let at = self.glyphs + 2 * usize::from(start);
        let glyphs: Option<Vec<GlyphId>> = (0..count)
            .map(|i| peek::<u16>(self.table.data(), at + 2 * i).map(GlyphId))
            .collect();
        glyphs.unwrap_or_default()
    }

    /// Inserts the `count` glyphs listed at `start` before or after the
    /// glyph under the cursor; `false` when the buffer gave out.
    fn insert(buffer: &mut AatBuffer<'_, '_>, glyphs: &[GlyphId], before: bool) -> bool {
        let after = buffer.idx() < buffer.len() && !before;
        if after && !buffer.copy_glyph() {
            return false;
        }
        if !buffer.output_glyphs(glyphs) {
            return false;
        }
        if after {
            buffer.skip_glyph();
        }
        true
    }
}

impl Machine for Insertion<'_> {
    const WRITES_OUTPUT: bool = true;

    fn initiates(&self, entry: Entry) -> bool {
        entry.flags & SET_MARK != 0
    }

    fn acts(&self, entry: Entry) -> bool {
        entry.flags & (CURRENT_INSERT_COUNT | MARKED_INSERT_COUNT) != 0
            && (entry.data[0] != NONE || entry.data[1] != NONE)
    }

    fn transition(&mut self, buffer: &mut AatBuffer<'_, '_>, entry: Entry) {
        let flags = entry.flags;
        let mark_location = buffer.out_len();
        if entry.data[1] != NONE {
            let count = usize::from(flags & MARKED_INSERT_COUNT);
            if !buffer.spend(count) {
                return;
            }
            let glyphs = self.list(entry.data[1], count);
            let end = buffer.out_len();
            if !buffer.move_to(self.mark)
                || !Self::insert(buffer, &glyphs, flags & MARKED_INSERT_BEFORE != 0)
                || !buffer.move_to(end + glyphs.len())
            {
                return;
            }
        }
        if flags & SET_MARK != 0 {
            self.mark = mark_location;
        }
        if entry.data[0] != NONE {
            let count = usize::from((flags & CURRENT_INSERT_COUNT) >> 5);
            if !buffer.spend(count) {
                return;
            }
            let glyphs = self.list(entry.data[0], count);
            let end = buffer.out_len();
            if !Self::insert(buffer, &glyphs, flags & CURRENT_INSERT_BEFORE != 0) {
                return;
            }
            // Without DontAdvance the inserted glyphs are passed over; with
            // it, the machine sees them next.
            let target = if flags & DONT_ADVANCE != 0 {
                end
            } else {
                end + glyphs.len()
            };
            buffer.move_to(target);
        }
    }
}
