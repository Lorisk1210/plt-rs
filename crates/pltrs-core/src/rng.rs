//! `SplitMix64`: the small deterministic generator behind every
//! reproducible pseudo-random stream in pltrs (the spec's sample data,
//! fuzz-style tests, retried port numbers).

/// Sebastiano Vigna's `SplitMix64` generator: a 64-bit state advanced by
/// the golden-ratio increment and mixed into each output.
///
/// ```
/// use pltrs_core::rng::SplitMix64;
/// let mut rng = SplitMix64::new(42);
/// let first = rng.next_u64();
/// assert_eq!(SplitMix64::new(42).next_u64(), first);
/// assert!(rng.below(6) < 6);
/// assert!((0.0..1.0).contains(&rng.next_f64()));
/// ```
#[derive(Clone, Debug)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// A generator starting from `seed`.
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// The next 64-bit output.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A uniform draw in `[0, 1)` from the top 53 bits of the next output.
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1_u64 << 53) as f64)
    }

    /// The next output reduced modulo `bound`: a number in `0..bound`.
    ///
    /// # Panics
    ///
    /// When `bound` is zero.
    pub fn below(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_reference_stream() {
        // Vigna's reference outputs for seed 1234567.
        let mut rng = SplitMix64::new(1_234_567);
        assert_eq!(rng.next_u64(), 6_457_827_717_110_365_317);
        assert_eq!(rng.next_u64(), 3_203_168_211_198_807_973);
        assert_eq!(rng.next_u64(), 9_817_491_932_198_370_423);
    }

    #[test]
    fn floats_and_bounds() {
        let mut rng = SplitMix64::new(7);
        for _ in 0..1000 {
            assert!((0.0..1.0).contains(&rng.next_f64()));
            assert!(rng.below(3) < 3);
        }
    }
}
