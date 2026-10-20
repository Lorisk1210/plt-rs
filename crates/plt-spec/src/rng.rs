//! Bit-exact mirror of `spec_support.data.SpecRng` (`SplitMix64`).
//!
//! Figure cases draw pseudo-random data from this generator in both the
//! Python reference suite and the Rust harness; any change here must be
//! mirrored in `spec/reference/spec_support/data.py`.

/// `SplitMix64` stream with uniform, normal, gamma and integer helpers.
#[derive(Clone, Debug)]
pub struct SpecRng {
    state: u64,
}

impl SpecRng {
    /// Creates a generator from a seed.
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Next raw 64-bit output.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[0, 1)` with 53 bits of precision.
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1_u64 << 53) as f64)
    }

    /// `n` uniform samples in `[low, high)`.
    pub fn uniform(&mut self, n: usize, low: f64, high: f64) -> Vec<f64> {
        (0..n)
            .map(|_| low + (high - low) * self.next_f64())
            .collect()
    }

    /// `n` normal samples (Box–Muller, two uniforms per sample).
    pub fn normal(&mut self, n: usize, loc: f64, scale: f64) -> Vec<f64> {
        (0..n)
            .map(|_| {
                let u1 = 1.0 - self.next_f64();
                let u2 = self.next_f64();
                loc + scale * (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
            })
            .collect()
    }

    /// `n` gamma samples of integer `shape` and unit scale: each the sum
    /// of `shape` exponential samples made by inversion.
    pub fn gamma(&mut self, n: usize, shape: u32) -> Vec<f64> {
        (0..n)
            .map(|_| (0..shape).fold(0.0, |sum, _| sum - (1.0 - self.next_f64()).ln()))
            .collect()
    }

    /// `n` integers in `[low, high)` via multiply-shift.
    pub fn integers(&mut self, n: usize, low: i64, high: i64) -> Vec<i64> {
        let span = u128::from(high.abs_diff(low));
        (0..n)
            .map(|_| {
                let offset = (u128::from(self.next_u64() >> 11) * span) >> 53;
                low + offset as i64
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_python_reference_stream() {
        // Values printed by spec_support.data.rng(42) in the reference venv.
        let mut rng = SpecRng::new(42);
        assert_eq!(rng.next_u64(), 13_679_457_532_755_275_413);
        let u = rng.uniform(2, 0.0, 1.0);
        assert!((u[0] - 0.159_910_39).abs() < 1e-8 && (u[1] - 0.278_601_13).abs() < 1e-8);
        let n = rng.normal(2, 0.0, 1.0);
        assert!((n[0] - 0.892_470_39).abs() < 1e-8 && (n[1] - 0.397_051_83).abs() < 1e-8);
        assert_eq!(rng.integers(3, 0, 10), vec![8, 3, 6]);
    }

    #[test]
    fn gamma_matches_python_reference_stream() {
        // Values printed by spec_support.data.rng(7).gamma(2, 3).
        let g = SpecRng::new(7).gamma(2, 3);
        assert!((g[0] - 2.821_169_05).abs() < 1e-8 && (g[1] - 1.763_712_99).abs() < 1e-8);
        assert!(SpecRng::new(7).gamma(3, 0).iter().all(|&v| v == 0.0));
    }
}
