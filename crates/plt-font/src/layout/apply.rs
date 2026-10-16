//! Lookup application machinery shared by `GSUB` and `GPOS`: glyph
//! skipping per lookup flags, sequence matching and nested lookups.

use crate::face::Face;
use crate::layout::buffer::{Buffer, GlyphInfo, props};
use crate::layout::common::{LayoutTable, Lookup, lookup_flag};
use crate::layout::{gpos, gsub};
use crate::scale::Scaler;

/// Maximum nesting of contextual lookups (HarfBuzz's limit).
const MAX_NESTING: u8 = 64;
/// Maximum length of a matched input sequence (HarfBuzz's limit).
const MAX_CONTEXT_LENGTH: usize = 64;

/// Which layout table lookups come from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum TableKind {
    Gsub,
    Gpos,
}

/// Whether a glyph may be skipped while matching.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Skip {
    No,
    Yes,
    Maybe,
}

/// A nested lookup to apply at a position of a matched sequence.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SequenceLookup {
    pub(crate) sequence_index: u16,
    pub(crate) lookup_index: u16,
}

/// State for applying lookups of one table to a buffer.
pub(crate) struct Applier<'f, 'a> {
    pub(crate) face: &'f Face<'a>,
    pub(crate) scaler: &'f Scaler,
    pub(crate) buffer: &'f mut Buffer,
    table: LayoutTable<'a>,
    kind: TableKind,
    flags: u16,
    mark_set: Option<u16>,
    nesting: u8,
}

impl<'f, 'a> Applier<'f, 'a> {
    pub(crate) fn new(
        face: &'f Face<'a>,
        table: LayoutTable<'a>,
        kind: TableKind,
        scaler: &'f Scaler,
        buffer: &'f mut Buffer,
    ) -> Self {
        Self {
            face,
            scaler,
            buffer,
            table,
            kind,
            flags: 0,
            mark_set: None,
            nesting: 0,
        }
    }

    /// Current glyph.
    pub(crate) fn cur(&self) -> Option<&GlyphInfo> {
        self.buffer.info.get(self.buffer.idx)
    }

    /// HarfBuzz's `check_glyph_property`: whether the current lookup
    /// processes `info` at all.
    fn check_glyph_property(&self, info: &GlyphInfo) -> bool {
        let class = info.props & props::CLASS_MASK;
        if class & self.flags & props::CLASS_MASK != 0 {
            return false;
        }
        if class & props::MARK == 0 {
            return true;
        }
        if self.flags & lookup_flag::USE_MARK_FILTERING_SET != 0 {
            return self
                .mark_set
                .is_some_and(|set| self.face.gdef().is_in_mark_set(set, info.glyph));
        }
        let wanted = self.flags & lookup_flag::MARK_ATTACHMENT_TYPE;
        wanted == 0 || wanted == info.props & lookup_flag::MARK_ATTACHMENT_TYPE
    }

    fn may_skip(&self, info: &GlyphInfo) -> Skip {
        if !self.check_glyph_property(info) {
            Skip::Yes
        } else if info.chars.default_ignorable {
            Skip::Maybe
        } else {
            Skip::No
        }
    }

    /// Finds the next (`forward`) or previous glyph from `from` that the
    /// lookup does not skip. With `matcher`, glyphs that may be skipped are
    /// only taken when they match; without, the first non-skippable glyph
    /// is returned. Returns `None` when the sequence cannot continue.
    fn step(
        &self,
        from: usize,
        forward: bool,
        matcher: Option<&dyn Fn(&GlyphInfo) -> bool>,
    ) -> Option<usize> {
        let mut index = from;
        loop {
            index = if forward {
                index + 1
            } else {
                index.checked_sub(1)?
            };
            let info = self.buffer.info.get(index)?;
            let skip = self.may_skip(info);
            if skip == Skip::Yes {
                continue;
            }
            let matched = matcher.map(|m| m(info));
            match (matched, skip) {
                (Some(true), _) | (None, Skip::No) => return Some(index),
                (_, Skip::No) => return None,
                _ => {}
            }
        }
    }

    /// Next glyph after `from` that the lookup does not skip.
    pub(crate) fn next(&self, from: usize) -> Option<usize> {
        self.step(from, true, None)
    }

    /// Previous glyph before `from` that the lookup does not skip.
    pub(crate) fn prev(&self, from: usize) -> Option<usize> {
        self.step(from, false, None)
    }

    /// Previous glyph before `from` under the current lookup's mark
    /// filtering, but without skipping any glyph class: the candidate a
    /// mark attaches to in mark-to-mark positioning.
    pub(crate) fn prev_ignoring_classes(&mut self, from: usize) -> Option<usize> {
        let saved = self.flags;
        self.flags &= !(lookup_flag::IGNORE_BASE_GLYPHS
            | lookup_flag::IGNORE_LIGATURES
            | lookup_flag::IGNORE_MARKS);
        let found = self.prev(from);
        self.flags = saved;
        found
    }

