//! Building DVI, VF and TFM files for tests.

use std::collections::HashMap;

use super::files::{MemoryFinder, TexFiles};
use super::tfm::tests::tfm_bytes;

/// Writes DVI opcodes.
#[derive(Debug, Default)]
pub(crate) struct DviWriter(pub(crate) Vec<u8>);

impl DviWriter {
    /// The preamble of a DVI file (format 2, TeX's units).
    pub(crate) fn dvi() -> Self {
        let mut writer = Self::default();
        writer
            .op(247)
            .u1(2)
            .u4(25_400_000)
            .u4(7227 << 16)
            .u4(1000)
            .u1(0);
        writer
    }

    /// The preamble of a VF file.
    pub(crate) fn vf() -> Self {
        let mut writer = Self::default();
        writer.op(247).u1(202).u1(0).u4(0).u4(10 << 20);
        writer
    }

    pub(crate) fn op(&mut self, byte: u8) -> &mut Self {
        self.0.push(byte);
        self
    }

    pub(crate) fn u1(&mut self, value: u8) -> &mut Self {
        self.0.push(value);
        self
    }

    pub(crate) fn u4(&mut self, value: u32) -> &mut Self {
        self.0.extend_from_slice(&value.to_be_bytes());
        self
    }

    pub(crate) fn s4(&mut self, value: i32) -> &mut Self {
        self.0.extend_from_slice(&value.to_be_bytes());
        self
    }

    /// `fnt_def1 k` of font `name` at `scale`.
    pub(crate) fn font_def(&mut self, k: u8, name: &str, scale: u32) -> &mut Self {
        self.op(243).u1(k).u4(0).u4(scale).u4(scale).u1(0);
        self.u1(name.len() as u8);
        self.0.extend_from_slice(name.as_bytes());
        self
    }

    /// `bop` with zero counts.
    pub(crate) fn bop(&mut self) -> &mut Self {
        self.op(139);
        for _ in 0..10 {
            self.s4(0);
        }
        self.s4(-1)
    }

    /// `down4 a`.
    pub(crate) fn down(&mut self, a: i32) -> &mut Self {
        self.op(160).s4(a)
    }

    /// `right4 b`.
    pub(crate) fn right(&mut self, b: i32) -> &mut Self {
        self.op(146).s4(b)
    }

    /// `set_rule a b`.
    pub(crate) fn rule(&mut self, a: i32, b: i32) -> &mut Self {
        self.op(132).s4(a).s4(b)
    }

    /// `eop` then `post`.
    pub(crate) fn end(&mut self) -> &mut Self {
        self.op(140).op(248)
    }
}

/// The files of a test: font `test` (characters 65 and 66, widths 0.5 and
/// 0.7 of the design size, height 100 and depth 30 units) plus `extra`.
pub(crate) fn files(extra: &[(&str, Vec<u8>)]) -> TexFiles {
    let mut files: HashMap<String, Vec<u8>> = HashMap::new();
    files.insert("test.tfm".into(), tfm_bytes(65, 66, &[1 << 19, 734_003], 0));
    for (name, data) in extra {
        files.insert((*name).to_owned(), data.clone());
    }
    TexFiles::new(MemoryFinder(files))
}

/// A page setting `AB` in font `test` at 10 pt after the structure that
/// marks the baseline (`down` four times, then three `push`es), with a
/// rule after it.
pub(crate) fn sample_dvi() -> Vec<u8> {
    let mut dvi = DviWriter::dvi();
    dvi.font_def(0, "test", 10 << 16).bop();
    dvi.down(100)
        .op(141)
        .down(100)
        .down(100)
        .op(141)
        .down(1000)
        .op(141);
    dvi.op(171)
        .op(65)
        .op(66)
        .rule(20_000, 30_000)
        .op(142)
        .op(142)
        .op(142);
    dvi.end();
    dvi.0
}
