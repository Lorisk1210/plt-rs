//! `cargo xtask bench --report`: every workload guarded and timed in
//! matplotlib, pltrs (Python) and pltrs (Rust), warm and cold; the raw
//! data written to `results.json`, and the report (`README.md`) and its
//! charts (`charts/*.svg`) written from it. `--rewrite-report` writes the
//! report and the charts again from the raw data alone.

mod analysis;
mod charts;
mod markdown;
mod measure;

use std::error::Error;
use std::path::{Path, PathBuf};

use super::child;
use super::cold::{ColdCommand, time_cold};
use super::machine::{self, Machine};
use super::python::PythonRunner;
use super::results::{self, Plan, Results, Startup, Versions};
use super::workloads::Workload;
use measure::Bench;

/// Where the report, its raw data and its charts go by default.
pub(super) fn default_dir() -> PathBuf {
    crate::python::root().join("benchmarks")
}

/// Measures `workloads`, `runs` rounds of each at least, and writes the
/// raw data, the report and the charts into `dir`.
pub(super) fn run(
    workloads: &[&'static Workload],
    runs: usize,
    dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let root = crate::python::root();
    crate::python::build(crate::python::Profile::Release)?;
    let out = root.join("target/bench");
    let bench = Bench {
        matplotlib: PythonRunner::matplotlib(&root, &out.join("config/matplotlib"))?,
        pltrs: PythonRunner::pltrs(&root, &out.join("pltrs"), &out.join("config/pltrs"))?,
        config: out.join("config/pltrs"),
        out,
    };
    let plan = Plan::new(runs);
    eprintln!("priming the font caches of both Python columns");
    prime(&bench)?;
    let load_before = machine::load_average();
    let versions = Versions {
        reference: bench.matplotlib.versions()?,
        pltrs: bench.pltrs.versions()?,
    };
    if !versions.same_python_and_numpy() {
        eprintln!(
            "warning: the venvs run different interpreters or numpy builds; the report says so"
        );
    }
    let machine = Machine::describe();
    eprintln!("timing start-up and imports");
    let startup = startup(&bench, &machine, plan)?;
    let mut measured = Vec::with_capacity(workloads.len());
    for (i, workload) in workloads.iter().enumerate() {
        eprintln!(
            "[{}/{}] {} (load {})",
            i + 1,
            workloads.len(),
            workload.name,
            machine::load_average()
        );
        let result = measure::measure(workload, &bench, plan)
            .map_err(|err| format!("{}: {err}", workload.name))?;
        eprintln!("    {}", markdown::progress_line(&result));
        measured.push(result);
    }
    let results = Results {
        machine,
        versions,
        load_before,
        load_after: machine::load_average(),
        plan,
        startup,
        workloads: measured,
    };
    std::fs::create_dir_all(dir)?;
    std::fs::write(dir.join("results.json"), results::to_text(&results))?;
    write(&results, dir)
}

/// Writes the report and the charts into `dir` from the raw data there.
pub(super) fn rewrite(dir: &Path) -> Result<(), Box<dyn Error>> {
    let raw = dir.join("results.json");
    let text = std::fs::read_to_string(&raw)
        .map_err(|err| format!("cannot read {}: {err}", raw.display()))?;
    write(&results::from_text(&text)?, dir)
}

/// Writes the report and its charts for `results` into `dir`.
fn write(results: &Results, dir: &Path) -> Result<(), Box<dyn Error>> {
    let charts = charts::write(results, &dir.join("charts"))?;
    let path = dir.join("README.md");
    std::fs::write(&path, markdown::render(results, &charts))?;
    println!("wrote {}", path.display());
    Ok(())
}

/// Runs each Python column's pyplot import once, so that the font caches
/// both keep on disk exist before anything is timed (matplotlib builds
/// its font list in its configuration directory on first import).
fn prime(bench: &Bench) -> Result<(), Box<dyn Error>> {
    let commands = [
        bench.matplotlib.code_command("import matplotlib.pyplot"),
        bench.pltrs.code_command("import pltrs.pyplot"),
        child::start_command()?,
    ];
    let once = super::stats::Sampling {
        min_runs: 1,
        min_seconds: 0.0,
        max_runs: 1,
    };
    time_cold(&bench.matplotlib.python, &commands, once, &bench.logs())?;
    Ok(())
}

/// Times the processes that only start and import.
fn startup(bench: &Bench, machine: &Machine, plan: Plan) -> Result<Vec<Startup>, Box<dyn Error>> {
    let labelled = |label: &str, command: ColdCommand| ColdCommand {
        label: label.to_owned(),
        ..command
    };
    let mut threaded = labelled(
        "numpy-threaded",
        bench.matplotlib.code_command("import numpy"),
    );
    threaded.env.push((
        "OPENBLAS_NUM_THREADS".to_owned(),
        machine.threads.max(1).to_string(),
    ));
    let commands = [
        labelled("python", bench.matplotlib.code_command("pass")),
        labelled("numpy", bench.matplotlib.code_command("import numpy")),
        threaded,
        bench.matplotlib.code_command("import matplotlib.pyplot"),
        bench.pltrs.code_command("import pltrs.pyplot"),
        child::start_command()?,
    ];
    let results = time_cold(
        &bench.matplotlib.python,
        &commands,
        plan.startup,
        &bench.logs(),
    )?;
    Ok(commands
        .into_iter()
        .zip(results)
        .map(|(command, result)| Startup {
            key: command.label,
            result,
        })
        .collect())
}
