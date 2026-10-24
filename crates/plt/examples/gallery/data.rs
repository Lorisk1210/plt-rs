//! Reproducible sample data for the gallery: a `SplitMix64` generator with
//! normally distributed draws, so every run draws the same figures.

use plt_core::rng::SplitMix64;

/// Normally distributed draws from a `SplitMix64` stream.
pub(crate) struct Samples(SplitMix64);

impl Samples {
    /// A generator starting from `seed`.
    pub(crate) fn new(seed: u64) -> Self {
        Self(SplitMix64::new(seed))
    }

    /// A uniform draw in `[0, 1)` from the top 32 bits of the next output.
    pub(crate) fn uniform(&mut self) -> f64 {
        let bits = u32::try_from(self.0.next_u64() >> 32).unwrap_or(u32::MAX);
        f64::from(bits) / 4_294_967_296.0
    }

    /// A draw from the normal distribution with `mean` and `sd`
    /// (Box–Muller).
    pub(crate) fn normal(&mut self, mean: f64, sd: f64) -> f64 {
        let u = 1.0 - self.uniform();
        let v = self.uniform();
        mean + sd * (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos()
    }

    /// `n` normal draws.
    pub(crate) fn normals(&mut self, n: usize, mean: f64, sd: f64) -> Vec<f64> {
        (0..n).map(|_| self.normal(mean, sd)).collect()
    }
}
