//! Kerning between pairs of glyphs: listed pairs (format 0), class
//! tables into a value array (format 2), byte classes indexing a value
//! list (`kern` format 3) and index lookups into a matrix (`kerx`
//! format 6), applied by HarfBuzz's kern machine (`hb_kern_machine_t`).

use std::cmp::Ordering;
use std::collections::HashSet;

use super::KernContext;
use crate::aat::lookup::{ClassTable, ClassWidth, Lookup, ValueSize};
use crate::aat::state::Flavor;
use crate::layout::buffer::{Buffer, GlyphInfo};
use crate::reader::peek;

/// A class table's class for glyphs it does not cover (class tables of
/// the older form do not count such glyphs as theirs).
const CLASS_OUT_OF_BOUNDS: u16 = 1;

/// Where a format 2 subtable finds the classes of glyphs.
#[derive(Clone, Copy, Debug)]
enum Classes<'a> {
    /// A lookup table (`kerx`).
    Lookup(Lookup<'a>),
    /// A class table of 16-bit values (`kern`).
    Table(ClassTable<'a>),
}

impl Classes<'_> {
    /// The class of `glyph`, zero where the table has none.
    fn class(&self, glyph: u16, num_glyphs: u16) -> u32 {
        match self {
            Self::Lookup(lookup) => lookup.value(glyph, num_glyphs).unwrap_or(0),
            Self::Table(table) => table.class(glyph).map_or(0, u32::from),
        }
    }

    /// Whether the table counts `glyph` as its own.
    fn covers(&self, glyph: u16, num_glyphs: u16) -> bool {
        match self {
            Self::Lookup(lookup) => lookup.covers(glyph, num_glyphs),
            Self::Table(table) => table.class(glyph).is_some_and(|c| c != CLASS_OUT_OF_BOUNDS),
        }
    }
}

/// The kinds of pair subtables.
#[derive(Debug)]
enum Pairs<'a> {
    /// Format 0: sorted `(left, right, value)` records.
    List {
        records: &'a [u8],
        count: usize,
        lefts: HashSet<u16>,
    },
    /// Format 2: left and right classes add up to an array position.
    Classes {
        left: Classes<'a>,
        right: Classes<'a>,
        array: usize,
    },
    /// `kern` format 3: byte classes index a list of values.
    Indexed { glyph_count: u16 },
    /// `kerx` format 6: row and column indices into a matrix.
    Matrix {
        long: bool,
        rows: Lookup<'a>,
        columns: Lookup<'a>,
        array: usize,
        vector: usize,
    },
}

/// A pair subtable.
#[derive(Debug)]
pub(super) struct PairTable<'a> {
    /// The subtable from its header on.
    data: &'a [u8],
    /// Where the format's fields start.
    body: usize,
    flavor: Flavor,
    tuple_count: u32,
    pairs: Pairs<'a>,
}

