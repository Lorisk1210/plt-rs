//! One workload measured in the three implementations: the guard, then
//! warm rounds, then cold rounds.

use std::error::Error;
use std::path::PathBuf;

use super::super::child;
use super::super::cold::{ColdCommand, time_cold};
use super::super::guard::{Guarded, compare_documents, compare_frames, compare_pngs};
use super::super::implementation::{Implementation, PerImplementation};
use super::super::pack;
use super::super::python::PythonRunner;
use super::super::results::{Column, Guard, Plan, WarmResult, WorkloadResult};
use super::super::rounds::{Warm, interleave};
use super::super::session::{Outcome, RustSession, ServedSession, Session};
use super::super::workloads::{Output, Script, Workload};

/// The two Python columns, and where the files of a run go.
pub(super) struct Bench {
    /// matplotlib.
    pub(super) matplotlib: PythonRunner,
    /// pltrs (Python).
    pub(super) pltrs: PythonRunner,
    /// `target/bench`: renderings of the guard, documents of the cold
    /// runs, standard errors.
    pub(super) out: PathBuf,
    /// The configuration directory of pltrs (Rust)'s cold runs.
    pub(super) config: PathBuf,
}

impl Bench {
    /// The file `workload` makes in `implementation` for `kind` of run,
    /// with `extension`: `out/<kind>/<implementation>/<group>__<name>.<extension>`.
    fn artifact(
        &self,
        kind: &str,
        implementation: Implementation,
        workload: &Workload,
        extension: &str,
    ) -> Result<PathBuf, Box<dyn Error>> {
        let dir = self.out.join(kind).join(implementation.key());
        std::fs::create_dir_all(&dir)?;
        Ok(dir.join(format!("{}.{extension}", workload.name.replace('/', "__"))))
    }

    /// Where the standard errors go.
    pub(super) fn logs(&self) -> PathBuf {
        self.out.join("logs")
    }

    /// The cold run of pltrs (Rust) writing `workload`'s document.
    fn rust_cold(&self, workload: &Workload) -> Result<ColdCommand, Box<dyn Error>> {
        let extension = workload.extension().unwrap_or("txt");
        let document = self.artifact("cold", Implementation::Rust, workload, extension)?;
        let mut command = child::cold_command(pack::current(), workload, &document)?;
        command.env.push((
            "PLTRSCONFIGDIR".to_owned(),
            super::super::python::path_text(&self.config)?.to_owned(),
        ));
        Ok(command)
    }
}

/// Guards and times `workload` in the three implementations.
pub(super) fn measure(
    workload: &'static Workload,
    bench: &Bench,
    plan: Plan,
) -> Result<WorkloadResult, Box<dyn Error>> {
    let mut rust = RustSession::new(workload);
    let script = match workload.script {
        Script::Python(script) => script,
        Script::NotComparable(why) => return measure_alone(workload, bench, plan, rust, why),
    };
    let logs = bench.logs();
    std::fs::create_dir_all(&logs)?;
    let log = |i: Implementation| logs.join(format!("{}.serve.stderr", i.key()));
    let mut matplotlib = match ServedSession::start(
        &bench.matplotlib,
        workload,
        script,
        &log(Implementation::Matplotlib),
    )? {
        Outcome::Done(session) => session,
        Outcome::Unsupported(why) => {
            return Err(format!("matplotlib cannot run {script}.py: {why}").into());
        }
    };
    let mut python =
        ServedSession::start(&bench.pltrs, workload, script, &log(Implementation::Python))?;
    let reference = save_all(workload, bench, &mut matplotlib)?;
    let reference = required(reference, script)?;
    let python_guard = match &mut python {
        Outcome::Done(session) => guard(workload, bench, session, &reference)?,
        Outcome::Unsupported(why) => Column::NotSupported(why.clone()),
    };
    let guard = Guard {
        rust: guard(workload, bench, &mut rust, &reference)?,
        python: python_guard,
    };
    let warm = {
        let mut sessions: Vec<&mut dyn Session> = vec![&mut matplotlib];
        if let (Outcome::Done(session), None) = (&mut python, guard.python.skipped::<()>()) {
            sessions.push(session);
        }
        sessions.push(&mut rust);
        let measured = interleave(&mut sessions, plan.warm)?;
        let implementations: Vec<Implementation> =
            sessions.iter().map(|s| s.implementation()).collect();
        warm_columns(&implementations, measured, &guard)
    };
    drop((matplotlib, python, rust));
    if let Column::NotSupported(why) = &warm.matplotlib {
        return Err(format!("matplotlib cannot run {script}.py: {why}").into());
    }
    let cold = cold(workload, script, bench, &warm, plan)?;
    Ok(WorkloadResult {
        name: workload.name.to_owned(),
        tier: workload.tier,
        format: workload.output.name().to_owned(),
        calls: workload.calls,
        guard,
        warm,
        cold,
    })
}

/// A workload without a matplotlib counterpart, timed in pltrs (Rust)
/// alone.
fn measure_alone(
    workload: &'static Workload,
    bench: &Bench,
    plan: Plan,
    mut rust: RustSession,
    why: &str,
) -> Result<WorkloadResult, Box<dyn Error>> {
    let measured = interleave(&mut [&mut rust], plan.warm)?;
    let warm = warm_columns(
        &[Implementation::Rust],
        measured,
        &Guard {
            python: not_comparable(why),
            rust: not_comparable(why),
        },
    );
    let cold = time_cold(
        &bench.matplotlib.python,
        &[bench.rust_cold(workload)?],
        plan.cold,
        &bench.logs(),
    )?;
    let rust_cold = cold.into_iter().next().ok_or("no cold result")?;
    Ok(WorkloadResult {
        name: workload.name.to_owned(),
        tier: workload.tier,
        format: workload.output.name().to_owned(),
        calls: workload.calls,
        guard: Guard {
            python: not_comparable(why),
            rust: not_comparable(why),
        },
        warm: PerImplementation {
            matplotlib: not_comparable(why),
            python: not_comparable(why),
            rust: warm.rust,
        },
        cold: PerImplementation {
            matplotlib: not_comparable(why),
            python: not_comparable(why),
            rust: Column::Measured(rust_cold),
        },
    })
}

