//! Contextual kerning (format 1): a state machine pushes glyphs on a
//! stack, and actions pop them, each with a kerning value (HarfBuzz's
//! `KerxSubTableFormat1`).

use super::KernContext;
use crate::aat::buffer::AatBuffer;
use crate::aat::state::{Entry, Flavor, Machine, StartClasses, StateTable, drive};
use crate::layout::buffer::AttachKind;
use crate::reader::peek;

/// Pushes the current glyph on the stack.
const PUSH: u16 = 0x8000;
/// Clears the stack (extended tables).
const RESET: u16 = 0x2000;
/// The byte offset of the values to pop with (older tables, which act
/// when it is not zero).
const OFFSET: u16 = 0x3FFF;
/// The entry data meaning "no action" (extended tables).
const NONE: u16 = 0xFFFF;
/// The cross-stream value that cancels the glyph's attachment.
const RESET_CROSS_STREAM: i32 = -0x8000;
/// The depth of the stack.
const STACK: usize = 8;

/// A format 1 subtable.
#[derive(Clone, Copy, Debug)]
pub(super) struct ContextualKerning<'a> {
    /// The state table's data, from which offsets count.
    data: &'a [u8],
    flavor: Flavor,
    /// Offset (from the state table) of the kerning values.
    actions: usize,
    tuple_count: u32,
    cross_stream: bool,
}

impl<'a> ContextualKerning<'a> {
    /// The subtable whose state table is `table`, with its values at
    /// `actions`.
    pub(super) fn new(
        table: StateTable<'a>,
        flavor: Flavor,
        actions: usize,
        tuple_count: u32,
        cross_stream: bool,
    ) -> Self {
        Self {
            data: table.data(),
            flavor,
            actions,
            tuple_count,
            cross_stream,
        }
    }

    /// Whether `glyph` can start the machine.
    pub(super) fn can_start(&self, table: &StateTable<'_>, glyph: u16, num_glyphs: u16) -> bool {
        let start: StartClasses = table.start_classes(&Run {
            kerning: *self,
            context: None,
            stack: [0; STACK],
            depth: 0,
        });
        table.starts_with(&start, glyph, num_glyphs)
    }

    /// Runs the machine over the buffer.
    pub(super) fn run(
        &self,
        table: &StateTable<'_>,
        buffer: &mut AatBuffer<'_, '_>,
        context: &KernContext<'_>,
    ) {
        let mut run = Run {
            kerning: *self,
            context: Some(context),
            stack: [0; STACK],
            depth: 0,
        };
        drive(table, &mut run, buffer, context.num_glyphs, &[], 0);
    }

    /// Whether `entry` pops kerning values.
    fn acts(&self, entry: Entry) -> bool {
        match self.flavor {
            Flavor::Extended => entry.data[0] != NONE,
            Flavor::Obsolete => entry.flags & OFFSET != 0,
        }
    }

    /// Where the values `entry` names start (HarfBuzz's
    /// `byteOffsetToIndex`).
    fn values(&self, entry: Entry) -> Option<usize> {
        let index = match self.flavor {
            Flavor::Extended => usize::from(entry.data[0]) / 2,
            Flavor::Obsolete => usize::from(entry.flags & OFFSET).checked_sub(self.actions)? / 2,
        };
        self.actions.checked_add(2 * index)
    }
}

/// A run of a format 1 subtable: its stack, and what kerning needs.
struct Run<'a, 'c> {
    kerning: ContextualKerning<'a>,
    /// The kerning context (none while only the start classes are asked).
    context: Option<&'c KernContext<'c>>,
    stack: [usize; STACK],
    depth: usize,
}

impl Machine for Run<'_, '_> {
    const WRITES_OUTPUT: bool = false;

    fn initiates(&self, entry: Entry) -> bool {
        entry.flags & PUSH != 0
    }

    fn acts(&self, entry: Entry) -> bool {
        self.kerning.acts(entry)
    }

    fn transition(&mut self, buffer: &mut AatBuffer<'_, '_>, entry: Entry) {
        let Some(context) = self.context else {
            return;
        };
        let kerning = self.kerning;
        if kerning.flavor == Flavor::Extended && entry.flags & RESET != 0 {
            self.depth = 0;
        }
        if entry.flags & PUSH != 0 {
            if self.depth < STACK {
                self.stack[self.depth] = buffer.idx();
                self.depth += 1;
            } else {
                self.depth = 0;
            }
        }
        if !kerning.acts(entry) || self.depth == 0 {
            return;
        }
        let stride = 2 * usize::try_from(kerning.tuple_count.max(1)).unwrap_or(1);
        let Some(first) = kerning.values(entry) else {
            self.depth = 0;
            return;
        };
        // Every value the stack may pop must be there.
        if kerning.data.len() < first + self.depth * stride {
            self.depth = 0;
            return;
        }
        let mut at = first;
        let mut last = false;
        while !last && self.depth > 0 {
            self.depth -= 1;
            let index = self.stack[self.depth];
            let value = peek::<i16>(kerning.data, at).map_or(0, i32::from);
            at += stride;
            if index >= buffer.len() {
                continue;
            }
            last = value & 1 != 0;
            let value = value & !1;
            let mask = buffer.buffer.info[index].mask;
            let pos = &mut buffer.buffer.pos[index];
            if kerning.cross_stream {
                if value == RESET_CROSS_STREAM {
                    pos.attach_kind = AttachKind::None;
                    pos.attach_chain = 0;
                    pos.y_offset = 0;
                } else if pos.attach_kind != AttachKind::None {
                    pos.y_offset += context.scaler.y(value);
                }
            } else if mask & context.kern_mask != 0 {
                let scaled = context.scaler.x(value);
                pos.x_advance += scaled;
                pos.x_offset += scaled;
            }
        }
    }
}