impl<'a> PairTable<'a> {
    /// The pair subtable of `format` starting `data`, whose header is
    /// `header_len` bytes.
    pub(super) fn new(
        data: &'a [u8],
        header_len: usize,
        format: u8,
        flavor: Flavor,
        tuple_count: u32,
    ) -> Option<Self> {
        let field = flavor.field();
        let at = header_len;
        let pairs = match format {
            0 => {
                let count = flavor.read_field(data, at)?;
                let records = data.get(at + 4 * field..)?;
                let lefts = (0..count)
                    .map_while(|i| peek::<u16>(records, 6 * i))
                    .collect();
                Pairs::List {
                    records,
                    count,
                    lefts,
                }
            }
            2 => {
                let table = |offset: usize| -> Option<Classes<'a>> {
                    let data = data.get(offset..)?;
                    Some(match flavor {
                        Flavor::Extended => Classes::Lookup(Lookup::new(data, ValueSize::U16)),
                        Flavor::Obsolete => Classes::Table(ClassTable::new(data, ClassWidth::Wide)),
                    })
                };
                Pairs::Classes {
                    left: table(flavor.read_field(data, at + field)?)?,
                    right: table(flavor.read_field(data, at + 2 * field)?)?,
                    array: flavor.read_field(data, at + 3 * field)?,
                }
            }
            3 => Pairs::Indexed {
                glyph_count: peek::<u16>(data, at)?,
            },
            6 => {
                let flags = peek::<u32>(data, at)?;
                let long = flags & 1 != 0;
                let values = if long { ValueSize::U32 } else { ValueSize::U16 };
                let lookup = |offset: u32| -> Option<Lookup<'a>> {
                    Some(Lookup::new(
                        data.get(usize::try_from(offset).ok()?..)?,
                        values,
                    ))
                };
                Pairs::Matrix {
                    long,
                    rows: lookup(peek::<u32>(data, at + 8)?)?,
                    columns: lookup(peek::<u32>(data, at + 12)?)?,
                    array: usize::try_from(peek::<u32>(data, at + 16)?).ok()?,
                    vector: usize::try_from(peek::<u32>(data, at + 20)?).ok()?,
                }
            }
            _ => return None,
        };
        Some(Self {
            data,
            body: at,
            flavor,
            tuple_count,
            pairs,
        })
    }

    /// Whether `glyph` may start a kerned pair (HarfBuzz's first set).
    pub(super) fn has_left(&self, glyph: u16, num_glyphs: u16) -> bool {
        match &self.pairs {
            Pairs::List { lefts, .. } => lefts.contains(&glyph),
            Pairs::Classes { left, .. } => left.covers(glyph, num_glyphs),
            Pairs::Indexed { glyph_count } => *glyph_count != 0 && glyph < num_glyphs,
            Pairs::Matrix { rows, .. } => rows.covers(glyph, num_glyphs),
        }
    }

    /// Whether `glyph` may end a kerned pair (HarfBuzz's second set).
    fn has_right(&self, glyph: u16, num_glyphs: u16) -> bool {
        match &self.pairs {
            Pairs::List { .. } => true,
            Pairs::Classes { right, .. } => right.covers(glyph, num_glyphs),
            Pairs::Indexed { glyph_count } => *glyph_count != 0 && glyph < num_glyphs,
            Pairs::Matrix { columns, .. } => columns.covers(glyph, num_glyphs),
        }
    }

    /// The kerning between `left` and `right`, in font units.
    pub(super) fn kerning(&self, left: u16, right: u16, num_glyphs: u16) -> i32 {
        if !self.has_left(left, num_glyphs) || !self.has_right(right, num_glyphs) {
            return 0;
        }
        self.value(left, right, num_glyphs).unwrap_or(0)
    }

    fn value(&self, left: u16, right: u16, num_glyphs: u16) -> Option<i32> {
        let data = self.data;
        match &self.pairs {
            Pairs::List { records, count, .. } => {
                let (mut min, mut max) = (0_isize, isize::try_from(*count).ok()? - 1);
                while min <= max {
                    let mid = isize::midpoint(min, max);
                    let at = 6 * usize::try_from(mid).ok()?;
                    let pair = (peek::<u16>(records, at)?, peek::<u16>(records, at + 2)?);
                    match (left, right).cmp(&pair) {
                        Ordering::Less => max = mid - 1,
                        Ordering::Greater => min = mid + 1,
                        Ordering::Equal => {
                            let value = i32::from(peek::<i16>(records, at + 4)?);
                            return self.tuple(value, 0);
                        }
                    }
                }
                Some(0)
            }
            Pairs::Classes {
                left: lefts,
                right: rights,
                array,
            } => {
                let position = lefts
                    .class(left, num_glyphs)
                    .wrapping_add(rights.class(right, num_glyphs));
                let index = match self.flavor {
                    Flavor::Extended => usize::try_from(position).ok()?,
                    Flavor::Obsolete => (usize::try_from(position).ok()?.checked_sub(*array)?) / 2,
                };
                let value = i32::from(peek::<i16>(data, array.checked_add(2 * index)?)?);
                self.tuple(value, 0)
            }
            Pairs::Indexed { glyph_count } => {
                let at = self.body;
                let glyph_count = usize::from(*glyph_count);
                let value_count = usize::from(peek::<u8>(data, at + 2)?);
                let left_count = peek::<u8>(data, at + 3)?;
                let right_count = peek::<u8>(data, at + 4)?;
                let values = at + 6;
                let left_classes = values + 2 * value_count;
                let right_classes = left_classes + glyph_count;
                let indices = right_classes + glyph_count;
                let class_of = |table: usize, glyph: u16| {
                    if usize::from(glyph) < glyph_count {
                        peek::<u8>(data, table + usize::from(glyph)).unwrap_or(0)
                    } else {
                        0
                    }
                };
                let (left_class, right_class) =
                    (class_of(left_classes, left), class_of(right_classes, right));
                if left_class >= left_count || right_class >= right_count {
                    return Some(0);
                }
                let cell =
                    usize::from(left_class) * usize::from(right_count) + usize::from(right_class);
                let index = usize::from(peek::<u8>(data, indices + cell).unwrap_or(0));
                if index >= value_count {
                    return Some(0);
                }
                Some(i32::from(peek::<i16>(data, values + 2 * index)?))
            }
            Pairs::Matrix {
                long,
                rows,
                columns,
                array,
                vector,
            } => {
                let row = rows.value_or_zero(left, num_glyphs);
                let column = columns.value_or_zero(right, num_glyphs);
                let cell = usize::try_from(row.checked_add(column)?).ok()?;
                let value = if *long {
                    peek::<i32>(data, array.checked_add(cell.checked_mul(4)?)?)?
                } else {
                    i32::from(peek::<i16>(data, array.checked_add(cell.checked_mul(2)?)?)?)
                };
                self.tuple(value, *vector)
            }
        }
    }

    /// HarfBuzz's `kerxTupleKern`: with variation tuples, `value` is the
    /// offset (from `base` of the subtable) of the value of the first.
    fn tuple(&self, value: i32, base: usize) -> Option<i32> {
        if self.tuple_count == 0 {
            return Some(value);
        }
        let offset = usize::try_from(value.cast_unsigned()).ok()?;
        Some(peek::<i16>(self.data, base.checked_add(offset)?).map_or(0, i32::from))
    }
}

