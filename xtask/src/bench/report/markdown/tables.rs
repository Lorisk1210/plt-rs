//! The detailed tables: start-up, each tier's warm and cold times, memory
//! and document sizes, the cost of a call, the guard's verdicts, and the
//! workloads pltrs (Python) cannot run yet.

use std::fmt::Write as _;

use super::super::super::cold::ColdResult;
use super::super::super::guard::{Guarded, Verdict};
use super::super::super::implementation::Implementation;
use super::super::super::results::{Column, Results, WorkloadResult};
use super::super::super::stats::Samples;
use super::super::super::workloads::Tier;
use super::super::analysis::{Mode, ratio, samples};
use super::{cell, mib, ms, speedup_interval, timing};

/// Every detailed table.
pub(super) fn write(out: &mut String, results: &Results) {
    startup(out, results);
    for tier in Tier::ALL {
        let rows: Vec<&WorkloadResult> = results
            .workloads
            .iter()
            .filter(|w| w.tier == tier)
            .collect();
        if !rows.is_empty() {
            tier_tables(out, tier, &rows);
        }
    }
    guard(out, results);
    unsupported(out, results);
}

/// The start-up and import table.
fn startup(out: &mut String, results: &Results) {
    let threads = results.machine.threads;
    out.push_str(
        "## Start-up and imports\n\n\
         These fresh processes only start, import and exit. Every cold run costs at least \
         as much.\n\n\
         | process | median ms [IQR] | peak memory MiB |\n|---|---:|---:|\n",
    );
    let rows = [
        ("python", "`python -c pass`".to_owned()),
        ("numpy", "`python -c \"import numpy\"`".to_owned()),
        (
            "numpy-threaded",
            format!("`python -c \"import numpy\"`, OpenBLAS's default of {threads} threads"),
        ),
        (
            "matplotlib",
            "`python -c \"import matplotlib.pyplot\"`".to_owned(),
        ),
        (
            "pltrs-python",
            "`python -c \"import pltrs.pyplot\"`".to_owned(),
        ),
        (
            "pltrs-rust",
            "the executable of pltrs (Rust), starting and exiting".to_owned(),
        ),
    ];
    for (key, name) in rows {
        if let Some(result) = results.startup(key) {
            let _ = writeln!(
                out,
                "| {name} | {} | {} |",
                timing(&result.seconds),
                mib(result.peak_kib())
            );
        }
    }
    if let (Some(numpy), Some(mpl), Some(pltrs)) = (
        results.startup("numpy"),
        results.startup("matplotlib"),
        results.startup("pltrs-python"),
    ) {
        let numpy = numpy.seconds.median();
        let _ = writeln!(
            out,
            "\nBoth pyplot modules import numpy. Beyond numpy, importing `matplotlib.pyplot` \
             costs {} ms and importing `pltrs.pyplot` {} ms.",
            ms(mpl.seconds.median() - numpy),
            ms(pltrs.seconds.median() - numpy),
        );
    }
    out.push('\n');
}

/// The tables of one tier.
fn tier_tables(out: &mut String, tier: Tier, rows: &[&WorkloadResult]) {
    let _ = writeln!(out, "## {}\n", tier.title());
    for mode in Mode::ALL {
        let intro = match mode {
            Mode::Warm => {
                "### Warm, in-process\n\nThe figure alone: built and saved to memory in a \
                 running program."
            }
            Mode::Cold => {
                "### Cold, end to end\n\nA fresh process that starts, imports, makes the \
                 data, builds the figure, writes the file and exits."
            }
        };
        let _ = write!(
            out,
            "{intro} Times are medians in ms with the interquartile range in brackets. \
             Speed-ups are against matplotlib, with their 95% interval.\n\n\
             | workload | matplotlib | pltrs (Python) | speed-up | pltrs (Rust) | speed-up |\n\
             |---|---:|---:|---:|---:|---:|\n"
        );
        for w in rows {
            times_row(out, w, mode);
        }
        out.push('\n');
    }
    memory_and_size(out, rows);
    per_call(out, rows);
    footnotes(out, rows);
}

/// A row of times in `mode`.
fn times_row(out: &mut String, w: &WorkloadResult, mode: Mode) {
    let time = |i: Implementation| match samples(w, i, mode) {
        Some(samples) => timing(samples),
        None => cell(w.warm.get(i), |_| "n/a".to_owned()),
    };
    let speed = |i: Implementation| {
        ratio(w, i, mode).map_or_else(
            || "n/a".to_owned(),
            |r| {
                let mark = if w.guard.diverges(i) { " †" } else { "" };
                format!("{}{mark}", speedup_interval(&r))
            },
        )
    };
    let _ = writeln!(
        out,
        "| `{}` | {} | {} | {} | {} | {} |",
        w.name,
        time(Implementation::Matplotlib),
        time(Implementation::Python),
        speed(Implementation::Python),
        time(Implementation::Rust),
        speed(Implementation::Rust),
    );
}

/// The peak memory of the cold runs and the size of the documents.
fn memory_and_size(out: &mut String, rows: &[&WorkloadResult]) {
    out.push_str(
        "### Peak memory and document size\n\n\
         Peak memory is the median peak resident memory of the cold processes, in MiB. The \
         size is that of the document the warm runs saved, in KiB.\n\n\
         | workload | format | matplotlib MiB | pltrs (Python) MiB | pltrs (Rust) MiB | \
         matplotlib KiB | pltrs (Python) KiB | pltrs (Rust) KiB |\n\
         |---|---|---:|---:|---:|---:|---:|---:|\n",
    );
    for w in rows {
        let peak = |i: Implementation| cell(w.cold.get(i), |c: &ColdResult| mib(c.peak_kib()));
        let size = |i: Implementation| {
            cell(w.warm.get(i), |m| {
                if m.bytes == 0 {
                    "-".to_owned()
                } else {
                    format!("{:.0}", m.bytes as f64 / 1024.0)
                }
            })
        };
        let _ = writeln!(
            out,
            "| `{}` | {} | {} | {} | {} | {} | {} | {} |",
            w.name,
            w.format,
            peak(Implementation::Matplotlib),
            peak(Implementation::Python),
            peak(Implementation::Rust),
            size(Implementation::Matplotlib),
            size(Implementation::Python),
            size(Implementation::Rust),
        );
    }
    out.push('\n');
}

