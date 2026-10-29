//! `cargo xtask bench --report`: every workload timed in matplotlib,
//! pltrs (Python) and pltrs (Rust), warm and cold, after the correctness
//! guard, and the report written to `benchmarks/README.md`.

mod markdown;

use std::error::Error;
use std::path::{Path, PathBuf};

use super::child;
use super::cold::{ColdCommand, ColdResult, time_cold};
use super::guard::{self, Verdict};
use super::machine::{self, Machine};
use super::python::{Outcome, PythonRunner};
use super::render::{render, time_warm};
use super::stats::{Samples, Sampling};
use super::workloads::{Script, Workload};

/// A Python column's value for one workload.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Column<T> {
    /// Measured.
    Measured(T),
    /// pltrs (Python) raised this exception: it lacks something the
    /// script uses.
    NotSupported(String),
    /// matplotlib cannot be timed doing the same work, for this reason.
    NotComparable(&'static str),
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
    fn skipped<U>(&self) -> Option<Column<U>> {
        match self {
            Self::Measured(_) => None,
            Self::NotSupported(why) => Some(Column::NotSupported(why.clone())),
            Self::NotComparable(why) => Some(Column::NotComparable(why)),
        }
    }
}

/// A measurement in the three implementations.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Columns<T> {
    /// matplotlib.
    pub(super) matplotlib: Column<T>,
    /// pltrs (Python), the same scripts.
    pub(super) python: Column<T>,
    /// pltrs (Rust), which runs every workload.
    pub(super) rust: T,
}

/// How the PNGs of pltrs (Python) and pltrs (Rust) compare with
/// matplotlib's.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Guard {
    /// pltrs (Python)'s.
    pub(super) python: Column<Verdict>,
    /// pltrs (Rust)'s, unless matplotlib has no counterpart of the
    /// workload.
    pub(super) rust: Option<Verdict>,
}

/// Everything measured of one workload.
#[derive(Clone, Debug)]
pub(super) struct Measurement {
    /// The workload.
    pub(super) workload: &'static Workload,
    /// The correctness guard's verdicts.
    pub(super) guard: Guard,
    /// Warm, in-process times.
    pub(super) warm: Columns<Samples>,
    /// Cold, fresh-process times and peak memory.
    pub(super) cold: Columns<ColdResult>,
}

/// Fresh processes that only start, import or exit.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Startup {
    /// `python -c pass`.
    pub(super) interpreter: ColdResult,
    /// `python -c "import numpy"`, BLAS on one thread.
    pub(super) numpy: ColdResult,
    /// The same with `OpenBLAS`'s default of a thread per hardware thread.
    pub(super) numpy_threaded: ColdResult,
    /// `python -c "import matplotlib.pyplot"`.
    pub(super) matplotlib: ColdResult,
    /// `python -c "import pltrs.pyplot"`.
    pub(super) python: ColdResult,
    /// The benchmark executable of pltrs (Rust) starting and exiting.
    pub(super) rust: ColdResult,
}

/// The versions of the Python side: `(name, version)` pairs of each venv
/// (Python, numpy, matplotlib where installed) and the date.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Versions {
    /// The reference venv's.
    pub(super) reference: Vec<(String, String)>,
    /// The pltrs (Python) venv's.
    pub(super) pltrs: Vec<(String, String)>,
}

impl Versions {
    /// The reference venv's version of `name`, or `unknown`.
    pub(super) fn reference(&self, name: &str) -> &str {
        lookup(&self.reference, name)
    }

    /// The pltrs (Python) venv's version of `name`, or `unknown`.
    pub(super) fn pltrs(&self, name: &str) -> &str {
        lookup(&self.pltrs, name)
    }
}

/// The value of `name` among `pairs`, or `unknown`.
fn lookup<'a>(pairs: &'a [(String, String)], name: &str) -> &'a str {
    pairs
        .iter()
        .find(|(key, _)| key == name)
        .map_or("unknown", |(_, value)| value.as_str())
}

/// How often each kind of measurement runs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Plan {
    /// Warm runs.
    pub(super) warm: Sampling,
    /// Cold runs of a workload.
    pub(super) cold: Sampling,
    /// Runs of the start-up processes.
    pub(super) startup: Sampling,
}

