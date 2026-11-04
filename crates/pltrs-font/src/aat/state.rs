//! AAT state tables and the machine that runs them over the glyphs
//! (HarfBuzz's `StateTable` and `StateTableDriver`).
//!
//! A state table maps each glyph to a class, and each state and class to
//! an entry: the next state, flags and data telling the subtable what to
//! do. The extended tables (`morx`, `kerx`) number their states and give
//! 16-bit entry indices; the older ones (`kern`) give byte offsets, whose
//! first state need not be the start of the state array, so their states
//! may be negative.

use crate::aat::buffer::AatBuffer;
use crate::aat::lookup::{ClassTable, ClassWidth, DELETED_GLYPH, Lookup, ValueSize};
use crate::reader::peek;

/// The class of the end of the text.
pub(crate) const CLASS_END_OF_TEXT: u16 = 0;
/// The class of glyphs the class table does not cover.
const CLASS_OUT_OF_BOUNDS: u16 = 1;
/// The class of deleted glyphs.
const CLASS_DELETED_GLYPH: u16 = 2;
/// The state a run starts in.
const START_OF_TEXT: i32 = 0;
/// The entry flag every subtable uses to stay on the current glyph.
pub(crate) const DONT_ADVANCE: u16 = 0x4000;
/// Classes HarfBuzz filters with one bit page; machines with more classes
/// may start with any glyph their class table covers.
const FILTER_CLASSES: usize = 512;

/// Which generation of AAT tables a structure belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Flavor {
    /// `morx` and `kerx`: 32-bit header fields, lookup tables for classes,
    /// numbered states with 16-bit entries.
    Extended,
    /// `kern` (and `mort`): 16-bit header fields, byte class tables, byte
    /// offsets for states.
    Obsolete,
}

impl Flavor {
    /// The size of a header field (`HBUINT`).
    pub(crate) const fn field(self) -> usize {
        match self {
            Self::Extended => 4,
            Self::Obsolete => 2,
        }
    }

    /// The header field at `offset` of `data`.
    pub(crate) fn read_field(self, data: &[u8], offset: usize) -> Option<usize> {
        match self {
            Self::Extended => peek::<u32>(data, offset).and_then(|v| usize::try_from(v).ok()),
            Self::Obsolete => peek::<u16>(data, offset).map(usize::from),
        }
    }
}

/// One entry of a state table.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Entry {
    /// The next state, as stored (a byte offset in obsolete tables).
    pub(crate) new_state: u16,
    /// The subtable's flags.
    pub(crate) flags: u16,
    /// The subtable's per-entry data, as up to two 16-bit words.
    pub(crate) data: [u16; 2],
}

/// Where a state table finds the classes of glyphs.
#[derive(Clone, Copy, Debug)]
enum Classes<'a> {
    /// A lookup table (extended tables).
    Lookup(Lookup<'a>),
    /// A byte class table (obsolete tables).
    Table(ClassTable<'a>),
}

/// A state table, read where it lies.
#[derive(Clone, Copy, Debug)]
pub(crate) struct StateTable<'a> {
    /// The data from the state table header on, to which its offsets are
    /// relative.
    data: &'a [u8],
    flavor: Flavor,
    n_classes: usize,
    classes: Classes<'a>,
    state_array: usize,
    entry_table: usize,
    /// The number of 16-bit data words each entry carries.
    data_words: usize,
}

impl<'a> StateTable<'a> {
    /// The state table whose header starts `data`, with entries carrying
    /// `data_words` words; `None` when its header is incomplete or it has
    /// fewer than the four predefined classes.
    pub(crate) fn new(data: &'a [u8], flavor: Flavor, data_words: usize) -> Option<Self> {
        let field = flavor.field();
        let n_classes = flavor.read_field(data, 0)?;
        let class_table = data.get(flavor.read_field(data, field)?..)?;
        let state_array = flavor.read_field(data, 2 * field)?;
        let entry_table = flavor.read_field(data, 3 * field)?;
        if n_classes < 4 {
            return None;
        }
        let classes = match flavor {
            Flavor::Extended => Classes::Lookup(Lookup::new(class_table, ValueSize::U16)),
            Flavor::Obsolete => Classes::Table(ClassTable::new(class_table, ClassWidth::Narrow)),
        };
        Some(Self {
            data,
            flavor,
            n_classes,
            classes,
            state_array,
            entry_table,
            data_words,
        })
    }

    /// The size of the header, after which a subtable's own fields follow.
    pub(crate) const fn header_len(&self) -> usize {
        4 * self.flavor.field()
    }

