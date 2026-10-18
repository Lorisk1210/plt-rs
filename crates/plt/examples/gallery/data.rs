//! Reproducible sample data for the gallery: a `SplitMix64` generator with
//! normally distributed draws, so every run draws the same figures.

/// A small deterministic random number generator (`SplitMix64`).
pub(crate) struct Samples(u64);

impl Samples {
    /// A generator starting from `seed`.
    pub(crate) fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// A uniform draw in `[0, 1)`.
    pub(crate) fn uniform(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        // The top 53 bits, exactly representable.
        let bits = u32::try_from((z ^ (z >> 31)) >> 32).unwrap_or(u32::MAX);
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
