//! Glyph metamorphosis: the `morx` table, or the older `mort` (HarfBuzz's
//! `hb-aat-layout-morx-table.hh`).
//!
//! The table holds chains of subtables. Each chain starts from its
//! default flags, which features switch; a subtable runs when the flags
//! of some range of the text include its own. Subtables are applied in
//! order, each in the direction its coverage asks for:
//!
//! * [`rearrangement`]: reordering at the ends of a marked range;
//! * [`contextual`]: substitution of the marked and the current glyph;
//! * [`ligature`]: ligatures of glyphs pushed on a stack;
//! * noncontextual: one glyph for another, by a lookup table;
//! * [`insertion`]: glyph lists inserted at the marked and current glyph.

mod contextual;
mod insertion;
mod ligature;
mod rearrangement;

use crate::aat::buffer::AatBuffer;
use crate::aat::features::{
    ActiveFeatures, LANGUAGE_TAG_TYPE, LETTER_CASE, LOWER_CASE, LOWER_CASE_SMALL_CAPS, SMALL_CAPS,
    language_matches, ltag_language,
};
use crate::aat::lookup::{Lookup, ValueSize};
use crate::aat::state::{Flavor, Machine, RangeFlags, StateTable, drive, range_of};
use crate::face::Face;
use crate::reader::peek;
use crate::types::{Direction, GlyphId, Tag};
use contextual::Contextual;
use insertion::Insertion;
use ligature::Ligature;
use rearrangement::Rearrangement;

/// The subtable only applies to vertical text (else to horizontal).
const VERTICAL: u32 = 0x80;
/// The subtable processes glyphs from the end.
const BACKWARDS: u32 = 0x40;
/// The subtable applies to text of any direction.
const ALL_DIRECTIONS: u32 = 0x20;
/// The subtable processes glyphs in logical rather than layout order.
const LOGICAL: u32 = 0x10;

/// A `morx` or `mort` table.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Morx<'a> {
    data: &'a [u8],
    flavor: Flavor,
}

/// One chain of a [`Morx`] table.
#[derive(Clone, Copy, Debug)]
struct Chain<'a> {
    /// The chain's bytes.
    data: &'a [u8],
    flavor: Flavor,
}

/// One subtable of a [`Chain`].
#[derive(Clone, Copy, Debug)]
struct Subtable<'a> {
    /// The subtable's body, after its header.
    body: &'a [u8],
    kind: u8,
    coverage: u32,
    flags: u32,
}

impl<'a> Morx<'a> {
    /// The `morx` table of `face`, else its `mort` table, when one has
    /// data (HarfBuzz's `hb_aat_layout_has_substitution`).
    pub(crate) fn of(face: &Face<'a>) -> Option<Self> {
        [(b"morx", Flavor::Extended), (b"mort", Flavor::Obsolete)]
            .into_iter()
            .find_map(|(tag, flavor)| {
                let data = face.table(Tag::new(tag))?;
                (peek::<u16>(data, 0)? != 0).then_some(Self { data, flavor })
            })
    }

    /// The chains, as far as the table holds them.
    fn chains(&self) -> impl Iterator<Item = Chain<'a>> + use<'a> {
        let (data, flavor) = (self.data, self.flavor);
        let count = peek::<u32>(data, 4).unwrap_or(0);
        let mut at = 8;
        (0..count).map_while(move |_| {
            let length = usize::try_from(peek::<u32>(data, at + 4)?).ok()?;
            let chain = data.get(at..at.checked_add(length)?)?;
            at += length;
            Some(Chain {
                data: chain,
                flavor,
            })
        })
    }

    /// The flags of every chain for the features `active` and the text's
    /// `language` (HarfBuzz's `compile_flags`).
    pub(crate) fn chain_flags(
        &self,
        active: &ActiveFeatures,
        language: Option<&str>,
        ltag: Option<&[u8]>,
    ) -> Vec<u32> {
        self.chains()
            .map(|chain| chain.flags(active, language, ltag))
            .collect()
    }