/// What the Python layer of pltrs (Python) adds to each call, for the
/// workloads of many calls among `rows`.
fn per_call(out: &mut String, rows: &[&WorkloadResult]) {
    let counted: Vec<(&WorkloadResult, u32)> =
        rows.iter().filter_map(|w| Some((*w, w.calls?))).collect();
    if counted.is_empty() {
        return;
    }
    out.push_str(
        "### What a call costs in pltrs (Python)\n\n\
         pltrs (Python) and pltrs (Rust) run the same Rust code for these figures. On every \
         call pltrs (Python) also handles matplotlib's signature and keywords, converts the \
         arguments, crosses into the extension and makes the Python object of the artist it \
         returns. The table divides the difference of their warm medians by the number of \
         calls, and gives matplotlib's whole time per call beside it.\n\n\
         | workload | calls | pltrs (Python) over pltrs (Rust), µs per call | matplotlib, µs \
         per call |\n|---|---:|---:|---:|\n",
    );
    for (w, calls) in counted {
        let per_call = |seconds: f64| format!("{:.1}", seconds * 1e6 / f64::from(calls));
        let rust = samples(w, Implementation::Rust, Mode::Warm).map(Samples::median);
        let extra = match (samples(w, Implementation::Python, Mode::Warm), rust) {
            (Some(python), Some(rust)) => per_call(python.median() - rust),
            _ => "n/a".to_owned(),
        };
        let matplotlib = samples(w, Implementation::Matplotlib, Mode::Warm)
            .map_or_else(|| "n/a".to_owned(), |s| per_call(s.median()));
        let _ = writeln!(out, "| `{}` | {calls} | {extra} | {matplotlib} |", w.name);
    }
    out.push('\n');
}

/// The notes under a tier's tables.
fn footnotes(out: &mut String, rows: &[&WorkloadResult]) {
    if rows.iter().any(|w| {
        Implementation::CONTENDERS
            .iter()
            .any(|&i| w.guard.diverges(i))
    }) {
        out.push_str(
            "† The figure differs from matplotlib's, as the [correctness \
             guard](#correctness-guard) shows.\n\n",
        );
    }
    for w in rows {
        if let Column::NotComparable(why) = &w.warm.matplotlib {
            let _ = writeln!(out, "`{}` is not comparable: {why}.\n", w.name);
        }
    }
}

/// The guard's verdicts for every workload.
fn guard(out: &mut String, results: &Results) {
    out.push_str(
        "## Correctness guard\n\n\
         Pixels give the RMS difference from matplotlib's PNG (for an animation, the \
         largest over its frames). Structure gives the largest difference between the \
         numbers of matching drawing operations of a vector document, or what differs. \
         The renderings are in `target/bench/guard/` after a run.\n\n\
         | workload | format | pltrs (Python) pixels | pltrs (Python) structure | pltrs (Rust) \
         pixels | pltrs (Rust) structure |\n|---|---|---:|---:|---:|---:|\n",
    );
    for w in &results.workloads {
        let checks = |column: &Column<Guarded>| match column {
            Column::Measured(g) => (
                verdict(&g.pixels),
                g.structure.as_ref().map_or_else(|| "-".to_owned(), verdict),
            ),
            other => {
                let why = cell(other, |_| String::new());
                (why.clone(), why)
            }
        };
        let (python_pixels, python_structure) = checks(&w.guard.python);
        let (rust_pixels, rust_structure) = checks(&w.guard.rust);
        let _ = writeln!(
            out,
            "| `{}` | {} | {python_pixels} | {python_structure} | {rust_pixels} | \
             {rust_structure} |",
            w.name, w.format
        );
    }
    out.push('\n');
}

/// A verdict in a table, failures in bold.
fn verdict(verdict: &Verdict) -> String {
    let text = verdict.to_string().replace('|', "\\|");
    if verdict.diverges() {
        format!("**{}**", shortened(&text))
    } else {
        text
    }
}

/// `text` cut to 160 characters.
fn shortened(text: &str) -> String {
    let short: String = text.chars().take(160).collect();
    if short.len() < text.len() {
        format!("{short}…")
    } else {
        short
    }
}

/// The workloads pltrs (Python) cannot run yet, with the exception each
/// raised.
fn unsupported(out: &mut String, results: &Results) {
    let missing: Vec<(&str, &str)> = results
        .workloads
        .iter()
        .filter_map(|w| match &w.warm.python {
            Column::NotSupported(why) => Some((w.name.as_str(), why.as_str())),
            Column::Measured(_) | Column::NotComparable(_) => None,
        })
        .collect();
    if missing.is_empty() {
        return;
    }
    out.push_str(
        "## Not supported yet by pltrs (Python)\n\n\
         Each row gives the exception the script raised under pltrs (Python). As pltrs \
         grows, a new run moves these workloads into the tables.\n\n\
         | workload | exception |\n|---|---|\n",
    );
    for (name, why) in missing {
        let _ = writeln!(
            out,
            "| `{name}` | {} |",
            shortened(&why.replace('|', "\\|"))
        );
    }
    out.push('\n');
}
