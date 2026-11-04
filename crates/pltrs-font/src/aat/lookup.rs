//! The tables AAT maps glyphs through (HarfBuzz's
//! `hb-aat-layout-common.hh`): lookup tables of the extended tables
//! (`morx`, `kerx`), in their six formats, and the class tables of the
//! older ones (`kern`'s state tables and class-based kerning).
//!
//! Tables are read where they lie; a value outside its table reads as
//! absent.

use std::cmp::Ordering;

use crate::reader::peek;

/// The glyph id AAT gives a deleted glyph, and the value that ends a
/// binary-searched array.
pub(crate) const DELETED_GLYPH: u16 = 0xFFFF;

/// The width of the values a lookup table holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ValueSize {
    /// 16-bit values (glyphs, classes, offsets).
    U16,
    /// 32-bit values (`kerx` format 6's long indices).
    U32,
}

impl ValueSize {
    /// The width in bytes.
    const fn bytes(self) -> usize {
        match self {
            Self::U16 => 2,
            Self::U32 => 4,
        }
    }

    /// The value at `offset` of `data`.
    fn read(self, data: &[u8], offset: usize) -> Option<u32> {
        match self {
            Self::U16 => peek::<u16>(data, offset).map(u32::from),
            Self::U32 => peek::<u32>(data, offset),
        }
    }
}

/// A binary-searchable array of fixed-size units (`VarSizedBinSearchArrayOf`):
/// a header giving the unit size and count, then the units, the last of
/// which may be a terminator (its first `termination_words` words all
/// `0xFFFF`) that is not searched.
#[derive(Clone, Copy, Debug)]
struct BinSearch<'a> {
    /// The data from the first unit on.
    units: &'a [u8],
    unit_size: usize,
    len: usize,
}

impl<'a> BinSearch<'a> {
    /// The array whose header starts at `offset` of `data`.
    fn new(data: &'a [u8], offset: usize, termination_words: usize) -> Option<Self> {
        let unit_size = usize::from(peek::<u16>(data, offset)?);
        let count = usize::from(peek::<u16>(data, offset + 2)?);
        let units = data.get(offset + 10..)?;
        let is_terminator = |index: usize| {
            (0..termination_words)
                .all(|word| peek::<u16>(units, index * unit_size + 2 * word) == Some(DELETED_GLYPH))
        };
        let len = if count > 0 && is_terminator(count - 1) {
            count - 1
        } else {
            count
        };
        Some(Self {
            units,
            unit_size,
            len,
        })
    }

    /// The offset in `units` of the unit `compare` finds, by HarfBuzz's
    /// binary search (`compare` tells where the key lies relative to a
    /// unit: `Less` before it).
    fn search(&self, compare: impl Fn(usize) -> Option<Ordering>) -> Option<usize> {
        let (mut min, mut max) = (0_isize, isize::try_from(self.len).ok()? - 1);
        while min <= max {
            let mid = isize::midpoint(min, max);
            let offset = usize::try_from(mid).ok()? * self.unit_size;
            match compare(offset)? {
                Ordering::Less => max = mid - 1,
                Ordering::Greater => min = mid + 1,
                Ordering::Equal => return Some(offset),
            }
        }
        None
    }

    /// The unit holding `glyph` in an array of `(last, first, ...)`
    /// segments.
    fn segment(&self, glyph: u16) -> Option<usize> {
        self.search(|unit| {
            let last = peek::<u16>(self.units, unit)?;
            let first = peek::<u16>(self.units, unit + 2)?;
            Some(if glyph < first {
                Ordering::Less
            } else if glyph <= last {
                Ordering::Equal
            } else {
                Ordering::Greater
            })
        })
    }
}

/// An AAT lookup table (`Lookup<T>`), from its format field on.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Lookup<'a> {
    data: &'a [u8],
    values: ValueSize,
}

impl<'a> Lookup<'a> {
    /// The lookup at the start of `data`, holding `values`.
    pub(crate) const fn new(data: &'a [u8], values: ValueSize) -> Self {
        Self { data, values }
    }

