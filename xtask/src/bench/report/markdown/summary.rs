//! The summary: each claim on its own, then start-up, memory and the
//! thinnest margins.

use std::fmt::Write as _;

use super::super::super::implementation::Implementation;
use super::super::super::results::{Column, Results, WorkloadResult};
use super::super::super::stats::{geometric_mean, speedup};
use super::super::super::workloads::Tier;
use super::super::analysis::{Mode, Tally, ratio};
use super::super::charts::Charts;
use super::{ms, speedup_interval, typical};

/// Speed-ups below this count as thin margins.
const THIN: f64 = 1.5;

/// The summary section.
pub(super) fn write(out: &mut String, results: &Results, charts: &Charts) {
    out.push_str(
        "## Summary\n\n\
         The report makes two claims and measures them apart. pltrs (Python) against \
         matplotlib compares one Python program with itself under two libraries. pltrs \
         (Rust) against matplotlib compares a Rust program with a Python program that draws \
         the same figure. Each claim has a warm number (the figure alone, in a running \
         program) and a cold number (a whole program in a fresh process, start-up \
         included). A workload counts as faster or slower only when the 95% confidence \
         interval of its speed-up excludes 1. Otherwise it is within the noise. A tier's \
         typical speed-up is the geometric mean over its workloads.\n\n",
    );
    python_claim(out, results, charts);
    rust_claim(out, results, charts);
    startup(out, results, charts);
    memory(out, results);
    thin_margins(out, results);
}

/// The drop-in claim.
fn python_claim(out: &mut String, results: &Results, charts: &Charts) {
    let v = &results.versions;
    out.push_str("### pltrs (Python) as a drop-in replacement for matplotlib\n\n");
    if v.same_python_and_numpy() {
        let _ = writeln!(
            out,
            "Both columns run the same scripts on the same interpreter (Python {python}, \
             `{interpreter}`) and the same numpy build ({numpy}, core library digest \
             `{digest}`). Only the imports differ: `import matplotlib.pyplot as plt` reads \
             `import pltrs.pyplot as plt`.\n",
            python = v.pltrs("python"),
            interpreter = v.pltrs("interpreter"),
            numpy = v.pltrs("numpy"),
            digest = v.pltrs("numpy core"),
        );
    } else {
        let _ = writeln!(
            out,
            "**The two venvs ran different interpreters or numpy builds** (matplotlib: \
             Python {} with numpy {}, pltrs (Python): Python {} with numpy {}), so this \
             run does not support the drop-in claim on its own.\n",
            v.reference("python"),
            v.reference("numpy"),
            v.pltrs("python"),
            v.pltrs("numpy"),
        );
    }
    claim_table(out, results, Implementation::Python);
    chart(out, charts.python.as_deref(), "Speed-ups of pltrs (Python)");
    closest(out, results, Implementation::Python);
    coverage(out, results);
}

/// The cross-language claim.
fn rust_claim(out: &mut String, results: &Results, charts: &Charts) {
    let interpreter = |key| results.startup(key).map(|s| s.seconds.median());
    out.push_str("### pltrs (Rust) against matplotlib\n\n");
    let _ = writeln!(
        out,
        "This claim compares programs in two languages that draw the same figure. A cold \
         run of pltrs (Rust) starts a native executable, while a cold run of matplotlib \
         starts the Python interpreter and imports numpy and pyplot, which alone take {} ms \
         here. The warm numbers leave start-up out on both sides.\n",
        interpreter(Implementation::Matplotlib.key()).map_or_else(|| "n/a".to_owned(), ms),
    );
    claim_table(out, results, Implementation::Rust);
    chart(out, charts.rust.as_deref(), "Speed-ups of pltrs (Rust)");
    closest(out, results, Implementation::Rust);
}

