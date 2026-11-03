//! The forward transforms. Each inverse transform of the specification
//! approximates `√(N / 2)` times an orthonormal matrix, and a square 2D
//! inverse, with its shifts, rounds the orthonormal inverse of its input
//! divided by 8; the forward transform is the orthonormal one, times 8.
//! Lossless blocks use libaom's forward Walsh-Hadamard transform, the
//! exact inverse of the specification's.

use std::f32::consts::PI;

use super::{Kind, TxType};

/// The gain that makes the orthonormal forward transform the inverse of
/// the specification's 2D inverse transform.
const GAIN: f32 = 8.0;

/// The orthonormal bases of the transforms, row `k` holding basis
/// function `k`.
#[derive(Debug)]
pub(crate) struct Bases {
    /// The DCTs of 4 to 64 samples.
    dct: [Vec<f32>; 5],
    /// The ADSTs of 4 to 16 samples.
    adst: [Vec<f32>; 3],
}

impl Bases {
    /// The bases of every transform the encoder uses.
    pub(crate) fn new() -> Self {
        Self {
            dct: [2, 3, 4, 5, 6].map(|log2| basis(1 << log2, dct_basis)),
            adst: [2, 3, 4].map(|log2| basis(1 << log2, adst_basis)),
        }
    }

    /// The basis of `kind` over `2^log2` samples; `None` for the identity.
    fn matrix(&self, kind: Kind, log2: u32) -> Option<&[f32]> {
        let index = log2 as usize - 2;
        match kind {
            Kind::Dct => Some(&self.dct[index]),
            Kind::Adst => Some(&self.adst[index]),
            Kind::Identity => None,
        }
    }
}

/// The `n × n` matrix of `f(n, k, i)`, basis function `k` at sample `i`.
fn basis(n: usize, f: fn(usize, usize, usize) -> f32) -> Vec<f32> {
    (0..n * n).map(|index| f(n, index / n, index % n)).collect()
}

/// The orthonormal DCT-II.
fn dct_basis(n: usize, k: usize, i: usize) -> f32 {
    let scale = if k == 0 {
        (1.0 / n as f32).sqrt()
    } else {
        (2.0 / n as f32).sqrt()
    };
    scale * (PI * (2 * i + 1) as f32 * k as f32 / (2 * n) as f32).cos()
}

/// AV1's ADSTs: the DST-VII for 4 samples, the DST-IV beyond.
fn adst_basis(n: usize, k: usize, i: usize) -> f32 {
    if n == 4 {
        2.0 / 3.0 * (PI * (2 * k + 1) as f32 * (i + 1) as f32 / 9.0).sin()
    } else {
        (2.0 / n as f32).sqrt() * (PI * ((2 * i + 1) * (2 * k + 1)) as f32 / (4 * n) as f32).sin()
    }
}

/// The 1D forward transform of `input` (`stride` apart) into `output`:
/// the first `output.len()` coefficients.
fn transform_1d(matrix: Option<&[f32]>, input: &[f32], output: &mut [f32]) {
    let n = input.len();
    match matrix {
        Some(m) => {
            for (k, out) in output.iter_mut().enumerate() {
                let row = &m[k * n..(k + 1) * n];
                *out = row.iter().zip(input).map(|(b, x)| b * x).sum();
            }
        }
        None => output.copy_from_slice(&input[..output.len()]),
    }
}

/// The forward 2D transform of a square residual of `2^log2` samples a
/// side (row by row): the coefficients a decoder codes, `Min(32, side)`
/// to a row and column (row `i` holds vertical frequency `i`), scaled to
/// the specification's inverse.
pub(crate) fn forward_2d(bases: &Bases, residual: &[i32], log2: u32, tx_type: TxType) -> Vec<f32> {
    let side = 1usize << log2;
    let coded = side.min(32);
    let rows = bases.matrix(tx_type.row(), log2);
    let columns = bases.matrix(tx_type.column(), log2);
    // Horizontal frequencies of each row.
    let mut horizontal = vec![0.0; side * coded];
    let mut line = vec![0.0; side];
    for (y, out) in horizontal.chunks_exact_mut(coded).enumerate() {
        for (v, &r) in line.iter_mut().zip(&residual[y * side..(y + 1) * side]) {
            *v = r as f32;
        }
        transform_1d(rows, &line, out);
    }
    let mut coefficients = vec![0.0; coded * coded];
    let mut column = vec![0.0; coded];
    for j in 0..coded {
        for (y, v) in line.iter_mut().enumerate() {
            *v = horizontal[y * coded + j];
        }
        transform_1d(columns, &line, &mut column);
        for (i, &c) in column.iter().enumerate() {
            coefficients[i * coded + j] = c * GAIN;
        }
    }
    coefficients
}