    /// The data from the header on.
    pub(crate) const fn data(&self) -> &'a [u8] {
        self.data
    }

    /// The class of `glyph` (HarfBuzz's `get_class`).
    pub(crate) fn class(&self, glyph: u16, num_glyphs: u16) -> u16 {
        if glyph == DELETED_GLYPH {
            return CLASS_DELETED_GLYPH;
        }
        let class = match self.classes {
            Classes::Lookup(lookup) => lookup.value(glyph, num_glyphs).map(|value| value as u16),
            Classes::Table(table) => table.class(glyph),
        };
        class.unwrap_or(CLASS_OUT_OF_BOUNDS)
    }

    /// The entry for `class` in `state` (HarfBuzz's `get_entry`); a
    /// default entry where the table does not hold one.
    pub(crate) fn entry(&self, state: i32, class: u16) -> Entry {
        self.try_entry(state, class).unwrap_or_default()
    }

    fn try_entry(&self, state: i32, class: u16) -> Option<Entry> {
        let class = if usize::from(class) < self.n_classes {
            class
        } else {
            CLASS_OUT_OF_BOUNDS
        };
        let n_classes = i64::try_from(self.n_classes).ok()?;
        let cell = i64::from(state) * n_classes + i64::from(class);
        let index = match self.flavor {
            Flavor::Extended => {
                let offset = i64::try_from(self.state_array).ok()? + 2 * cell;
                usize::from(peek::<u16>(self.data, usize::try_from(offset).ok()?)?)
            }
            Flavor::Obsolete => {
                let offset = i64::try_from(self.state_array).ok()? + cell;
                usize::from(peek::<u8>(self.data, usize::try_from(offset).ok()?)?)
            }
        };
        let size = 4 + 2 * self.data_words;
        let at = self.entry_table.checked_add(index.checked_mul(size)?)?;
        let mut data = [0; 2];
        for (word, value) in data.iter_mut().enumerate().take(self.data_words) {
            *value = peek::<u16>(self.data, at + 4 + 2 * word)?;
        }
        Some(Entry {
            new_state: peek::<u16>(self.data, at)?,
            flags: peek::<u16>(self.data, at + 2)?,
            data,
        })
    }

    /// The state an entry leads to (HarfBuzz's `new_state`): its number
    /// in extended tables, else its byte offset made a row number, which
    /// may be negative.
    pub(crate) fn new_state(&self, raw: u16) -> i32 {
        match self.flavor {
            Flavor::Extended => i32::from(raw),
            Flavor::Obsolete => {
                let n_classes = i32::try_from(self.n_classes).unwrap_or(i32::MAX);
                let array = i32::try_from(self.state_array).unwrap_or(i32::MAX);
                (i32::from(raw) - array) / n_classes
            }
        }
    }

    /// The classes with which a glyph can make `machine` act from the
    /// start state (HarfBuzz's `collect_initial_glyphs`).
    pub(crate) fn start_classes(&self, machine: &impl Machine) -> StartClasses {
        if self.n_classes > FILTER_CLASSES {
            return StartClasses::Any;
        }
        let mut filter = [false; FILTER_CLASSES];
        for (class, starts) in filter.iter_mut().enumerate().take(self.n_classes) {
            let entry = self.entry(START_OF_TEXT, class as u16);
            *starts = self.new_state(entry.new_state) != START_OF_TEXT
                || machine.initiates(entry)
                || machine.acts(entry);
        }
        StartClasses::Filter(Box::new(filter))
    }

    /// Whether `glyph` belongs to the start classes `start` (HarfBuzz's
    /// glyph set of initial glyphs).
    pub(crate) fn starts_with(&self, start: &StartClasses, glyph: u16, num_glyphs: u16) -> bool {
        match start {
            StartClasses::Any => match self.classes {
                Classes::Lookup(lookup) => lookup.covers(glyph, num_glyphs),
                Classes::Table(table) => table
                    .class(glyph)
                    .is_some_and(|class| class != CLASS_OUT_OF_BOUNDS),
            },
            StartClasses::Filter(filter) => {
                let class = if glyph == DELETED_GLYPH {
                    Some(u32::from(CLASS_DELETED_GLYPH))
                } else {
                    match self.classes {
                        Classes::Lookup(lookup) => lookup.entry(glyph, num_glyphs),
                        Classes::Table(table) => table.class(glyph).map(u32::from),
                    }
                };
                class.is_some_and(|class| filter[class as usize % FILTER_CLASSES])
            }
        }
    }
}