/// The tally of `contender` per tier and over all workloads.
fn claim_table(out: &mut String, results: &Results, contender: Implementation) {
    out.push_str(
        "| tier | workloads | warm, faster / within noise / slower | warm, typical speed-up \
         | cold, faster / within noise / slower | cold, typical speed-up |\n\
         |---|---:|---:|---:|---:|---:|\n",
    );
    let row = |out: &mut String, title: &str, workloads: &[&WorkloadResult]| {
        let warm = Tally::of(workloads.iter().copied(), contender, Mode::Warm);
        let cold = Tally::of(workloads.iter().copied(), contender, Mode::Cold);
        if warm.len() == 0 {
            return;
        }
        let tally = |t: &Tally<'_>| format!("{} / {} / {}", t.faster, t.tied, t.slower);
        let _ = writeln!(
            out,
            "| {title} | {} | {} | {} | {} | {} |",
            warm.len(),
            tally(&warm),
            typical(warm.typical()),
            tally(&cold),
            typical(cold.typical()),
        );
    };
    for tier in Tier::ALL {
        let in_tier: Vec<&WorkloadResult> = results
            .workloads
            .iter()
            .filter(|w| w.tier == tier)
            .collect();
        row(out, tier.title(), &in_tier);
    }
    let all: Vec<&WorkloadResult> = results.workloads.iter().collect();
    row(out, "**all workloads**", &all);
    out.push('\n');
}

/// An embedded chart, when there is one.
fn chart(out: &mut String, file: Option<&str>, title: &str) {
    if let Some(file) = file {
        let _ = writeln!(out, "![{title}](charts/{file})\n");
    }
}

/// How many of the smallest speed-ups the summary names.
const CLOSEST: usize = 3;

