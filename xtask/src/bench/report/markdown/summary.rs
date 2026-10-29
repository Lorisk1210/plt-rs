//! The summary: how much faster or slower pltrs (Python) and pltrs (Rust)
//! are than matplotlib, tier by tier, warm and cold.

use std::fmt::Write as _;

use super::super::super::cold::ColdResult;
use super::super::super::guard::{TOLERANCE, Verdict};
use super::super::super::stats::{Samples, geometric_mean, speedup};
use super::super::super::workloads::Tier;
use super::super::{Column, Measurement, Report};
use super::{PYTHON, RUST, ms};

/// An implementation measured against matplotlib.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Contender {
    /// pltrs (Python), the same scripts.
    Python,
    /// pltrs (Rust).
    Rust,
}

impl Contender {
    /// Its name in the report.
    const fn name(self) -> &'static str {
        match self {
            Self::Python => PYTHON,
            Self::Rust => RUST,
        }
    }
}

/// Warm or cold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    /// In-process, after a warm-up.
    Warm,
    /// A fresh process, end to end.
    Cold,
}

impl Mode {
    /// Its name in the report.
    const fn name(self) -> &'static str {
        match self {
            Self::Warm => "warm",
            Self::Cold => "cold",
        }
    }
}

/// `contender`'s median over matplotlib's for `m` in `mode`, when both
/// were measured.
fn ratio_of(m: &Measurement, contender: Contender, mode: Mode) -> Option<f64> {
    let (reference, value) = match mode {
        Mode::Warm => (
            m.warm.matplotlib.measured().map(Samples::median),
            match contender {
                Contender::Python => m.warm.python.measured().map(Samples::median),
                Contender::Rust => Some(m.warm.rust.median()),
            },
        ),
        Mode::Cold => {
            let median = |c: &ColdResult| c.seconds.median();
            (
                m.cold.matplotlib.measured().map(median),
                match contender {
                    Contender::Python => m.cold.python.measured().map(median),
                    Contender::Rust => Some(median(&m.cold.rust)),
                },
            )
        }
    };
    Some(value? / reference?)
}

/// The typical speed-up of `ratios`: their geometric mean, as a speed-up.
fn typical(ratios: &[f64]) -> String {
    geometric_mean(ratios).map_or_else(|| "n/a".to_owned(), speedup)
}

/// The summary section.
pub(super) fn write(out: &mut String, report: &Report) {
    out.push_str(
        "## Summary\n\n\
         A speed-up compares an implementation's median time with matplotlib's: 4.2× faster \
         means matplotlib takes 4.2 times as long, and 1.3× slower means the implementation \
         takes 1.3 times as long as matplotlib. A tier's typical speed-up is the geometric \
         mean over its workloads. A warm run times the figure alone in a running program, \
         and a cold run times a whole program in a fresh process, interpreter start and \
         imports included.\n\n\
         | | tier | timed | warm, faster / slower | warm, typical speed-up | \
         cold, faster / slower | cold, typical speed-up |\n\
         |---|---|---:|---:|---:|---:|---:|\n",
    );
    for contender in [Contender::Python, Contender::Rust] {
        for tier in Tier::ALL {
            tier_row(out, report, contender, tier);
        }
    }
    same_workloads(out, report);
    startup(out, report);
    losses(out, report);
    coverage(out, report);
}

