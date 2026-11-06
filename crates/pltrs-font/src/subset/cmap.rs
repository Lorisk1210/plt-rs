//! The `cmap` of a subset, as fontTools' subsetter writes it for
//! matplotlib (`Subsetter.populate(gids=...)` with the default options):
//! every Unicode subtable of the font (platform 0, Windows' BMP and full
//! repertoires) in formats 4, 6, 12 and 13, restricted to the characters of
//! the requested glyphs, without the full-repertoire subtables that repeat
//! their BMP sibling, sorted and compiled as fontTools compiles them, with
//! identical subtables stored once.
//!
//! A format 4 subtable grows by a segment for every run of consecutive
//! characters; past about 8,000 segments its 16-bit length and offsets
//! overflow, where fontTools fails. Its characters then go to a format 12
//! subtable of the platform's full repertoire instead, unless the font has
//! one.

use std::collections::{BTreeMap, BTreeSet};

use crate::face::Face;
use crate::tables::cmap::CharmapInfo;
use crate::types::GlyphId;

/// One subtable of the subset: its encoding record and its characters'
/// new glyphs.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Subtable {
    platform: u16,
    encoding: u16,
    language: u32,
    format: u16,
    map: BTreeMap<u32, u16>,
}

impl Subtable {
    /// The key fontTools sorts subtables by.
    fn key(&self) -> (u16, u16, u32) {
        (self.platform, self.encoding, self.language)
    }

    /// The subtable compiled, or `None` for a format 4 subtable too big
    /// for its 16-bit fields.
    fn compile(&self) -> Option<Vec<u8>> {
        match self.format {
            4 => format4(&self.map, self.language),
            6 => Some(format6(&self.map, self.language)),
            _ => Some(groups(&self.map, self.format, self.language)),
        }
    }
}

/// Whether fontTools' subsetter keeps a subtable of `info`: a Unicode one
/// (`isUnicode`, without the Windows symbol encoding it drops) in a format
/// it subsets and pltrs reads.
fn kept(info: CharmapInfo) -> bool {
    let unicode =
        info.platform_id == 0 || (info.platform_id == 3 && matches!(info.encoding_id, 1 | 10));
    unicode && matches!(info.format, 4 | 6 | 12 | 13)
}

/// The subtables of `face` mapping the characters of `requested` glyphs
/// to their new ids in `renumber`.
fn subtables(
    face: &Face<'_>,
    requested: &BTreeSet<GlyphId>,
    renumber: &BTreeMap<GlyphId, GlyphId>,
) -> Vec<Subtable> {
    face.cmap_subtables()
        .filter(|(info, ..)| kept(*info))
        .map(|(info, language, subtable)| {
            let mut map = BTreeMap::new();
            subtable.for_each(&mut |code, glyph| {
                if requested.contains(&glyph)
                    && let Some(new) = renumber.get(&glyph)
                {
                    map.insert(code, new.0);
                }
            });
            if info.format == 4 {
                // The closing segment's code maps to no glyph.
                map.remove(&0xFFFF);
            }
            Subtable {
                platform: info.platform_id,
                encoding: info.encoding_id,
                language,
                format: info.format,
                map,
            }
        })
        .collect()
}

/// The BMP sibling of a full-repertoire encoding (fontTools drops a
/// format 12 subtable that maps exactly what its sibling maps).
const fn bmp_sibling(platform: u16, encoding: u16) -> Option<u16> {
    match (platform, encoding) {
        (0, 4) => Some(3),
        (3, 10) => Some(1),
        _ => None,
    }
}

/// The full-repertoire encoding of a platform's BMP encoding, for the
/// characters of a format 4 subtable that overflows.
const fn full_repertoire(platform: u16) -> u16 {
    if platform == 3 { 10 } else { 4 }
}

/// `tables` without the format 12 subtables of BMP characters only that
/// repeat their BMP sibling, nor empty ones (fontTools'
/// `subset_glyphs`).
fn without_repeats(mut tables: Vec<Subtable>) -> Vec<Subtable> {
    let siblings: BTreeMap<(u16, u16, u32), BTreeMap<u32, u16>> = tables
        .iter()
        .filter(|t| (t.platform == 0 && t.encoding == 3) || (t.platform == 3 && t.encoding == 1))
        .map(|t| (t.key(), t.map.clone()))
        .collect();
    for table in &mut tables {
        let bmp_only = table
            .map
            .last_key_value()
            .is_some_and(|(&code, _)| code < 0x1_0000);
        if table.format == 12
            && bmp_only
            && let Some(sibling) = bmp_sibling(table.platform, table.encoding)
            && siblings.get(&(table.platform, sibling, table.language)) == Some(&table.map)
        {
            table.map.clear();
        }
    }
    tables.retain(|t| !t.map.is_empty());
    tables
}