/// The classes that can start a machine's work.
#[derive(Clone, Debug)]
pub(crate) enum StartClasses {
    /// Any class (the machine has too many to filter).
    Any,
    /// The classes marked.
    Filter(Box<[bool; FILTER_CLASSES]>),
}

/// A subtable whose state table drives it (HarfBuzz's driver context).
pub(crate) trait Machine {
    /// Whether the subtable writes its glyphs to an output (ligatures and
    /// insertions) rather than changing them in place.
    const WRITES_OUTPUT: bool;

    /// Whether `entry` starts an action (marks a glyph, pushes one).
    fn initiates(&self, entry: Entry) -> bool;

    /// Whether `entry` performs an action.
    fn acts(&self, entry: Entry) -> bool;

    /// Performs `entry` at the buffer's cursor.
    fn transition(&mut self, buffer: &mut AatBuffer<'_, '_>, entry: Entry);
}

/// The feature flags of a range of clusters (HarfBuzz's
/// `range_flags_t`): a subtable only acts on glyphs whose range enables
/// one of its flags.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RangeFlags {
    /// The flags.
    pub(crate) flags: u32,
    /// The first cluster of the range.
    pub(crate) first: usize,
    /// The last cluster of the range.
    pub(crate) last: usize,
}

/// The range of `ranges` holding `cluster`, searched from `from` as
/// HarfBuzz walks its ranges.
pub(crate) fn range_of(ranges: &[RangeFlags], from: usize, cluster: usize) -> usize {
    let mut range = from.min(ranges.len().saturating_sub(1));
    while range > 0 && cluster < ranges[range].first {
        range -= 1;
    }
    while range + 1 < ranges.len() && cluster > ranges[range].last {
        range += 1;
    }
    range
}

