//! What a benchmark run measured, as the report shows it and as its raw
//! data (`benchmarks/results.json`) keeps it: every time of every run, so
//! that anyone can recompute the medians, intervals and speed-ups.

mod json;
mod read;

use super::cold::ColdResult;
use super::guard::Guarded;
use super::implementation::{Implementation, PerImplementation};
use super::machine::Machine;
use super::stats::{Samples, Sampling};
use super::workloads::Tier;

pub(super) use json::to_text;
pub(super) use read::from_text;

/// An implementation's value for one workload.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Column<T> {
    /// Measured.
    Measured(T),
    /// pltrs (Python) raised this exception: it lacks something the
    /// script uses.
    NotSupported(String),
    /// matplotlib cannot be timed doing the same work, for this reason.
    NotComparable(String),
}

impl<T> Column<T> {
    /// The measured value, if any.
    pub(super) fn measured(&self) -> Option<&T> {
        match self {
            Self::Measured(value) => Some(value),
            Self::NotSupported(_) | Self::NotComparable(_) => None,
        }
    }

    /// Why nothing was measured, as a column of another measurement; `None`
    /// when something was.
    pub(super) fn skipped<U>(&self) -> Option<Column<U>> {
        match self {
            Self::Measured(_) => None,
            Self::NotSupported(why) => Some(Column::NotSupported(why.clone())),
            Self::NotComparable(why) => Some(Column::NotComparable(why.clone())),
        }
    }
}

/// The warm runs of one implementation.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct WarmResult {
    /// The times, round by round.
    pub(super) seconds: Samples,
    /// The size of the document saved, in bytes.
    pub(super) bytes: u64,
}

/// The guard's verdicts on pltrs' two renderings of a workload.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Guard {
    /// pltrs (Python)'s.
    pub(super) python: Column<Guarded>,
    /// pltrs (Rust)'s.
    pub(super) rust: Column<Guarded>,
}

impl Guard {
    /// The verdicts on `contender`'s rendering (matplotlib's is the
    /// reference, not judged).
    pub(super) fn of(&self, contender: Implementation) -> Option<&Column<Guarded>> {
        match contender {
            Implementation::Matplotlib => None,
            Implementation::Python => Some(&self.python),
            Implementation::Rust => Some(&self.rust),
        }
    }

    /// Whether `contender`'s rendering failed a check.
    pub(super) fn diverges(&self, contender: Implementation) -> bool {
        self.of(contender)
            .and_then(Column::measured)
            .is_some_and(Guarded::diverges)
    }
}

/// Everything measured of one workload.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct WorkloadResult {
    /// `group/name`.
    pub(super) name: String,
    /// The table it belongs to.
    pub(super) tier: Tier,
    /// What it saves: an extension, `layout` or `html` (see
    /// `Output::name`).
    pub(super) format: String,
    /// How many plotting and setter calls build it, for a workload of
    /// many small calls.
    pub(super) calls: Option<u32>,
    /// The guard's verdicts.
    pub(super) guard: Guard,
    /// Warm, in-process runs.
    pub(super) warm: PerImplementation<Column<WarmResult>>,
    /// Cold, fresh-process runs with their peak memory.
    pub(super) cold: PerImplementation<Column<ColdResult>>,
}

/// A process that only starts, imports or exits.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Startup {
    /// Its key: `python` (the interpreter alone), `numpy` and
    /// `numpy-threaded` (numpy imported with BLAS on one thread and on its
    /// default of one per hardware thread), `matplotlib` and `pltrs-python`
    /// (each pyplot imported), `pltrs-rust` (the executable of pltrs (Rust)
    /// starting and exiting).
    pub(super) key: String,
    /// Its runs.
    pub(super) result: ColdResult,
}

/// How often each kind of measurement runs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Plan {
    /// Warm rounds.
    pub(super) warm: Sampling,
    /// Cold rounds of a workload.
    pub(super) cold: Sampling,
    /// Rounds of the start-up processes.
    pub(super) startup: Sampling,
}

impl Plan {
    /// At least `runs` rounds of everything (twice as many start-ups,
    /// which are quick), more for quick workloads.
    pub(super) fn new(runs: usize) -> Self {
        Self {
            warm: Sampling {
                min_runs: runs,
                min_seconds: 1.0,
                max_runs: runs.max(50),
            },
            cold: Sampling {
                min_runs: runs,
                min_seconds: 2.0,
                max_runs: runs.max(20),
            },
            startup: Sampling {
                min_runs: 2 * runs,
                min_seconds: 2.0,
                max_runs: (2 * runs).max(40),
            },
        }
    }
}

/// The versions of the Python side: `(name, version)` pairs of each venv
/// (see `bench.py versions`).
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Versions {
    /// The reference venv's.
    pub(super) reference: Vec<(String, String)>,
    /// The pltrs (Python) venv's.
    pub(super) pltrs: Vec<(String, String)>,
}

impl Versions {
    /// The reference venv's `name`, or `unknown`.
    pub(super) fn reference(&self, name: &str) -> &str {
        lookup(&self.reference, name)
    }

    /// The pltrs (Python) venv's `name`, or `unknown`.
    pub(super) fn pltrs(&self, name: &str) -> &str {
        lookup(&self.pltrs, name)
    }

