//! Quantization: the quantizers of a quantizer index (section 7.12.2),
//! the encoder's rounding of coefficients to levels, and the decoder's
//! dequantization, which the encoder repeats to reconstruct.

use super::syntax::Square;
use super::tables::lookup::{AC_QLOOKUP, DC_QLOOKUP};

/// How far below one half a coefficient's fraction must be to round its
/// level up: the dead zone that leaves small coefficients at zero.
const AC_ROUNDING: f32 = 0.375;
/// The rounding of DC coefficients, which carry the most energy.
const DC_ROUNDING: f32 = 0.45;
/// The weight of a bit against squared sample error, relative to the
/// square of the AC quantizer step in samples.
const LAMBDA_SCALE: f64 = 0.05;

/// The quantizers of a quantizer index, with the frame's offset of the
/// chroma planes' indices, set as libaom's image quality tuning sets it
/// (finer chroma in 4:2:0, coarser chroma AC in 4:4:4).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Quantizer {
    /// `base_q_idx`, or the `CurrentQIndex` of a superblock.
    qindex: u8,
    /// `DeltaQUDc` and `DeltaQUAc`, which V shares.
    chroma_delta: (i32, i32),
    /// The DC and AC quantizers of luma, then of chroma.
    steps: [(i32, i32); 2],
}

/// `dc_q` or `ac_q` of a quantizer index, clipped to `[0, 255]`.
fn lookup(table: &[u16; 256], qindex: i32) -> i32 {
    i32::from(table[qindex.clamp(0, 255) as usize])
}

impl Quantizer {
    /// The quantizers of a frame of base index `qindex` (0 codes
    /// losslessly), its chroma planes subsampled by `ss`.
    pub(crate) fn new(qindex: u8, ss: (u32, u32)) -> Self {
        let q = i32::from(qindex);
        let chroma_delta = match ss {
            (1, 1) => {
                let delta = -((q / 2) - 14).clamp(0, 16);
                (delta, delta)
            }
            _ => (0, (q / 2).clamp(0, 24)),
        };
        Self {
            qindex,
            chroma_delta,
            steps: [(0, 0); 2],
        }
        .at(qindex)
    }

    /// The quantizers of a superblock whose `CurrentQIndex` is `qindex`,
    /// with the frame's chroma offsets.
    pub(crate) fn at(self, qindex: u8) -> Self {
        let q = i32::from(qindex);
        let (dc, ac) = self.chroma_delta;
        let steps = |dc: i32, ac: i32| (lookup(&DC_QLOOKUP, q + dc), lookup(&AC_QLOOKUP, q + ac));
        Self {
            qindex,
            steps: [steps(0, 0), steps(dc, ac)],
            ..self
        }
    }

    /// The quantizer index: `base_q_idx`, or a superblock's.
    pub(crate) const fn qindex(self) -> u8 {
        self.qindex
    }

    /// `DeltaQUDc` and `DeltaQUAc`.
    pub(crate) const fn chroma_delta(self) -> (i32, i32) {
        self.chroma_delta
    }

    /// Whether blocks are lossless: index 0, no delta (`Lossless`).
    pub(crate) const fn lossless(self) -> bool {
        self.qindex == 0
    }

    /// The weight of a bit against squared sample error in coding
    /// decisions: a multiple of the squared step of a luma AC coefficient
    /// in the sample domain (the inverse transforms divide by 8).
    pub(crate) fn lambda(self) -> f64 {
        let step = f64::from(self.steps[0].1) / 8.0;
        LAMBDA_SCALE * step * step
    }

    /// `dqDenom`: how much the dequantizer divides the coefficients of
    /// large transforms.
    fn denominator(tx: Square) -> i32 {
        match tx {
            Square::S32 => 2,
            Square::S64 => 4,
            _ => 1,
        }
    }

    /// The quantizer of coefficient `index` of `plane`: DC first.
    fn step(self, plane: usize, index: usize) -> i32 {
        let (dc, ac) = self.steps[usize::from(plane > 0)];
        if index == 0 { dc } else { ac }
    }

