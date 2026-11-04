//! Contextual substitution subtables (type 1): replace the marked glyph
//! and the current one through lookup tables the state machine picks
//! (HarfBuzz's `ContextualSubtable`).

use crate::aat::buffer::AatBuffer;
use crate::aat::lookup::{Lookup, ValueSize};
use crate::aat::state::{Entry, Flavor, Machine, StateTable};
use crate::reader::peek;
use crate::types::GlyphId;

/// Makes the current glyph the marked one.
const SET_MARK: u16 = 0x8000;
/// The entry data meaning "no substitution".
const NONE: u16 = 0xFFFF;

/// The running state of a contextual subtable.
#[derive(Debug)]
pub(super) struct Contextual<'a> {
    table: StateTable<'a>,
    flavor: Flavor,
    /// Offset (from the state table) of the substitution tables.
    substitutions: usize,
    num_glyphs: u16,
    mark_set: bool,
    mark: usize,
}

impl<'a> Contextual<'a> {
    /// The subtable whose state table is `table`, of a font with
    /// `num_glyphs` glyphs.
    pub(super) fn new(table: StateTable<'a>, flavor: Flavor, num_glyphs: u16) -> Option<Self> {
        Some(Self {
            substitutions: flavor.read_field(table.data(), table.header_len())?,
            table,
            flavor,
            num_glyphs,
            mark_set: false,
            mark: 0,
        })
    }

    /// The glyph `glyph` becomes through the substitution `index` names.
    fn substitute(&self, index: u16, glyph: GlyphId) -> Option<GlyphId> {
        let data = self.table.data();
        match self.flavor {
            Flavor::Extended => {
                if index == NONE {
                    return None;
                }
                let list = data.get(self.substitutions..)?;
                let offset = peek::<u32>(list, 4 * usize::from(index))?;
                let lookup =
                    Lookup::new(list.get(usize::try_from(offset).ok()?..)?, ValueSize::U16);
                lookup
                    .value(glyph.0, self.num_glyphs)
                    .map(|value| GlyphId(value as u16))
            }
            Flavor::Obsolete => {
                // A word offset from the state table, made an index into
                // the substitution array (HarfBuzz's `wordOffsetToIndex`).
                let byte = 2 * (usize::from(index) + usize::from(glyph.0));
                let element = byte.checked_sub(self.substitutions)? / 2;
                let replacement = peek::<u16>(data, self.substitutions + 2 * element)?;
                (replacement != 0).then_some(GlyphId(replacement))
            }
        }
    }
}

impl Machine for Contextual<'_> {
    const WRITES_OUTPUT: bool = false;

    fn initiates(&self, entry: Entry) -> bool {
        entry.flags & SET_MARK != 0
    }

    fn acts(&self, entry: Entry) -> bool {
        entry.data[0] != NONE || entry.data[1] != NONE
    }

    fn transition(&mut self, buffer: &mut AatBuffer<'_, '_>, entry: Entry) {
        let (idx, len) = (buffer.idx(), buffer.len());
        // CoreText substitutes nothing at the end of the text unless a
        // glyph was marked.
        if idx == len && !self.mark_set {
            return;
        }
        if let Some(glyph) = buffer
            .buffer
            .info
            .get(self.mark)
            .and_then(|info| self.substitute(entry.data[0], info.glyph))
        {
            buffer.replace_in_place(self.mark, glyph);
        }
        let current = idx.min(len.saturating_sub(1));
        if let Some(glyph) = buffer
            .buffer
            .info
            .get(current)
            .and_then(|info| self.substitute(entry.data[1], info.glyph))
        {
            buffer.replace_in_place(current, glyph);
        }
        if entry.flags & SET_MARK != 0 {
            self.mark_set = true;
            self.mark = idx;
        }
    }
}
