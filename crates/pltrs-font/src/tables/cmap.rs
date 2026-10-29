//! `cmap`: character to glyph mapping (formats 0, 4, 6, 10, 12 and 13).

use crate::error::{FontError, Result};
use crate::reader::{Array, FromBytes, Reader, peek, tail};
use crate::types::{GlyphId, Tag};

const CMAP: Tag = Tag::new(b"cmap");

/// Identification of one character map subtable.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CharmapInfo {
    /// Platform id (0 Unicode, 1 Macintosh, 3 Windows).
    pub platform_id: u16,
    /// Platform-specific encoding id.
    pub encoding_id: u16,
    /// Subtable format number.
    pub format: u16,
}

impl CharmapInfo {
    /// Whether the subtable maps Unicode code points (FreeType's
    /// `FT_ENCODING_UNICODE`).
    pub const fn is_unicode(&self) -> bool {
        match self.platform_id {
            0 => self.encoding_id != 5,
            2 => true,
            3 => matches!(self.encoding_id, 1 | 10),
            _ => false,
        }
    }

    /// Whether the subtable covers the full 32-bit Unicode range (FreeType
    /// 2.14 prefers these: Windows UCS-4 and Unicode 2.0+ full repertoire).
    pub const fn is_ucs4(&self) -> bool {
        matches!((self.platform_id, self.encoding_id), (3, 10) | (0, 4 | 6))
    }

    /// Whether this is a Windows symbol encoding.
    pub const fn is_symbol(&self) -> bool {
        self.platform_id == 3 && self.encoding_id == 0
    }
}

#[derive(Clone, Copy, Debug)]
struct EncodingRecord {
    platform_id: u16,
    encoding_id: u16,
    offset: u32,
}

impl FromBytes for EncodingRecord {
    const SIZE: usize = 8;
    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            platform_id: u16::from_bytes(&bytes[0..2]),
            encoding_id: u16::from_bytes(&bytes[2..4]),
            offset: u32::from_bytes(&bytes[4..8]),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Group {
    start: u32,
    end: u32,
    glyph: u32,
}

impl FromBytes for Group {
    const SIZE: usize = 12;
    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            start: u32::from_bytes(&bytes[0..4]),
            end: u32::from_bytes(&bytes[4..8]),
            glyph: u32::from_bytes(&bytes[8..12]),
        }
    }
}