    /// The value the table gives `glyph`, in every format (format 10
    /// included), for a font of `num_glyphs` glyphs; `None` for a glyph
    /// it does not cover.
    pub(crate) fn entry(&self, glyph: u16, num_glyphs: u16) -> Option<u32> {
        let data = self.data;
        let width = self.values.bytes();
        match peek::<u16>(data, 0)? {
            0 => (glyph < num_glyphs)
                .then(|| self.values.read(data, 2 + usize::from(glyph) * width))
                .flatten(),
            2 => {
                let segments = BinSearch::new(data, 2, 2)?;
                let found = segments.segment(glyph)?;
                self.values.read(segments.units, found + 4)
            }
            4 => {
                let segments = BinSearch::new(data, 2, 2)?;
                let found = segments.segment(glyph)?;
                let first = peek::<u16>(segments.units, found + 2)?;
                let values = usize::from(peek::<u16>(segments.units, found + 4)?);
                self.values
                    .read(data, values + usize::from(glyph - first) * width)
            }
            6 => {
                let entries = BinSearch::new(data, 2, 1)?;
                let found =
                    entries.search(|unit| Some(glyph.cmp(&peek::<u16>(entries.units, unit)?)))?;
                self.values.read(entries.units, found + 2)
            }
            8 => {
                let first = peek::<u16>(data, 2)?;
                let count = peek::<u16>(data, 4)?;
                let index = glyph.checked_sub(first).filter(|&index| index < count)?;
                self.values.read(data, 6 + usize::from(index) * width)
            }
            10 => {
                let size = usize::from(peek::<u16>(data, 2)?);
                let first = peek::<u16>(data, 4)?;
                let count = peek::<u16>(data, 6)?;
                let index = glyph.checked_sub(first).filter(|&index| index < count)?;
                if size > 4 {
                    return None;
                }
                let start = 8 + usize::from(index) * size;
                let bytes = data.get(start..start + size)?;
                Some(bytes.iter().fold(0, |v, &b| (v << 8) | u32::from(b)))
            }
            _ => None,
        }
    }

    /// HarfBuzz's `get_value`: the value of [`entry`](Self::entry), but
    /// never one of format 10, whose values are not addressable.
    pub(crate) fn value(&self, glyph: u16, num_glyphs: u16) -> Option<u32> {
        if peek::<u16>(self.data, 0) == Some(10) {
            return None;
        }
        self.entry(glyph, num_glyphs)
    }

    /// HarfBuzz's `get_value_or_null`: the value, format 10's included, or
    /// zero.
    pub(crate) fn value_or_zero(&self, glyph: u16, num_glyphs: u16) -> u32 {
        self.entry(glyph, num_glyphs).unwrap_or(0)
    }

    /// Whether HarfBuzz's `collect_glyphs` would collect `glyph`: whether
    /// the table covers it (the deleted glyph never counts).
    pub(crate) fn covers(&self, glyph: u16, num_glyphs: u16) -> bool {
        glyph != DELETED_GLYPH && self.entry(glyph, num_glyphs).is_some()
    }
}

/// A class table of the older AAT tables (`ClassTable<T>`): the classes of
/// a range of glyphs, as bytes (state tables) or 16-bit values (`kern`
/// format 2).
#[derive(Clone, Copy, Debug)]
pub(crate) struct ClassTable<'a> {
    data: &'a [u8],
    values: ClassWidth,
}

/// The width of a [`ClassTable`]'s classes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ClassWidth {
    /// One byte per glyph.
    Narrow,
    /// Two bytes per glyph.
    Wide,
}

impl<'a> ClassTable<'a> {
    /// The table at the start of `data`.
    pub(crate) const fn new(data: &'a [u8], values: ClassWidth) -> Self {
        Self { data, values }
    }