    /// Runs every chain over the buffer, with the flags `chain_flags`
    /// gives each chain by range of clusters.
    pub(crate) fn apply(
        &self,
        buffer: &mut AatBuffer<'_, '_>,
        chain_flags: &[Vec<RangeFlags>],
        num_glyphs: u16,
        direction: Direction,
    ) {
        for (chain, ranges) in self.chains().zip(chain_flags) {
            chain.apply(buffer, ranges, num_glyphs, direction);
            if buffer.failed() {
                return;
            }
        }
    }
}

impl<'a> Chain<'a> {
    /// The number of features and the offset of the first.
    fn features(&self) -> (usize, usize) {
        let field = self.flavor.field();
        let count = self.flavor.read_field(self.data, 8).unwrap_or(0);
        (count, 8 + 2 * field)
    }

    /// The flags of the chain for the features `active`.
    fn flags(&self, active: &ActiveFeatures, language: Option<&str>, ltag: Option<&[u8]>) -> u32 {
        let mut flags = peek::<u32>(self.data, 0).unwrap_or(0);
        let (count, first) = self.features();
        for at in (0..count).map(|i| first + 12 * i) {
            let (Some(kind), Some(setting), Some(enable), Some(disable)) = (
                peek::<u16>(self.data, at),
                peek::<u16>(self.data, at + 2),
                peek::<u32>(self.data, at + 4),
                peek::<u32>(self.data, at + 8),
            ) else {
                break;
            };
            let small_caps = kind == LETTER_CASE
                && setting == SMALL_CAPS
                && active.contains(LOWER_CASE, LOWER_CASE_SMALL_CAPS);
            let language_tag = kind == LANGUAGE_TAG_TYPE
                && setting != 0
                && ltag
                    .and_then(|ltag| ltag_language(ltag, u32::from(setting) - 1))
                    .zip(language)
                    .is_some_and(|(tag, language)| language_matches(&tag, language));
            if active.contains(kind, setting) || small_caps || language_tag {
                flags &= disable;
                flags |= enable;
            }
        }
        flags
    }

    /// The subtables, as far as the chain holds them.
    fn subtables(&self) -> impl Iterator<Item = Subtable<'a>> + use<'a> {
        let (data, flavor) = (self.data, self.flavor);
        let field = flavor.field();
        let (features, first) = self.features();
        let count = flavor.read_field(data, 8 + field).unwrap_or(0);
        let mut at = first + 12 * features;
        let header = 2 * field + 4;
        (0..count).map_while(move |_| {
            let length = flavor.read_field(data, at)?;
            let coverage = match flavor {
                Flavor::Extended => peek::<u32>(data, at + field)?,
                Flavor::Obsolete => u32::from(peek::<u16>(data, at + field)?),
            };
            let flags = peek::<u32>(data, at + 2 * field)?;
            if length < header {
                return None;
            }
            let body = data.get(at + header..at.checked_add(length)?)?;
            at += length;
            Some(Subtable {
                body,
                kind: (coverage & 0xFF) as u8,
                coverage: coverage >> (8 * field - 8),
                flags,
            })
        })
    }

    /// Applies the subtables enabled in `ranges` (HarfBuzz's
    /// `Chain::apply`), restoring the logical order at the end.
    fn apply(
        &self,
        buffer: &mut AatBuffer<'_, '_>,
        ranges: &[RangeFlags],
        num_glyphs: u16,
        direction: Direction,
    ) {
        for subtable in self.subtables() {
            if !ranges.iter().any(|range| subtable.flags & range.flags != 0) {
                continue;
            }
            // Horizontal text: vertical-only subtables never apply.
            if subtable.coverage & ALL_DIRECTIONS == 0 && subtable.coverage & VERTICAL != 0 {
                continue;
            }
            let backwards = subtable.coverage & BACKWARDS != 0;
            let reverse = if subtable.coverage & LOGICAL != 0 {
                backwards
            } else {
                backwards == direction.is_forward()
            };
            let run = Run {
                ranges,
                flags: subtable.flags,
                num_glyphs,
                reverse,
            };
            subtable.apply(self.flavor, buffer, &run);
            if buffer.failed() {
                break;
            }
        }
        if buffer.is_reversed() {
            buffer.reverse();
        }
    }
}

