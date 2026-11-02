//! How many times a measurement runs, and what its times say.

/// How many runs a measurement takes: at least `min_runs`, and more until
/// they add up to `min_seconds`, but never more than `max_runs`. Quick
/// workloads run more often, so that their medians settle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Sampling {
    /// The fewest runs.
    pub(super) min_runs: usize,
    /// The least total time, in seconds, the runs add up to.
    pub(super) min_seconds: f64,
    /// The most runs.
    pub(super) max_runs: usize,
}

impl Sampling {
    /// Whether another run is wanted after `runs` runs that took
    /// `seconds` in all.
    pub(super) fn wants_more(&self, runs: usize, seconds: f64) -> bool {
        runs < self.max_runs && (runs < self.min_runs || seconds < self.min_seconds)
    }
}

/// The times of a measurement's runs, in seconds, in the order they ran.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Samples {
    /// The times, in run order.
    runs: Vec<f64>,
}

impl Samples {
    /// The samples of `times`, or `None` when there are none or one is not
    /// finite.
    pub(super) fn new(times: Vec<f64>) -> Option<Self> {
        if times.is_empty() || !times.iter().all(|t| t.is_finite()) {
            return None;
        }
        Some(Self { runs: times })
    }

    /// The times in run order.
    pub(super) fn runs(&self) -> &[f64] {
        &self.runs
    }

    /// The median run.
    pub(super) fn median(&self) -> f64 {
        median(&self.runs)
    }

    /// The first and third quartiles: the middle half of the runs lies
    /// between them.
    pub(super) fn quartiles(&self) -> (f64, f64) {
        let sorted = sorted(&self.runs);
        (quantile(&sorted, 0.25), quantile(&sorted, 0.75))
    }
}

/// `values` in increasing order.
fn sorted(values: &[f64]) -> Vec<f64> {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted
}

/// The median of `values` (not empty).
fn median(values: &[f64]) -> f64 {
    quantile(&sorted(values), 0.5)
}

/// The `q`-quantile of the increasing `sorted`, interpolated linearly
/// between the closest values (numpy's default method).
fn quantile(sorted: &[f64], q: f64) -> f64 {
    let position = q * (sorted.len() - 1) as f64;
    let below = position.floor();
    let lower = sorted[below as usize];
    let upper = sorted[(below as usize + 1).min(sorted.len() - 1)];
    lower + (upper - lower) * (position - below)
}

/// How long one implementation takes against another: the ratio of the
/// medians of their times, with a 95% confidence interval.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Ratio {
    /// The other's median over the reference's: below 1 when the other is
    /// faster.
    pub(super) value: f64,
    /// The lower end of the 95% confidence interval of `value`.
    pub(super) low: f64,
    /// The upper end.
    pub(super) high: f64,
}

/// How a [`Ratio`] reads once its uncertainty is taken into account.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Verdict {
    /// Faster: the whole interval lies below 1.
    Faster,
    /// The interval holds 1: the difference is within the noise.
    Tied,
    /// Slower: the whole interval lies above 1.
    Slower,
}

impl Ratio {
    /// The ratio of the median of `other` to that of `reference`, with a
    /// percentile bootstrap interval: [`RESAMPLES`] resamples of the runs,
    /// drawn in pairs (run `i` of both) when both ran equally often, as
    /// the interleaved rounds of the benchmark make them, else apart. The
    /// resampling is seeded, so the interval is the same for the same
    /// times.
    pub(super) fn of(reference: &Samples, other: &Samples) -> Self {
        let value = other.median() / reference.median();
        let (a, b) = (reference.runs(), other.runs());
        let paired = a.len() == b.len();
        let mut rng = SplitMix64(0x5EED_BE4C);
        let mut ratios: Vec<f64> = (0..RESAMPLES)
            .map(|_| {
                let first: Vec<usize> = (0..a.len()).map(|_| rng.below(a.len())).collect();
                let second: Vec<usize> = if paired {
                    first.clone()
                } else {
                    (0..b.len()).map(|_| rng.below(b.len())).collect()
                };
                let pick = |values: &[f64], at: &[usize]| -> Vec<f64> {
                    at.iter().map(|&i| values[i]).collect()
                };
                median(&pick(b, &second)) / median(&pick(a, &first))
            })
            .collect();
        ratios.sort_by(f64::total_cmp);
        Self {
            value,
            low: quantile(&ratios, 0.025),
            high: quantile(&ratios, 0.975),
        }
    }

    /// Whether the other is faster, slower, or within the noise.
    pub(super) fn verdict(&self) -> Verdict {
        if self.high < 1.0 {
            Verdict::Faster
        } else if self.low > 1.0 {
            Verdict::Slower
        } else {
            Verdict::Tied
        }
    }
}

/// How many resamples a bootstrap interval draws.
pub(super) const RESAMPLES: usize = 2000;

/// The `SplitMix64` generator: a seeded stream for the bootstrap.
struct SplitMix64(u64);

impl SplitMix64 {
    /// The next 64 random bits.
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in `0..n` (n > 0), by multiplying out the top bits.
    fn below(&mut self, n: usize) -> usize {
        ((u128::from(self.next() >> 32) * n as u128) >> 32) as usize
    }
}