    /// The class of `glyph`, or `None` outside the table.
    pub(crate) fn class(&self, glyph: u16) -> Option<u16> {
        let first = peek::<u16>(self.data, 0)?;
        let count = peek::<u16>(self.data, 2)?;
        let index = usize::from(glyph.checked_sub(first).filter(|&i| i < count)?);
        match self.values {
            ClassWidth::Narrow => peek::<u8>(self.data, 4 + index).map(u16::from),
            ClassWidth::Wide => peek::<u16>(self.data, 4 + 2 * index),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(values: &[u16]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_be_bytes()).collect()
    }

    #[test]
    fn simple_arrays_and_trimmed_arrays() {
        let format0 = words(&[0, 7, 8, 9]);
        let lookup = Lookup::new(&format0, ValueSize::U16);
        assert_eq!(lookup.value(1, 3), Some(8));
        assert_eq!(lookup.value(3, 3), None);
        assert!(lookup.covers(2, 3) && !lookup.covers(DELETED_GLYPH, 3));
        let format8 = words(&[8, 10, 2, 5, 6]);
        let lookup = Lookup::new(&format8, ValueSize::U16);
        assert_eq!(
            (lookup.value(10, 0), lookup.value(11, 0)),
            (Some(5), Some(6))
        );
        assert_eq!((lookup.value(9, 0), lookup.value(12, 0)), (None, None));
    }

    #[test]
    fn segments_singles_and_terminators() {
        // Two segments plus a terminator: 3..=5 -> 1, 10..=12 -> 2.
        let format2 = words(&[2, 6, 3, 0, 0, 0, 5, 3, 1, 12, 10, 2, 0xFFFF, 0xFFFF, 0]);
        let lookup = Lookup::new(&format2, ValueSize::U16);
        assert_eq!(lookup.value(4, 0), Some(1));
        assert_eq!(lookup.value(12, 0), Some(2));
        assert_eq!(lookup.value(7, 0), None);
        assert_eq!(lookup.value(DELETED_GLYPH, 0), None);
        // A segment array: 20..=21 -> values at byte 24.
        let format4 = words(&[4, 6, 1, 0, 0, 0, 21, 20, 24, 0, 0, 0, 40, 41]);
        let lookup = Lookup::new(&format4, ValueSize::U16);
        assert_eq!(
            (lookup.value(20, 0), lookup.value(21, 0)),
            (Some(40), Some(41))
        );
        let format6 = words(&[6, 4, 2, 0, 0, 0, 5, 50, 9, 90]);
        let lookup = Lookup::new(&format6, ValueSize::U16);
        assert_eq!((lookup.value(9, 0), lookup.value(6, 0)), (Some(90), None));
    }

    #[test]
    fn format_ten_only_answers_by_value() {
        let mut format10 = words(&[10, 3, 4, 2]);
        format10.extend_from_slice(&[0, 1, 2, 3, 4, 5]);
        let lookup = Lookup::new(&format10, ValueSize::U16);
        assert_eq!(lookup.value(4, 0), None);
        assert_eq!(lookup.value_or_zero(5, 0), 0x0003_0405);
        assert_eq!(lookup.value_or_zero(6, 0), 0);
        assert!(lookup.covers(4, 0));
    }

    #[test]
    fn long_values_and_class_tables() {
        let format8: Vec<u8> = [8_u16, 1, 1]
            .iter()
            .flat_map(|v| v.to_be_bytes())
            .chain(70_000_u32.to_be_bytes())
            .collect();
        assert_eq!(
            Lookup::new(&format8, ValueSize::U32).value(1, 0),
            Some(70_000)
        );
        let narrow = [0, 4, 0, 2, 7, 9];
        let table = ClassTable::new(&narrow, ClassWidth::Narrow);
        assert_eq!(
            (table.class(4), table.class(5), table.class(6)),
            (Some(7), Some(9), None)
        );
        assert_eq!(table.class(3), None);
        let wide = words(&[2, 1, 300]);
        assert_eq!(ClassTable::new(&wide, ClassWidth::Wide).class(2), Some(300));
        assert_eq!(Lookup::new(&[], ValueSize::U16).value(0, 1), None);
    }
}