/// What start-up costs, and how much of a small figure's cold run it is.
fn startup(out: &mut String, report: &Report) {
    let s = &report.startup;
    let _ = write!(
        out,
        "\nA fresh Python interpreter that imports `matplotlib.pyplot` takes {} ms here, and \
         one that imports `pltrs.pyplot` takes {} ms. The benchmark executable of {RUST} \
         starts and exits in {} ms.",
        ms(s.matplotlib.seconds.median()),
        ms(s.python.seconds.median()),
        ms(s.rust.seconds.median()),
    );
    let line = report
        .measurements
        .iter()
        .find(|m| m.workload.name == SMALLEST);
    if let Some(line) = line {
        let share = |startup: &ColdResult, column: &Column<ColdResult>| {
            column.measured().map_or_else(
                || "n/a".to_owned(),
                |cold| {
                    format!(
                        "{:.0}%",
                        100.0 * startup.seconds.median() / cold.seconds.median()
                    )
                },
            )
        };
        let _ = write!(
            out,
            " In the cold run of `{SMALLEST}`, start-up and imports are {} of matplotlib's \
             time and {} of the time of {PYTHON}.",
            share(&s.matplotlib, &line.cold.matplotlib),
            share(&s.python, &line.cold.python),
        );
    }
    out.push_str("\n\n");
}

/// The smallest everyday figure, whose cold run shows the weight of
/// start-up.
const SMALLEST: &str = "small/line_png";

/// The summary row of `contender` in `tier`.
fn tier_row(out: &mut String, report: &Report, contender: Contender, tier: Tier) {
    let comparable: Vec<&Measurement> = report
        .measurements
        .iter()
        .filter(|m| m.workload.tier == tier && m.warm.matplotlib.measured().is_some())
        .collect();
    if comparable.is_empty() {
        return;
    }
    let ratios = |mode| -> Vec<f64> {
        comparable
            .iter()
            .filter_map(|m| ratio_of(m, contender, mode))
            .collect()
    };
    let (warm, cold) = (ratios(Mode::Warm), ratios(Mode::Cold));
    let tally = |ratios: &[f64]| {
        let faster = ratios.iter().filter(|&&r| r < 1.0).count();
        format!("{faster} / {}", ratios.len() - faster)
    };
    let _ = writeln!(
        out,
        "| {} | {} | {} of {} | {} | {} | {} | {} |",
        contender.name(),
        tier.title(),
        warm.len(),
        comparable.len(),
        tally(&warm),
        typical(&warm),
        tally(&cold),
        typical(&cold),
    );
}

/// The typical speed-ups of both contenders over the workloads pltrs
/// (Python) runs, the fair comparison of the two: its rows above cover
/// fewer workloads.
fn same_workloads(out: &mut String, report: &Report) {
    let shared: Vec<&Measurement> = report
        .measurements
        .iter()
        .filter(|m| m.warm.python.measured().is_some())
        .collect();
    if shared.is_empty() {
        return;
    }
    let mean = |contender, mode| {
        let ratios: Vec<f64> = shared
            .iter()
            .filter_map(|m| ratio_of(m, contender, mode))
            .collect();
        typical(&ratios)
    };
    let _ = writeln!(
        out,
        "\nThe rows of {PYTHON} cover only the workloads it runs. On those {} workloads, the \
         typical speed-up is {} warm and {} cold for {PYTHON}, and {} warm and {} cold for \
         {RUST}.",
        shared.len(),
        mean(Contender::Python, Mode::Warm),
        mean(Contender::Python, Mode::Cold),
        mean(Contender::Rust, Mode::Warm),
        mean(Contender::Rust, Mode::Cold),
    );
}

/// How many of the smallest speed-ups the summary names.
const CLOSEST: usize = 3;

