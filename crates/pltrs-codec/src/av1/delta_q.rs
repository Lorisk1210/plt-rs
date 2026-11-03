//! Superblock quantizer deltas: libaom's "Variance Boost", the adaptive
//! quantization of its image quality tuning, which gives superblocks of
//! low variance (flat backgrounds with thin lines and text, as in plots)
//! finer quantizers; and the `delta_qindex` syntax that signals them.

use super::entropy::SymbolSink;
use super::plane::Plane;
use super::tables::lookup::AC_QLOOKUP;

/// `DELTA_Q_SMALL`: delta sizes from here on are coded in bits.
const DELTA_Q_SMALL: u32 = 3;
/// The strongest boost: the factor by which a quantizer step shrinks.
const MAX_BOOST: f64 = 8.0;
/// The largest lowering of the quantizer index.
const MAX_DELTA: i32 = 80;
/// The strength of the boost (libaom's `deltaq-strength` of 100%, times 3).
const STRENGTH: f64 = 3.0;

/// `delta_q_res`: the base-2 logarithm of the step of the deltas, coarser
/// at coarser base indices, where signalling costs more.
pub(crate) fn resolution_log2(base_qindex: u8) -> u32 {
    match base_qindex {
        160.. => 3,
        120..=159 => 2,
        80..=119 => 1,
        _ => 0,
    }
}

/// The variance of the 8×8 block at `(x, y)`, per sample, truncated.
fn block_variance(plane: &Plane, x: usize, y: usize) -> u32 {
    let samples = plane.square(x, y, 8);
    let sum: i64 = samples.iter().map(|&s| i64::from(s)).sum();
    let squares: i64 = samples.iter().map(|&s| i64::from(s) * i64::from(s)).sum();
    ((squares - ((sum * sum) >> 6)) / 64) as u32
}

/// The variance that sets the boost of the superblock at `(x, y)`: of
/// its 64 blocks of 8×8, sorted by variance, a 1:2:1 weighting of the
/// last of the fourth, fifth and sixth eighths.
fn superblock_variance(luma: &Plane, x: usize, y: usize) -> u32 {
    let mut variances: Vec<u32> = (0..64)
        .map(|i| block_variance(luma, x + 8 * (i % 8), y + 8 * (i / 8)))
        .collect();
    variances.sort_unstable();
    (variances[31] + 2 * variances[39] + variances[47] + 2) / 4
}

/// The quantizer index whose AC step is the first to reach `q` (in
/// quarters of the step, as libaom's `av1_convert_q_to_qindex`).
fn qindex_of_step(q: f64) -> i32 {
    AC_QLOOKUP
        .iter()
        .position(|&step| f64::from(step) / 4.0 >= q)
        .unwrap_or(255) as i32
}

/// The quantizer index Variance Boost aims at for the superblock at
/// `(x, y)` of `luma`, in a frame of base index `base`.
pub(crate) fn target(luma: &Plane, (x, y): (usize, usize), base: u8) -> i32 {
    let variance = superblock_variance(luma, x, y).max(1);
    let ratio = (0.15 * STRENGTH * (10.0 - f64::from(variance).log2()) + 1.0).clamp(1.0, MAX_BOOST);
    let base = i32::from(base);
    let base_q = f64::from(AC_QLOOKUP[base as usize]) / 4.0;
    let target = qindex_of_step(base_q / ratio);
    let boost = (f64::from((base + 544) * (base - target)) / 1279.0).round() as i32;
    (base - boost.min(MAX_DELTA)).max(1)
}

/// The `CurrentQIndex` a decoder reaches from `current` when the encoder
/// aims at `target`: the difference rounded to a multiple of the
/// resolution, with libaom's dead zone of a quarter step.
pub(crate) fn adjust(resolution_log2: u32, current: u8, target: i32) -> (u8, i32) {
    let step = 1 << resolution_log2;
    let target = target.clamp(step, 256 - step);
    let difference = target - i32::from(current);
    let magnitude = (difference.abs() + step / 4) & !(step - 1);
    let reduced = difference.signum() * (magnitude >> resolution_log2);
    let next = (i32::from(current) + (reduced << resolution_log2)).clamp(1, 255);
    (next as u8, reduced)
}

/// Writes a reduced delta (`delta_q_abs`, the remaining bits and the
/// sign: `read_delta_qindex`).
pub(crate) fn write<S: SymbolSink>(sink: &mut S, cdf: &mut [u16; 4], reduced: i32) {
    let magnitude = reduced.unsigned_abs();
    sink.symbol(cdf, magnitude.min(DELTA_Q_SMALL) as usize);
    if magnitude >= DELTA_Q_SMALL {
        let rest = magnitude - 1;
        let bits = rest.ilog2();
        sink.literal(bits - 1, 3);
        sink.literal(rest - (1 << bits), bits);
    }
    if magnitude > 0 {
        sink.literal(u32::from(reduced < 0), 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::av1::entropy::SymbolEncoder;

    #[test]
    fn flat_superblocks_get_the_strongest_boost() {
        let flat = Plane::padded(&[255; 64 * 64], (64, 64), (64, 64));
        // Variance 1: a step ratio of 5.5.
        let base = 148;
        let boosted = target(&flat, (0, 0), base);
        assert!(boosted < 100, "{boosted}");
        assert_eq!(target(&flat, (0, 0), 1), 1);
        // Noise has a high variance: no boost.
        let noise: Vec<u8> = (0..64 * 64u32)
            .map(|i| (i.wrapping_mul(2_654_435_761) >> 24) as u8)
            .collect();
        let noisy = Plane::padded(&noise, (64, 64), (64, 64));
        assert_eq!(target(&noisy, (0, 0), base), i32::from(base));
    }

    #[test]
    fn deltas_round_to_the_resolution() {
        assert_eq!(resolution_log2(148), 2);
        assert_eq!(resolution_log2(32), 0);
        // From 148 towards 97 in steps of 4: -51 rounds to -52.
        assert_eq!(adjust(2, 148, 97), (96, -13));
        // A small move within the dead zone stays put.
        assert_eq!(adjust(3, 200, 199), (200, 0));
        assert_eq!(adjust(0, 10, 1), (1, -9));
    }

    #[test]
    fn deltas_code_as_the_specification_reads_them() {
        // Small deltas are a symbol and a sign; from 3, the size of the
        // remainder follows in 3 bits.
        let mut cdf = [28160, 32120, 32677, 0];
        let mut encoder = SymbolEncoder::new();
        for reduced in [0, 2, -3, 17, -200] {
            write(&mut encoder, &mut cdf, reduced);
        }
        assert_ne!(encoder.finish(), []);
        assert_eq!(cdf[3], 5);
    }
}
