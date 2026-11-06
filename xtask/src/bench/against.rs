//! `cargo xtask bench [PREFIX...] --against DIR [--runs N]`: this build
//! against another, the pack in `DIR` (see `pack`), so that a change can
//! show what it costs on a machine as it is, load and all.
//!
//! Both faces of pltrs run alike in both builds, on the venvs of this
//! one: pltrs (Rust) in a process serving the workload (`--child serve`),
//! pltrs (Python) in the harness's `serve` mode, each build with its own
//! package and configuration directory. Each workload is timed warm, the
//! four sessions taking turns round by round as the report's do, then
//! cold, in rounds of fresh processes after one untimed round; the
//! start-ups (`import pltrs.pyplot`, `--child start`) come first.
//!
//! The table gives this build's change against the other: its median over
//! the other's, with the 95% interval of the ratio, a change counting only
//! when the interval excludes 1. Every median also goes to
//! `target/bench/against.tsv`, so that passes can be combined.

use std::error::Error;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use super::child;
use super::cold::{ColdCommand, time_cold};
use super::implementation::Implementation;
use super::pack::{self, Tree};
use super::python::{PythonRunner, path_text};
use super::results::Plan;
use super::rounds::{Warm, interleave};
use super::session::{Outcome, ServedSession, Session};
use super::stats::{Ratio, Samples, Sampling, Verdict, geometric_mean};
use super::workloads::{Script, Workload};

/// The columns of the table: a face of pltrs, warm or cold.
const COLUMNS: [&str; 4] = ["Rust warm", "Python warm", "Rust cold", "Python cold"];

/// A change of more than this much counts as a slowdown in the summary.
const NOTABLE: f64 = 1.03;

/// One build of the comparison.
struct Side<'a> {
    /// `this` or `other`.
    name: &'static str,
    /// Where its files are.
    tree: &'a Tree,
    /// Its pltrs (Python).
    python: PythonRunner,
    /// The configuration directory of both its faces.
    config: PathBuf,
}

impl<'a> Side<'a> {
    /// The build in `tree`, its files under `out/<name>`.
    fn new(name: &'static str, tree: &'a Tree, out: &Path) -> Result<Self, Box<dyn Error>> {
        let dir = out.join(name);
        let config = dir.join("config");
        Ok(Self {
            name,
            tree,
            python: PythonRunner::pltrs(&tree.root, &dir.join("scripts"), &config)?,
            config,
        })
    }

    /// `command` with this build's configuration directory, labelled with
    /// the build's name and `face`.
    fn labelled(
        &self,
        face: &str,
        mut command: ColdCommand,
    ) -> Result<ColdCommand, Box<dyn Error>> {
        command.label = format!("{}-{face}", self.name);
        command.env.push((
            "PLTRSCONFIGDIR".to_owned(),
            path_text(&self.config)?.to_owned(),
        ));
        Ok(command)
    }
}

/// The medians of one column of one row, and their ratio.
struct Change {
    /// This build's median, in seconds.
    this: f64,
    /// The other build's.
    other: f64,
    /// This build's time over the other's.
    ratio: Ratio,
}

impl Change {
    /// The change from the runs of the other build to those of this one.
    fn of(this: &Samples, other: &Samples) -> Self {
        Self {
            this: this.median(),
            other: other.median(),
            ratio: Ratio::of(other, this),
        }
    }
}

/// A row of the table: a workload (or the start-ups) and its columns.
struct Row {
    /// What was timed.
    name: String,
    /// The changes, in the order of [`COLUMNS`]; `None` where a face does
    /// not run it.
    changes: [Option<Change>; 4],
}