/// How a subtable is run: the chain's ranges, the subtable's flags, the
/// face's glyph count and the order to process glyphs in.
#[derive(Clone, Copy, Debug)]
struct Run<'r> {
    ranges: &'r [RangeFlags],
    flags: u32,
    num_glyphs: u16,
    reverse: bool,
}

impl Run<'_> {
    /// Puts the glyphs in the order to process them.
    fn orient(&self, buffer: &mut AatBuffer<'_, '_>) {
        if self.reverse != buffer.is_reversed() {
            buffer.reverse();
        }
    }

    /// Runs the state machine `machine` of `table`, if a glyph of the
    /// buffer can start it.
    fn drive<M: Machine>(
        &self,
        table: &StateTable<'_>,
        machine: &mut M,
        buffer: &mut AatBuffer<'_, '_>,
    ) {
        let start = table.start_classes(machine);
        if !buffer.intersects(|glyph| table.starts_with(&start, glyph, self.num_glyphs)) {
            return;
        }
        self.orient(buffer);
        drive(
            table,
            machine,
            buffer,
            self.num_glyphs,
            self.ranges,
            self.flags,
        );
    }
}

impl Subtable<'_> {
    /// Applies the subtable (HarfBuzz's `ChainSubtable::dispatch`); types
    /// it does not know do nothing.
    fn apply(&self, flavor: Flavor, buffer: &mut AatBuffer<'_, '_>, run: &Run<'_>) {
        let table = |words| StateTable::new(self.body, flavor, words);
        match self.kind {
            0 => {
                if let Some(table) = table(0) {
                    run.drive(&table, &mut Rearrangement::default(), buffer);
                }
            }
            1 => {
                if let Some((table, mut machine)) =
                    table(2).and_then(|t| Some((t, Contextual::new(t, flavor, run.num_glyphs)?)))
                {
                    run.drive(&table, &mut machine, buffer);
                }
            }
            2 => {
                let words = usize::from(flavor == Flavor::Extended);
                if let Some((table, mut machine)) =
                    table(words).and_then(|t| Some((t, Ligature::new(t, flavor)?)))
                {
                    run.drive(&table, &mut machine, buffer);
                }
            }
            4 => self.substitute(buffer, run),
            5 => {
                if let Some((table, mut machine)) =
                    table(2).and_then(|t| Some((t, Insertion::new(t, flavor)?)))
                {
                    run.drive(&table, &mut machine, buffer);
                }
            }
            _ => {}
        }
    }

    /// A noncontextual subtable: every glyph in an enabled range that the
    /// lookup table covers is replaced (HarfBuzz's
    /// `NoncontextualSubtable::apply`).
    fn substitute(&self, buffer: &mut AatBuffer<'_, '_>, run: &Run<'_>) {
        let lookup = Lookup::new(self.body, ValueSize::U16);
        if !buffer.intersects(|glyph| lookup.covers(glyph, run.num_glyphs)) {
            return;
        }
        run.orient(buffer);
        let mut last_range = (run.ranges.len() > 1).then_some(0);
        for index in 0..buffer.len() {
            let info = buffer.buffer.info[index];
            if let Some(from) = last_range {
                let range = range_of(run.ranges, from, info.cluster);
                last_range = Some(range);
                if run.ranges[range].flags & run.flags == 0 {
                    continue;
                }
            }
            if let Some(glyph) = lookup.value(info.glyph.0, run.num_glyphs) {
                buffer.replace_in_place(index, GlyphId(glyph as u16));
            }
        }
    }
}