/// The geometric mean of `ratios`, the fair average of speed ratios;
/// `None` for none.
pub(super) fn geometric_mean(ratios: &[f64]) -> Option<f64> {
    if ratios.is_empty() {
        return None;
    }
    let logs: f64 = ratios.iter().map(|r| r.ln()).sum();
    Some((logs / ratios.len() as f64).exp())
}

/// A time `ratio` (a median over matplotlib's) as a speed-up, worded so
/// that it cannot be misread: `4.2× faster` when matplotlib takes 4.2 times
/// as long, `1.27× slower` when this takes 1.27 times as long as
/// matplotlib, `as fast` when they agree to within half a percent. Factors
/// under 2 keep two decimals, under 10 one, larger ones none.
pub(super) fn speedup(ratio: f64) -> String {
    let shown = factor(ratio.max(1.0 / ratio));
    if shown == "1.00" {
        "as fast".to_owned()
    } else if ratio < 1.0 {
        format!("{shown}× faster")
    } else {
        format!("{shown}× slower")
    }
}

/// The speed-up factor matplotlib's time is of this one (`1 / ratio`),
/// rounded as [`speedup`] rounds: `4.2` for a ratio of 1/4.2, `0.79` for a
/// ratio of 1.27.
pub(super) fn factor_of(ratio: f64) -> String {
    factor(1.0 / ratio)
}

/// `value` with two decimals under 2, one under 10, none above.
fn factor(value: f64) -> String {
    if value < 1.995 {
        format!("{value:.2}")
    } else if value < 9.95 {
        format!("{value:.1}")
    } else {
        format!("{value:.0}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speedups_say_faster_or_slower() {
        assert_eq!(speedup(1.0 / 4.2), "4.2× faster");
        assert_eq!(speedup(1.27), "1.27× slower");
        assert_eq!(speedup(0.8), "1.25× faster");
        assert_eq!(speedup(0.02), "50× faster");
        assert_eq!(speedup(1.004), "as fast");
        assert_eq!(speedup(1.0 / 9.96), "10× faster");
        assert_eq!(speedup(1.0 / 1.996), "2.0× faster");
        assert_eq!(factor_of(1.0 / 4.2), "4.2");
        assert_eq!(factor_of(1.27), "0.79");
    }

    #[test]
    fn sampling_runs_at_least_the_count_and_the_time() {
        let sampling = Sampling {
            min_runs: 5,
            min_seconds: 1.0,
            max_runs: 20,
        };
        assert!(sampling.wants_more(4, 10.0));
        assert!(!sampling.wants_more(5, 1.0));
        assert!(sampling.wants_more(5, 0.5));
        assert!(!sampling.wants_more(20, 0.1));
    }

    #[test]
    fn statistics_follow_numpy() {
        let samples = Samples::new(vec![4.0, 1.0, 3.0, 2.0]).expect("finite times");
        assert_eq!(samples.median(), 2.5);
        assert_eq!(samples.quartiles(), (1.75, 3.25));
        assert_eq!(samples.runs(), [4.0, 1.0, 3.0, 2.0]);
        let one = Samples::new(vec![7.0]).expect("one time");
        assert_eq!((one.median(), one.quartiles()), (7.0, (7.0, 7.0)));
    }

    #[test]
    fn no_samples_without_finite_times() {
        assert_eq!(Samples::new(Vec::new()), None);
        assert_eq!(Samples::new(vec![1.0, f64::NAN]), None);
    }

    #[test]
    fn geometric_mean_of_ratios() {
        let mean = geometric_mean(&[0.5, 2.0]).expect("two ratios");
        assert!((mean - 1.0).abs() < 1e-12);
        assert_eq!(geometric_mean(&[]), None);
    }

    #[test]
    fn intervals_bracket_the_ratio_and_tell_noise_from_speed() {
        let slow =
            Samples::new((0..20).map(|i| 2.0 + 0.01 * f64::from(i)).collect()).expect("times");
        let fast =
            Samples::new((0..20).map(|i| 1.0 + 0.01 * f64::from(i)).collect()).expect("times");
        let ratio = Ratio::of(&slow, &fast);
        assert!(ratio.low <= ratio.value && ratio.value <= ratio.high);
        assert_eq!(ratio.verdict(), Verdict::Faster);
        assert_eq!(Ratio::of(&fast, &slow).verdict(), Verdict::Slower);
        let noisy = Samples::new(vec![1.0, 3.0, 1.0, 3.0, 2.0]).expect("times");
        let other = Samples::new(vec![3.0, 1.0, 2.0, 1.0, 3.0]).expect("times");
        assert_eq!(Ratio::of(&noisy, &other).verdict(), Verdict::Tied);
        assert_eq!(Ratio::of(&noisy, &other), Ratio::of(&noisy, &other));
    }

    #[test]
    fn the_generator_stays_in_range() {
        let mut rng = SplitMix64(1);
        assert!((0..1000).all(|_| rng.below(7) < 7));
    }
}