/// Times `workloads` in this build and in the pack in `other`, `runs`
/// rounds of each at least, and prints the table.
pub(super) fn run(
    workloads: &[&'static Workload],
    runs: usize,
    other: &Path,
) -> Result<(), Box<dyn Error>> {
    let other = Tree::pack(other)?;
    let this = pack::current();
    let out = this.root.join("target/bench/against");
    let sides = [
        Side::new("this", this, &out)?,
        Side::new("other", &other, &out)?,
    ];
    let logs = out.join("logs");
    std::fs::create_dir_all(&logs)?;
    let plan = Plan::new(runs);
    println!(
        "this build against {} ({}): this build's median over the other's [95% interval]",
        other.root.display(),
        other.build.as_ref().map_or("?", |b| b.revision.as_str())
    );
    println!("{}", header());
    let mut rows = vec![startup(&sides, plan, &logs)?];
    println!("{}", line(&rows[0]));
    for workload in workloads {
        let row = compare(workload, &sides, plan, &logs)
            .map_err(|err| format!("{}: {err}", workload.name))?;
        println!("{}", line(&row));
        rows.push(row);
    }
    println!("{}", summary(&rows));
    let tsv = this.root.join("target/bench/against.tsv");
    std::fs::write(&tsv, table_tsv(&rows))?;
    println!("medians in {}", tsv.display());
    Ok(())
}

/// The start-ups of both builds: a fresh interpreter importing
/// `pltrs.pyplot`, and pltrs (Rust)'s executable starting and exiting.
fn startup(sides: &[Side<'_>; 2], plan: Plan, logs: &Path) -> Result<Row, Box<dyn Error>> {
    let mut commands = Vec::new();
    for side in sides {
        let start = child::start_command(side.tree)?;
        commands.push(side.labelled("rust", start)?);
        let import = side.python.code_command("import pltrs.pyplot");
        commands.push(side.labelled("python", import)?);
    }
    let cold = timed_cold(&commands, plan.startup, logs)?;
    Ok(Row {
        name: "start-up".to_owned(),
        changes: [
            None,
            None,
            Some(Change::of(&cold[0], &cold[2])),
            Some(Change::of(&cold[1], &cold[3])),
        ],
    })
}

/// Times `workload` in both builds, warm and cold.
fn compare(
    workload: &'static Workload,
    sides: &[Side<'_>; 2],
    plan: Plan,
    logs: &Path,
) -> Result<Row, Box<dyn Error>> {
    let script = match workload.script {
        Script::Python(script) => Some(script),
        Script::NotComparable(_) => None,
    };
    let warm = warm(workload, script, sides, plan.warm, logs)?;
    let cold = cold(workload, script, sides, plan.cold, logs)?;
    let faces = if script.is_some() { 2 } else { 1 };
    let pair = |runs: &[Samples], face: usize| {
        (face < faces).then(|| Change::of(&runs[face], &runs[faces + face]))
    };
    Ok(Row {
        name: workload.name.to_owned(),
        changes: [
            pair(&warm, 0),
            pair(&warm, 1),
            pair(&cold, 0),
            pair(&cold, 1),
        ],
    })
}

/// The warm runs of `workload` in both builds, the sessions of every face
/// taking turns: per build, pltrs (Rust)'s, then pltrs (Python)'s running
/// `script` where there is one.
fn warm(
    workload: &'static Workload,
    script: Option<&str>,
    sides: &[Side<'_>; 2],
    sampling: Sampling,
    logs: &Path,
) -> Result<Vec<Samples>, Box<dyn Error>> {
    let mut sessions = Vec::new();
    for side in sides {
        let mut serve = child::serve_command(side.tree, workload)?;
        serve.env("PLTRSCONFIGDIR", &side.config);
        let log = logs.join(format!("{}-rust.serve.stderr", side.name));
        let title = format!("pltrs (Rust) of the {} build", side.name);
        sessions.push(served(ServedSession::spawn(
            Implementation::Rust,
            title,
            serve,
            &log,
        )?)?);
        if let Some(script) = script {
            let log = logs.join(format!("{}-python.serve.stderr", side.name));
            sessions.push(served(ServedSession::start(
                &side.python,
                workload,
                script,
                &log,
            )?)?);
        }
    }
    let mut running: Vec<&mut dyn Session> =
        sessions.iter_mut().map(|s| s as &mut dyn Session).collect();
    interleave(&mut running, sampling)?
        .into_iter()
        .map(|warm| match warm {
            Warm::Measured { seconds, .. } => Ok(seconds),
            Warm::Unsupported(why) => Err(why.into()),
        })
        .collect()
}

/// The cold runs of `workload` in both builds, in the order of [`warm`].
fn cold(
    workload: &Workload,
    script: Option<&str>,
    sides: &[Side<'_>; 2],
    sampling: Sampling,
    logs: &Path,
) -> Result<Vec<Samples>, Box<dyn Error>> {
    let extension = workload.extension().unwrap_or("txt");
    let document =
        |side: &Side<'_>, face: &str| logs.join(format!("{}-{face}.{extension}", side.name));
    let mut commands = Vec::new();
    for side in sides {
        let rust = child::cold_command(side.tree, workload, &document(side, "rust"))?;
        commands.push(side.labelled("rust", rust)?);
        if let Some(script) = script {
            let python = side
                .python
                .script_command(workload, script, &document(side, "python"));
            commands.push(side.labelled("python", python)?);
        }
    }
    timed_cold(&commands, sampling, logs)
}

/// A session that started, or the reason it did not.
fn served(outcome: Outcome<ServedSession>) -> Result<ServedSession, Box<dyn Error>> {
    match outcome {
        Outcome::Done(session) => Ok(session),
        Outcome::Unsupported(why) => Err(why.into()),
    }
}

/// Times `commands` cold after one untimed round, which puts both builds'
/// files in the page cache and their font catalogues on disk.
fn timed_cold(
    commands: &[ColdCommand],
    sampling: Sampling,
    logs: &Path,
) -> Result<Vec<Samples>, Box<dyn Error>> {
    let python = pack::reference_python();
    let once = Sampling {
        min_runs: 1,
        min_seconds: 0.0,
        max_runs: 1,
    };
    time_cold(&python, commands, once, logs)?;
    Ok(time_cold(&python, commands, sampling, logs)?
        .into_iter()
        .map(|result| result.seconds)
        .collect())
}

/// The table's header.
fn header() -> String {
    let mut header = format!("{:<38}", "workload");
    for column in COLUMNS {
        let _ = write!(header, " {column:>22}");
    }
    header
}

/// The table's line of `row`.
fn line(row: &Row) -> String {
    let mut line = format!("{:<38}", row.name);
    for change in &row.changes {
        let cell = change.as_ref().map_or_else(|| "-".to_owned(), cell);
        let _ = write!(line, " {cell:>22}");
    }
    line
}

/// A change as a signed percentage with its interval.
fn cell(change: &Change) -> String {
    let percent = |ratio: f64| (ratio - 1.0) * 100.0;
    format!(
        "{:+.1}% [{:+.1}, {:+.1}]",
        percent(change.ratio.value),
        percent(change.ratio.low),
        percent(change.ratio.high)
    )
}

/// Per column: the typical change (the geometric mean of the ratios),
/// how many rows were faster, within the noise and slower, and the rows
/// slower by more than [`NOTABLE`].
fn summary(rows: &[Row]) -> String {
    let mut text = String::new();
    for (i, column) in COLUMNS.iter().enumerate() {
        let changes: Vec<(&str, &Change)> = rows
            .iter()
            .filter_map(|row| Some((row.name.as_str(), row.changes[i].as_ref()?)))
            .collect();
        let ratios: Vec<f64> = changes.iter().map(|(_, c)| c.ratio.value).collect();
        let Some(mean) = geometric_mean(&ratios) else {
            continue;
        };
        let count = |verdict| {
            changes
                .iter()
                .filter(|(_, c)| c.ratio.verdict() == verdict)
                .count()
        };
        let notable: Vec<String> = changes
            .iter()
            .filter(|(_, c)| c.ratio.verdict() == Verdict::Slower && c.ratio.value > NOTABLE)
            .map(|(name, c)| format!("{name} {}", cell(c)))
            .collect();
        let _ = writeln!(
            text,
            "{column}: typically {:+.1}%; faster / within noise / slower: {} / {} / {}",
            (mean - 1.0) * 100.0,
            count(Verdict::Faster),
            count(Verdict::Tied),
            count(Verdict::Slower)
        );
        for slow in notable {
            let _ = writeln!(text, "    slower: {slow}");
        }
    }
    text
}

/// Every change as tab-separated values: row, column, both medians in ms,
/// the ratio and its interval.
fn table_tsv(rows: &[Row]) -> String {
    let mut tsv = "row\tcolumn\tthis_ms\tother_ms\tratio\tlow\thigh\n".to_owned();
    for row in rows {
        for (column, change) in COLUMNS.iter().zip(&row.changes) {
            if let Some(c) = change {
                let _ = writeln!(
                    tsv,
                    "{}\t{column}\t{:.4}\t{:.4}\t{:.4}\t{:.4}\t{:.4}",
                    row.name,
                    c.this * 1e3,
                    c.other * 1e3,
                    c.ratio.value,
                    c.ratio.low,
                    c.ratio.high
                );
            }
        }
    }
    tsv
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change(this: f64, other: f64) -> Change {
        let samples = |t: f64| Samples::new(vec![t, t * 1.001, t * 0.999]).expect("times");
        Change::of(&samples(this), &samples(other))
    }

    #[test]
    fn changes_read_as_this_build_against_the_other() {
        let slower = change(1.1, 1.0);
        assert!((slower.ratio.value - 1.1).abs() < 1e-9);
        assert!(cell(&slower).starts_with("+10.0%"));
        let row = Row {
            name: "small/line_png".to_owned(),
            changes: [Some(slower), Some(change(0.9, 1.0)), None, None],
        };
        let text = summary(std::slice::from_ref(&row));
        assert!(
            text.contains("Rust warm: typically +10.0%; faster / within noise / slower: 0 / 0 / 1")
        );
        assert!(text.contains("    slower: small/line_png +10.0%"));
        assert!(
            text.contains(
                "Python warm: typically -10.0%; faster / within noise / slower: 1 / 0 / 0"
            )
        );
        assert!(!text.contains("Rust cold"));
        assert!(line(&row).ends_with(" -"));
        let tsv = table_tsv(&[row]);
        assert_eq!(tsv.lines().count(), 3);
        assert!(tsv.contains("small/line_png\tRust warm\t1100.0000\t1000.0000\t1.1000"));
    }
}
