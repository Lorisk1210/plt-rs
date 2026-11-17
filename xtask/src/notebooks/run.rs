//! `cargo xtask notebooks [PATTERN...] [--failures]`: the notebooks of
//! `spec/notebooks` executed under matplotlib and under pltrs, their
//! records compared.
//!
//! Both runs execute at once, each with its venv's Python running
//! `spec/notebooks/execute.py`, which writes a record per notebook to
//! `target/notebooks/<mode>/records/` (and its log beside). Every check of
//! the comparison (`compare`) becomes an outcome of the notebook's
//! `notebooks/<name>.ipynb` in `spec/notebooks/<mode>.json`, replacing
//! those of the notebooks run.

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Instant;

use pltrs_core::json;

use super::compare::{Check, compare};
use super::record::Record;
use crate::mpl_suite::outcomes::Outcomes;
use crate::mpl_suite::{Mode, Options, pytest, report};
use crate::python;

/// The directory of the notebooks, and the prefix of their outcome ids.
const NOTEBOOKS: &str = "spec/notebooks";

/// The file part of the outcome ids of the notebook `name`.
fn file_id(name: &str) -> String {
    format!("notebooks/{name}.ipynb")
}

/// Executes and compares the notebooks `args` selects.
pub(super) fn run(args: &[String]) -> Result<(), String> {
    let options = Options::parse(args);
    if options.imports || options.mode == Mode::Reference {
        return Err(
            "`notebooks` runs both matplotlib and pltrs; it takes patterns and --failures"
                .to_owned(),
        );
    }
    let names = select(&notebook_names()?, &options.patterns)?;
    python::build(python::Profile::Release)?;

    let started = Instant::now();
    let runs = [Mode::Reference, Mode::Pltrs].map(|mode| execute(mode, &names));
    for (mode, child) in [Mode::Reference, Mode::Pltrs].into_iter().zip(runs) {
        let status = child?
            .wait()
            .map_err(|err| format!("cannot wait for the {} run: {err}", mode.name()))?;
        if !status.success() {
            return Err(format!(
                "the {} run failed ({status}); see {}",
                mode.name(),
                log_file(mode).display()
            ));
        }
    }
    println!(
        "executed {} notebooks under matplotlib and pltrs in {:.1} s",
        names.len(),
        started.elapsed().as_secs_f64()
    );

    let mut checks = Vec::new();
    for name in &names {
        let reference = Record::read(&records_dir(Mode::Reference).join(format!("{name}.json")))?;
        let pltrs = Record::read(&records_dir(Mode::Pltrs).join(format!("{name}.json")))?;
        checks.extend(compare(&file_id(name), &reference, &pltrs));
    }
    let ran: Vec<String> = names.iter().map(|name| file_id(name)).collect();
    let mut merged = Vec::new();
    for mode in [Mode::Reference, Mode::Pltrs] {
        let run = Outcomes::from_results(&results(&checks, mode))?;
        let path = super::outcome_file(mode);
        let details = super::details_file(mode);
        let mut outcomes = Outcomes::load(&path, &details)?;
        outcomes.merge(run, &ran);
        outcomes.save(&path, &details, &super::versions())?;
        println!("wrote {}", path.display());
        merged.push(outcomes);
    }
    report::print(&merged[0], Some(&merged[1]), options.failures);
    Ok(())
}

/// The names (without `.ipynb`) of the notebooks in `spec/notebooks`,
/// sorted.
fn notebook_names() -> Result<Vec<String>, String> {
    let dir = python::root().join(NOTEBOOKS);
    let mut names: Vec<String> = fs::read_dir(&dir)
        .map_err(|err| format!("{}: {err}", dir.display()))?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.strip_suffix(".ipynb").map(str::to_owned)
        })
        .collect();
    names.sort();
    Ok(names)
}

/// The notebooks of `names` whose names contain a pattern of `patterns`
/// (all of them without patterns).
fn select(names: &[String], patterns: &[String]) -> Result<Vec<String>, String> {
    for pattern in patterns {
        if !names.iter().any(|name| name.contains(pattern.as_str())) {
            return Err(format!("no notebook of {NOTEBOOKS} matches {pattern:?}"));
        }
    }
    Ok(names
        .iter()
        .filter(|name| patterns.is_empty() || patterns.iter().any(|p| name.contains(p.as_str())))
        .cloned()
        .collect())
}

/// The directory of `mode`'s run.
fn run_dir(mode: Mode) -> PathBuf {
    super::notebooks_dir().join(mode.name())
}

/// Where `mode`'s run writes its records.
fn records_dir(mode: Mode) -> PathBuf {
    run_dir(mode).join("records")
}

