//! Deterministic test data shared by the unit tests.

use crate::raster::{Pixels, Raster};

/// The bytes of the array matplotlib's `pil_to_array` returns for an image
/// Pillow opens as `raster`: modes `L`, `I;16` (little-endian), `RGB` and
/// `RGBA` as they are, every other mode converted to RGBA.
pub(crate) fn pil_to_array_bytes(raster: &Raster) -> Vec<u8> {
    match raster.pixels() {
        Pixels::Gray(levels) => levels.clone(),
        Pixels::Gray16(levels) => levels.iter().flat_map(|v| v.to_le_bytes()).collect(),
        Pixels::Rgb(pixels) => pixels.as_flattened().to_vec(),
        Pixels::Rgba(pixels) => pixels.as_flattened().to_vec(),
        _ => raster.to_rgba8().as_flattened().to_vec(),
    }
}

/// Flips 1 to 4 random bits, bytes or lengths of `data`, or cuts it short,
/// for fuzz-style tests.
pub(crate) fn mutate(data: &[u8], rng: &mut Rng) -> Vec<u8> {
    let mut out = data.to_vec();
    for _ in 0..=rng.below(4) {
        if out.is_empty() {
            break;
        }
        let at = rng.below(out.len());
        match rng.below(4) {
            0 => out[at] ^= 1 << rng.below(8),
            1 => out[at] = [0, 0xFF, 0x7F, 0x80][rng.below(4)],
            2 => out[at] = rng.next_u64() as u8,
            _ => out.truncate(at),
        }
    }
    out
}

/// `SplitMix64`: a tiny, high-quality deterministic generator.
pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub(crate) fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..bound` (`bound > 0`).
    pub(crate) fn below(&mut self, bound: usize) -> usize {
        ((u128::from(self.next_u64()) * bound as u128) >> 64) as usize
    }

    pub(crate) fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.next_u64() as u8).collect()
    }
}

/// Inputs of many shapes and sizes: empty, tiny, runs, small alphabets,
/// text-like data, repeats with mutations, incompressible noise, and inputs
/// straddling stored-block and window sizes.
pub(crate) fn varied_inputs(rng: &mut Rng) -> Vec<Vec<u8>> {
    let mut inputs = vec![
        Vec::new(),
        vec![0],
        vec![7; 2],
        vec![42; 3],
        b"abcabcabcabcabcabcabcabcabc".to_vec(),
        vec![0; 100_000],
        rng.bytes(70_000),
    ];
    for size in [
        1, 2, 5, 17, 258, 259, 1000, 4097, 32_768, 65_535, 65_536, 131_073,
    ] {
        let alphabet = 1 + rng.below(8);
        inputs.push(
            (0..size)
                .map(|_| b'a' + rng.below(alphabet) as u8)
                .collect(),
        );
    }
    let words = [
        &b"the "[..],
        b"quick ",
        b"brown ",
        b"fox ",
        b"jumps ",
        b"over ",
        b"lazy ",
        b"dog\n",
    ];
    inputs.push(
        (0..20_000)
            .flat_map(|_| words[rng.below(words.len())].iter().copied())
            .collect(),
    );
    let block = rng.bytes(40_000);
    let mut repeated = block.repeat(3);
    for _ in 0..200 {
        let index = rng.below(repeated.len());
        repeated[index] = rng.next_u64() as u8;
    }
    inputs.push(repeated);
    let window = rng.bytes(32_768);
    inputs.push(window.repeat(2));
    let mut runs = Vec::new();
    while runs.len() < 50_000 {
        let byte = rng.next_u64() as u8;
        runs.extend(std::iter::repeat_n(byte, 1 + rng.below(600)));
    }
    inputs.push(runs);
    inputs
}
