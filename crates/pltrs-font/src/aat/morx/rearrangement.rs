//! Rearrangement subtables (type 0): reorder up to two glyphs at each end
//! of a marked range (HarfBuzz's `RearrangementSubtable`).

use crate::aat::buffer::AatBuffer;
use crate::aat::state::{Entry, Machine};

/// Makes the current glyph the first of the range.
const MARK_FIRST: u16 = 0x8000;
/// Makes the current glyph the last of the range.
const MARK_LAST: u16 = 0x2000;
/// The rearrangement to perform.
const VERB: u16 = 0x000F;
/// The longest range rearranged (HarfBuzz's `HB_MAX_CONTEXT_LENGTH`).
const MAX_RANGE: usize = 64;

/// For each verb, how many glyphs move from the start side (high nibble)
/// and the end side (low nibble); 3 moves two and swaps them.
const VERBS: [u8; 16] = [
    0x00, // no change
    0x10, // Ax => xA
    0x01, // xD => Dx
    0x11, // AxD => DxA
    0x20, // ABx => xAB
    0x30, // ABx => xBA
    0x02, // xCD => CDx
    0x03, // xCD => DCx
    0x12, // AxCD => CDxA
    0x13, // AxCD => DCxA
    0x21, // ABxD => DxAB
    0x31, // ABxD => DxBA
    0x22, // ABxCD => CDxAB
    0x32, // ABxCD => CDxBA
    0x23, // ABxCD => DCxAB
    0x33, // ABxCD => DCxBA
];

/// The running state of a rearrangement subtable.
#[derive(Debug, Default)]
pub(super) struct Rearrangement {
    start: usize,
    end: usize,
}

impl Machine for Rearrangement {
    const WRITES_OUTPUT: bool = false;

    fn initiates(&self, entry: Entry) -> bool {
        entry.flags & MARK_FIRST != 0
    }

    fn acts(&self, entry: Entry) -> bool {
        entry.flags & VERB != 0
    }

    fn transition(&mut self, buffer: &mut AatBuffer<'_, '_>, entry: Entry) {
        let (idx, len) = (buffer.idx(), buffer.len());
        if entry.flags & MARK_FIRST != 0 {
            self.start = idx;
        }
        if entry.flags & MARK_LAST != 0 {
            self.end = (idx + 1).min(len);
        }
        let (start, end) = (self.start, self.end);
        if entry.flags & VERB == 0 || start >= end {
            return;
        }
        let map = VERBS[usize::from(entry.flags & VERB)];
        let left = usize::from(map >> 4).min(2);
        let right = usize::from(map & 0x0F).min(2);
        if end - start < left + right || end - start > MAX_RANGE {
            return;
        }
        buffer.merge_clusters(start, (idx + 1).min(len));
        buffer.merge_clusters(start, end);
        let info = &mut buffer.buffer.info;
        let moved_left: Vec<_> = info[start..start + left].to_vec();
        let moved_right: Vec<_> = info[end - right..end].to_vec();
        if left != right {
            info.copy_within(start + left..end - right, start + right);
        }
        info[start..start + right].copy_from_slice(&moved_right);
        info[end - left..end].copy_from_slice(&moved_left);
        if map >> 4 == 3 {
            info.swap(end - 1, end - 2);
        }
        if map & 0x0F == 3 {
            info.swap(start, start + 1);
        }
    }
}
