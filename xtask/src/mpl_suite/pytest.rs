//! One pytest run of a test suite with the outcome plugin
//! (`python/mpl_suite/mpl_suite_report.py`): matplotlib's, or the
//! libraries' of `cargo xtask ecosystem suite`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::outcomes::Outcomes;

/// A variable a run sets (to `Some` value) or removes (`None`) besides
/// its own.
pub(crate) type Variable = (String, Option<OsString>);

/// The pytest workers of a run.
const WORKERS: &str = "8";

/// The seconds a single test may take before pytest-timeout fails it.
const TEST_TIMEOUT: &str = "600";

/// The variable choosing pltrs' look, set to `matplotlib` for every run:
/// pltrs (Python) draws in its modern look by default, and the suites
/// expect matplotlib's defaults (matplotlib itself ignores it).
const LOOK_VARIABLE: &str = "PLTRS_LOOK";

/// pltrs' own variable naming its configuration directory, which cargo
/// sets for what it runs (`.cargo/config.toml`) and a suite must not
/// pass on: pltrs (Python) then reads matplotlib's `MPLCONFIGDIR`.
pub(crate) const CONFIG_DIR_VARIABLE: &str = "PLTRSCONFIGDIR";

/// A pytest run: the tests `targets` of the site `site`, with `python`.
///
/// Its files go to `dir`: `results.jsonl` (the plugin's), `pytest.log`
/// (pytest's output), `pytest-tmp` (`tmp_path`) and `mplconfig` (the
/// configuration directory).
#[derive(Debug)]
pub(crate) struct Run<'a> {
    /// What the progress line calls the suite ("matplotlib's suite").
    pub(crate) suite: &'a str,
    /// The interpreter.
    pub(crate) python: &'a Path,
    /// The directory pytest runs in.
    pub(crate) site: &'a Path,
    /// The run's own directory.
    pub(crate) dir: &'a Path,
    /// What goes on `PYTHONPATH` ahead of the plugin's directory (and of
    /// the inherited entries).
    pub(crate) python_path: Vec<PathBuf>,
    /// The suite's own pytest options, given before the targets.
    pub(crate) options: &'a [String],
    /// The test files, directories or ids to run.
    pub(crate) targets: &'a [String],
    /// Variables set for the run on top of the inherited ones (the suite
    /// tools' fonts and TeX tree, `tools::environment`).
    pub(crate) environment: &'a [(String, OsString)],
    /// Whether pytest's cache plugin runs, leaving `.pytest_cache` in the
    /// site (scanpy's image comparisons save into its `cache` fixture);
    /// the suites leave it out otherwise.
    pub(crate) cache: bool,
}

impl Run<'_> {
    /// Runs pytest with `variables` set or removed besides the run's own
    /// (a display, the GUI toolkits' libraries), and reads what the
    /// plugin recorded.
    pub(crate) fn outcomes_with(&self, variables: &[Variable]) -> Result<Outcomes, String> {
        self.outcomes_with_workers(WORKERS, variables)
    }

    /// Runs pytest as [`Run::outcomes_with`] does, with `workers` workers
    /// (one where a suite's fixtures can't share a process pool).
    pub(crate) fn outcomes_with_workers(
        &self,
        workers: &str,
        variables: &[Variable],
    ) -> Result<Outcomes, String> {
        let root = crate::python::root();
        let results = self.dir.join("results.jsonl");
        let log_path = self.dir.join("pytest.log");
        std::fs::create_dir_all(self.dir)
            .map_err(|err| format!("{}: {err}", self.dir.display()))?;
        let _ = std::fs::remove_file(&results);
        let log = std::fs::File::create(&log_path)
            .map_err(|err| format!("{}: {err}", log_path.display()))?;
        let stderr = log
            .try_clone()
            .map_err(|err| format!("{}: {err}", log_path.display()))?;
        let mut path = self.python_path.clone();
        path.push(root.join("python/mpl_suite"));
        let python_path = crate::python::python_path(path)?;
        let config = self.dir.join("mplconfig");
        forget_font_list(&config)?;
        println!(
            "running {} with {} (log: {})",
            self.suite,
            self.python.display(),
            log_path.display()
        );
        let mut command = Command::new(self.python);
        for (name, value) in variables {
            match value {
                Some(value) => command.env(name, value),
                None => command.env_remove(name),
            };
        }
        let status = command
            .args(["-m", "pytest", "-p", "mpl_suite_report"])
            .args(if self.cache {
                &[][..]
            } else {
                &["-p", "no:cacheprovider"][..]
            })
            .args(["-n", workers, "--timeout", TEST_TIMEOUT])
            .args(["--continue-on-collection-errors", "-q", "-rN"])
            // pytest's temporary directories (`tmp_path`) under the run's
            // own directory, not in the system's temporary directory.
            .arg(crate::python::basetemp(&self.dir.join("pytest-tmp")))
            .args(self.options)
            .args(self.targets)
            .current_dir(self.site)
            .env("PYTHONPATH", python_path)
            .env(LOOK_VARIABLE, "matplotlib")
            .env("MPL_SUITE_RESULTS", &results)
            // The run's own configuration directory, which pltrs reads
            // from `MPLCONFIGDIR` as matplotlib does: some of matplotlib's
            // tests make theirs unwritable, which must not reach the one
            // cargo's tests share (`target/pltrs-config`), and expect the
            // warning to name `MPLCONFIGDIR`.
            .env("MPLCONFIGDIR", &config)
            .env_remove(CONFIG_DIR_VARIABLE)
            .envs(self.environment.iter().map(|(name, value)| (name, value)))
            .stdin(Stdio::null())
            .stdout(log)
            .stderr(stderr)
            .status()
            .map_err(|err| format!("cannot run {}: {err}", self.python.display()))?;
        // 0: all passed, 1: some failed, 5: nothing collected; anything
        // else means pytest itself broke (internal error, bad usage,
        // interrupted).
        if !matches!(status.code(), Some(0 | 1 | 5)) {
            return Err(format!(
                "pytest failed ({status}); see {}",
                log_path.display()
            ));
        }
        let text = std::fs::read_to_string(&results)
            .map_err(|err| format!("{}: {err}", results.display()))?;
        Outcomes::from_results(&text)
    }
}

/// Removes matplotlib's font list cache (`fontlist-v*.json`) from the
/// configuration directory `config`: matplotlib reads it without looking
/// at the font directories again, so a run would keep the fonts of the
/// previous one, with or without the suite tools' fonts.
fn forget_font_list(config: &Path) -> Result<(), String> {
    let Ok(entries) = std::fs::read_dir(config) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("fontlist-") && name.ends_with(".json") {
            std::fs::remove_file(entry.path())
                .map_err(|err| format!("{}: {err}", entry.path().display()))?;
        }
    }
    Ok(())
}