/// The `cmap` table of a subset of `face` keeping the glyphs `renumber`
/// renumbers, mapping the characters of the `requested` ones.
pub(super) fn cmap(
    face: &Face<'_>,
    requested: &BTreeSet<GlyphId>,
    renumber: &BTreeMap<GlyphId, GlyphId>,
) -> Vec<u8> {
    let mut tables = without_repeats(subtables(face, requested, renumber));
    tables.sort_by_key(Subtable::key);
    let mut compiled: Vec<(Subtable, Vec<u8>)> = Vec::with_capacity(tables.len());
    let mut overflowed = Vec::new();
    for table in tables {
        match table.compile() {
            Some(data) => compiled.push((table, data)),
            None => overflowed.push(table),
        }
    }
    for table in overflowed {
        let wide = Subtable {
            encoding: full_repertoire(table.platform),
            format: 12,
            ..table
        };
        if compiled.iter().all(|(t, _)| t.key() != wide.key()) {
            let data = groups(&wide.map, 12, wide.language);
            compiled.push((wide, data));
        }
    }
    compiled.sort_by_key(|(table, _)| table.key());
    assemble(&compiled)
}

/// The table of the compiled subtables: the encoding records, then each
/// distinct subtable once, in record order.
fn assemble(compiled: &[(Subtable, Vec<u8>)]) -> Vec<u8> {
    let count = compiled.len();
    let mut records = Vec::with_capacity(4 + 8 * count);
    records.extend(0u16.to_be_bytes());
    records.extend(u16::try_from(count).unwrap_or(u16::MAX).to_be_bytes());
    let mut data: Vec<u8> = Vec::new();
    let mut offsets: Vec<(&[u8], u32)> = Vec::new();
    let base = 4 + 8 * count;
    for (table, chunk) in compiled {
        let seen = offsets.iter().find(|(seen, _)| *seen == chunk.as_slice());
        let offset = if let Some(&(_, offset)) = seen {
            offset
        } else {
            let offset = u32::try_from(base + data.len()).unwrap_or(u32::MAX);
            data.extend(chunk);
            offsets.push((chunk, offset));
            offset
        };
        records.extend(table.platform.to_be_bytes());
        records.extend(table.encoding.to_be_bytes());
        records.extend(offset.to_be_bytes());
    }
    records.extend(data);
    records
}

/// fontTools' `getSearchRange(n, item_size)`: the binary search fields of
/// `n` items (`searchRange`, `entrySelector`, `rangeShift`).
pub(super) fn search_range(n: usize, item_size: usize) -> [usize; 3] {
    let exponent = usize::try_from(n.max(1).ilog2()).unwrap_or(0);
    let range = (1 << exponent) * item_size;
    [range, exponent, (n * item_size).saturating_sub(range)]
}

/// fontTools' `splitRange`: the run of consecutive characters `first..=last`
/// split into segments of consecutive glyphs where that saves bytes (a
/// segment costs 8 bytes, a character in a glyph array 2); the starts
/// after the first and the ends.
fn split_range(first: u32, last: u32, map: &BTreeMap<u32, u16>) -> (Vec<u32>, Vec<u32>) {
    if first == last {
        return (Vec::new(), vec![last]);
    }
    let glyph = |code: u32| i64::from(map[&code]);
    let mut ordered: Vec<(u32, u32)> = Vec::new();
    let mut run_start = None;
    for code in first + 1..=last {
        if glyph(code) - 1 == glyph(code - 1) {
            run_start.get_or_insert(code - 1);
        } else if let Some(start) = run_start.take() {
            ordered.push((start, code - 1));
        }
    }
    if let Some(start) = run_start {
        ordered.push((start, last));
    }
    let mut ranges: Vec<(u32, u32)> = Vec::new();
    for &(b, e) in &ordered {
        if b == first && e == last {
            break;
        }
        let threshold = if b == first || e == last { 4 } else { 8 };
        if e - b + 1 > threshold {
            ranges.push((b, e));
        }
    }
    let (Some(&(head, _)), Some(&(_, tail))) = (ranges.first(), ranges.last()) else {
        return (Vec::new(), vec![last]);
    };
    if head != first {
        ranges.insert(0, (first, head - 1));
    }
    if tail != last {
        ranges.push((tail + 1, last));
    }
    let mut filled: Vec<(u32, u32)> = Vec::with_capacity(ranges.len() * 2);
    for range in ranges {
        if let Some(&(_, end)) = filled.last()
            && end + 1 != range.0
        {
            filled.push((end + 1, range.0 - 1));
        }
        filled.push(range);
    }
    let starts = filled.iter().skip(1).map(|&(b, _)| b).collect();
    let ends = filled.iter().map(|&(_, e)| e).collect();
    (starts, ends)
}

/// The segments of a format 4 subtable mapping `map`: starts and ends,
/// the closing `0xFFFF` segment last.
fn segments(map: &BTreeMap<u32, u16>) -> (Vec<u32>, Vec<u32>) {
    let codes: Vec<u32> = map.keys().copied().collect();
    let (mut starts, mut ends) = (Vec::new(), Vec::new());
    if let Some(&first) = codes.first() {
        starts.push(first);
        let mut last = first;
        for &code in &codes[1..] {
            if code == last + 1 {
                last = code;
                continue;
            }
            let start = *starts.last().expect("a segment is open");
            let (more_starts, more_ends) = split_range(start, last, map);
            starts.extend(more_starts);
            ends.extend(more_ends);
            starts.push(code);
            last = code;
        }
        let start = *starts.last().expect("a segment is open");
        let (more_starts, more_ends) = split_range(start, last, map);
        starts.extend(more_starts);
        ends.extend(more_ends);
    }
    starts.push(0xFFFF);
    ends.push(0xFFFF);
    (starts, ends)
}

