//! Ligature subtables (type 2): glyphs pushed on a component stack are
//! replaced by a ligature found by summing their component values
//! (HarfBuzz's `LigatureSubtable`).

use crate::aat::buffer::AatBuffer;
use crate::aat::state::{Entry, Flavor, Machine, StateTable};
use crate::reader::peek;
use crate::types::GlyphId;

/// Pushes the current glyph on the component stack.
const SET_COMPONENT: u16 = 0x8000;
/// Processes the ligature actions (extended tables).
const PERFORM_ACTION: u16 = 0x2000;
/// The byte offset of the actions (obsolete tables, which act when it is
/// not zero).
const ACTION_OFFSET: u16 = 0x3FFF;
/// The last action of a list (it also stores).
const ACTION_LAST: u32 = 0x8000_0000;
/// Stores the ligature in place of the popped glyph.
const ACTION_STORE: u32 = 0x4000_0000;
/// The 30-bit signed offset added to the glyph id.
const ACTION_COMPONENT: u32 = 0x3FFF_FFFF;
/// Positions of components remembered (HarfBuzz's
/// `HB_MAX_CONTEXT_LENGTH`; older ones are overwritten).
const STACK: usize = 64;

/// The running state of a ligature subtable.
#[derive(Debug)]
pub(super) struct Ligature<'a> {
    table: StateTable<'a>,
    flavor: Flavor,
    /// Offsets (from the state table) of the action, component and
    /// ligature arrays.
    actions: usize,
    components: usize,
    ligatures: usize,
    match_length: usize,
    match_positions: [usize; STACK],
}

impl<'a> Ligature<'a> {
    /// The subtable whose state table is `table`.
    pub(super) fn new(table: StateTable<'a>, flavor: Flavor) -> Option<Self> {
        let data = table.data();
        let field = flavor.field();
        let at = table.header_len();
        let actions = flavor.read_field(data, at)?;
        let components = flavor.read_field(data, at + field)?;
        let ligatures = flavor.read_field(data, at + 2 * field)?;
        if actions == 0 || components == 0 || ligatures == 0 {
            return None;
        }
        Some(Self {
            table,
            flavor,
            actions,
            components,
            ligatures,
            match_length: 0,
            match_positions: [0; STACK],
        })
    }

    /// The element of the array of `size`-byte elements at `array` that
    /// `value` designates: the index itself in extended tables, a byte
    /// offset from the state table in obsolete ones (HarfBuzz's
    /// `offsetToIndex`).
    fn element(&self, value: i64, array: usize, size: usize) -> Option<usize> {
        let index = match self.flavor {
            Flavor::Extended => value,
            Flavor::Obsolete => (value - i64::try_from(array).ok()?) / i64::try_from(size).ok()?,
        };
        let index = usize::try_from(index).ok()?;
        array.checked_add(index.checked_mul(size)?)
    }

    /// The position of the component `slot` of the stack.
    const fn position(&self, slot: usize) -> usize {
        self.match_positions[slot % STACK]
    }

    /// Runs the ligature actions starting at `first`; `false` when the
    /// buffer gave out.
    fn perform(&mut self, buffer: &mut AatBuffer<'_, '_>, first: usize) -> bool {
        let data = self.table.data();
        let mut cursor = self.match_length;
        let mut action_at = first;
        let mut ligature_sum: u32 = 0;
        loop {
            if cursor == 0 {
                self.match_length = 0;
                return true;
            }
            cursor -= 1;
            if !buffer.move_to(self.position(cursor)) {
                return false;
            }
            let Some(action) = peek::<u32>(data, action_at) else {
                return true;
            };
            let mut offset = action & ACTION_COMPONENT;
            if offset & 0x2000_0000 != 0 {
                offset |= 0xC000_0000;
            }
            let component = u32::from(buffer.cur().glyph.0).wrapping_add(offset);
            let word = match self.flavor {
                Flavor::Extended => i64::from(component),
                Flavor::Obsolete => 2 * i64::from(component),
            };
            let Some(component) = self
                .element(word, self.components, 2)
                .and_then(|at| peek::<u16>(data, at))
            else {
                return true;
            };
            ligature_sum = ligature_sum.wrapping_add(u32::from(component));
            if action & (ACTION_STORE | ACTION_LAST) != 0 {
                let Some(ligature) = self
                    .element(i64::from(ligature_sum), self.ligatures, 2)
                    .and_then(|at| peek::<u16>(data, at))
                else {
                    return true;
                };
                if !buffer.replace_glyph(GlyphId(ligature)) {
                    return false;
                }
                let ligature_end = self.position(self.match_length - 1) + 1;
                while self.match_length - 1 > cursor {
                    self.match_length -= 1;
                    if !buffer.move_to(self.position(self.match_length)) || !buffer.delete_glyph() {
                        return false;
                    }
                }
                if !buffer.move_to(ligature_end) {
                    return false;
                }
                let out_len = buffer.out_len();
                buffer.merge_out_clusters(self.position(cursor), out_len);
            }
            action_at += 4;
            if action & ACTION_LAST != 0 {
                return true;
            }
        }
    }
}

impl Machine for Ligature<'_> {
    const WRITES_OUTPUT: bool = true;

    fn initiates(&self, entry: Entry) -> bool {
        entry.flags & SET_COMPONENT != 0
    }

    fn acts(&self, entry: Entry) -> bool {
        match self.flavor {
            Flavor::Extended => entry.flags & PERFORM_ACTION != 0,
            Flavor::Obsolete => entry.flags & ACTION_OFFSET != 0,
        }
    }

    fn transition(&mut self, buffer: &mut AatBuffer<'_, '_>, entry: Entry) {
        if entry.flags & SET_COMPONENT != 0 {
            let out_len = buffer.out_len();
            // Never the same position twice, as with DontAdvance.
            if self.match_length > 0 && self.position(self.match_length - 1) == out_len {
                self.match_length -= 1;
            }
            self.match_positions[self.match_length % STACK] = out_len;
            self.match_length += 1;
        }
        if !self.acts(entry) {
            return;
        }
        let end = buffer.out_len();
        if self.match_length == 0 || buffer.idx() >= buffer.len() {
            return;
        }
        let action = match self.flavor {
            Flavor::Extended => i64::from(entry.data[0]),
            Flavor::Obsolete => i64::from(entry.flags & ACTION_OFFSET),
        };
        let Some(first) = self.element(action, self.actions, 4) else {
            return;
        };
        if self.perform(buffer, first) {
            buffer.move_to(end);
        }
    }
}