/// A column left empty for `why`.
fn not_comparable<T>(why: &str) -> Column<T> {
    Column::NotComparable(why.to_owned())
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

/// The files one implementation saved for the guard.
struct Saved {
    /// The PNG, unless the workload renders frames.
    png: Option<PathBuf>,
    /// The document in the workload's format, unless that is PNG or
    /// nothing.
    document: Option<PathBuf>,
}

/// Saves what the guard compares of `workload` through `session`.
fn save_all(
    workload: &Workload,
    bench: &Bench,
    session: &mut dyn Session,
) -> Result<Outcome<Saved>, Box<dyn Error>> {
    let implementation = session.implementation();
    let mut saved = Saved {
        png: None,
        document: None,
    };
    if let Some(png) = workload.png_output() {
        let path = bench.artifact("guard", implementation, workload, "png")?;
        if let Outcome::Unsupported(why) = session.save(png, &path)? {
            return Ok(Outcome::Unsupported(why));
        }
        saved.png = Some(path);
    }
    let document = match workload.output {
        Output::Save { format, .. } => (format.extension() != "png").then_some(format.extension()),
        Output::Frames => Some("html"),
        Output::LayoutOnly => None,
    };
    if let Some(extension) = document {
        let path = bench.artifact("guard", implementation, workload, extension)?;
        if let Outcome::Unsupported(why) = session.save(workload.output, &path)? {
            return Ok(Outcome::Unsupported(why));
        }
        saved.document = Some(path);
    }
    Ok(Outcome::Done(saved))
}

/// The guard's verdicts on what `session` saves, against matplotlib's
/// `reference`.
fn guard(
    workload: &Workload,
    bench: &Bench,
    session: &mut dyn Session,
    reference: &Saved,
) -> Result<Column<Guarded>, Box<dyn Error>> {
    let saved = match save_all(workload, bench, session)? {
        Outcome::Done(saved) => saved,
        Outcome::Unsupported(why) => return Ok(Column::NotSupported(why)),
    };
    let pair = |a: &Option<PathBuf>, b: &Option<PathBuf>| a.clone().zip(b.clone());
    let pixels = match (pair(&reference.png, &saved.png), workload.output) {
        (Some((expected, actual)), _) => compare_pngs(&expected, &actual),
        (None, Output::Frames) => {
            let (expected, actual) =
                pair(&reference.document, &saved.document).ok_or("an animation saves its page")?;
            let frames = actual.with_extension("frames");
            compare_frames(&expected, &actual, &frames)
        }
        (None, _) => return Err("the guard compares a PNG of every figure".into()),
    };
    let structure = pair(&reference.document, &saved.document).and_then(|(e, a)| {
        let extension = workload.extension()?;
        compare_documents(extension, &e, &a)
    });
    Ok(Column::Measured(Guarded { pixels, structure }))
}

/// The warm columns of the three implementations, from the rounds of the
/// sessions of `implementations` (in that order) and the reasons the
/// guard gave for leaving one out.
fn warm_columns(
    implementations: &[Implementation],
    measured: Vec<Warm>,
    guard: &Guard,
) -> PerImplementation<Column<WarmResult>> {
    let mut columns = PerImplementation::from_fn(|i| match i {
        Implementation::Python => guard
            .python
            .skipped()
            .unwrap_or_else(|| Column::NotSupported("not timed".to_owned())),
        _ => Column::NotComparable("not timed".to_owned()),
    });
    for (implementation, warm) in implementations.iter().zip(measured) {
        *columns.get_mut(*implementation) = match warm {
            Warm::Measured { seconds, bytes } => Column::Measured(WarmResult { seconds, bytes }),
            Warm::Unsupported(why) => Column::NotSupported(why),
        };
    }
    columns
}

/// Times `script` (in matplotlib, and in pltrs (Python) where it ran
/// warm) and pltrs (Rust) in fresh processes, in rounds.
fn cold(
    workload: &Workload,
    script: &str,
    bench: &Bench,
    warm: &PerImplementation<Column<WarmResult>>,
    plan: Plan,
) -> Result<PerImplementation<Column<super::super::cold::ColdResult>>, Box<dyn Error>> {
    let extension = workload.extension().unwrap_or("txt");
    let mut commands = Vec::new();
    let mut timed = Vec::new();
    for runner in [&bench.matplotlib, &bench.pltrs] {
        if warm.get(runner.implementation).measured().is_some() {
            let document = bench.artifact("cold", runner.implementation, workload, extension)?;
            commands.push(runner.script_command(workload, script, &document));
            timed.push(runner.implementation);
        }
    }
    commands.push(bench.rust_cold(workload)?);
    timed.push(Implementation::Rust);
    let results = time_cold(
        &bench.matplotlib.python,
        &commands,
        plan.cold,
        &bench.logs(),
    )?;
    let mut columns = PerImplementation::from_fn(|i| {
        warm.get(i)
            .skipped()
            .unwrap_or_else(|| Column::NotComparable("not timed".to_owned()))
    });
    for (implementation, result) in timed.into_iter().zip(results) {
        *columns.get_mut(implementation) = Column::Measured(result);
    }
    Ok(columns)
}