/// A format 4 subtable of `map` (characters below `0xFFFF`), as fontTools
/// compiles it; `None` when a field overflows 16 bits.
fn format4(map: &BTreeMap<u32, u16>, language: u32) -> Option<Vec<u8>> {
    let (starts, ends) = segments(map);
    let count = ends.len();
    let (mut deltas, mut range_offsets, mut glyph_array) = (Vec::new(), Vec::new(), Vec::new());
    for (i, (&start, &end)) in starts.iter().zip(&ends).take(count - 1).enumerate() {
        let glyphs: Vec<u16> = (start..=end).map(|code| map[&code]).collect();
        let consecutive = glyphs
            .windows(2)
            .all(|pair| u32::from(pair[1]) == u32::from(pair[0]) + 1);
        if consecutive {
            let delta = (i64::from(glyphs[0]) - i64::from(start)).rem_euclid(0x1_0000);
            deltas.push(u32::try_from(delta).ok()?);
            range_offsets.push(0);
        } else {
            deltas.push(0);
            range_offsets.push(u32::try_from(2 * (count + glyph_array.len() - i)).ok()?);
            glyph_array.extend(glyphs.iter().copied().map(u32::from));
        }
    }
    deltas.push(1);
    range_offsets.push(0);
    let [range, selector, shift] = search_range(count, 2);
    let words: Vec<u32> = ends
        .iter()
        .copied()
        .chain([0])
        .chain(starts.iter().copied())
        .chain(deltas)
        .chain(range_offsets)
        .chain(glyph_array)
        .collect();
    let length = 14 + 2 * words.len();
    let header = [
        4,
        length,
        usize::try_from(language).ok()?,
        2 * count,
        range,
        selector,
        shift,
    ];
    let mut out = Vec::with_capacity(length);
    for value in header {
        out.extend(u16::try_from(value).ok()?.to_be_bytes());
    }
    for word in words {
        out.extend(u16::try_from(word).ok()?.to_be_bytes());
    }
    Some(out)
}

/// A format 6 subtable of `map`: the glyphs of every character from the
/// first to the last, 0 for the unmapped ones.
fn format6(map: &BTreeMap<u32, u16>, language: u32) -> Vec<u8> {
    let (first, last) = match (map.first_key_value(), map.last_key_value()) {
        (Some((&first, _)), Some((&last, _))) => (first, last),
        _ => (0, 0),
    };
    let glyphs: Vec<u16> = if map.is_empty() {
        Vec::new()
    } else {
        (first..=last)
            .map(|code| map.get(&code).copied().unwrap_or(0))
            .collect()
    };
    let field = |value: usize| u16::try_from(value).unwrap_or(u16::MAX);
    let language = u16::try_from(language).unwrap_or(u16::MAX);
    let mut out = Vec::with_capacity(10 + 2 * glyphs.len());
    for value in [6, field(10 + 2 * glyphs.len()), language] {
        out.extend(value.to_be_bytes());
    }
    out.extend(u16::try_from(first).unwrap_or(u16::MAX).to_be_bytes());
    out.extend(field(glyphs.len()).to_be_bytes());
    out.extend(glyphs.iter().flat_map(|glyph| glyph.to_be_bytes()));
    out
}

/// A format 12 or 13 subtable of `map`: groups of consecutive characters
/// mapped to consecutive glyphs (12) or to one glyph (13).
fn groups(map: &BTreeMap<u32, u16>, format: u16, language: u32) -> Vec<u8> {
    let step = u32::from(format == 12);
    let mut found: Vec<(u32, u32, u32)> = Vec::new();
    for (&code, &glyph) in map {
        let glyph = u32::from(glyph);
        match found.last_mut() {
            Some((start, end, first_glyph))
                if code == *end + 1 && glyph == *first_glyph + step * (code - *start) =>
            {
                *end = code;
            }
            _ => found.push((code, code, glyph)),
        }
    }
    let length = 16 + 12 * found.len();
    let mut out = Vec::with_capacity(length);
    out.extend(format.to_be_bytes());
    out.extend(0u16.to_be_bytes());
    out.extend(u32::try_from(length).unwrap_or(u32::MAX).to_be_bytes());
    out.extend(language.to_be_bytes());
    out.extend(u32::try_from(found.len()).unwrap_or(u32::MAX).to_be_bytes());
    for (start, end, glyph) in found {
        out.extend(start.to_be_bytes());
        out.extend(end.to_be_bytes());
        out.extend(glyph.to_be_bytes());
    }
    out
}

#[cfg(test)]
mod tests;