    /// Whether both venvs run the same interpreter and the same numpy
    /// binary, as the drop-in claim requires.
    pub(super) fn same_python_and_numpy(&self) -> bool {
        ["interpreter", "python", "numpy", "numpy core"]
            .iter()
            .all(|name| self.reference(name) == self.pltrs(name) && self.pltrs(name) != "unknown")
    }
}

/// The value of `name` among `pairs`, or `unknown`.
fn lookup<'a>(pairs: &'a [(String, String)], name: &str) -> &'a str {
    pairs
        .iter()
        .find(|(key, _)| key == name)
        .map_or("unknown", |(_, value)| value.as_str())
}

/// Everything a benchmark run measured.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Results {
    /// The machine and tools.
    pub(super) machine: Machine,
    /// The Python side's versions.
    pub(super) versions: Versions,
    /// The load averages when the run started.
    pub(super) load_before: String,
    /// The load averages when it ended.
    pub(super) load_after: String,
    /// How often things ran.
    pub(super) plan: Plan,
    /// Start-up and import times.
    pub(super) startup: Vec<Startup>,
    /// The workloads, in table order.
    pub(super) workloads: Vec<WorkloadResult>,
}

impl Results {
    /// The start-up process `key`.
    pub(super) fn startup(&self, key: &str) -> Option<&ColdResult> {
        self.startup
            .iter()
            .find(|s| s.key == key)
            .map(|s| &s.result)
    }

    /// Whether some workload could not run under pltrs (Python).
    pub(super) fn has_unsupported(&self) -> bool {
        self.workloads
            .iter()
            .any(|w| matches!(w.warm.python, Column::NotSupported(_)))
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::bench::guard::Verdict;

    /// A small but complete result set, for the tests of the raw data and
    /// the report.
    pub(in crate::bench) fn sample() -> Results {
        let cold = |seconds: Vec<f64>| ColdResult {
            peaks_kib: vec![1024; seconds.len()],
            seconds: Samples::new(seconds).expect("times"),
        };
        let warm = |seconds: Vec<f64>| {
            Column::Measured(WarmResult {
                seconds: Samples::new(seconds).expect("times"),
                bytes: 1000,
            })
        };
        let same = Column::Measured(Guarded {
            pixels: Verdict::Same(0.0),
            structure: Some(Verdict::Same(0.001)),
        });
        let line = WorkloadResult {
            name: "small/line_svg".to_owned(),
            tier: Tier::Small,
            format: "svg".to_owned(),
            calls: None,
            guard: Guard {
                python: same.clone(),
                rust: same,
            },
            warm: PerImplementation {
                matplotlib: warm(vec![0.020, 0.021, 0.019]),
                python: warm(vec![0.004, 0.004, 0.005]),
                rust: warm(vec![0.001, 0.001, 0.001]),
            },
            cold: PerImplementation {
                matplotlib: Column::Measured(cold(vec![0.25, 0.24, 0.26])),
                python: Column::Measured(cold(vec![0.17, 0.16, 0.18])),
                rust: Column::Measured(cold(vec![0.012, 0.011, 0.013])),
            },
        };
        let pgf = WorkloadResult {
            name: "output/mid_pgf".to_owned(),
            tier: Tier::Medium,
            format: "pgf".to_owned(),
            calls: Some(3),
            guard: Guard {
                python: Column::NotComparable("LaTeX".to_owned()),
                rust: Column::NotComparable("LaTeX".to_owned()),
            },
            warm: PerImplementation {
                matplotlib: Column::NotComparable("LaTeX".to_owned()),
                python: Column::NotSupported("KeyError: 'x'".to_owned()),
                rust: warm(vec![0.01, 0.011]),
            },
            cold: PerImplementation {
                matplotlib: Column::NotComparable("LaTeX".to_owned()),
                python: Column::NotSupported("KeyError: 'x'".to_owned()),
                rust: Column::Measured(cold(vec![0.02, 0.021])),
            },
        };
        Results {
            machine: Machine::default(),
            versions: Versions {
                reference: vec![("python".to_owned(), "3.14.8".to_owned())],
                pltrs: vec![("python".to_owned(), "3.14.8".to_owned())],
            },
            load_before: "1.00 1.00 1.00".to_owned(),
            load_after: "2.00 1.50 1.00".to_owned(),
            plan: Plan::new(3),
            startup: [
                "python",
                "numpy",
                "numpy-threaded",
                "matplotlib",
                "pltrs-python",
                "pltrs-rust",
            ]
            .iter()
            .map(|key| Startup {
                key: (*key).to_owned(),
                result: cold(vec![0.1, 0.11]),
            })
            .collect(),
            workloads: vec![line, pgf],
        }
    }

    #[test]
    fn the_drop_in_claim_needs_one_python_and_one_numpy() {
        let mut versions = Versions::default();
        assert!(!versions.same_python_and_numpy());
        for (name, value) in [
            ("interpreter", "/usr/bin/python3.14"),
            ("python", "3.14.8"),
            ("numpy", "2.5.4"),
            ("numpy core", "0123"),
        ] {
            versions.reference.push((name.to_owned(), value.to_owned()));
            versions.pltrs.push((name.to_owned(), value.to_owned()));
        }
        assert!(versions.same_python_and_numpy());
        versions.pltrs[3].1 = "4567".to_owned();
        assert!(!versions.same_python_and_numpy());
    }
}
