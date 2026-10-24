//! `cargo xtask bench`: times realistic large workloads in plt-rs and,
//! with `--reference`, the same workloads in matplotlib (Agg and its
//! vector backends) through the reference venv.
//!
//! Every workload builds a figure under the spec's reference configuration
//! (matplotlib's look) and saves it to memory; its data is made before the
//! clock starts. The best of `--runs` runs (default 3) is reported.
//! `--out DIR` also writes each document to `DIR`, so that the output of
//! two builds can be compared byte for byte: documents carry no metadata
//! and SVG ids a fixed salt.

mod workloads;

use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{Duration, Instant};

use plt::figure::SaveOptions;
use plt_render::Metadata;
use plt_spec::figures::FigureCase;

use workloads::Workload;

/// What `cargo xtask bench` was asked to do.
struct Request<'a> {
    /// Workloads whose names start with one of these (all without any).
    prefixes: Vec<&'a str>,
    /// Also time matplotlib.
    reference: bool,
    /// Runs per workload; the fastest counts.
    runs: usize,
    /// Where to write the documents, if anywhere.
    out: Option<PathBuf>,
}

impl<'a> Request<'a> {
    fn parse(args: &'a [String]) -> Result<Self, String> {
        let mut request = Self {
            prefixes: Vec::new(),
            reference: false,
            runs: 3,
            out: None,
        };
        let mut rest = args.iter();
        while let Some(arg) = rest.next() {
            match arg.as_str() {
                "--reference" => request.reference = true,
                "--runs" => {
                    request.runs = rest
                        .next()
                        .and_then(|n| n.parse().ok())
                        .filter(|&n| n > 0)
                        .ok_or("--runs needs a positive count")?;
                }
                "--out" => {
                    request.out = Some(rest.next().ok_or("--out needs a directory")?.into());
                }
                flag if flag.starts_with("--") => return Err(format!("unknown flag {flag}")),
                prefix => request.prefixes.push(prefix),
            }
        }
        Ok(request)
    }

    fn selects(&self, name: &str) -> bool {
        self.prefixes.is_empty() || self.prefixes.iter().any(|p| name.starts_with(p))
    }
}

/// Runs the benchmark and prints its table.
pub(crate) fn run(args: &[String]) -> ExitCode {
    let request = match Request::parse(args) {
        Ok(request) => request,
        Err(err) => {
            eprintln!("{err}");
            eprintln!("usage: cargo xtask bench [PREFIX...] [--reference] [--runs N] [--out DIR]");
            return ExitCode::FAILURE;
        }
    };
    let selected: Vec<&Workload> = workloads::all()
        .filter(|w| request.selects(w.name))
        .collect();
    let reference = if request.reference {
        match reference_times(&selected, request.runs) {
            Ok(times) => times,
            Err(err) => {
                eprintln!("matplotlib reference failed: {err}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        BTreeMap::new()
    };

    println!(
        "{:<34} {:>11} {:>14} {:>7}",
        "workload", "plt-rs ms", "matplotlib ms", "ratio"
    );
    let mut status = ExitCode::SUCCESS;
    for workload in selected {
        let timed = time_workload(workload, request.runs).and_then(|(best, document)| {
            if let Some(out) = &request.out {
                write_document(out, workload, &document)?;
            }
            Ok(best)
        });
        match timed {
            Ok(best) => {
                let ms = best.as_secs_f64() * 1e3;
                let row = match reference.get(workload.name) {
                    Some(&mpl) => format!("{mpl:>14.1} {:>7.2}", ms / mpl),
                    None => format!("{:>14} {:>7}", "-", "-"),
                };
                println!("{:<34} {ms:>11.1} {row}", workload.name);
            }
            Err(err) => {
                eprintln!("{}: {err}", workload.name);
                status = ExitCode::FAILURE;
            }
        }
    }
    status
}

/// The fastest of `runs` builds and saves of `workload`, and the document
/// saved.
fn time_workload(workload: &Workload, runs: usize) -> Result<(Duration, Vec<u8>), Box<dyn Error>> {
    let build = (workload.setup)();
    let (width, height) = workload.figsize;
    let case = FigureCase::new(workload.name, |_| Ok(())).sized(width, height, workloads::DPI);
    let options = SaveOptions {
        dpi: Some(workloads::DPI),
        metadata: Some(Metadata::none()),
        ..SaveOptions::default()
    };
    let mut best = Duration::MAX;
    let mut document = Vec::new();
    for _ in 0..runs {
        let start = Instant::now();
        document = case.with_figure(|fig| {
            plt::rc::set("svg.hashsalt", "plt-rs")?;
            build(fig)?;
            Ok(fig.to_format(workload.format, options.clone())?)
        })?;
        best = best.min(start.elapsed());
    }
    Ok((best, document))
}

/// Writes `document` to `out/<group>__<name>.<extension>`.
fn write_document(out: &Path, workload: &Workload, document: &[u8]) -> Result<(), Box<dyn Error>> {
    std::fs::create_dir_all(out)?;
    let name = format!(
        "{}.{}",
        workload.name.replace('/', "__"),
        workload.format.extension()
    );
    Ok(std::fs::write(out.join(name), document)?)
}

/// matplotlib's best time in milliseconds for each workload, from
/// `spec/reference/bench/bench.py` run through the reference venv.
fn reference_times(
    workloads: &[&Workload],
    runs: usize,
) -> Result<BTreeMap<String, f64>, Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let output = Command::new(root.join(".venv/bin/python"))
        .arg(root.join("spec/reference/bench/bench.py"))
        .arg("--runs")
        .arg(runs.to_string())
        .args(workloads.iter().map(|w| w.name))
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "bench.py failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    String::from_utf8(output.stdout)?
        .lines()
        .map(|line| {
            let (name, ms) = line
                .split_once('\t')
                .ok_or_else(|| format!("unexpected bench.py output: {line}"))?;
            Ok((name.to_owned(), ms.parse()?))
        })
        .collect()
}
