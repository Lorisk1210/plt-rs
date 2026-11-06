//! `cargo xtask bench`: times figures in matplotlib, pltrs (Python) and
//! pltrs (Rust).
//!
//! ```text
//! cargo xtask bench [PREFIX...] [--reference] [--runs N] [--out DIR]
//! cargo xtask bench [PREFIX...] --report [--runs N] [--to DIR]
//! cargo xtask bench --rewrite-report [--to DIR]
//! cargo xtask bench --pack DIR [--revision NAME]
//! cargo xtask bench [PREFIX...] --against DIR [--runs N]
//! ```
//!
//! Every workload (`workloads`) draws the figure of a matplotlib-syntax
//! script of `benchmarks/python` from the same data, under matplotlib's
//! look and default rcParams, and saves it in one format, draws it
//! without output, or renders the frames of an animation. Its data is
//! made before the clock starts; one run warms up before the timed ones,
//! which take turns among the implementations (`rounds`).
//!
//! Without `--report`, the warm times of pltrs (Rust) are printed as a
//! table, with matplotlib's and the speed-up beside them under
//! `--reference` (the scripts served by `spec/reference/bench/bench.py`
//! in the reference venv). `--runs N` (default 3) is the least number of
//! rounds; the median counts. `--out DIR` also writes each document of
//! pltrs (Rust) to `DIR`, without metadata and with SVG ids of a fixed
//! salt, so that the output of two builds can be compared byte for byte.
//!
//! `--report` guards and measures the workloads in matplotlib, pltrs
//! (Python) and pltrs (Rust), warm and cold, and writes the raw data
//! (`results.json`), the report (`README.md`) and its charts
//! (`charts/`) into `benchmarks/`, or the directory `--to` names; `--runs`
//! defaults to 10 there. `--rewrite-report` writes the report and the
//! charts again from the raw data.
//!
//! `--pack DIR` gathers this build into `DIR` (`pack`), to be benchmarked
//! on a machine that cannot build it, where `DIR/xtask bench ...` runs as
//! `cargo xtask bench ...` does here. `--against DIR` times this build
//! against the pack in `DIR`, both faces of pltrs, warm and cold, in
//! turns (`against`); `--runs` defaults to 10 there.

mod against;
mod child;
mod cold;
mod guard;
mod implementation;
mod machine;
mod pack;
mod python;
mod render;
mod report;
mod results;
mod rounds;
mod session;
mod stats;
mod workloads;

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use python::PythonRunner;
use render::{Documents, render, user_configuration};
use rounds::{Warm, interleave};
use session::{Outcome, RustSession, ServedSession, Session};
use stats::{Ratio, Sampling, speedup};
use workloads::{Output, Script, Workload};

/// What `cargo xtask bench` was asked to do.
#[derive(Debug, Default, PartialEq)]
struct Request<'a> {
    /// Workloads whose names start with one of these (all without any).
    prefixes: Vec<&'a str>,
    /// Also time matplotlib.
    reference: bool,
    /// Measure the three implementations and write the report.
    report: bool,
    /// Write the report again from its raw data.
    rewrite: bool,
    /// The least number of rounds per measurement, if given.
    runs: Option<usize>,
    /// Where to write the documents, if anywhere.
    out: Option<PathBuf>,
    /// Where the report goes, if not to `benchmarks/`.
    to: Option<PathBuf>,
    /// Where to write the pack of this build, if asked to.
    pack: Option<PathBuf>,
    /// The commit the pack names, if not `git describe`'s.
    revision: Option<&'a str>,
    /// The pack of the build to time this one against, if any.
    against: Option<PathBuf>,
}

/// How to call the command.
const USAGE: &str = "usage: cargo xtask bench [PREFIX...] [--reference] [--runs N] [--out DIR]\n       \
                     cargo xtask bench [PREFIX...] --report [--runs N] [--to DIR]\n       \
                     cargo xtask bench --rewrite-report [--to DIR]\n       \
                     cargo xtask bench --pack DIR [--revision NAME]\n       \
                     cargo xtask bench [PREFIX...] --against DIR [--runs N]";

impl<'a> Request<'a> {
    fn parse(args: &'a [String]) -> Result<Self, String> {
        let mut request = Self::default();
        let mut rest = args.iter();
        while let Some(arg) = rest.next() {
            match arg.as_str() {
                "--reference" => request.reference = true,
                "--report" => request.report = true,
                "--rewrite-report" => request.rewrite = true,
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
                "--to" => {
                    request.to = Some(rest.next().ok_or("--to needs a directory")?.into());
                }
                "--pack" => {
                    request.pack = Some(rest.next().ok_or("--pack needs a directory")?.into());
                }
                "--revision" => {
                    request.revision = Some(rest.next().ok_or("--revision needs a name")?);
                }
                "--against" => {
                    request.against = Some(rest.next().ok_or("--against needs a pack")?.into());
                }
                flag if flag.starts_with("--") => return Err(format!("unknown flag {flag}")),
                prefix => request.prefixes.push(prefix),
            }
        }
        request.check_modes()?;
        Ok(request)
    }