impl Plan {
    /// At least `runs` runs of everything (twice as many start-ups, which
    /// are quick), more for quick workloads.
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

/// Everything the report shows.
#[derive(Clone, Debug)]
pub(super) struct Report {
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
    pub(super) startup: Startup,
    /// The workloads, in table order.
    pub(super) measurements: Vec<Measurement>,
}

/// The two Python columns, and where the files of a run go.
struct Bench {
    /// matplotlib.
    matplotlib: PythonRunner,
    /// pltrs (Python).
    pltrs: PythonRunner,
    /// `target/bench`: PNGs of the guard, documents of the cold runs,
    /// standard errors.
    out: PathBuf,
}

impl Bench {
    /// The file `workload` makes in `column` for `kind` of run, with
    /// `extension`: `out/<kind>/<column>/<group>__<name>.<extension>`.
    fn artifact(
        &self,
        kind: &str,
        column: &str,
        workload: &Workload,
        extension: &str,
    ) -> Result<PathBuf, Box<dyn Error>> {
        let dir = self.out.join(kind).join(column);
        std::fs::create_dir_all(&dir)?;
        Ok(dir.join(format!("{}.{extension}", workload.name.replace('/', "__"))))
    }

    /// Where the standard errors of cold runs go.
    fn logs(&self) -> PathBuf {
        self.out.join("logs")
    }
}

/// Measures `workloads` and writes the report to `benchmarks/README.md`.
pub(super) fn run(workloads: &[&'static Workload], runs: usize) -> Result<(), Box<dyn Error>> {
    let root = crate::python::root();
    crate::python::build(crate::python::Profile::Release)?;
    let bench = Bench {
        matplotlib: PythonRunner::matplotlib(&root)?,
        pltrs: PythonRunner::pltrs(&root)?,
        out: root.join("target/bench"),
    };
    let plan = Plan::new(runs);
    let load_before = machine::load_average();
    let versions = Versions {
        reference: bench.matplotlib.versions()?,
        pltrs: bench.pltrs.versions()?,
    };
    let machine = Machine::describe();
    eprintln!("timing start-up and imports");
    let startup = startup(&bench, &machine, plan)?;
    let mut measurements = Vec::with_capacity(workloads.len());
    for (i, workload) in workloads.iter().enumerate() {
        eprintln!("[{}/{}] {}", i + 1, workloads.len(), workload.name);
        let measurement =
            measure(workload, &bench, plan).map_err(|err| format!("{}: {err}", workload.name))?;
        eprintln!("    {}", markdown::progress_line(&measurement));
        measurements.push(measurement);
    }
    let report = Report {
        machine,
        versions,
        load_before,
        load_after: machine::load_average(),
        plan,
        startup,
        measurements,
    };
    let path = root.join("benchmarks/README.md");
    std::fs::write(&path, markdown::render(&report))?;
    println!("wrote {}", path.display());
    Ok(())
}

/// Times the processes that only start and import.
fn startup(bench: &Bench, machine: &Machine, plan: Plan) -> Result<Startup, Box<dyn Error>> {
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
    let [interpreter, numpy, numpy_threaded, matplotlib, python, rust] =
        <[ColdResult; 6]>::try_from(results)
            .map_err(|_| "the start-up runs returned the wrong number of results")?;
    Ok(Startup {
        interpreter,
        numpy,
        numpy_threaded,
        matplotlib,
        python,
        rust,
    })
}

/// Guards and times `workload` in the three implementations.
fn measure(
    workload: &'static Workload,
    bench: &Bench,
    plan: Plan,
) -> Result<Measurement, Box<dyn Error>> {
    let build = (workload.setup)();
    let rust_png = bench.artifact("guard", child::LABEL, workload, "png")?;
    std::fs::write(&rust_png, render(workload, &build, workload.png_output())?)?;
    let (rust_warm, _) = time_warm(workload, &build, plan.warm)?;
    drop(build);
    let document = bench.artifact(
        "cold",
        child::LABEL,
        workload,
        workload.extension().unwrap_or("txt"),
    )?;
    let rust_cold = child::cold_command(workload, &document)?;
    let script = match workload.script {
        Script::Python(script) => script,
        Script::NotComparable(why) => {
            let cold = time_cold(
                &bench.matplotlib.python,
                &[rust_cold],
                plan.cold,
                &bench.logs(),
            )?;
            return Ok(Measurement {
                workload,
                guard: Guard {
                    python: Column::NotComparable(why),
                    rust: None,
                },
                warm: Columns {
                    matplotlib: Column::NotComparable(why),
                    python: Column::NotComparable(why),
                    rust: rust_warm,
                },
                cold: Columns {
                    matplotlib: Column::NotComparable(why),
                    python: Column::NotComparable(why),
                    rust: cold.into_iter().next().ok_or("no cold result")?,
                },
            });
        }
    };
    let guard = guard(workload, script, bench, &rust_png)?;
    let warm = Columns {
        matplotlib: Column::Measured(required(
            bench.matplotlib.warm(workload, script, plan.warm)?,
            script,
        )?),
        python: match guard.python.skipped() {
            Some(skipped) => skipped,
            None => match bench.pltrs.warm(workload, script, plan.warm)? {
                Outcome::Done(samples) => Column::Measured(samples),
                Outcome::Unsupported(why) => Column::NotSupported(why),
            },
        },
        rust: rust_warm,
    };
    let cold = cold(workload, script, bench, &warm, rust_cold, plan)?;
    Ok(Measurement {
        workload,
        guard,
        warm,
        cold,
    })
}

/// What matplotlib made of a request, which must succeed.
fn required<T>(outcome: Outcome<T>, script: &str) -> Result<T, Box<dyn Error>> {
    match outcome {
        Outcome::Done(value) => Ok(value),
        Outcome::Unsupported(why) => {
            Err(format!("matplotlib cannot run {script}.py: {why}").into())
        }
    }
}

/// Saves the PNG of `script` in matplotlib and pltrs (Python) and compares
/// it, and the PNG of pltrs (Rust) at `rust_png`, with matplotlib's.
fn guard(
    workload: &Workload,
    script: &str,
    bench: &Bench,
    rust_png: &Path,
) -> Result<Guard, Box<dyn Error>> {
    let reference = bench.artifact("guard", "matplotlib", workload, "png")?;
    required(bench.matplotlib.png(workload, script, &reference)?, script)?;
    let python_png = bench.artifact("guard", bench.pltrs.flavor.name(), workload, "png")?;
    let python = match bench.pltrs.png(workload, script, &python_png)? {
        Outcome::Done(()) => Column::Measured(guard::compare(&reference, &python_png)),
        Outcome::Unsupported(why) => Column::NotSupported(why),
    };
    Ok(Guard {
        python,
        rust: Some(guard::compare(&reference, rust_png)),
    })
}

/// Times `script` (in matplotlib, and in pltrs (Python) where it ran warm)
/// and `rust_cold` of pltrs (Rust) in fresh processes, taking turns.
fn cold(
    workload: &Workload,
    script: &str,
    bench: &Bench,
    warm: &Columns<Samples>,
    rust_cold: ColdCommand,
    plan: Plan,
) -> Result<Columns<ColdResult>, Box<dyn Error>> {
    let extension = workload.extension().unwrap_or("txt");
    let mut commands = Vec::new();
    for (runner, column) in [
        (&bench.matplotlib, &warm.matplotlib),
        (&bench.pltrs, &warm.python),
    ] {
        if column.measured().is_some() {
            let name = runner.flavor.name();
            let document = bench.artifact("cold", name, workload, extension)?;
            commands.push(runner.script_command(workload, script, &document));
        }
    }
    commands.push(rust_cold);
    let mut results = time_cold(
        &bench.matplotlib.python,
        &commands,
        plan.cold,
        &bench.logs(),
    )?
    .into_iter();
    let mut take = |column: &Column<Samples>| -> Result<Column<ColdResult>, Box<dyn Error>> {
        Ok(match column.skipped() {
            Some(skipped) => skipped,
            None => Column::Measured(results.next().ok_or("missing cold result")?),
        })
    };
    let matplotlib = take(&warm.matplotlib)?;
    let python = take(&warm.python)?;
    Ok(Columns {
        matplotlib,
        python,
        rust: results.next().ok_or("missing cold result")?,
    })
}