/// One decoded mapping subtable.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Subtable<'a> {
    ByteEncoding(&'a [u8]),
    SegmentDelta(SegmentDelta<'a>),
    Trimmed {
        first: u32,
        glyphs: Array<'a, GlyphId>,
    },
    SegmentedCoverage(Array<'a, Group>),
    ManyToOne(Array<'a, Group>),
}

/// Format 4 segment arrays.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SegmentDelta<'a> {
    ends: Array<'a, u16>,
    starts: Array<'a, u16>,
    deltas: Array<'a, u16>,
    /// The idRangeOffset array followed by the glyph id array.
    range_offsets: &'a [u8],
}

impl SegmentDelta<'_> {
    fn parse(data: &[u8]) -> Result<SegmentDelta<'_>> {
        let mut r = Reader::new(data, CMAP);
        r.skip(6)?;
        let seg_count = usize::from(r.u16()? / 2);
        r.skip(6)?;
        let ends = r.array(seg_count)?;
        r.skip(2)?;
        let starts = r.array(seg_count)?;
        let deltas = r.array(seg_count)?;
        let range_offsets = tail(data, r.pos(), CMAP)?;
        if range_offsets.len() < seg_count * 2 {
            return Err(FontError::Truncated(CMAP));
        }
        Ok(SegmentDelta {
            ends,
            starts,
            deltas,
            range_offsets,
        })
    }

    fn segment_glyph(&self, segment: usize, code: u32) -> Option<u16> {
        let start = u32::from(self.starts.get(segment)?);
        let delta = self.deltas.get(segment)?;
        let range_offset = usize::from(peek::<u16>(self.range_offsets, segment * 2)?);
        if code < start {
            return None;
        }
        let glyph = if range_offset == 0 || range_offset == 0xFFFF {
            (code as u16).wrapping_add(delta)
        } else {
            let at = segment * 2 + range_offset + 2 * (code - start) as usize;
            match peek::<u16>(self.range_offsets, at)? {
                0 => 0,
                raw => raw.wrapping_add(delta),
            }
        };
        Some(glyph)
    }

    fn glyph(&self, code: u32) -> Option<u16> {
        if code > 0xFFFF {
            return None;
        }
        // First segment whose end code is >= code.
        let (mut low, mut high) = (0, self.ends.len());
        while low < high {
            let mid = low + (high - low) / 2;
            if u32::from(self.ends.get(mid)?) < code {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        self.segment_glyph(low, code)
    }

    /// Calls `f` for every mapped code in increasing order, like
    /// FreeType's character iteration: segments are taken by start code and
    /// overlaps clipped, so that each code is visited once (overlapping
    /// segments would otherwise repeat up to 65 536 codes each).
    fn for_each(&self, f: &mut dyn FnMut(u32, GlyphId)) {
        let mut segments: Vec<(u16, u16, usize)> = (0..self.ends.len())
            .map_while(|segment| {
                Some((self.starts.get(segment)?, self.ends.get(segment)?, segment))
            })
            .collect();
        segments.sort_by_key(|&(start, ..)| start);
        let mut next = 0;
        for (start, end, segment) in segments {
            let first = u32::from(start).max(next);
            next = next.max(u32::from(end) + 1);
            for code in first..=u32::from(end) {
                if code == 0xFFFF {
                    break;
                }
                if let Some(glyph) = self.segment_glyph(segment, code).filter(|&g| g != 0) {
                    f(code, GlyphId(glyph));
                }
            }
        }
    }
}

impl<'a> Subtable<'a> {
    /// Decodes the subtable starting at `data[0]`; `Ok(None)` for formats
    /// that are not character maps of interest (2, 8, 14).
    fn parse(data: &'a [u8]) -> Result<Option<Self>> {
        let data = Self::limit_to_length(data)?;
        let mut r = Reader::new(data, CMAP);
        let format = r.u16()?;
        let subtable = match format {
            0 => {
                r.skip(4)?;
                Self::ByteEncoding(r.bytes(256)?)
            }
            4 => Self::SegmentDelta(SegmentDelta::parse(data)?),
            6 => {
                r.skip(4)?;
                let first = u32::from(r.u16()?);
                let count = r.u16()?;
                Self::Trimmed {
                    first,
                    glyphs: r.array(usize::from(count))?,
                }
            }
            10 => {
                r.skip(10)?;
                let first = r.u32()?;
                let count = r.u32()?;
                Self::Trimmed {
                    first,
                    glyphs: r.array(count as usize)?,
                }
            }
            12 | 13 => {
                r.skip(10)?;
                let count = r.u32()?;
                let groups = r.array(count as usize)?;
                if format == 12 {
                    Self::SegmentedCoverage(groups)
                } else {
                    Self::ManyToOne(groups)
                }
            }
            2 | 8 | 14 => return Ok(None),
            other => {
                return Err(FontError::UnsupportedFormat {
                    table: CMAP,
                    format: u32::from(other),
                });
            }
        };
        Ok(Some(subtable))
    }

    /// Restricts `data` to the subtable's declared length. Like FreeType, a
    /// length running past the end of the table is clamped.
    fn limit_to_length(data: &'a [u8]) -> Result<&'a [u8]> {
        let mut r = Reader::new(data, CMAP);
        let length = match r.u16()? {
            0..=6 => usize::from(r.u16()?),
            14 => r.u32()? as usize,
            _ => {
                r.skip(2)?;
                r.u32()? as usize
            }
        };
        Ok(&data[..length.min(data.len())])
    }

    /// Maps a character code to a glyph; `None` when unmapped.
    pub(crate) fn glyph(&self, code: u32) -> Option<GlyphId> {
        let raw = match self {
            Self::ByteEncoding(table) => u16::from(*table.get(usize::try_from(code).ok()?)?),
            Self::SegmentDelta(segments) => segments.glyph(code)?,
            Self::Trimmed { first, glyphs } => glyphs.get(code.checked_sub(*first)? as usize)?.0,
            Self::SegmentedCoverage(groups) | Self::ManyToOne(groups) => {
                let (_, group) = groups.binary_search_by(|g| {
                    if g.end < code {
                        std::cmp::Ordering::Less
                    } else if g.start > code {
                        std::cmp::Ordering::Greater
                    } else {
                        std::cmp::Ordering::Equal
                    }
                })?;
                let glyph = match self {
                    Self::ManyToOne(_) => group.glyph,
                    _ => group.glyph.checked_add(code - group.start)?,
                };
                u16::try_from(glyph).ok()?
            }
        };
        (raw != 0).then_some(GlyphId(raw))
    }

    /// Calls `f` for every mapped character, in subtable order.
    pub(crate) fn for_each(&self, f: &mut dyn FnMut(u32, GlyphId)) {
        match self {
            Self::ByteEncoding(table) => {
                for (code, &glyph) in (0u32..).zip(table.iter()) {
                    if glyph != 0 {
                        f(code, GlyphId(u16::from(glyph)));
                    }
                }
            }
            Self::SegmentDelta(segments) => segments.for_each(f),
            Self::Trimmed { first, glyphs } => {
                for (code, glyph) in (*first..).zip(glyphs.iter()) {
                    if glyph.0 != 0 {
                        f(code, glyph);
                    }
                }
            }
            Self::SegmentedCoverage(groups) | Self::ManyToOne(groups) => {
                // Ordered and clipped as in `SegmentDelta::for_each`.
                let many_to_one = matches!(self, Self::ManyToOne(_));
                let mut groups: Vec<Group> = groups.iter().collect();
                groups.sort_by_key(|group| group.start);
                let mut next = 0;
                for group in groups {
                    let (first, last) = (group.start.max(next), group.end.min(0x10_FFFF));
                    next = next.max(last.saturating_add(1));
                    for code in first..=last {
                        let offset = if many_to_one { 0 } else { code - group.start };
                        match u16::try_from(group.glyph.saturating_add(offset)) {
                            Ok(0) => {}
                            Ok(glyph) => f(code, GlyphId(glyph)),
                            Err(_) => break,
                        }
                    }
                }
            }
        }
    }
}

/// The `cmap` table: a list of usable subtables.
#[derive(Clone, Debug, Default)]
pub(crate) struct Cmap<'a> {
    maps: Vec<(CharmapInfo, Subtable<'a>)>,
}

impl<'a> Cmap<'a> {
    /// Parses every encoding record. Subtables that are malformed or of an
    /// unsupported format are skipped, like FreeType does.
    pub(crate) fn parse(data: &'a [u8]) -> Result<Self> {
        let mut r = Reader::new(data, CMAP);
        r.skip(2)?;
        let count = r.u16()?;
        let records: Array<'_, EncodingRecord> = r.array(usize::from(count))?;
        let mut maps = Vec::new();
        for record in records.iter() {
            let Ok(sub) = tail(data, record.offset as usize, CMAP) else {
                continue;
            };
            let Some(format) = peek::<u16>(sub, 0) else {
                continue;
            };
            if let Ok(Some(subtable)) = Subtable::parse(sub) {
                let info = CharmapInfo {
                    platform_id: record.platform_id,
                    encoding_id: record.encoding_id,
                    format,
                };
                maps.push((info, subtable));
            }
        }
        Ok(Self { maps })
    }

    /// All usable subtables in file order.
    pub(crate) fn infos(&self) -> impl Iterator<Item = CharmapInfo> + '_ {
        self.maps.iter().map(|(info, _)| *info)
    }

    /// Subtable `index` (in file order among usable subtables).
    pub(crate) fn get(&self, index: usize) -> Option<(CharmapInfo, Subtable<'a>)> {
        self.maps.get(index).copied()
    }

    /// The subtable `FT_Select_Charmap(face, FT_ENCODING_UNICODE)` picks
    /// (`find_unicode_charmap`): the last UCS-4 subtable (`(3, 10)`,
    /// `(0, 4)`, or `(0, 6)` in format 13), else the last Unicode one.
    pub(crate) fn freetype_unicode_index(&self) -> Option<usize> {
        let ucs4 = |info: &CharmapInfo| match (info.platform_id, info.encoding_id) {
            (3, 10) | (0, 4) => true,
            (0, 6) => info.format == 13,
            _ => false,
        };
        self.maps
            .iter()
            .rposition(|(info, _)| info.is_unicode() && ucs4(info))
            .or_else(|| self.maps.iter().rposition(|(info, _)| info.is_unicode()))
    }

    /// The subtable HarfBuzz's own character map uses
    /// (`find_best_subtable`): a Windows symbol subtable first, then the
    /// 32-bit and 16-bit Unicode encodings in HarfBuzz's order.
    pub(crate) fn harfbuzz_index(&self) -> Option<usize> {
        const ORDER: [(u16, u16); 9] = [
            (3, 0),
            (3, 10),
            (0, 6),
            (0, 4),
            (3, 1),
            (0, 3),
            (0, 2),
            (0, 1),
            (0, 0),
        ];
        ORDER.iter().find_map(|&(platform, encoding)| {
            self.maps
                .iter()
                .position(|(info, _)| (info.platform_id, info.encoding_id) == (platform, encoding))
        })
    }

    /// FreeType's default charmap choice: the last UCS-4 Unicode subtable,
    /// else the last Unicode subtable, else a Windows symbol subtable.
    pub(crate) fn default_index(&self) -> Option<usize> {
        let last = |pred: &dyn Fn(&CharmapInfo) -> bool| {
            self.maps.iter().rposition(|(info, _)| pred(info))
        };
        last(&|info| info.is_unicode() && info.is_ucs4())
            .or_else(|| last(&CharmapInfo::is_unicode))
            .or_else(|| last(&CharmapInfo::is_symbol))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wrap(records: &[(u16, u16, Vec<u8>)]) -> Vec<u8> {
        let mut out = vec![0, 0];
        out.extend_from_slice(&(records.len() as u16).to_be_bytes());
        let mut offset = 4 + 8 * records.len();
        let mut body = Vec::new();
        for (platform, encoding, sub) in records {
            let mut sub = sub.clone();
            let len = sub.len();
            if sub[1] <= 6 {
                sub[2..4].copy_from_slice(&(len as u16).to_be_bytes());
            } else {
                sub[4..8].copy_from_slice(&(len as u32).to_be_bytes());
            }
            out.extend_from_slice(&platform.to_be_bytes());
            out.extend_from_slice(&encoding.to_be_bytes());
            out.extend_from_slice(&(offset as u32).to_be_bytes());
            offset += sub.len();
            body.extend_from_slice(&sub);
        }
        out.extend_from_slice(&body);
        out
    }

    fn format4(segments: &[(u16, u16, i16)]) -> Vec<u8> {
        let mut all = segments.to_vec();
        all.push((0xFFFF, 0xFFFF, 1));
        let n = all.len() as u16;
        let mut out = vec![0, 4, 0, 0, 0, 0];
        out.extend_from_slice(&(n * 2).to_be_bytes());
        out.extend_from_slice(&[0; 6]);
        for s in &all {
            out.extend_from_slice(&s.1.to_be_bytes());
        }
        out.extend_from_slice(&[0, 0]);
        for s in &all {
            out.extend_from_slice(&s.0.to_be_bytes());
        }
        for s in &all {
            out.extend_from_slice(&s.2.to_be_bytes());
        }
        out.resize(out.len() + 2 * all.len(), 0);
        out
    }

    fn format12(format: u16, groups: &[(u32, u32, u32)]) -> Vec<u8> {
        let mut out = format.to_be_bytes().to_vec();
        out.extend_from_slice(&[0; 10]);
        out.extend_from_slice(&(groups.len() as u32).to_be_bytes());
        for g in groups {
            for v in [g.0, g.1, g.2] {
                out.extend_from_slice(&v.to_be_bytes());
            }
        }
        out
    }

    #[test]
    fn format4_delta_lookup() {
        let data = wrap(&[(3, 1, format4(&[(0x20, 0x7E, -29)]))]);
        let cmap = Cmap::parse(&data).unwrap();
        let (_, sub) = cmap.get(cmap.default_index().unwrap()).unwrap();
        assert_eq!(sub.glyph(u32::from('A')), Some(GlyphId(36)));
        assert_eq!(sub.glyph(0x1F), None);
        assert_eq!(sub.glyph(0x1_0000), None);
        let mut count = 0;
        sub.for_each(&mut |_, _| count += 1);
        assert_eq!(count, 0x7E - 0x20 + 1);
    }

    #[test]
    fn format4_range_offsets() {
        // One segment 'a'..='c' with idRangeOffset pointing at a glyph array.
        let mut sub = vec![0, 4, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0];
        sub.extend_from_slice(&[0, b'c', 0xFF, 0xFF, 0, 0, 0, b'a', 0xFF, 0xFF, 0, 0, 0, 1]);
        sub.extend_from_slice(&[0, 4, 0, 0]);
        sub.extend_from_slice(&[0, 10, 0, 0, 0, 12]);
        let data = wrap(&[(3, 1, sub)]);
        let cmap = Cmap::parse(&data).unwrap();
        let (_, sub) = cmap.get(0).unwrap();
        assert_eq!(sub.glyph(u32::from('a')), Some(GlyphId(10)));
        assert_eq!(sub.glyph(u32::from('b')), None);
        assert_eq!(sub.glyph(u32::from('c')), Some(GlyphId(12)));
    }

    #[test]
    fn groups_and_selection() {
        let data = wrap(&[
            (1, 0, {
                let mut s = vec![0, 0, 1, 6, 0, 0];
                s.extend((0..=255u8).map(|b| b / 2));
                s
            }),
            (3, 1, format4(&[(0x41, 0x41, 1)])),
            (
                0,
                6,
                format12(13, &[(0x0, 0xFFFF, 7), (0x1_0000, 0x10_FFFF, 8)]),
            ),
            (3, 10, format12(12, &[(0x1F600, 0x1F602, 100)])),
        ]);
        let cmap = Cmap::parse(&data).unwrap();
        assert_eq!(cmap.infos().count(), 4);
        assert_eq!(cmap.default_index(), Some(3));
        let (_, mac) = cmap.get(0).unwrap();
        assert_eq!(mac.glyph(9), Some(GlyphId(4)));
        assert_eq!(mac.glyph(300), None);
        let (info, last_resort) = cmap.get(2).unwrap();
        assert!(info.is_unicode() && info.is_ucs4());
        assert_eq!(last_resort.glyph(0x4E00), Some(GlyphId(7)));
        assert_eq!(last_resort.glyph(0x2_0000), Some(GlyphId(8)));
        let (_, ucs4) = cmap.get(3).unwrap();
        assert_eq!(ucs4.glyph(0x1F601), Some(GlyphId(101)));
        assert_eq!(ucs4.glyph(0x1F603), None);
    }

    #[test]
    fn overlapping_ranges_are_enumerated_once() {
        // 200 copies of the full range would enumerate 200 million codes.
        let mut groups = vec![(0, 0x10_FFFF, 7); 200];
        let mut segments = vec![(0x20, 0xFFFE, 1); 200];
        // Unsorted ranges are still enumerated in increasing order.
        groups.insert(0, (0x41, 0x5A, 9));
        segments.insert(0, (0x41, 0x5A, 1));
        let data = wrap(&[(0, 6, format12(13, &groups)), (3, 1, format4(&segments))]);
        let cmap = Cmap::parse(&data).unwrap();
        for (index, codes) in [(0, 0x11_0000), (1, 0xFFFE - 0x20 + 1)] {
            let (_, sub) = cmap.get(index).unwrap();
            let mut count = 0;
            let mut last = None;
            sub.for_each(&mut |code, _| {
                assert!(last < Some(code));
                last = Some(code);
                count += 1;
            });
            assert_eq!(count, codes);
        }
    }

    #[test]
    fn malformed_subtables_are_skipped_not_fatal() {
        let mut sub = format4(&[(0x20, 0x7E, -29)]);
        sub.truncate(12);
        let data = wrap(&[(3, 1, sub), (3, 0, format12(12, &[(0xF020, 0xF0FF, 3)]))]);
        let cmap = Cmap::parse(&data).unwrap();
        assert_eq!(cmap.infos().count(), 1);
        assert_eq!(cmap.default_index(), Some(0));
        assert!(Cmap::parse(&data[..10]).is_err());
    }
}