    /// Fails unless the flags given make one of the modes of [`USAGE`].
    fn check_modes(&self) -> Result<(), String> {
        let others = self.reference
            || self.report
            || self.rewrite
            || self.out.is_some()
            || self.to.is_some();
        if self.pack.is_some()
            && (others
                || self.runs.is_some()
                || self.against.is_some()
                || !self.prefixes.is_empty())
        {
            return Err("--pack measures nothing: it takes only --revision".into());
        }
        if self.revision.is_some() && self.pack.is_none() {
            return Err("--revision goes with --pack".into());
        }
        if self.against.is_some() && others {
            return Err("--against takes only workload prefixes and --runs".into());
        }
        let request = self;
        let table = request.reference || request.out.is_some();
        if (request.report || request.rewrite) && table {
            return Err("--report and --rewrite-report take neither --reference nor --out".into());
        }
        if request.rewrite
            && (request.report || request.runs.is_some() || !request.prefixes.is_empty())
        {
            return Err("--rewrite-report measures nothing: it takes only --to".into());
        }
        if request.to.is_some() && !(request.report || request.rewrite) {
            return Err("--to goes with --report or --rewrite-report".into());
        }
        Ok(())
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
    let dir = request.to.clone().unwrap_or_else(report::default_dir);
    let outcome = if let Some(pack) = &request.pack {
        pack::write(pack, request.revision)
    } else if let Some(other) = &request.against {
        against::run(&selected, request.runs.unwrap_or(10), other)
    } else if request.rewrite {
        report::rewrite(&dir)
    } else if request.report {
        report::run(&selected, request.runs.unwrap_or(10), &dir)
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
fn table(request: &Request<'_>, selected: &[&'static Workload]) -> Result<(), Box<dyn Error>> {
    let runs = request.runs.unwrap_or(3);
    let sampling = Sampling {
        min_runs: runs,
        min_seconds: 0.0,
        max_runs: runs,
    };
    let root = pack::root();
    let config = root.join("target/bench/config/matplotlib");
    let matplotlib = if request.reference {
        Some(PythonRunner::matplotlib(root, &config)?)
    } else {
        None
    };
    println!(
        "{:<38} {:>15} {:>14} {:>13}",
        "workload", "pltrs (Rust) ms", "matplotlib ms", "speed-up"
    );
    let mut failed = 0;
    for workload in selected {
        match table_row(
            workload,
            matplotlib.as_ref(),
            sampling,
            request.out.as_deref(),
        ) {
            Ok(row) => println!("{row}"),
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

/// The table's row of `workload`: pltrs (Rust)'s warm median, and
/// matplotlib's with the speed-up when `matplotlib` is given and runs the
/// workload's script.
fn table_row(
    workload: &'static Workload,
    matplotlib: Option<&PythonRunner>,
    sampling: Sampling,
    out: Option<&Path>,
) -> Result<String, Box<dyn Error>> {
    let mut rust = RustSession::new(workload);
    if let Some(out) = out {
        write_document(out, workload, &rust_document(workload)?)?;
    }
    let reference = match (matplotlib, workload.script) {
        (Some(runner), Script::Python(script)) => {
            let log = pack::root().join("target/bench/logs/matplotlib.serve.stderr");
            std::fs::create_dir_all(log.parent().ok_or("a log directory")?)?;
            match ServedSession::start(runner, workload, script, &log)? {
                Outcome::Done(session) => Some(session),
                Outcome::Unsupported(why) => {
                    return Err(format!("matplotlib cannot run {script}.py: {why}").into());
                }
            }
        }
        _ => None,
    };
    let mut sessions: Vec<&mut dyn Session> = vec![&mut rust];
    let mut reference = reference;
    if let Some(session) = reference.as_mut() {
        sessions.push(session);
    }
    let warm = interleave(&mut sessions, sampling)?;
    let seconds = |warm: &Warm| match warm {
        Warm::Measured { seconds, .. } => Ok(seconds.clone()),
        Warm::Unsupported(why) => Err(why.clone()),
    };
    let ours = seconds(&warm[0])?;
    let theirs = match warm.get(1) {
        Some(warm) => {
            let theirs = seconds(warm)?;
            let ratio = Ratio::of(&theirs, &ours);
            format!(
                "{:>14.1} {:>13}",
                theirs.median() * 1e3,
                speedup(ratio.value)
            )
        }
        None => format!("{:>14} {:>13}", "-", "-"),
    };
    Ok(format!(
        "{:<38} {:>15.1} {theirs}",
        workload.name,
        ours.median() * 1e3
    ))
}

/// The document of `workload` in pltrs (Rust), written reproducibly.
fn rust_document(workload: &Workload) -> Result<Vec<u8>, Box<dyn Error>> {
    let built = workload.build();
    Ok(render(workload, &built, workload.output, Documents::Reproducible)?.document)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(args: &[&str]) -> Result<Request<'static>, String> {
        let owned: Vec<String> = args.iter().map(|&a| a.to_owned()).collect();
        let leaked: &'static [String] = Vec::leak(owned);
        Request::parse(leaked)
    }

    #[test]
    fn requests_combine_only_what_makes_sense() {
        let report = parsed(&["small/", "--report", "--runs", "4", "--to", "dir"]).expect("valid");
        assert!(report.report && report.selects("small/line_png") && !report.selects("calls/x"));
        assert_eq!(report.runs, Some(4));
        assert!(parsed(&["--rewrite-report"]).is_ok());
        assert!(parsed(&["--report", "--reference"]).is_err());
        assert!(parsed(&["--rewrite-report", "small/"]).is_err());
        assert!(parsed(&["--to", "dir"]).is_err());
        assert!(parsed(&["--runs", "0"]).is_err());
        assert!(parsed(&["--fast"]).is_err());
        let pack = parsed(&["--pack", "dir", "--revision", "abc"]).expect("valid");
        assert_eq!(
            (pack.pack, pack.revision),
            (Some("dir".into()), Some("abc"))
        );
        assert!(parsed(&["--pack", "dir", "small/"]).is_err());
        assert!(parsed(&["--revision", "abc"]).is_err());
        let against = parsed(&["small/", "--against", "dir", "--runs", "5"]).expect("valid");
        assert_eq!(against.against, Some("dir".into()));
        assert!(parsed(&["--against", "dir", "--report"]).is_err());
    }
}
