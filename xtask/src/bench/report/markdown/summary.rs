//! The summary: where pltrs and plt are faster or slower than matplotlib,
//! tier by tier, warm and cold.

use std::fmt::Write as _;

use super::super::super::cold::ColdResult;
use super::super::super::guard::{TOLERANCE, Verdict};
use super::super::super::stats::{Samples, geometric_mean};
use super::super::super::workloads::Tier;
use super::super::{Column, Measurement, Report};
use super::{ms, ratio};

/// An implementation measured against matplotlib.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Contender {
    /// pltrs, the same scripts.
    Pltrs,
    /// plt, from Rust.
    Plt,
}

impl Contender {
    /// Its name in the report.
    const fn name(self) -> &'static str {
        match self {
            Self::Pltrs => "pltrs",
            Self::Plt => "plt",
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
                Contender::Pltrs => m.warm.pltrs.measured().map(Samples::median),
                Contender::Plt => Some(m.warm.plt.median()),
            },
        ),
        Mode::Cold => {
            let median = |c: &ColdResult| c.seconds.median();
            (
                m.cold.matplotlib.measured().map(median),
                match contender {
                    Contender::Pltrs => m.cold.pltrs.measured().map(median),
                    Contender::Plt => Some(median(&m.cold.plt)),
                },
            )
        }
    };
    Some(value? / reference?)
}

/// The summary section.
pub(super) fn write(out: &mut String, report: &Report) {
    out.push_str(
        "## Summary\n\n\
         A ratio is an implementation's median time divided by matplotlib's. Below 1 it is \
         faster, above 1 slower. A tier's geometric mean averages its ratios. A warm run \
         times the figure alone in a running program, and a cold run times a whole program \
         in a fresh process, interpreter start and imports included.\n\n\
         | | tier | timed | warm, faster / slower | warm, geometric mean | \
         cold, faster / slower | cold, geometric mean |\n\
         |---|---|---:|---:|---:|---:|---:|\n",
    );
    for contender in [Contender::Pltrs, Contender::Plt] {
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
         one that imports `pltrs.pyplot` takes {} ms. plt's executable starts and exits in \
         {} ms.",
        ms(s.matplotlib.seconds.median()),
        ms(s.pltrs.seconds.median()),
        ms(s.plt.seconds.median()),
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
             time and {} of pltrs'.",
            share(&s.matplotlib, &line.cold.matplotlib),
            share(&s.pltrs, &line.cold.pltrs),
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
    let mean = |ratios: &[f64]| geometric_mean(ratios).map_or_else(|| "n/a".to_owned(), ratio);
    let _ = writeln!(
        out,
        "| {} | {} | {} of {} | {} | {} | {} | {} |",
        contender.name(),
        tier.title(),
        warm.len(),
        comparable.len(),
        tally(&warm),
        mean(&warm),
        tally(&cold),
        mean(&cold),
    );
}

/// pltrs' and plt's geometric means over the workloads pltrs runs, the
/// fair comparison of the two: pltrs' rows above cover fewer workloads.
fn same_workloads(out: &mut String, report: &Report) {
    let shared: Vec<&Measurement> = report
        .measurements
        .iter()
        .filter(|m| m.warm.pltrs.measured().is_some())
        .collect();
    if shared.is_empty() {
        return;
    }
    let mean = |contender, mode| {
        let ratios: Vec<f64> = shared
            .iter()
            .filter_map(|m| ratio_of(m, contender, mode))
            .collect();
        geometric_mean(&ratios).map_or_else(|| "n/a".to_owned(), ratio)
    };
    let _ = writeln!(
        out,
        "\npltrs' rows cover only the workloads it runs. On those {} workloads, the geometric \
         mean ratio is {} warm and {} cold for pltrs, and {} warm and {} cold for plt.",
        shared.len(),
        mean(Contender::Pltrs, Mode::Warm),
        mean(Contender::Pltrs, Mode::Cold),
        mean(Contender::Plt, Mode::Warm),
        mean(Contender::Plt, Mode::Cold),
    );
}

/// How many of the closest margins the summary names.
const CLOSEST: usize = 3;

/// The workloads where pltrs or plt is slower than matplotlib, or else
/// where they come closest to it.
fn losses(out: &mut String, report: &Report) {
    out.push_str(
        "### Where pltrs or plt is slower, or closest to matplotlib\n\n\
         Ratios of 1 or more are losses.\n\n",
    );
    for contender in [Contender::Pltrs, Contender::Plt] {
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
                    .map(|(name, r)| format!("`{name}` ({})", ratio(*r)))
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
                    "* {who} is faster {how} on all {total} workloads. The closest margins are \
                     {}.",
                    named(CLOSEST.min(total))
                )
            };
        }
    }
    out.push('\n');
    pltrs_against_plt(out, report);
}

/// How far pltrs' warm times are from plt's, which run the same Rust code.
fn pltrs_against_plt(out: &mut String, report: &Report) {
    let gaps: Vec<(&str, f64, f64)> = report
        .measurements
        .iter()
        .filter_map(|m| {
            let pltrs = m.warm.pltrs.measured()?.median();
            let plt = m.warm.plt.median();
            Some((m.workload.name, pltrs - plt, pltrs / plt - 1.0))
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
        "pltrs runs the same Rust code as plt, plus its Python layer. Warm, on the {} \
         workloads it runs, it takes at most {:.0}% longer than plt (`{}`) and at most {} ms \
         longer (`{}`). The tier of many small calls measures [what a call \
         costs](#what-a-call-costs-in-pltrs) in pltrs.\n",
        gaps.len(),
        100.0 * widest.2,
        widest.0,
        ms(longest.1),
        longest.0,
    );
}

/// What pltrs runs, and whether every figure matched matplotlib's.
fn coverage(out: &mut String, report: &Report) {
    let comparable = report
        .measurements
        .iter()
        .filter(|m| m.warm.matplotlib.measured().is_some())
        .count();
    let unsupported = report
        .measurements
        .iter()
        .filter(|m| matches!(m.warm.pltrs, Column::NotSupported(_)))
        .count();
    let _ = writeln!(
        out,
        "pltrs runs {} of the {comparable} workloads matplotlib runs. The other {unsupported} \
         need parts of matplotlib's API it does not have yet, which [Not supported \
         yet](#not-supported-yet-by-pltrs) lists.\n",
        comparable - unsupported,
    );
    let divergent: Vec<String> = report
        .measurements
        .iter()
        .flat_map(|m| {
            let plt = m
                .guard
                .plt
                .as_ref()
                .filter(|v| v.diverges())
                .map(|v| ("plt", v));
            let pltrs = m
                .guard
                .pltrs
                .measured()
                .filter(|v| v.diverges())
                .map(|v| ("pltrs", v));
            [plt, pltrs]
                .into_iter()
                .flatten()
                .map(|(name, v): (&str, &Verdict)| format!("`{}` in {name} ({v})", m.workload.name))
        })
        .collect();
    if divergent.is_empty() {
        let largest = report
            .measurements
            .iter()
            .flat_map(|m| [m.guard.plt.as_ref(), m.guard.pltrs.measured()])
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