    /// Matches `count` glyphs after the current one with `matcher(i, info)`
    /// (`i` counts from 1). Returns the buffer indices of the whole input
    /// sequence, starting with the current glyph.
    pub(crate) fn match_input(
        &self,
        count: usize,
        matcher: &dyn Fn(usize, &GlyphInfo) -> bool,
    ) -> Option<Vec<usize>> {
        if count + 1 > MAX_CONTEXT_LENGTH {
            return None;
        }
        let mut positions = Vec::with_capacity(count + 1);
        positions.push(self.buffer.idx);
        for i in 1..=count {
            let last = *positions.last()?;
            positions.push(self.step(last, true, Some(&|info: &GlyphInfo| matcher(i, info)))?);
        }
        Some(positions)
    }

    /// Matches `count` glyphs before `start` (closest first).
    pub(crate) fn match_backtrack(
        &self,
        start: usize,
        count: usize,
        matcher: &dyn Fn(usize, &GlyphInfo) -> bool,
    ) -> bool {
        let mut index = start;
        (0..count).all(|i| {
            match self.step(index, false, Some(&|info: &GlyphInfo| matcher(i, info))) {
                Some(found) => {
                    index = found;
                    true
                }
                None => false,
            }
        })
    }

    /// Matches `count` glyphs after `end` (the last input glyph).
    pub(crate) fn match_lookahead(
        &self,
        end: usize,
        count: usize,
        matcher: &dyn Fn(usize, &GlyphInfo) -> bool,
    ) -> bool {
        let mut index = end;
        (0..count).all(|i| {
            match self.step(index, true, Some(&|info: &GlyphInfo| matcher(i, info))) {
                Some(found) => {
                    index = found;
                    true
                }
                None => false,
            }
        })
    }

    /// Applies one lookup to the whole buffer (forward, or backward for
    /// reverse chaining substitutions).
    pub(crate) fn apply_lookup(&mut self, lookup_index: u16) {
        let Some(lookup) = self.table.lookup(lookup_index) else {
            return;
        };
        self.flags = lookup.flags;
        self.mark_set = lookup.mark_filtering_set;
        if self.kind == TableKind::Gsub && lookup.kind == gsub::REVERSE_CHAIN {
            for index in (0..self.buffer.len()).rev() {
                self.buffer.idx = index;
                let info = self.buffer.info[index];
                if self.check_glyph_property(&info) {
                    self.apply_subtables(&lookup);
                }
            }
            return;
        }
        self.buffer.idx = 0;
        while self.buffer.idx < self.buffer.len() {
            let info = self.buffer.info[self.buffer.idx];
            if !(self.check_glyph_property(&info) && self.apply_subtables(&lookup)) {
                self.buffer.idx += 1;
            }
        }
    }

    /// Tries each subtable at the current position; the first one that
    /// applies wins.
    fn apply_subtables(&mut self, lookup: &Lookup<'a>) -> bool {
        lookup.subtables().any(|data| match self.kind {
            TableKind::Gsub => gsub::apply(self, lookup.kind, data),
            TableKind::Gpos => gpos::apply(self, lookup.kind, data),
        })
    }

    /// Applies a nested lookup once at the current position.
    fn apply_nested(&mut self, lookup_index: u16) -> bool {
        if self.nesting >= MAX_NESTING {
            return false;
        }
        let Some(lookup) = self.table.lookup(lookup_index) else {
            return false;
        };
        let saved = (self.flags, self.mark_set);
        self.flags = lookup.flags;
        self.mark_set = lookup.mark_filtering_set;
        self.nesting += 1;
        let applied = self.apply_subtables(&lookup);
        self.nesting -= 1;
        (self.flags, self.mark_set) = saved;
        applied
    }

    /// HarfBuzz's `apply_lookup`: runs the sequence lookups of a matched
    /// context, tracking how substitutions change the matched positions,
    /// then moves the cursor past the sequence.
    pub(crate) fn apply_sequence_lookups(
        &mut self,
        mut positions: Vec<usize>,
        records: &[SequenceLookup],
    ) {
        let mut count = positions.len();
        let mut end = positions.last().map_or(self.buffer.idx, |&last| last + 1) as isize;
        for record in records {
            let idx = usize::from(record.sequence_index);
            if idx >= count {
                continue;
            }
            let original_len = self.buffer.len() as isize;
            self.buffer.idx = positions[idx];
            if !self.apply_nested(record.lookup_index) {
                continue;
            }
            let mut delta = self.buffer.len() as isize - original_len;
            if delta == 0 {
                continue;
            }
            end += delta;
            let here = positions[idx] as isize;
            if end < here {
                delta += here - end;
                end = here;
            }
            let mut next = idx + 1;
            if delta > 0 {
                if count + delta as usize > MAX_CONTEXT_LENGTH {
                    break;
                }
            } else {
                delta = delta.max(next as isize - count as isize);
                next = (next as isize - delta) as usize;
            }
            // Shift the positions after `next` by `delta` slots.
            let tail: Vec<usize> = positions[next.min(count)..count].to_vec();
            let new_next = (next as isize + delta) as usize;
            count = (count as isize + delta) as usize;
            positions.resize(count.max(positions.len()), 0);
            positions[new_next..new_next + tail.len()].copy_from_slice(&tail);
            for j in idx + 1..new_next {
                positions[j] = positions[j - 1] + 1;
            }
            for position in &mut positions[new_next..count] {
                *position = (*position as isize + delta) as usize;
            }
            positions.truncate(count);
        }
        self.buffer.idx = end.max(0) as usize;
    }
}