/// Glyphs the kern machine passes over: marks and default ignorables.
fn passed_over(info: &GlyphInfo) -> bool {
    info.is_mark() || info.is_default_ignorable()
}

/// HarfBuzz's kern machine: each glyph carrying the `kern` mask is kerned
/// with the next glyph that is not passed over, if that one carries the
/// mask too; the value `kerning` gives is split between the two advances
/// (or, across the text, offsets the second glyph).
pub(super) fn kern_pairs(
    buffer: &mut Buffer,
    context: &KernContext<'_>,
    cross_stream: bool,
    kerning: impl Fn(u16, u16) -> i32,
) {
    let count = buffer.len();
    let mut idx = 0;
    while idx < count {
        if buffer.info[idx].mask & context.kern_mask == 0 {
            idx += 1;
            continue;
        }
        let next = (idx + 1..count).find(|&j| !passed_over(&buffer.info[j]));
        let Some(j) = next.filter(|&j| buffer.info[j].mask & context.kern_mask != 0) else {
            idx += 1;
            continue;
        };
        let kern = kerning(buffer.info[idx].glyph.0, buffer.info[j].glyph.0);
        if kern != 0 {
            let kern = context.scaler.x(kern);
            if cross_stream {
                buffer.pos[j].y_offset = kern;
            } else {
                let first = kern >> 1;
                let second = kern - first;
                buffer.pos[idx].x_advance += first;
                buffer.pos[j].x_advance += second;
                buffer.pos[j].x_offset += second;
            }
        }
        idx = j;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(values: &[u16]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_be_bytes()).collect()
    }

    #[test]
    fn listed_pairs_are_searched() {
        // An Apple `kern` format 0 subtable: 8-byte header, 8-byte search
        // header, two pairs.
        let mut data = words(&[0, 30, 0, 0]);
        data.extend(words(&[
            2,
            12,
            1,
            0,
            3,
            4,
            (-50_i16).cast_unsigned(),
            3,
            9,
            20,
        ]));
        let table = PairTable::new(&data, 8, 0, Flavor::Obsolete, 0).unwrap();
        assert_eq!(table.kerning(3, 4, 100), -50);
        assert_eq!(table.kerning(3, 9, 100), 20);
        assert_eq!(table.kerning(3, 5, 100), 0);
        assert!(table.has_left(3, 100) && !table.has_left(4, 100));
    }

    #[test]
    fn classes_add_up_to_positions() {
        // An Apple `kern` format 2 subtable (header 8 bytes): fields at 8,
        // left classes at 16 (glyph 5 -> 28), right classes at 22 (glyph 7
        // -> 2), values at 28.
        let mut data = words(&[0, 0, 0, 0]);
        data.extend(words(&[4, 16, 22, 28]));
        data.extend(words(&[5, 1, 28, 7, 1, 2, 11, 33]));
        let table = PairTable::new(&data, 8, 2, Flavor::Obsolete, 0).unwrap();
        assert_eq!(table.kerning(5, 7, 100), 33);
        assert_eq!(table.kerning(5, 8, 100), 0);
        assert_eq!(table.kerning(6, 7, 100), 0);
    }

    #[test]
    fn byte_classes_index_values() {
        // `kern` format 3: 3 glyphs, 2 values, 2 left and 2 right classes.
        let mut data = words(&[0, 0, 0, 0]);
        data.extend(words(&[3]));
        data.extend([2, 2, 2, 0]);
        data.extend(words(&[(-7_i16).cast_unsigned(), 9]));
        data.extend([0, 1, 1, 0, 1, 0, 0, 0, 1, 0]);
        let table = PairTable::new(&data, 8, 3, Flavor::Obsolete, 0).unwrap();
        assert_eq!(table.kerning(1, 1, 3), -7);
        assert_eq!(table.kerning(1, 0, 3), 9);
        assert_eq!(table.kerning(0, 2, 3), -7);
        assert_eq!(table.kerning(5, 0, 3), 0);
    }
}
