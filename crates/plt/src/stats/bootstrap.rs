//! Bootstrapped confidence intervals of the median (the `bootstrap`
//! option of matplotlib's `cbook.boxplot_stats`).

use plt_core::rng::SplitMix64;

use super::StatsError;
use super::summary::{median, percentile_array};

/// The most values the resamples of one dataset may draw in total
/// (resamples × samples), which bounds the time a bootstrap takes;
/// beyond, [`boxplot_stats`](super::boxplot_stats) returns
/// [`StatsError::TooManyBootstrapDraws`].
pub const MAX_BOOTSTRAP_DRAWS: usize = 100_000_000;

/// Notches from a bootstrap of the median (matplotlib's `bootstrap=N`):
/// each dataset is resampled with replacement `resamples` times, and the
/// notches are the 2.5th and 97.5th percentiles of the resamples' medians.
///
/// matplotlib draws the resamples from numpy's global random generator,
/// so its notches change from run to run. plt-rs draws them from a
/// `SplitMix64` generator started at `seed`, once for all the datasets:
/// the same seed gives the same notches, which differ from any of
/// matplotlib's.
///
/// ```
/// use plt::stats::{Bootstrap, BoxplotStatsOptions, boxplot_stats};
/// let data: Vec<f64> = (0..200).map(|i| f64::from(i % 17)).collect();
/// let options = BoxplotStatsOptions {
///     bootstrap: Some(Bootstrap { resamples: 1000, seed: 7 }),
///     ..BoxplotStatsOptions::default()
/// };
/// let stats = &boxplot_stats(&[&data], options.clone())?[0];
/// assert!(stats.cilo <= stats.med && stats.med <= stats.cihi);
/// assert_eq!(boxplot_stats(&[&data], options)?[0], *stats, "seeded");
/// # Ok::<(), plt::stats::StatsError>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Bootstrap {
    /// How many times each dataset is resampled (matplotlib's `N`); at
    /// least one.
    pub resamples: usize,
    /// Where the generator drawing the resamples starts.
    pub seed: u64,
}

impl Bootstrap {
    /// The generator drawing the resamples.
    pub(super) const fn generator(&self) -> SplitMix64 {
        SplitMix64::new(self.seed)
    }

    /// The 95 % confidence interval of the median of `data`, from
    /// resamples drawn with `rng` (matplotlib's `_bootstrap_median`): NaN
    /// when the data hold NaN.
    pub(super) fn median_interval(
        &self,
        data: &[f64],
        rng: &mut SplitMix64,
    ) -> Result<(f64, f64), StatsError> {
        if self.resamples == 0 {
            return Err(StatsError::NoResamples);
        }
        let draws = self.resamples.saturating_mul(data.len());
        if draws > MAX_BOOTSTRAP_DRAWS {
            return Err(StatsError::TooManyBootstrapDraws(draws));
        }
        let mut resample = vec![0.0; data.len()];
        let medians: Vec<f64> = (0..self.resamples)
            .map(|_| {
                for value in &mut resample {
                    *value = data[rng.below(data.len())];
                }
                median(&resample)
            })
            .collect();
        let [low, high] = percentile_array(&medians, [2.5, 97.5])?;
        Ok((low, high))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interval(data: &[f64], resamples: usize, seed: u64) -> Result<(f64, f64), StatsError> {
        let bootstrap = Bootstrap { resamples, seed };
        bootstrap.median_interval(data, &mut bootstrap.generator())
    }

    #[test]
    fn intervals_lie_within_the_data_around_the_median() {
        let data: Vec<f64> = (0..101).map(|i| f64::from(i * i % 37)).collect();
        let (low, high) = interval(&data, 500, 1).unwrap();
        let med = median(&data);
        assert!(low <= med && med <= high, "{low} <= {med} <= {high}");
        assert!(low >= 0.0 && high <= 36.0);
        assert_eq!(interval(&data, 500, 1), Ok((low, high)), "seeded");
    }

    /// The half-width matches the large-sample theory of the median:
    /// `1.96 / (2 f(m) √n)` for a density `f` at the median, 1 for the
    /// uniform distribution sampled here.
    #[test]
    fn the_width_follows_the_theory_of_the_median() {
        let n = 1001;
        let data: Vec<f64> = (0..n).map(|i| f64::from(i) / f64::from(n - 1)).collect();
        let (low, high) = interval(&data, 2000, 3).unwrap();
        let expected = 1.96 / (2.0 * f64::from(n).sqrt());
        let half_width = (high - low) / 2.0;
        assert!(
            (half_width / expected - 1.0).abs() < 0.25,
            "{half_width} against {expected}"
        );
        assert!((f64::midpoint(low, high) - 0.5).abs() < 0.01);
    }

    #[test]
    fn degenerate_data_and_resamples() {
        assert_eq!(interval(&[4.0; 9], 50, 0), Ok((4.0, 4.0)), "constant");
        assert_eq!(interval(&[5.0], 50, 0), Ok((5.0, 5.0)), "one value");
        let (low, high) = interval(&[1.0, 2.0, 3.0, 10.0], 1, 0).unwrap();
        assert_eq!(low, high, "one resample: one median");
        let (low, high) = interval(&[1.0, f64::NAN, 3.0], 50, 0).unwrap();
        assert!(low.is_nan() && high.is_nan());
        assert_eq!(interval(&[1.0, 2.0], 0, 0), Err(StatsError::NoResamples));
        assert_eq!(
            interval(&[0.0; 1000], MAX_BOOTSTRAP_DRAWS, 0),
            Err(StatsError::TooManyBootstrapDraws(
                1000 * MAX_BOOTSTRAP_DRAWS
            ))
        );
        assert_eq!(
            interval(&[0.0; 3], usize::MAX, 0),
            Err(StatsError::TooManyBootstrapDraws(usize::MAX)),
            "saturating"
        );
    }
}
