//! Legacy `kern` table (Microsoft version 0, subtable format 0).

use crate::error::Result;
use crate::reader::{Array, FromBytes, Reader};
use crate::types::{GlyphId, Tag};

const KERN: Tag = Tag::new(b"kern");

const COVERAGE_HORIZONTAL: u16 = 0x0001;
const COVERAGE_MINIMUM: u16 = 0x0002;
const COVERAGE_CROSS_STREAM: u16 = 0x0004;
const COVERAGE_OVERRIDE: u16 = 0x0008;

#[derive(Clone, Copy, Debug)]
struct KernPair {
    key: u32,
    value: i16,
}

impl FromBytes for KernPair {
    const SIZE: usize = 6;
    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            key: u32::from_bytes(&bytes[0..4]),
            value: i16::from_bytes(&bytes[4..6]),
        }
    }
}

/// One format 0 subtable.
#[derive(Clone, Copy, Debug)]
pub(crate) struct KernSubtable<'a> {
    coverage: u16,
    pairs: Array<'a, KernPair>,
}

impl KernSubtable<'_> {
    /// Kerning value for the ordered pair, if listed.
    pub(crate) fn get(&self, left: GlyphId, right: GlyphId) -> Option<i16> {
        let key = u32::from(left.0) << 16 | u32::from(right.0);
        // Pairs should be sorted; fall back to a scan for broken fonts.
        self.pairs
            .binary_search_by(|pair| pair.key.cmp(&key))
            .map(|(_, pair)| pair.value)
            .or_else(|| {
                self.pairs
                    .iter()
                    .find(|pair| pair.key == key)
                    .map(|pair| pair.value)
            })
    }

    /// Whether the subtable kerns horizontally along the text direction
    /// (the only kind HarfBuzz and FreeType apply by default).
    pub(crate) const fn is_plain_horizontal(&self) -> bool {
        self.coverage & (COVERAGE_HORIZONTAL | COVERAGE_MINIMUM | COVERAGE_CROSS_STREAM)
            == COVERAGE_HORIZONTAL
    }

    /// Whether the subtable applies to horizontal text.
    pub(crate) const fn is_horizontal(&self) -> bool {
        self.coverage & COVERAGE_HORIZONTAL != 0
    }

    /// Whether values shift glyphs perpendicular to the text direction.
    pub(crate) const fn is_cross_stream(&self) -> bool {
        self.coverage & COVERAGE_CROSS_STREAM != 0
    }

    const fn overrides(&self) -> bool {
        self.coverage & COVERAGE_OVERRIDE != 0
    }
}

/// The legacy `kern` table.
#[derive(Clone, Debug, Default)]
pub(crate) struct Kern<'a> {
    subtables: Vec<KernSubtable<'a>>,
}

impl<'a> Kern<'a> {
    /// Parses Microsoft-style (version 0) tables; Apple version 1 tables
    /// and non-zero subtable formats are ignored, as in FreeType.
    pub(crate) fn parse(data: &'a [u8]) -> Result<Self> {
        let mut r = Reader::new(data, KERN);
        if r.u16()? != 0 {
            return Ok(Self::default());
        }
        let count = r.u16()?;
        let mut subtables = Vec::new();
        for _ in 0..count {
            let start = r.pos();
            r.skip(2)?;
            let length = usize::from(r.u16()?);
            let coverage = r.u16()?;
            let body_end = start.saturating_add(length).min(data.len());
            if coverage >> 8 == 0 && length > 14 {
                let declared = usize::from(r.u16()?);
                r.skip(6)?;
                // Like FreeType, never read pairs beyond the subtable length.
                let available = body_end.saturating_sub(r.pos()) / KernPair::SIZE;
                let pairs = r.array(declared.min(available))?;
                subtables.push(KernSubtable { coverage, pairs });
            }
            r = Reader::at(data, body_end.max(r.pos()), KERN)?;
        }
        Ok(Self { subtables })
    }

    /// Subtables in file order.
    pub(crate) fn subtables(&self) -> &[KernSubtable<'a>] {
        &self.subtables
    }

    /// FreeType's `tt_face_get_kerning`: sums (or overrides with) the values
    /// of every plain horizontal subtable, in font units.
    pub(crate) fn kerning(&self, left: GlyphId, right: GlyphId) -> i32 {
        self.subtables
            .iter()
            .filter(|s| s.is_plain_horizontal())
            .fold(0, |total, subtable| match subtable.get(left, right) {
                Some(value) if subtable.overrides() => i32::from(value),
                Some(value) => total + i32::from(value),
                None => total,
            })
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.subtables.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Pairs<'p> = &'p [(u16, u16, i16)];

    fn table(subtables: &[(u16, Pairs<'_>)]) -> Vec<u8> {
        let mut out = vec![0, 0];
        out.extend_from_slice(&(subtables.len() as u16).to_be_bytes());
        for (coverage, pairs) in subtables {
            let length = 14 + 6 * pairs.len();
            out.extend_from_slice(&[0, 0]);
            out.extend_from_slice(&(length as u16).to_be_bytes());
            out.extend_from_slice(&coverage.to_be_bytes());
            out.extend_from_slice(&(pairs.len() as u16).to_be_bytes());
            out.extend_from_slice(&[0; 6]);
            for (l, r, v) in *pairs {
                out.extend_from_slice(&l.to_be_bytes());
                out.extend_from_slice(&r.to_be_bytes());
                out.extend_from_slice(&v.to_be_bytes());
            }
        }
        out
    }

    #[test]
    fn sums_and_overrides() {
        let data = table(&[
            (1, &[(36, 57, -131), (36, 58, -50)]),
            (1, &[(36, 57, -10)]),
            (1 | COVERAGE_OVERRIDE, &[(36, 58, 7)]),
            (1 | COVERAGE_CROSS_STREAM, &[(36, 57, 1000)]),
        ]);
        let kern = Kern::parse(&data).unwrap();
        assert_eq!(kern.subtables().len(), 4);
        assert_eq!(kern.kerning(GlyphId(36), GlyphId(57)), -141);
        assert_eq!(kern.kerning(GlyphId(36), GlyphId(58)), 7);
        assert_eq!(kern.kerning(GlyphId(57), GlyphId(36)), 0);
        assert!(!kern.subtables()[3].is_plain_horizontal());
    }

    #[test]
    fn truncated_pairs_are_clamped() {
        let mut data = table(&[(1, &[(1, 2, 3), (4, 5, 6)])]);
        data.truncate(data.len() - 3);
        let kern = Kern::parse(&data).unwrap();
        assert_eq!(kern.kerning(GlyphId(1), GlyphId(2)), 3);
        assert_eq!(kern.kerning(GlyphId(4), GlyphId(5)), 0);
        assert!(Kern::parse(&data[..5]).is_err());
        assert!(Kern::parse(&[0, 1, 0, 0]).unwrap().is_empty());
    }
}
