//! TeX font metrics (matplotlib's `dviread.Tfm`): the width, height and
//! depth of every character, in 12.20 fixed point relative to the design
//! size.

use std::collections::HashMap;

use super::bytes::Bytes;
use super::error::{TexError, TexFile};

/// The metrics of one character, with TeX's meaning (matplotlib's
/// `TexMetrics`): the width is the advance, the height how far the
/// character rises above the baseline, the depth how far it reaches below.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TexMetrics {
    /// The advance width.
    pub width: i32,
    /// The height above the baseline.
    pub height: i32,
    /// The depth below the baseline (positive downwards).
    pub depth: i32,
}

/// A TeX font metric file, reduced to what DVI files need.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tfm {
    checksum: u32,
    design_size: u32,
    metrics: HashMap<i64, TexMetrics>,
}

/// The next `n` signed 32-bit numbers of `file`
/// (`struct.unpack(f"!{n}i", file.read(4 * n))`), failing where Python's
/// `struct` would find too few bytes.
fn words(file: &mut Bytes<'_>, n: usize) -> Result<Vec<i32>, TexError> {
    let bytes = file.read(n.saturating_mul(4));
    if bytes.len() != n * 4 {
        return Err(truncated());
    }
    Ok(bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|&word| i32::from_be_bytes(word))
        .collect())
}

fn truncated() -> TexError {
    TexError::malformed(TexFile::Tfm, "unpack requires more bytes")
}

impl Tfm {
    /// Parses a TFM file (matplotlib's `Tfm(filename)`).
    ///
    /// # Errors
    ///
    /// When the file is too short for the tables its header declares or a
    /// character refers to a width, height or depth that is not there.
    pub fn parse(data: &[u8]) -> Result<Self, TexError> {
        let mut file = Bytes::new(data);
        let header1 = file.read(24);
        if header1.len() < 14 {
            return Err(truncated());
        }
        let half = |i: usize| usize::from(u16::from_be_bytes([header1[i], header1[i + 1]]));
        let (lh, bc, ec, nw, nh, nd) = (half(2), half(4), half(6), half(8), half(10), half(12));
        let header2 = file.read(4 * lh);
        if header2.len() < 8 {
            return Err(truncated());
        }
        let checksum = u32::from_be_bytes([header2[0], header2[1], header2[2], header2[3]]);
        let design_size = u32::from_be_bytes([header2[4], header2[5], header2[6], header2[7]]);
        // `file.read(4 * (ec - bc + 1))`: a negative count reads to the end.
        let count = ec as i64 - bc as i64 + 1;
        let char_info = if count < 0 {
            file.read(usize::MAX)
        } else {
            file.read(4 * count as usize)
        };
        let widths = words(&mut file, nw)?;
        let heights = words(&mut file, nh)?;
        let depths = words(&mut file, nd)?;
        let mut metrics = HashMap::new();
        for (index, code) in (bc..=ec).enumerate() {
            let entry = |i: usize| {
                char_info
                    .get(4 * index + i)
                    .copied()
                    .ok_or_else(|| TexError::malformed(TexFile::Tfm, "index out of range"))
            };
            let (byte0, byte1) = (usize::from(entry(0)?), usize::from(entry(1)?));
            let pick = |table: &[i32], i: usize| {
                table
                    .get(i)
                    .copied()
                    .ok_or_else(|| TexError::malformed(TexFile::Tfm, "tuple index out of range"))
            };
            metrics.insert(
                code as i64,
                TexMetrics {
                    width: pick(&widths, byte0)?,
                    height: pick(&heights, byte1 >> 4)?,
                    depth: pick(&depths, byte1 & 0xF)?,
                },
            );
        }
        Ok(Self {
            checksum,
            design_size,
            metrics,
        })
    }

    /// The checksum DVI files are verified against.
    pub fn checksum(&self) -> u32 {
        self.checksum
    }

    /// The design size in 12.20 TeX points (the scale in the DVI file
    /// overrides it).
    pub fn design_size(&self) -> u32 {
        self.design_size
    }

    /// The metrics of character `code`, if the font has it
    /// (`get_metrics`).
    pub fn metrics(&self, code: i64) -> Option<TexMetrics> {
        self.metrics.get(&code).copied()
    }

    /// The character codes the font has metrics for, ascending.
    pub fn codes(&self) -> Vec<i64> {
        let mut codes: Vec<i64> = self.metrics.keys().copied().collect();
        codes.sort_unstable();
        codes
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A TFM file with characters `bc..=ec`, every character using width
    /// `code - bc + 1`, height 1 and depth 1 of tables
    /// `widths`, `[0, 100]` and `[0, 30]`.
    pub(crate) fn tfm_bytes(bc: u16, ec: u16, widths: &[i32], checksum: u32) -> Vec<u8> {
        let nw = widths.len() as u16 + 1;
        let mut data = Vec::new();
        for value in [0, 2, bc, ec, nw, 2, 2, 0, 0, 0, 0, 0] {
            data.extend_from_slice(&u16::to_be_bytes(value));
        }
        data.extend_from_slice(&checksum.to_be_bytes());
        data.extend_from_slice(&(10u32 << 20).to_be_bytes());
        for code in bc..=ec {
            let width_index = (code - bc + 1).min(nw - 1) as u8;
            data.extend_from_slice(&[width_index, 0x11, 0, 0]);
        }
        for width in std::iter::once(&0).chain(widths) {
            data.extend_from_slice(&width.to_be_bytes());
        }
        for value in [0, 100, 0, 30] {
            data.extend_from_slice(&i32::to_be_bytes(value));
        }
        data
    }

    #[test]
    fn metrics_come_from_the_tables() {
        let tfm = Tfm::parse(&tfm_bytes(65, 66, &[500, 700], 42)).unwrap();
        assert_eq!(tfm.checksum(), 42);
        assert_eq!(tfm.design_size(), 10 << 20);
        assert_eq!(
            tfm.metrics(66),
            Some(TexMetrics {
                width: 700,
                height: 100,
                depth: 30
            })
        );
        assert_eq!(tfm.metrics(64), None);
        assert_eq!(tfm.codes(), [65, 66]);
    }

    #[test]
    fn truncated_and_inconsistent_files_are_errors() {
        let data = tfm_bytes(65, 66, &[500, 700], 0);
        for len in [0, 10, 20, 30, data.len() - 1] {
            assert!(Tfm::parse(&data[..len]).is_err(), "{len}");
        }
        let mut bad_index = data.clone();
        bad_index[32] = 9;
        assert!(Tfm::parse(&bad_index).is_err());
        let empty = Tfm::parse(&tfm_bytes(1, 0, &[], 0)).unwrap();
        assert_eq!(empty.codes(), Vec::<i64>::new());
    }
}