    /// The levels of transform coefficients (scaled as `forward_2d`
    /// makes them), each kept small enough that its dequantized value
    /// fits the decoder's range.
    pub(crate) fn quantize(self, plane: usize, coefficients: &[f32], tx: Square) -> Vec<i32> {
        let denominator = Self::denominator(tx);
        coefficients
            .iter()
            .enumerate()
            .map(|(index, &c)| {
                let step = self.step(plane, index);
                let rounding = if index == 0 { DC_ROUNDING } else { AC_ROUNDING };
                let limit = (32767 * denominator) / step;
                let level = ((c.abs() / step as f32 + rounding) as i32).min(limit);
                if c < 0.0 { -level } else { level }
            })
            .collect()
    }

    /// The dequantized coefficients of `levels` (section 7.12.3, step 1),
    /// the input of the 2D inverse transform.
    pub(crate) fn dequantize(self, plane: usize, levels: &[i32], tx: Square) -> Vec<i32> {
        let denominator = Self::denominator(tx);
        let limit = 1 << 15;
        levels
            .iter()
            .enumerate()
            .map(|(index, &level)| {
                let dq = i64::from(level) * i64::from(self.step(plane, index));
                let magnitude = ((dq.abs() & 0xFF_FFFF) / i64::from(denominator)) as i32;
                let dq2 = if dq < 0 { -magnitude } else { magnitude };
                dq2.clamp(-limit, limit - 1)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantizers_come_from_the_lookups() {
        let q = Quantizer::new(0, (1, 1));
        assert!(q.lossless());
        assert_eq!(q.steps, [(4, 4), (4, 4)]);
        assert_eq!(q.chroma_delta(), (0, 0));
        let q = Quantizer::new(255, (0, 0));
        assert_eq!(q.steps, [(1336, 1828), (1336, 1828)]);
        assert!(!q.lossless());
    }

    #[test]
    fn chroma_follows_the_image_quality_tuning() {
        // 4:2:0: chroma 16 indices finer from index 60, ramping down.
        assert_eq!(Quantizer::new(76, (1, 1)).chroma_delta(), (-16, -16));
        assert_eq!(Quantizer::new(32, (1, 1)).chroma_delta(), (-2, -2));
        assert_eq!(Quantizer::new(20, (1, 1)).chroma_delta(), (0, 0));
        let q = Quantizer::new(76, (1, 1));
        assert_eq!(
            q.steps[1],
            (i32::from(DC_QLOOKUP[60]), i32::from(AC_QLOOKUP[60]))
        );
        // 4:4:4: chroma AC up to 24 indices coarser.
        assert_eq!(Quantizer::new(20, (0, 0)).chroma_delta(), (0, 10));
        assert_eq!(Quantizer::new(200, (0, 0)).chroma_delta(), (0, 24));
    }

    #[test]
    fn levels_round_with_a_dead_zone() {
        let q = Quantizer::new(100, (1, 1));
        let step = q.steps[0].1 as f32;
        let levels = q.quantize(0, &[0.0, step * 0.6, -step * 0.6, step * 2.7], Square::S8);
        assert_eq!(&levels[1..], [0, 0, 3]);
        assert_eq!(q.quantize(0, &[step * 0.7, step * 0.7], Square::S8)[1], 1);
    }

    #[test]
    fn dequantized_values_stay_in_range() {
        let q = Quantizer::new(255, (1, 1));
        let levels = q.quantize(1, &[1e9, -1e9], Square::S64);
        let dequant = q.dequantize(1, &levels, Square::S64);
        assert!(dequant.iter().all(|v| v.abs() <= 1 << 15));
        // Large transforms divide by their dqDenom, rounding to zero.
        let q = Quantizer::new(10, (1, 1));
        let (dc, ac) = q.steps[0];
        assert_eq!(
            q.dequantize(0, &[-3, 3], Square::S32),
            [-(3 * dc) / 2, 3 * ac / 2]
        );
    }
}