/// libaom's forward Walsh-Hadamard transform of a 4×4 residual (row by
/// row), its coefficients already multiplied by the lossless quantizer 4.
pub(crate) fn walsh_hadamard_4x4(residual: &[i32]) -> [i32; 16] {
    let lift = |[mut a, mut b, mut c, mut d]: [i32; 4]| {
        a += b;
        d -= c;
        let e = (a - d) >> 1;
        b = e - b;
        c = e - c;
        a -= c;
        d += b;
        [a, c, d, b]
    };
    let mut temp = [0; 16];
    for x in 0..4 {
        let column = lift([0, 1, 2, 3].map(|y| residual[y * 4 + x]));
        for (y, v) in column.into_iter().enumerate() {
            temp[y * 4 + x] = v;
        }
    }
    let mut out = [0; 16];
    for y in 0..4 {
        let row = lift([0, 1, 2, 3].map(|x| temp[y * 4 + x]));
        for (x, v) in row.into_iter().enumerate() {
            out[y * 4 + x] = v * 4;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::inverse::{adst, dct, identity, inverse_2d};
    use super::*;

    /// The basis function `k` the specification's inverse produces,
    /// divided by its `√(N / 2)` gain.
    fn inverse_column(kind: Kind, log2: u32, k: usize) -> Vec<f32> {
        let n = 1 << log2;
        let mut t = vec![0; n];
        t[k] = 1 << 16;
        match kind {
            Kind::Dct => dct(&mut t, log2, 24),
            Kind::Adst => adst(&mut t, log2, 24),
            Kind::Identity => identity(&mut t, log2),
        }
        let gain = (n as f32 / 2.0).sqrt() * 65536.0;
        t.iter().map(|&v| v as f32 / gain).collect()
    }

    #[test]
    fn bases_match_the_specification_inverses() {
        let bases = Bases::new();
        for (kind, sizes) in [
            (Kind::Dct, 2..=6),
            (Kind::Adst, 2..=4),
            (Kind::Identity, 2..=5),
        ] {
            for log2 in sizes {
                let n = 1 << log2;
                for k in 0..n {
                    let expected: Vec<f32> = match bases.matrix(kind, log2) {
                        Some(m) => m[k * n..(k + 1) * n].to_vec(),
                        None => (0..n).map(|i| f32::from(u8::from(i == k))).collect(),
                    };
                    let got = inverse_column(kind, log2, k);
                    for (g, e) in got.iter().zip(&expected) {
                        assert!((g - e).abs() < 2e-3, "{kind:?} {n} basis {k}: {got:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn bases_are_orthonormal() {
        let bases = Bases::new();
        for m in bases.dct.iter().chain(&bases.adst) {
            let n = m.len().isqrt();
            for a in 0..n {
                for b in 0..n {
                    let dot: f32 = (0..n).map(|i| m[a * n + i] * m[b * n + i]).sum();
                    let expected = if a == b { 1.0 } else { 0.0 };
                    assert!(
                        (dot - expected).abs() < 1e-4,
                        "n {n}, rows {a} and {b}: {dot}"
                    );
                }
            }
        }
    }

    /// A residual with edges and gradients, in `[-255, 255]`.
    fn residual(side: usize) -> Vec<i32> {
        (0..side * side)
            .map(|i| {
                let (y, x) = ((i / side) as i32, (i % side) as i32);
                ((x * 37 + y * 11) % 200 - 100) + if x > y { 90 } else { -40 }
            })
            .collect()
    }

    #[test]
    fn forward_then_inverse_reconstructs() {
        let bases = Bases::new();
        let types = [
            TxType::DctDct,
            TxType::AdstDct,
            TxType::DctAdst,
            TxType::AdstAdst,
            TxType::Identity,
        ];
        for log2 in 2..=6 {
            for &tx_type in &types {
                let allowed = match tx_type {
                    TxType::DctDct => true,
                    TxType::Identity => log2 <= 5,
                    _ => log2 <= 4,
                };
                if !allowed {
                    continue;
                }
                let side = 1 << log2;
                // 64×64 blocks keep 32×32 coefficients: a smooth residual.
                let input = if log2 == 6 {
                    (0..side * side)
                        .map(|i| (i % side) as i32 * 3 - 90)
                        .collect()
                } else {
                    residual(side)
                };
                let coefficients = forward_2d(&bases, &input, log2, tx_type);
                // The dequantizer's `dqDenom`.
                let denominator = [1.0, 1.0, 1.0, 2.0, 4.0][log2 as usize - 2];
                let dequant: Vec<i32> = coefficients
                    .iter()
                    .map(|c| (c / denominator).round() as i32)
                    .collect();
                let output = inverse_2d(&dequant, log2, tx_type, false);
                let worst = input.iter().zip(&output).map(|(a, b)| (a - b).abs()).max();
                let tolerance = if log2 == 6 { 3 } else { 1 };
                assert!(
                    worst <= Some(tolerance),
                    "{tx_type:?} {side}: off by {worst:?}"
                );
            }
        }
    }

    #[test]
    fn walsh_hadamard_round_trips_exactly() {
        let mut state = 0x1234_5678u32;
        for _ in 0..200 {
            let input: Vec<i32> = (0..16)
                .map(|_| {
                    state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    (state >> 16) as i32 % 511 - 255
                })
                .collect();
            let coefficients = walsh_hadamard_4x4(&input);
            assert!(coefficients.iter().all(|c| c % 4 == 0));
            let output = inverse_2d(&coefficients, 2, TxType::DctDct, true);
            assert_eq!(output, input);
        }
    }
}
