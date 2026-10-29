//! Example graphs drawn with pltrs, one binary per graph in `src/bin/`.
//!
//! Each binary draws one figure and saves it as `<name>.png` (the
//! animation as `<name>.gif`) in the working directory:
//!
//! ```text
//! cargo run -p pltrs-examples --bin line_plot
//! ```
//!
//! `examples/python/` holds the same graphs written with pltrs
//! (Python), in matplotlib's syntax, and `cargo xtask examples`
//! renders both into the gallery of `examples/README.md`.
//!
//! This library holds what several examples share: [`Samples`], the seeded
//! generator of the examples that need noisy data.
#![forbid(unsafe_code)]

use pltrs_core::rng::SplitMix64;

/// Uniform and normal draws from a seeded `SplitMix64` stream.
///
/// The Python examples draw from the same generator
/// (`examples/python/samples.py`), so both languages plot the same points.
///
/// ```
/// use pltrs_examples::Samples;
///
/// let heights = Samples::new(7).normal(100, 1.75, 0.1);
/// assert_eq!(heights.len(), 100);
/// assert_eq!(heights, Samples::new(7).normal(100, 1.75, 0.1));
/// ```
#[derive(Clone, Debug)]
pub struct Samples(SplitMix64);

impl Samples {
    /// A stream starting from `seed`.
    pub const fn new(seed: u64) -> Self {
        Self(SplitMix64::new(seed))
    }

    /// `n` draws, uniform in `[low, high)`.
    pub fn uniform(&mut self, n: usize, low: f64, high: f64) -> Vec<f64> {
        (0..n)
            .map(|_| low + (high - low) * self.0.next_f64())
            .collect()
    }

    /// `n` normal draws of mean `mean` and standard deviation `sd`
    /// (Box–Muller, two uniform draws per normal draw).
    pub fn normal(&mut self, n: usize, mean: f64, sd: f64) -> Vec<f64> {
        (0..n)
            .map(|_| {
                let u = 1.0 - self.0.next_f64();
                let v = self.0.next_f64();
                mean + sd * (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_draws_stay_in_their_range() {
        let draws = Samples::new(1).uniform(1000, -2.0, 3.0);
        assert!(draws.iter().all(|x| (-2.0..3.0).contains(x)));
        assert_eq!(Samples::new(1).uniform(0, 0.0, 1.0), []);
    }

    #[test]
    fn normal_draws_have_their_mean_and_spread() {
        let draws = Samples::new(2).normal(20_000, 10.0, 2.0);
        let n = draws.len() as f64;
        let mean = draws.iter().sum::<f64>() / n;
        let sd = (draws.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n).sqrt();
        assert!((mean - 10.0).abs() < 0.05, "mean {mean}");
        assert!((sd - 2.0).abs() < 0.05, "sd {sd}");
        assert!(draws.iter().all(|x| x.is_finite()));
    }

    #[test]
    fn the_python_mirror_draws_the_same_numbers() {
        // `Samples(5).uniform(3)` and `Samples(5).normal(2)` in samples.py:
        // the uniform draws to the bit, the normal ones to the last bits
        // that the platforms' `log` and `cos` may round differently.
        let uniform = Samples::new(5).uniform(3, 0.0, 1.0);
        assert_eq!(
            uniform,
            [
                0.386_768_045_983_934,
                0.752_307_015_838_223_9,
                0.232_709_165_677_461_8
            ]
        );
        let normal = Samples::new(5).normal(2, 0.0, 1.0);
        for (draw, python) in normal
            .iter()
            .zip([0.014_334_745_996_352_224, 0.590_620_956_886_925_1])
        {
            assert!((draw - python).abs() < 1e-15, "{draw} != {python}");
        }
    }
}
