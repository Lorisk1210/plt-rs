//! `cargo xtask bench`: times figures in matplotlib, pltrs (Python) and
//! pltrs (Rust).
//!
//! ```text
//! cargo xtask bench [PREFIX...] [--reference] [--runs N] [--out DIR]
//! cargo xtask bench [PREFIX...] --report [--runs N]
//! ```
//!
//! Every workload (`workloads`) draws the figure of a matplotlib-syntax
//! script of `benchmarks/python` from the same data, under matplotlib's
//! look and default rcParams, and saves it in one format, or only lays it
//! out. Its data is made before the clock starts and one run warms up
//! before the timed ones.
//!
//! Without `--report`, the warm times of pltrs (Rust) are printed as a
//! table, with matplotlib's and the speed-up beside them under
//! `--reference` (the scripts run through `spec/reference/bench/bench.py`
//! in the reference venv). `--runs N`
//! (default 3) is the least number of timed runs; the median counts.
//! `--out DIR` also writes each document to `DIR`, so that the output of
//! two builds can be compared byte for byte: documents carry no metadata
//! and SVG ids a fixed salt.
//!
//! `--report` measures the workloads in matplotlib, pltrs (Python) and
//! pltrs (Rust), warm and cold, after checking that the three draw the same figure, and
//! writes `benchmarks/README.md` (see `report`); `--runs` defaults to 5
//! there.

mod child;
mod cold;
mod guard;
mod machine;
mod python;
mod render;
mod report;
mod stats;
mod workloads;

use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use python::{Outcome, PythonRunner};
use render::{time_warm, user_configuration};
use stats::{Sampling, speedup};
use workloads::{Output, Script, Workload};

/// What `cargo xtask bench` was asked to do.
struct Request<'a> {
    /// Workloads whose names start with one of these (all without any).
    prefixes: Vec<&'a str>,
    /// Also time matplotlib.
    reference: bool,
    /// Measure the three implementations and write the report.
    report: bool,
    /// The least number of timed runs per measurement, if given.
    runs: Option<usize>,
    /// Where to write the documents, if anywhere.
    out: Option<PathBuf>,
}

/// How to call the command.
const USAGE: &str = "usage: cargo xtask bench [PREFIX...] [--reference] [--runs N] [--out DIR]\n       \
                     cargo xtask bench [PREFIX...] --report [--runs N]";

impl<'a> Request<'a> {
    fn parse(args: &'a [String]) -> Result<Self, String> {
        let mut request = Self {
            prefixes: Vec::new(),
            reference: false,
            report: false,
            runs: None,
            out: None,
        };
        let mut rest = args.iter();
        while let Some(arg) = rest.next() {
            match arg.as_str() {
                "--reference" => request.reference = true,
                "--report" => request.report = true,
                "--runs" => {
                    request.runs = Some(
                        rest.next()
                            .and_then(|n| n.parse().ok())
                            .filter(|&n| n > 0)
                            .ok_or("--runs needs a positive count")?,
                    );
                }
                "--out" => {
                    request.out = Some(rest.next().ok_or("--out needs a directory")?.into());
                }
                flag if flag.starts_with("--") => return Err(format!("unknown flag {flag}")),
                prefix => request.prefixes.push(prefix),
            }
        }
        if request.report && (request.reference || request.out.is_some()) {
            return Err("--report takes neither --reference nor --out".to_owned());
        }
        Ok(request)
    }

    fn selects(&self, name: &str) -> bool {
        self.prefixes.is_empty() || self.prefixes.iter().any(|p| name.starts_with(p))
    }
}

/// Runs the benchmark.
pub(crate) fn run(args: &[String]) -> ExitCode {
    if let Some((flag, rest)) = args.split_first()
        && flag == "--child"
    {
        return match child::run(rest) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("{err}");
                ExitCode::FAILURE
            }
        };
    }
    let request = match Request::parse(args) {
        Ok(request) => request,
        Err(err) => {
            eprintln!("{err}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    if cfg!(debug_assertions) {
        eprintln!("warning: timing a debug build; `cargo xtask` builds in release mode");
    }
    user_configuration();
    let selected: Vec<&'static Workload> = workloads::all()
        .filter(|w| request.selects(w.name))
        .collect();
    let outcome = if request.report {
        report::run(&selected, request.runs.unwrap_or(5))
    } else {
        table(&request, &selected)
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("cargo xtask bench: {err}");
            ExitCode::FAILURE
        }
    }
}

/// Prints the warm median of each workload in pltrs (Rust), with
/// matplotlib's and the speed-up under `--reference`.
fn table(request: &Request<'_>, selected: &[&Workload]) -> Result<(), Box<dyn Error>> {
    let runs = request.runs.unwrap_or(3);
    let sampling = Sampling {
        min_runs: runs,
        min_seconds: 0.0,
        max_runs: runs,
    };
    let reference = if request.reference {
        reference_medians(selected, sampling)?
    } else {
        BTreeMap::new()
    };
    println!(
        "{:<34} {:>15} {:>14} {:>13}",
        "workload", "pltrs (Rust) ms", "matplotlib ms", "speed-up"
    );
    let mut failed = 0;
    for workload in selected {
        let build = (workload.setup)();
        let timed = time_warm(workload, &build, sampling).and_then(|(samples, document)| {
            if let Some(out) = &request.out {
                write_document(out, workload, &document)?;
            }
            Ok(samples.median())
        });
        match timed {
            Ok(median) => {
                let ms = median * 1e3;
                let row = match reference.get(workload.name) {
                    Some(&mpl) => format!("{mpl:>14.1} {:>13}", speedup(ms / mpl)),
                    None => format!("{:>14} {:>13}", "-", "-"),
                };
                println!("{:<34} {ms:>15.1} {row}", workload.name);
            }
            Err(err) => {
                eprintln!("{}: {err}", workload.name);
                failed += 1;
            }
        }
    }
    if failed == 0 {
        Ok(())
    } else {
        Err(format!("{failed} workload(s) failed").into())
    }
}

/// matplotlib's warm median in milliseconds for each workload that has a
/// script.
fn reference_medians(
    selected: &[&Workload],
    sampling: Sampling,
) -> Result<BTreeMap<&'static str, f64>, Box<dyn Error>> {
    let matplotlib = PythonRunner::matplotlib(&crate::python::root())?;
    let mut medians = BTreeMap::new();
    for workload in selected {
        let Script::Python(script) = workload.script else {
            continue;
        };
        match matplotlib.warm(workload, script, sampling)? {
            Outcome::Done(samples) => {
                medians.insert(workload.name, samples.median() * 1e3);
            }
            Outcome::Unsupported(why) => {
                return Err(format!("matplotlib cannot run {script}.py: {why}").into());
            }
        }
    }
    Ok(medians)
}

/// Writes `document` to `out/<group>__<name>.<extension>`, unless the
/// workload saves nothing.
fn write_document(out: &Path, workload: &Workload, document: &[u8]) -> Result<(), Box<dyn Error>> {
    if workload.output == Output::LayoutOnly {
        return Ok(());
    }
    std::fs::create_dir_all(out)?;
    let extension = workload.extension().unwrap_or_default();
    let name = format!("{}.{extension}", workload.name.replace('/', "__"));
    Ok(std::fs::write(out.join(name), document)?)
}
