//! The glyph sets HarfBuzz's collect functions fill, its set digest, and
//! coverage and class definition traversal with HarfBuzz's semantics.

use std::collections::BTreeSet;

use crate::reader::{Array, Reader, peek, tail};
use crate::types::{GlyphId, Tag};

/// A glyph set as HarfBuzz's collect functions fill it, failing where
/// `hb_set_t` does.
pub(crate) struct GlyphSink<'s>(pub(crate) &'s mut BTreeSet<u32>);

impl GlyphSink<'_> {
    pub(crate) fn add(&mut self, glyph: u32) {
        self.0.insert(glyph);
    }

    /// `hb_set_t::add_range`: fails on an inverted range.
    pub(crate) fn add_range(&mut self, first: u32, last: u32) -> bool {
        if first > last {
            return false;
        }
        self.0.extend(first..=last);
        true
    }

    /// `hb_set_t::add_sorted_array`: adds glyphs until one is smaller than
    /// its predecessor, then fails.
    pub(crate) fn add_sorted(&mut self, glyphs: impl Iterator<Item = u16>) -> bool {
        let mut last = 0;
        for glyph in glyphs {
            if glyph < last {
                return false;
            }
            last = glyph;
            self.0.insert(u32::from(glyph));
        }
        true
    }
}

/// HarfBuzz's `hb_set_digest_t`: three bit masks over shifted glyph ids,
/// a quick and coarse membership filter.
#[derive(Clone, Copy, Default)]
pub(crate) struct Digest {
    masks: [u64; 3],
}

impl Digest {
    const SHIFTS: [u32; 3] = [4, 0, 6];

    fn bit(glyph: u32, shift: u32) -> u64 {
        1 << ((glyph >> shift) & 63)
    }

    pub(crate) fn add(&mut self, glyph: u32) {
        for (mask, shift) in self.masks.iter_mut().zip(Self::SHIFTS) {
            *mask |= Self::bit(glyph, shift);
        }
    }

    pub(crate) fn add_range(&mut self, first: u32, last: u32) {
        if self.masks.iter().all(|&mask| mask == u64::MAX) {
            return;
        }
        for (mask, shift) in self.masks.iter_mut().zip(Self::SHIFTS) {
            if (last >> shift).wrapping_sub(first >> shift) >= 63 {
                *mask = u64::MAX;
            } else {
                let (low, high) = (Self::bit(first, shift), Self::bit(last, shift));
                *mask |= high
                    .wrapping_add(high.wrapping_sub(low))
                    .wrapping_sub(u64::from(high < low));
            }
        }
    }

    pub(crate) fn may_have(&self, glyph: u32) -> bool {
        self.masks
            .iter()
            .zip(Self::SHIFTS)
            .all(|(&mask, shift)| mask & Self::bit(glyph, shift) != 0)
    }
}

