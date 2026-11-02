//! What the report computes from the raw data: speed-ups with their
//! intervals, and tallies of them over a set of workloads.

use super::super::implementation::Implementation;
use super::super::results::WorkloadResult;
use super::super::stats::{Ratio, Samples, Verdict, geometric_mean};

/// Warm or cold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mode {
    /// In-process, after a warm-up.
    Warm,
    /// A fresh process, end to end.
    Cold,
}

impl Mode {
    /// Both modes, warm first.
    pub(super) const ALL: [Self; 2] = [Self::Warm, Self::Cold];

    /// Its name in the report.
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::Warm => "warm",
            Self::Cold => "cold",
        }
    }
}

/// The times of `implementation` on `w` in `mode`, if measured.
pub(super) fn samples(
    w: &WorkloadResult,
    implementation: Implementation,
    mode: Mode,
) -> Option<&Samples> {
    match mode {
        Mode::Warm => w.warm.get(implementation).measured().map(|m| &m.seconds),
        Mode::Cold => w.cold.get(implementation).measured().map(|m| &m.seconds),
    }
}

/// `contender`'s time on `w` in `mode` against matplotlib's, when both
/// were measured.
pub(super) fn ratio(w: &WorkloadResult, contender: Implementation, mode: Mode) -> Option<Ratio> {
    Some(Ratio::of(
        samples(w, Implementation::Matplotlib, mode)?,
        samples(w, contender, mode)?,
    ))
}

/// How a contender did over a set of workloads.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Tally<'a> {
    /// The workloads compared, with their ratios, slowest first.
    pub(super) ratios: Vec<(&'a str, Ratio)>,
    /// How many are faster beyond the noise.
    pub(super) faster: usize,
    /// How many are within the noise.
    pub(super) tied: usize,
    /// How many are slower beyond the noise.
    pub(super) slower: usize,
}

impl<'a> Tally<'a> {
    /// `contender` against matplotlib in `mode` over `workloads`, leaving
    /// out those whose figure failed the guard, which compare different
    /// work.
    pub(super) fn of(
        workloads: impl IntoIterator<Item = &'a WorkloadResult>,
        contender: Implementation,
        mode: Mode,
    ) -> Self {
        let mut ratios: Vec<(&str, Ratio)> = workloads
            .into_iter()
            .filter(|w| !w.guard.diverges(contender))
            .filter_map(|w| Some((w.name.as_str(), ratio(w, contender, mode)?)))
            .collect();
        ratios.sort_by(|a, b| b.1.value.total_cmp(&a.1.value));
        let count = |verdict| {
            ratios
                .iter()
                .filter(|(_, r)| r.verdict() == verdict)
                .count()
        };
        Self {
            faster: count(Verdict::Faster),
            tied: count(Verdict::Tied),
            slower: count(Verdict::Slower),
            ratios,
        }
    }

    /// The geometric mean of the ratios, the typical one.
    pub(super) fn typical(&self) -> Option<f64> {
        let values: Vec<f64> = self.ratios.iter().map(|(_, r)| r.value).collect();
        geometric_mean(&values)
    }

    /// How many workloads were compared.
    pub(super) fn len(&self) -> usize {
        self.ratios.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bench::results::tests::sample;

    #[test]
    fn tallies_count_the_comparable_workloads() {
        let results = sample();
        let tally = Tally::of(&results.workloads, Implementation::Rust, Mode::Warm);
        assert_eq!(tally.len(), 1);
        assert_eq!((tally.faster, tally.tied, tally.slower), (1, 0, 0));
        let typical = tally.typical().expect("one ratio");
        assert!((typical - 0.001 / 0.020).abs() < 1e-12);
        let python = Tally::of(&results.workloads, Implementation::Python, Mode::Cold);
        assert_eq!(python.ratios[0].0, "small/line_svg");
    }
}
