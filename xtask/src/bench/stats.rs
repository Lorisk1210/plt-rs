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

/// The times of a measurement's runs, in seconds.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Samples {
    /// The times, in increasing order.
    sorted: Vec<f64>,
}

impl Samples {
    /// The samples of `times`, or `None` when there are none or one is not
    /// finite.
    pub(super) fn new(mut times: Vec<f64>) -> Option<Self> {
        if times.is_empty() || !times.iter().all(|t| t.is_finite()) {
            return None;
        }
        times.sort_by(f64::total_cmp);
        Some(Self { sorted: times })
    }

    /// The median run.
    pub(super) fn median(&self) -> f64 {
        self.quantile(0.5)
    }

    /// The first and third quartiles: the middle half of the runs lies
    /// between them.
    pub(super) fn quartiles(&self) -> (f64, f64) {
        (self.quantile(0.25), self.quantile(0.75))
    }

    /// The `q`-quantile, interpolated linearly between the closest runs
    /// (numpy's default method).
    fn quantile(&self, q: f64) -> f64 {
        let position = q * (self.sorted.len() - 1) as f64;
        let below = position.floor();
        let lower = self.sorted[below as usize];
        let upper = self.sorted[(below as usize + 1).min(self.sorted.len() - 1)];
        lower + (upper - lower) * (position - below)
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
    let factor = ratio.max(1.0 / ratio);
    let shown = if factor < 1.995 {
        format!("{factor:.2}")
    } else if factor < 9.95 {
        format!("{factor:.1}")
    } else {
        format!("{factor:.0}")
    };
    if shown == "1.00" {
        "as fast".to_owned()
    } else if ratio < 1.0 {
        format!("{shown}× faster")
    } else {
        format!("{shown}× slower")
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
}