/// A coverage table read the way HarfBuzz walks it.
#[derive(Clone, Copy)]
pub(crate) enum CoverageTable<'a> {
    Glyphs(Array<'a, u16>),
    /// `(first, last, start coverage index)` records.
    Ranges(&'a [u8], usize),
    /// Unknown format or unreadable: HarfBuzz's empty `Null` coverage.
    Empty,
}

impl<'a> CoverageTable<'a> {
    /// The coverage table at `offset` in `parent`.
    pub(crate) fn at(parent: &'a [u8], offset: u16, tag: Tag) -> Self {
        let Ok(data) = tail(parent, usize::from(offset), tag) else {
            return Self::Empty;
        };
        let mut r = Reader::new(data, tag);
        match (r.u16(), r.u16()) {
            (Ok(1), Ok(count)) => r
                .array(usize::from(count))
                .map_or(Self::Empty, Self::Glyphs),
            (Ok(2), Ok(count)) => {
                let len = usize::from(count) * 6;
                data.get(4..4 + len).map_or(Self::Empty, |ranges| {
                    Self::Ranges(ranges, usize::from(count))
                })
            }
            _ => Self::Empty,
        }
    }

    fn range(ranges: &[u8], index: usize) -> (u16, u16, u16) {
        let field = |at: usize| peek::<u16>(ranges, index * 6 + at).unwrap_or(0);
        (field(0), field(2), field(4))
    }

    /// The covered glyphs in HarfBuzz's iteration order (`iter_t`): an
    /// inverted first range yields nothing, a later inverted range its
    /// first glyph, and iteration stops at a range whose coverage index
    /// does not follow the previous range's.
    pub(crate) fn glyphs(&self) -> Vec<u16> {
        match *self {
            Self::Glyphs(glyphs) => glyphs.iter().collect(),
            Self::Ranges(ranges, count) => {
                let mut out = Vec::new();
                if count == 0 {
                    return out;
                }
                let (first, last, _) = Self::range(ranges, 0);
                if first > last {
                    return out;
                }
                let (mut index, mut glyph, mut coverage) = (0, u32::from(first), 0u32);
                loop {
                    out.push(glyph as u16);
                    let (_, last, _) = Self::range(ranges, index);
                    if glyph < u32::from(last) {
                        coverage += 1;
                        glyph += 1;
                        continue;
                    }
                    index += 1;
                    if index == count {
                        return out;
                    }
                    let (first, _, value) = Self::range(ranges, index);
                    if u32::from(value) != coverage + 1 {
                        return out;
                    }
                    (glyph, coverage) = (u32::from(first), u32::from(value));
                }
            }
            Self::Empty => Vec::new(),
        }
    }

    /// `Coverage::collect_coverage` into a sink.
    pub(crate) fn collect(&self, sink: &mut GlyphSink<'_>) -> bool {
        match *self {
            Self::Glyphs(glyphs) => sink.add_sorted(glyphs.iter()),
            Self::Ranges(ranges, count) => (0..count).all(|index| {
                let (first, last, _) = Self::range(ranges, index);
                sink.add_range(u32::from(first), u32::from(last))
            }),
            Self::Empty => true,
        }
    }

    /// `Coverage::collect_coverage` into a set digest.
    pub(crate) fn digest(&self, digest: &mut Digest) {
        match *self {
            Self::Glyphs(glyphs) => glyphs.iter().for_each(|glyph| digest.add(u32::from(glyph))),
            Self::Ranges(ranges, count) => {
                for index in 0..count {
                    let (first, last, _) = Self::range(ranges, index);
                    digest.add_range(u32::from(first), u32::from(last));
                }
            }
            Self::Empty => {}
        }
    }

    /// The coverage index of `glyph`, if covered (HarfBuzz's
    /// `get_coverage`, a binary search).
    pub(crate) fn index(&self, glyph: GlyphId) -> Option<usize> {
        match *self {
            Self::Glyphs(glyphs) => glyphs
                .harfbuzz_search_by(|g| g.cmp(&glyph.0))
                .map(|(index, _)| index),
            Self::Ranges(ranges, count) => {
                let (mut low, mut high) = (0, count);
                while low < high {
                    let mid = low + (high - 1 - low) / 2;
                    let (first, last, value) = Self::range(ranges, mid);
                    if glyph.0 < first {
                        high = mid;
                    } else if glyph.0 <= last {
                        return Some(usize::from(value) + usize::from(glyph.0 - first));
                    } else {
                        low = mid + 1;
                    }
                }
                None
            }
            Self::Empty => None,
        }
    }

    /// Whether `glyph` is covered.
    pub(crate) fn contains(&self, glyph: GlyphId) -> bool {
        self.index(glyph).is_some()
    }
}

/// A class definition table read the way HarfBuzz's collect functions do.
#[derive(Clone, Copy)]
pub(crate) enum ClassTable<'a> {
    Array {
        start: u16,
        classes: Array<'a, u16>,
    },
    /// `(first, last, class)` records.
    Ranges(Array<'a, u16>),
    Empty,
}

impl<'a> ClassTable<'a> {
    /// The class definition at `offset` in `parent` (`Empty` for a null
    /// offset).
    pub(crate) fn at(parent: &'a [u8], offset: u16, tag: Tag) -> Self {
        if offset == 0 {
            return Self::Empty;
        }
        let Ok(data) = tail(parent, usize::from(offset), tag) else {
            return Self::Empty;
        };
        let mut r = Reader::new(data, tag);
        match r.u16() {
            Ok(1) => match (r.u16(), r.u16()) {
                (Ok(start), Ok(count)) => r
                    .array(usize::from(count))
                    .map_or(Self::Empty, |classes| Self::Array { start, classes }),
                _ => Self::Empty,
            },
            Ok(2) => r
                .u16()
                .and_then(|count| r.array(usize::from(count) * 3))
                .map_or(Self::Empty, Self::Ranges),
            _ => Self::Empty,
        }
    }

    fn ranges(values: Array<'a, u16>) -> impl Iterator<Item = (u16, u16, u16)> + 'a {
        (0..values.len() / 3).map(move |i| {
            let field = |at: usize| values.get(i * 3 + at).unwrap_or(0);
            (field(0), field(1), field(2))
        })
    }

    /// `ClassDef::collect_coverage`: the glyphs of non-zero classes, where
    /// format 1 also takes the glyph ending each run (HarfBuzz's inclusive
    /// ranges).
    pub(crate) fn collect_coverage(&self, sink: &mut GlyphSink<'_>) -> bool {
        match *self {
            Self::Array { start, classes } => {
                let start = u32::from(start);
                let count = classes.len() as u32;
                let mut run = 0;
                for (i, class) in (0u32..).zip(classes.iter()) {
                    if class != 0 {
                        continue;
                    }
                    if run != i && !sink.add_range(start + run, start + i) {
                        return false;
                    }
                    run = i + 1;
                }
                run == count || sink.add_range(start + run, start + count)
            }
            Self::Ranges(values) => Self::ranges(values)
                .filter(|&(_, _, class)| class != 0)
                .all(|(first, last, _)| sink.add_range(u32::from(first), u32::from(last))),
            Self::Empty => true,
        }
    }

    /// `ClassDef::collect_class`: the glyphs listed with class `class`.
    pub(crate) fn collect_class(&self, sink: &mut GlyphSink<'_>, class: u16) {
        match *self {
            Self::Array { start, classes } => {
                for (i, value) in (0u32..).zip(classes.iter()) {
                    if value == class {
                        sink.add(u32::from(start) + i);
                    }
                }
            }
            Self::Ranges(values) => {
                for (first, last, value) in Self::ranges(values) {
                    if value == class && !sink.add_range(u32::from(first), u32::from(last)) {
                        return;
                    }
                }
            }
            Self::Empty => {}
        }
    }

    /// The class of `glyph` (0 when unlisted).
    pub(crate) fn class(&self, glyph: GlyphId) -> u16 {
        match *self {
            Self::Array { start, classes } => glyph
                .0
                .checked_sub(start)
                .and_then(|i| classes.get(usize::from(i)))
                .unwrap_or(0),
            Self::Ranges(values) => {
                let count = values.len() / 3;
                let (mut low, mut high) = (0, count);
                while low < high {
                    let mid = low + (high - 1 - low) / 2;
                    let field = |at: usize| values.get(mid * 3 + at).unwrap_or(0);
                    if glyph.0 < field(0) {
                        high = mid;
                    } else if glyph.0 <= field(1) {
                        return field(2);
                    } else {
                        low = mid + 1;
                    }
                }
                0
            }
            Self::Empty => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::common::tests::{coverage_format1, ranges};

    const TAG: Tag = Tag::new(b"GSUB");

    #[test]
    fn digests_admit_what_was_added() {
        let mut digest = Digest::default();
        digest.add(70);
        digest.add_range(200, 210);
        assert!(digest.may_have(70) && digest.may_have(205));
        assert!(!digest.may_have(71));
        digest.add_range(0, 5000);
        assert!(digest.may_have(4321));
    }

    #[test]
    fn coverage_iteration_stops_at_broken_ranges() {
        let data = ranges(2, &[(5, 6, 0), (9, 9, 2), (20, 21, 7), (30, 30, 5)]);
        let coverage = CoverageTable::at(&data, 0, TAG);
        assert_eq!(coverage.glyphs(), [5, 6, 9]);
        assert!(coverage.contains(GlyphId(20)));
        let mut set = BTreeSet::new();
        assert!(coverage.collect(&mut GlyphSink(&mut set)));
        assert_eq!(set.len(), 6);
        let unsorted = coverage_format1(&[3, 9, 4]);
        let coverage = CoverageTable::at(&unsorted, 0, TAG);
        let mut set = BTreeSet::new();
        assert!(!coverage.collect(&mut GlyphSink(&mut set)));
        assert_eq!(set.into_iter().collect::<Vec<_>>(), [3, 9]);
    }

    #[test]
    fn class_coverage_takes_run_ends() {
        // A null offset is no class definition: tables start at offset 2.
        let padded = |table: &[u8]| [&[0, 0], table].concat();
        assert!(matches!(ClassTable::at(&[], 0, TAG), ClassTable::Empty));
        // Classes of glyphs 10..=14: 1, 1, 0, 2, 0.
        let data = padded(&[0, 1, 0, 10, 0, 5, 0, 1, 0, 1, 0, 0, 0, 2, 0, 0]);
        let classes = ClassTable::at(&data, 2, TAG);
        let collected = |classes: &ClassTable<'_>| {
            let mut set = BTreeSet::new();
            classes.collect_coverage(&mut GlyphSink(&mut set));
            set.into_iter().collect::<Vec<_>>()
        };
        assert_eq!(collected(&classes), [10, 11, 12, 13, 14]);
        assert_eq!(classes.class(GlyphId(13)), 2);
        let mut set = BTreeSet::new();
        classes.collect_class(&mut GlyphSink(&mut set), 1);
        assert_eq!(set.into_iter().collect::<Vec<_>>(), [10, 11]);
        let data = padded(&ranges(2, &[(3, 4, 1), (8, 9, 0)]));
        let classes = ClassTable::at(&data, 2, TAG);
        assert_eq!(collected(&classes), [3, 4]);
        assert_eq!(classes.class(GlyphId(9)), 0);
        assert_eq!(classes.class(GlyphId(4)), 1);
    }
}