/// Runs `machine` with `table` over the buffer (HarfBuzz's
/// `StateTableDriver::drive`): glyph by glyph, the entry of the current
/// state and the glyph's class acts, then sets the next state; the end of
/// the text gets an entry too. Where the chain's flags differ by range,
/// glyphs whose range does not enable `subtable_flags` reset the machine.
pub(crate) fn drive<M: Machine>(
    table: &StateTable<'_>,
    machine: &mut M,
    buffer: &mut AatBuffer<'_, '_>,
    num_glyphs: u16,
    ranges: &[RangeFlags],
    subtable_flags: u32,
) {
    if M::WRITES_OUTPUT {
        buffer.start_output();
    } else {
        buffer.buffer.idx = 0;
    }
    let mut state = START_OF_TEXT;
    let mut last_range = (ranges.len() > 1).then_some(0);
    while !buffer.failed() {
        let at_end = buffer.idx() >= buffer.len();
        let class = if at_end {
            CLASS_END_OF_TEXT
        } else {
            table.class(buffer.cur().glyph.0, num_glyphs)
        };
        let entry = table.entry(state, class);
        let next_state = table.new_state(entry.new_state);
        if let Some(from) = last_range {
            let range = if at_end {
                from
            } else {
                range_of(ranges, from, buffer.cur().cluster)
            };
            last_range = Some(range);
            if ranges[range].flags & subtable_flags == 0 {
                if at_end {
                    break;
                }
                state = START_OF_TEXT;
                buffer.next_glyph();
                continue;
            }
        }
        machine.transition(buffer, entry);
        state = next_state;
        if buffer.idx() >= buffer.len() {
            break;
        }
        if entry.flags & DONT_ADVANCE == 0 || !buffer.buffer.spend_op() {
            buffer.next_glyph();
        }
    }
    if M::WRITES_OUTPUT {
        buffer.finish_output();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::buffer::{Buffer, CharProps, GlyphInfo};
    use crate::layout::gdef::Gdef;
    use crate::types::GlyphId;

    /// An extended state table with 4 classes plus class 4 (glyph 10),
    /// two states, entries: 0 stays, 1 goes to state 1 marking, 2 acts.
    fn extended_table() -> Vec<u8> {
        let mut data = Vec::new();
        for field in [5_u32, 16, 28, 48] {
            data.extend_from_slice(&field.to_be_bytes());
        }
        // Class lookup, format 8: glyph 10 -> class 4 (bytes 16..28).
        for word in [8_u16, 10, 1, 4, 0, 0] {
            data.extend_from_slice(&word.to_be_bytes());
        }
        // States (bytes 28..48): state 0 sends class 4 to entry 1, state 1
        // sends class 4 to entry 2.
        for word in [0_u16, 0, 0, 0, 1, 0, 0, 0, 0, 2] {
            data.extend_from_slice(&word.to_be_bytes());
        }
        // Entries (bytes 48..): (new state, flags, one data word).
        for word in [0_u16, 0, 0xFFFF, 1, 0x8000, 0xFFFF, 0, 0, 7] {
            data.extend_from_slice(&word.to_be_bytes());
        }
        data
    }

    struct Recorder {
        acted: Vec<(usize, u16)>,
    }

    impl Machine for Recorder {
        const WRITES_OUTPUT: bool = false;
        fn initiates(&self, entry: Entry) -> bool {
            entry.flags & 0x8000 != 0
        }
        fn acts(&self, entry: Entry) -> bool {
            entry.data[0] != 0xFFFF
        }
        fn transition(&mut self, buffer: &mut AatBuffer<'_, '_>, entry: Entry) {
            if self.acts(entry) {
                self.acted.push((buffer.idx(), entry.data[0]));
            }
        }
    }

    fn buffer(glyphs: &[u16]) -> Buffer {
        Buffer::new(
            glyphs
                .iter()
                .zip(0..)
                .map(|(&g, cluster)| GlyphInfo::new(GlyphId(g), cluster, CharProps::default(), 1))
                .collect(),
        )
    }

    #[test]
    fn entries_classes_and_start_glyphs() {
        let data = extended_table();
        let table = StateTable::new(&data, Flavor::Extended, 1).unwrap();
        assert_eq!(table.class(10, 20), 4);
        assert_eq!(table.class(11, 20), CLASS_OUT_OF_BOUNDS);
        assert_eq!(table.class(DELETED_GLYPH, 20), CLASS_DELETED_GLYPH);
        assert_eq!(table.entry(0, 4).new_state, 1);
        assert_eq!(table.entry(1, 4).data, [7, 0]);
        assert_eq!(table.entry(5, 4), Entry::default());
        let start = table.start_classes(&Recorder { acted: Vec::new() });
        assert!(table.starts_with(&start, 10, 20));
        assert!(!table.starts_with(&start, 11, 20));
        assert!(StateTable::new(&data[..12], Flavor::Extended, 1).is_none());
    }

    #[test]
    fn the_driver_follows_states() {
        let data = extended_table();
        let table = StateTable::new(&data, Flavor::Extended, 1).unwrap();
        let gdef = Gdef::default();
        let mut glyphs = buffer(&[10, 3, 10, 10]);
        let mut aat = AatBuffer::new(&mut glyphs, &gdef);
        let mut recorder = Recorder { acted: Vec::new() };
        drive(&table, &mut recorder, &mut aat, 20, &[], 1);
        // Glyph 3 is out of bounds (class 1), whose entry returns to the
        // start; the second 10 marks again and the third acts.
        assert_eq!(recorder.acted, [(3, 7)]);
        let ranges = [
            RangeFlags {
                flags: 1,
                first: 0,
                last: 1,
            },
            RangeFlags {
                flags: 0,
                first: 2,
                last: usize::MAX,
            },
        ];
        let mut recorder = Recorder { acted: Vec::new() };
        drive(&table, &mut recorder, &mut aat, 20, &ranges, 1);
        assert_eq!(recorder.acted, []);
        assert_eq!(range_of(&ranges, 0, 3), 1);
        assert_eq!(range_of(&ranges, 1, 0), 0);
    }

    #[test]
    fn obsolete_states_are_byte_offsets() {
        // Header (8 bytes), class table at 8 (glyph 5 -> class 4), states
        // at 16 (two rows of 5 bytes), entries at 26.
        let mut data = Vec::new();
        for field in [5_u16, 8, 16, 26] {
            data.extend_from_slice(&field.to_be_bytes());
        }
        data.extend_from_slice(&[0, 5, 0, 1, 4, 0, 0, 0]);
        data.extend_from_slice(&[0, 0, 0, 0, 1, 0, 0, 0, 0, 0]);
        for word in [16_u16, 0, 21, 0] {
            data.extend_from_slice(&word.to_be_bytes());
        }
        let table = StateTable::new(&data, Flavor::Obsolete, 0).unwrap();
        assert_eq!(table.class(5, 10), 4);
        assert_eq!(table.class(6, 10), CLASS_OUT_OF_BOUNDS);
        assert_eq!(table.new_state(table.entry(0, 4).new_state), 1);
        assert_eq!(table.new_state(16), 0);
        assert_eq!(table.new_state(11), -1);
        assert_eq!(table.header_len(), 8);
    }
}
