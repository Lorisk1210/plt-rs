//! The tables of baseline JPEG: the zigzag order, the example quantization
//! tables of ITU T.81 Annex K scaled by quality as libjpeg does, the
//! standard Huffman tables, and the fixed-point constants of libjpeg's
//! accurate integer forward and inverse DCTs (`jfdctint.c`, `jidctint.c`).

/// The natural (row-major) position of each zigzag index.
pub(super) const ZIGZAG: [usize; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20,
    13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59,
    52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63,
];

/// Annex K's luminance quantization table, in natural order.
const LUMINANCE_QUANT: [u16; 64] = [
    16, 11, 10, 16, 24, 40, 51, 61, 12, 12, 14, 19, 26, 58, 60, 55, 14, 13, 16, 24, 40, 57, 69, 56,
    14, 17, 22, 29, 51, 87, 80, 62, 18, 22, 37, 56, 68, 109, 103, 77, 24, 35, 55, 64, 81, 104, 113,
    92, 49, 64, 78, 87, 103, 121, 120, 101, 72, 92, 95, 98, 112, 100, 103, 99,
];

/// Annex K's chrominance quantization table, in natural order.
const CHROMINANCE_QUANT: [u16; 64] = [
    17, 18, 24, 47, 99, 99, 99, 99, 18, 21, 26, 66, 99, 99, 99, 99, 24, 26, 56, 99, 99, 99, 99, 99,
    47, 66, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99, 99,
];

/// Which of the two quantization and Huffman table sets a component uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TableSet {
    /// Luminance (Y).
    Luminance,
    /// Chrominance (Cb and Cr).
    Chrominance,
}

impl TableSet {
    /// The table number in the file.
    pub(super) fn index(self) -> u8 {
        match self {
            Self::Luminance => 0,
            Self::Chrominance => 1,
        }
    }
}

/// The quantization table of `set` at `quality` (1–100), in natural
/// order: libjpeg's `jpeg_set_quality` with baseline limits.
pub(super) fn quant_table(set: TableSet, quality: u8) -> [u16; 64] {
    let quality = u32::from(quality.clamp(1, 100));
    let scale = if quality < 50 {
        5000 / quality
    } else {
        200 - quality * 2
    };
    let base = match set {
        TableSet::Luminance => &LUMINANCE_QUANT,
        TableSet::Chrominance => &CHROMINANCE_QUANT,
    };
    base.map(|value| {
        let scaled = ((u32::from(value) * scale + 50) / 100).clamp(1, 255);
        u16::try_from(scaled).expect("clamped to a byte")
    })
}

/// A Huffman table as stored in a DHT segment: the number of codes of each
/// length 1–16 and the symbols in code order.
#[derive(Debug)]
pub(super) struct HuffmanSpec {
    pub(super) counts: [u8; 16],
    pub(super) symbols: &'static [u8],
}

/// The standard luminance DC table.
pub(super) const DC_LUMINANCE: HuffmanSpec = HuffmanSpec {
    counts: [0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0],
    symbols: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
};

/// The standard chrominance DC table.
pub(super) const DC_CHROMINANCE: HuffmanSpec = HuffmanSpec {
    counts: [0, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0],
    symbols: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
};

/// The standard luminance AC table.
pub(super) const AC_LUMINANCE: HuffmanSpec = HuffmanSpec {
    counts: [0, 2, 1, 3, 3, 2, 4, 3, 5, 5, 4, 4, 0, 0, 1, 0x7d],
    symbols: &[
        0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06, 0x13, 0x51, 0x61,
        0x07, 0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xa1, 0x08, 0x23, 0x42, 0xb1, 0xc1, 0x15, 0x52,
        0xd1, 0xf0, 0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0a, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x25,
        0x26, 0x27, 0x28, 0x29, 0x2a, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x43, 0x44, 0x45,
        0x46, 0x47, 0x48, 0x49, 0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x63, 0x64,
        0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x83,
        0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99,
        0x9a, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6,
        0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xd2, 0xd3,
        0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xe1, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8,
        0xe9, 0xea, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9, 0xfa,
    ],
};

/// The standard chrominance AC table.
pub(super) const AC_CHROMINANCE: HuffmanSpec = HuffmanSpec {
    counts: [0, 2, 1, 2, 4, 4, 3, 4, 7, 5, 4, 4, 0, 1, 2, 0x77],
    symbols: &[
        0x00, 0x01, 0x02, 0x03, 0x11, 0x04, 0x05, 0x21, 0x31, 0x06, 0x12, 0x41, 0x51, 0x07, 0x61,
        0x71, 0x13, 0x22, 0x32, 0x81, 0x08, 0x14, 0x42, 0x91, 0xa1, 0xb1, 0xc1, 0x09, 0x23, 0x33,
        0x52, 0xf0, 0x15, 0x62, 0x72, 0xd1, 0x0a, 0x16, 0x24, 0x34, 0xe1, 0x25, 0xf1, 0x17, 0x18,
        0x19, 0x1a, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x43, 0x44,
        0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x63,
        0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a,
        0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97,
        0x98, 0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4,
        0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca,
        0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7,
        0xe8, 0xe9, 0xea, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9, 0xfa,
    ],
};

/// Fractional bits of the DCTs' fixed-point constants.
pub(super) const CONST_BITS: u32 = 13;
/// Extra precision the DCTs keep between their two passes.
pub(super) const PASS1_BITS: u32 = 2;

// `FIX(x)`: `x` with `CONST_BITS` fractional bits.
pub(super) const FIX_0_298631336: i32 = 2446;
pub(super) const FIX_0_390180644: i32 = 3196;
pub(super) const FIX_0_541196100: i32 = 4433;
pub(super) const FIX_0_765366865: i32 = 6270;
pub(super) const FIX_0_899976223: i32 = 7373;
pub(super) const FIX_1_175875602: i32 = 9633;
pub(super) const FIX_1_501321110: i32 = 12299;
pub(super) const FIX_1_847759065: i32 = 15137;
pub(super) const FIX_1_961570560: i32 = 16069;
pub(super) const FIX_2_053119869: i32 = 16819;
pub(super) const FIX_2_562915447: i32 = 20995;
pub(super) const FIX_3_072711026: i32 = 25172;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zigzag_visits_every_position_once() {
        let mut seen = [false; 64];
        for &position in &ZIGZAG {
            assert!(!seen[position]);
            seen[position] = true;
        }
    }

    #[test]
    fn quality_scales_like_libjpeg() {
        let table = quant_table(TableSet::Luminance, 75);
        let first: Vec<u16> = ZIGZAG[..8].iter().map(|&i| table[i]).collect();
        assert_eq!(first, [8, 6, 6, 7, 6, 5, 8, 7], "Pillow's quality-75 DQT");
        assert_eq!(quant_table(TableSet::Chrominance, 100), [1; 64]);
        assert_eq!(quant_table(TableSet::Luminance, 1)[0], 255);
    }

    #[test]
    fn huffman_specs_are_complete() {
        for spec in [
            &DC_LUMINANCE,
            &DC_CHROMINANCE,
            &AC_LUMINANCE,
            &AC_CHROMINANCE,
        ] {
            let total: usize = spec.counts.iter().map(|&n| usize::from(n)).sum();
            assert_eq!(total, spec.symbols.len());
        }
    }
}