/// The workloads where pltrs (Python) or pltrs (Rust) is slower than
/// matplotlib, or else where its speed-up is smallest.
fn losses(out: &mut String, report: &Report) {
    let _ = write!(
        out,
        "### Where {PYTHON} or {RUST} is slower, or closest to matplotlib\n\n"
    );
    for contender in [Contender::Python, Contender::Rust] {
        for mode in [Mode::Warm, Mode::Cold] {
            let mut ratios: Vec<(&str, f64)> = report
                .measurements
                .iter()
                .filter_map(|m| Some((m.workload.name, ratio_of(m, contender, mode)?)))
                .collect();
            if ratios.is_empty() {
                continue;
            }
            ratios.sort_by(|a, b| b.1.total_cmp(&a.1));
            let slower = ratios.iter().take_while(|(_, r)| *r >= 1.0).count();
            let named = |shown: usize| {
                ratios[..shown]
                    .iter()
                    .map(|(name, r)| format!("`{name}` ({})", speedup(*r)))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let (who, how) = (contender.name(), mode.name());
            let total = ratios.len();
            let _ = if slower > 0 {
                writeln!(
                    out,
                    "* {who} is slower {how} on {slower} of {total} workloads: {}.",
                    named(slower)
                )
            } else {
                writeln!(
                    out,
                    "* {who} is faster {how} on all {total} workloads. The smallest speed-ups \
                     are {}.",
                    named(CLOSEST.min(total))
                )
            };
        }
    }
    out.push('\n');
    python_against_rust(out, report);
}

/// How far the warm times of pltrs (Python) are from those of pltrs
/// (Rust), which runs the same Rust code.
fn python_against_rust(out: &mut String, report: &Report) {
    let gaps: Vec<(&str, f64, f64)> = report
        .measurements
        .iter()
        .filter_map(|m| {
            let python = m.warm.python.measured()?.median();
            let rust = m.warm.rust.median();
            Some((m.workload.name, python - rust, python / rust - 1.0))
        })
        .collect();
    let Some(widest) = gaps.iter().max_by(|a, b| a.2.total_cmp(&b.2)) else {
        return;
    };
    let longest = gaps
        .iter()
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap_or(widest);
    let _ = writeln!(
        out,
        "{PYTHON} runs the same Rust code as {RUST}, plus its Python layer. Warm, on the {} \
         workloads it runs, it takes at most {:.0}% longer than {RUST} (`{}`) and at most {} \
         ms longer (`{}`). The tier of many small calls measures [what a call \
         costs](#what-a-call-costs-in-pltrs-python) in {PYTHON}.\n",
        gaps.len(),
        100.0 * widest.2,
        widest.0,
        ms(longest.1),
        longest.0,
    );
}

/// What pltrs (Python) runs, and whether every figure matched
/// matplotlib's.
fn coverage(out: &mut String, report: &Report) {
    let comparable = report
        .measurements
        .iter()
        .filter(|m| m.warm.matplotlib.measured().is_some())
        .count();
    let unsupported = report
        .measurements
        .iter()
        .filter(|m| matches!(m.warm.python, Column::NotSupported(_)))
        .count();
    let _ = writeln!(
        out,
        "{PYTHON} runs {} of the {comparable} workloads matplotlib runs. The other \
         {unsupported} need parts of matplotlib's API it does not have yet, which [Not \
         supported yet](#not-supported-yet-by-pltrs-python) lists.\n",
        comparable - unsupported,
    );
    let divergent: Vec<String> = report
        .measurements
        .iter()
        .flat_map(|m| {
            let python = m
                .guard
                .python
                .measured()
                .filter(|v| v.diverges())
                .map(|v| (PYTHON, v));
            let rust = m
                .guard
                .rust
                .as_ref()
                .filter(|v| v.diverges())
                .map(|v| (RUST, v));
            [python, rust]
                .into_iter()
                .flatten()
                .map(|(name, v): (&str, &Verdict)| format!("`{}` in {name} ({v})", m.workload.name))
        })
        .collect();
    if divergent.is_empty() {
        let largest = report
            .measurements
            .iter()
            .flat_map(|m| [m.guard.python.measured(), m.guard.rust.as_ref()])
            .flatten()
            .filter_map(|verdict| match verdict {
                Verdict::Same { rms } => Some(*rms),
                Verdict::Different { .. } | Verdict::Incomparable(_) => None,
            })
            .fold(0.0, f64::max);
        let _ = writeln!(
            out,
            "Every figure timed matched matplotlib's within the correctness guard's tolerance \
             of {TOLERANCE}. The largest RMS difference was {largest:.2}.\n",
        );
    } else {
        let _ = writeln!(
            out,
            "Some figures differ from matplotlib's, so their timings compare different work. \
             They are {}.\n",
            divergent.join(", ")
        );
    }
}
