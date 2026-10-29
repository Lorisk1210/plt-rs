//! Deterministic test data shared by the unit tests.

use pltrs_core::rng::SplitMix64;

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

/// A file Pillow decoded: its label, width, height, the CRC-32 of the
/// bytes `pil_to_array` returns for it, and the file in base64.
pub(crate) type PillowFile = (&'static str, u32, u32, u32, &'static str);

/// The bytes of `file`.
pub(crate) fn pillow_bytes(file: &PillowFile) -> Vec<u8> {
    crate::base64::decode(file.4).expect("valid base64 fixture")
}

/// Asserts that `decode` reads every one of `files` with Pillow's size
/// and pixels.
pub(crate) fn assert_decodes_like_pillow<E: std::fmt::Debug>(
    files: &[PillowFile],
    decode: impl Fn(&[u8]) -> Result<Raster, E>,
) {
    for file @ &(label, width, height, checksum, _) in files {
        let image = decode(&pillow_bytes(file)).expect("Pillow decodes it");
        assert_eq!((image.width(), image.height()), (width, height), "{label}");
        assert_eq!(
            crate::checksum::crc32(&pil_to_array_bytes(&image)),
            checksum,
            "{label}"
        );
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

/// Deterministic test data from a `SplitMix64` stream.
pub(crate) struct Rng(SplitMix64);

impl Rng {
    pub(crate) fn new(seed: u64) -> Self {
        Self(SplitMix64::new(seed))
    }

    pub(crate) fn next_u64(&mut self) -> u64 {
        self.0.next_u64()
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