/// The log of `mode`'s run.
fn log_file(mode: Mode) -> PathBuf {
    run_dir(mode).join("execute.log")
}

/// Starts executing the notebooks `names` under `mode`.
fn execute(mode: Mode, names: &[String]) -> Result<Child, String> {
    let dir = run_dir(mode);
    fs::create_dir_all(&dir).map_err(|err| format!("{}: {err}", dir.display()))?;
    let log_path = log_file(mode);
    let log =
        fs::File::create(&log_path).map_err(|err| format!("{}: {err}", log_path.display()))?;
    let stderr = log
        .try_clone()
        .map_err(|err| format!("{}: {err}", log_path.display()))?;
    let root = python::root();
    let mut command = Command::new(super::interpreter(mode)?);
    command
        .arg(root.join(NOTEBOOKS).join("execute.py"))
        .arg(records_dir(mode))
        .args(
            names
                .iter()
                .map(|name| root.join(NOTEBOOKS).join(format!("{name}.ipynb"))),
        )
        .envs(super::environment(mode, &dir)?)
        .env("PYTHONPATH", python_path(mode)?)
        .env("PLTRS_LOOK", "matplotlib")
        .envs(
            pytest::own_directories(&dir)?
                .iter()
                .map(|(name, path)| (name, path)),
        )
        .env_remove(pytest::CONFIG_DIR_VARIABLE)
        .env_remove("MPLBACKEND")
        .current_dir(&dir)
        .stdin(Stdio::null())
        .stdout(log)
        .stderr(stderr);
    println!(
        "executing {} notebooks under {} (log: {})",
        names.len(),
        mode.name(),
        log_path.display()
    );
    command
        .spawn()
        .map_err(|err| format!("cannot run {}: {err}", command.get_program().display()))
}

/// The `PYTHONPATH` of `mode`'s run: under pltrs, `python/alias` and
/// `python`, so that `import matplotlib` gives pltrs.
fn python_path(mode: Mode) -> Result<std::ffi::OsString, String> {
    let sources = python::root().join("python");
    let dirs = match mode {
        Mode::Reference => Vec::new(),
        Mode::Pltrs => vec![sources.join("alias"), sources],
    };
    python::python_path(dirs)
}

/// The checks' outcomes under `mode`, as the suites' outcome plugin
/// writes them (a JSON object per line).
fn results(checks: &[Check], mode: Mode) -> String {
    checks.iter().fold(String::new(), |mut text, check| {
        let reason = match mode {
            Mode::Reference => &check.reference,
            Mode::Pltrs => &check.pltrs,
        };
        let outcome = if reason.is_some() { "failed" } else { "passed" };
        let _ = writeln!(
            text,
            "{{\"id\": {}, \"outcome\": \"{outcome}\", \"reason\": {}}}",
            json::string(&check.id),
            json::string(reason.as_deref().unwrap_or_default())
        );
        text
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patterns_select_notebooks_by_name() {
        let names = ["inline_display", "inline_formats", "widget"].map(str::to_owned);
        assert_eq!(select(&names, &[]).unwrap(), names);
        assert_eq!(
            select(&names, &["inline".to_owned()]).unwrap(),
            ["inline_display", "inline_formats"]
        );
        assert!(select(&names, &["nbagg".to_owned()]).is_err());
    }

    #[test]
    fn the_notebooks_are_found() {
        let names = notebook_names().unwrap();
        assert!(
            names.iter().any(|name| name == "inline_display"),
            "{names:?}"
        );
        let dir = python::root().join(NOTEBOOKS);
        assert!(
            names
                .iter()
                .all(|name| dir.join(format!("{name}.ipynb")).is_file())
        );
    }

    #[test]
    fn checks_become_outcome_lines() {
        let checks = [
            Check {
                id: "notebooks/n.ipynb::run".to_owned(),
                reference: None,
                pltrs: Some("cell \"c\": dead".to_owned()),
            },
            Check {
                id: "notebooks/n.ipynb::c::outputs".to_owned(),
                reference: None,
                pltrs: None,
            },
        ];
        let text = results(&checks, Mode::Pltrs);
        assert!(Outcomes::from_results(&text).is_ok());
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(
            lines,
            [
                r#"{"id": "notebooks/n.ipynb::run", "outcome": "failed", "reason": "cell \"c\": dead"}"#,
                r#"{"id": "notebooks/n.ipynb::c::outputs", "outcome": "passed", "reason": ""}"#,
            ]
        );
        let reference = results(&checks, Mode::Reference);
        assert_eq!(reference.matches(r#""outcome": "passed""#).count(), 2);
    }
}