/// Where `contender` is slower than matplotlib, or else closest to it.
fn closest(out: &mut String, results: &Results, contender: Implementation) {
    for mode in Mode::ALL {
        let tally = Tally::of(&results.workloads, contender, mode);
        if tally.len() == 0 {
            continue;
        }
        let named = |shown: &[(&str, _)]| {
            shown
                .iter()
                .map(|(name, r)| format!("`{name}` ({})", speedup_interval(r)))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let (who, how) = (contender.title(), mode.name());
        let slower: Vec<_> = tally
            .ratios
            .iter()
            .filter(|(_, r)| r.value >= 1.0)
            .copied()
            .collect();
        let _ = if slower.is_empty() {
            writeln!(
                out,
                "* {who} is faster {how} on all {} workloads. The smallest speed-ups are {}.",
                tally.len(),
                named(&tally.ratios[..CLOSEST.min(tally.len())]),
            )
        } else {
            writeln!(
                out,
                "* {who} is not faster {how} on {} of {} workloads: {}.",
                slower.len(),
                tally.len(),
                named(&slower),
            )
        };
    }
    out.push('\n');
}

/// What pltrs (Python) runs, and the figures the guard turned down.
fn coverage(out: &mut String, results: &Results) {
    let comparable = results
        .workloads
        .iter()
        .filter(|w| w.warm.matplotlib.measured().is_some())
        .count();
    let unsupported: Vec<&str> = results
        .workloads
        .iter()
        .filter(|w| matches!(w.warm.python, Column::NotSupported(_)))
        .map(|w| w.name.as_str())
        .collect();
    if unsupported.is_empty() {
        let _ = writeln!(
            out,
            "pltrs (Python) runs all {comparable} workloads matplotlib runs."
        );
    } else {
        let _ = writeln!(
            out,
            "pltrs (Python) runs {} of the {comparable} workloads matplotlib runs. The other \
             {} need parts of matplotlib's API it does not have yet ([Not supported \
             yet](#not-supported-yet-by-pltrs-python)).",
            comparable - unsupported.len(),
            unsupported.len(),
        );
    }
    for contender in Implementation::CONTENDERS {
        let diverging: Vec<String> = results
            .workloads
            .iter()
            .filter(|w| w.guard.diverges(contender))
            .map(|w| format!("`{}`", w.name))
            .collect();
        if !diverging.is_empty() {
            let _ = writeln!(
                out,
                " The guard found that {} draws a different figure from matplotlib's in {}, \
                 which the tallies leave out.",
                contender.title(),
                diverging.join(", "),
            );
        }
    }
    out.push('\n');
}

/// The start-up costs in one paragraph.
fn startup(out: &mut String, results: &Results, charts: &Charts) {
    let median = |key: &str| {
        results
            .startup(key)
            .map_or_else(|| "n/a".to_owned(), |s| ms(s.seconds.median()))
    };
    let _ = writeln!(
        out,
        "### Start-up\n\n\
         A fresh interpreter takes {python} ms to start here and {numpy} ms to import numpy. \
         One that imports `matplotlib.pyplot` takes {mpl} ms, and one that imports \
         `pltrs.pyplot` takes {pltrs} ms. The executable of pltrs (Rust) starts and exits in \
         {rust} ms. [Start-up and imports](#start-up-and-imports) has the details.\n",
        python = median("python"),
        numpy = median("numpy"),
        mpl = median(Implementation::Matplotlib.key()),
        pltrs = median(Implementation::Python.key()),
        rust = median(Implementation::Rust.key()),
    );
    chart(out, charts.startup.as_deref(), "Start-up and imports");
}

/// The peak memory of the cold runs, against matplotlib's.
fn memory(out: &mut String, results: &Results) {
    out.push_str("### Peak memory\n\n");
    for contender in Implementation::CONTENDERS {
        let ratios: Vec<(&str, f64)> = results
            .workloads
            .iter()
            .filter_map(|w| {
                let reference = w.cold.matplotlib.measured()?.peak_kib();
                let peak = w.cold.get(contender).measured()?.peak_kib();
                Some((w.name.as_str(), peak as f64 / reference as f64))
            })
            .collect();
        let values: Vec<f64> = ratios.iter().map(|(_, r)| *r).collect();
        let Some(mean) = geometric_mean(&values) else {
            continue;
        };
        let (low, high) = values
            .iter()
            .fold((f64::INFINITY, 0.0_f64), |(lo, hi), &r| {
                (lo.min(r), hi.max(r))
            });
        let _ = write!(
            out,
            "* A cold run of {} typically peaks at {mean:.2} times the resident memory of \
             matplotlib's run of the same workload, and at {low:.2} to {high:.2} times over \
             all {} workloads.",
            contender.title(),
            ratios.len(),
        );
        let heavier: Vec<String> = ratios
            .iter()
            .filter(|(_, r)| *r > HEAVIER)
            .map(|(name, r)| format!("`{name}` ({r:.2} times)"))
            .collect();
        if !heavier.is_empty() {
            let _ = write!(
                out,
                " It needs more than {HEAVIER} times matplotlib's memory on {}.",
                heavier.join(", ")
            );
        }
        out.push('\n');
    }
    out.push_str(
        "\nBoth Python columns include the interpreter and numpy, which [Start-up and \
         imports](#start-up-and-imports) measures alone. pltrs (Rust) has neither.\n\n",
    );
}

/// Peak memory beyond this many times matplotlib's is named in the
/// summary.
const HEAVIER: f64 = 1.25;

/// Every speed-up under [`THIN`], warm or cold, of either claim.
fn thin_margins(out: &mut String, results: &Results) {
    let mut rows = Vec::new();
    for contender in Implementation::CONTENDERS {
        for mode in Mode::ALL {
            for w in &results.workloads {
                if w.guard.diverges(contender) {
                    continue;
                }
                if let Some(r) = ratio(w, contender, mode).filter(|r| r.value > 1.0 / THIN) {
                    rows.push(format!(
                        "| `{}` | {} | {} | {} |",
                        w.name,
                        contender.title(),
                        mode.name(),
                        speedup_interval(&r)
                    ));
                }
            }
        }
    }
    let _ = writeln!(out, "### Where the margins are thin\n");
    if rows.is_empty() {
        let _ = writeln!(
            out,
            "Every speed-up of both claims, warm and cold, is at least {}.\n",
            speedup(1.0 / THIN)
        );
        return;
    }
    let _ = writeln!(
        out,
        "These speed-ups are under {}, or are slowdowns. Three causes account for most of \
         them. Cold, both Python columns spend most of a small figure's time starting the \
         interpreter and importing numpy, which caps the cold speed-up of pltrs (Python). \
         Where a large PNG or PDF stream is compressed, compression takes most of the time \
         on both sides, and matplotlib's PNG goes through zlib-ng, which is faster than the \
         zlib-like encoder of pltrs (see [Same work](#same-work)). Where every marker of a \
         scatter has its own size, both libraries stroke and fill each one with the same Agg \
         algorithms.\n\n\
         | workload | implementation | run | speed-up [95% interval] |\n|---|---|---|---:|\n{}\n",
        speedup(1.0 / THIN),
        rows.join("\n")
    );
}
