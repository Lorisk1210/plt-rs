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

/// The pytest workers of a run, unless [`WORKERS_VARIABLE`] says.
const WORKERS: &str = "8";

/// The variable giving another number of pytest workers, for a machine
/// shared with other work.
const WORKERS_VARIABLE: &str = "PLTRS_SUITE_WORKERS";

/// The length of the targets beyond which they go to pytest in a file of
/// arguments (`@targets.txt`, one a line) rather than on the command line,
/// which Windows holds to 32,767 characters (the failing tests of a run,
/// which runs making baselines take again, need more).
const TARGETS_ON_THE_COMMAND_LINE: usize = 16_384;

/// The seconds a single test may take before pytest-timeout fails it.
const TEST_TIMEOUT: &str = "600";

/// The variable choosing pltrs' look, set to `matplotlib` for every run:
/// pltrs (Python) draws in its modern look by default, and the suites
/// expect matplotlib's defaults (matplotlib itself ignores it).
const LOOK_VARIABLE: &str = "PLTRS_LOOK";

/// The variables holding the numeric libraries of every process a run
/// starts to one thread each: BLAS's (`OpenBLAS`, MKL, Accelerate) and
/// `OpenMP`'s pools, numexpr's and numba's. A run has a pytest worker per
/// core or so, and each starting a pool of a thread per core besides
/// (yellowbrick's workers took 100 to 240% of a core each) overloads the
/// machine, and the tests depending on timing then fail.
pub(crate) const ONE_THREAD: [(&str, &str); 6] = [
    ("OMP_NUM_THREADS", "1"),
    ("OPENBLAS_NUM_THREADS", "1"),
    ("MKL_NUM_THREADS", "1"),
    ("VECLIB_MAXIMUM_THREADS", "1"),
    ("NUMEXPR_NUM_THREADS", "1"),
    ("NUMBA_NUM_THREADS", "1"),
];

/// pltrs' own variable naming its configuration directory, which cargo
/// sets for what it runs (`.cargo/config.toml`) and a suite must not
/// pass on: pltrs (Python) then reads matplotlib's `MPLCONFIGDIR`.
pub(crate) const CONFIG_DIR_VARIABLE: &str = "PLTRSCONFIGDIR";

/// A pytest run: the tests `targets` of the site `site`, with `python`.
///
/// Its files go to `dir`: `results.jsonl` (the plugin's), `pytest.log`
/// (pytest's output), `pytest-tmp` (`tmp_path`) and the configuration
/// and cache directories ([`own_directories`]).
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
    /// (a display, the GUI toolkits' libraries, after [`ONE_THREAD`]), and
    /// reads what the plugin recorded.
    pub(crate) fn outcomes_with(&self, variables: &[Variable]) -> Result<Outcomes, String> {
        let workers = std::env::var(WORKERS_VARIABLE).ok();
        self.outcomes_with_workers(workers.as_deref().unwrap_or(WORKERS), variables)
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
        let own = own_directories(self.dir)?;
        println!(
            "running {} with {} (log: {})",
            self.suite,
            self.python.display(),
            log_path.display()
        );
        let mut command = Command::new(self.python);
        command.envs(ONE_THREAD);
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
            .args(self.target_args()?)
            .current_dir(self.site)
            .env("PYTHONPATH", python_path)
            .env(LOOK_VARIABLE, "matplotlib")
            .env("MPL_SUITE_RESULTS", &results)
            // The run's own configuration and cache directories, pltrs
            // reading `MPLCONFIGDIR` as matplotlib does: some of
            // matplotlib's tests make theirs unwritable, which must not
            // reach the one cargo's tests share (`target/pltrs-config`),
            // and expect the warning to name `MPLCONFIGDIR`.
            .envs(own.iter().map(|(name, path)| (name, path)))
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

    /// The arguments naming the targets: themselves, or when they are too
    /// many for the command line ([`TARGETS_ON_THE_COMMAND_LINE`]) the
    /// file of arguments in the run's directory that holds them.
    fn target_args(&self) -> Result<Vec<OsString>, String> {
        let length: usize = self.targets.iter().map(|target| target.len() + 1).sum();
        if length <= TARGETS_ON_THE_COMMAND_LINE {
            return Ok(self.targets.iter().map(OsString::from).collect());
        }
        let file = self.dir.join("targets.txt");
        let mut listed = self.targets.join("\n");
        listed.push('\n');
        std::fs::write(&file, listed).map_err(|err| format!("{}: {err}", file.display()))?;
        let mut argument = OsString::from("@");
        argument.push(&file);
        Ok(vec![argument])
    }
}

/// The configuration and cache directories a run in `dir` has to itself,
/// made empty: matplotlib's (`MPLCONFIGDIR`, which pltrs reads when
/// `PLTRSCONFIGDIR` is unset) and the XDG cache and configuration homes a
/// library writes to on its own, or a Python a test starts without
/// `MPLCONFIGDIR` (ultraplot's font cache). Nothing a run writes there
/// reaches the user's `~/.cache` or `~/.config`, or a later run: a font
/// list cached by an earlier run (with or without the suite tools'
/// fonts) would be read without looking at the font directories again.
pub(crate) fn own_directories(dir: &Path) -> Result<[(&'static str, PathBuf); 3], String> {
    let directories = [
        ("MPLCONFIGDIR", dir.join("mplconfig")),
        ("XDG_CACHE_HOME", dir.join("xdg-cache")),
        ("XDG_CONFIG_HOME", dir.join("xdg-config")),
    ];
    for (_, path) in &directories {
        super::prepare::remove(path)?;
        std::fs::create_dir_all(path).map_err(|err| format!("{}: {err}", path.display()))?;
    }
    Ok(directories)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn many_targets_go_to_pytest_in_a_file() {
        let dir = std::env::temp_dir().join(format!("xtask-targets-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let run = |targets: &[String]| {
            Run {
                suite: "a suite",
                python: Path::new("python"),
                site: &dir,
                dir: &dir,
                python_path: Vec::new(),
                options: &[],
                targets,
                environment: &[],
                cache: false,
            }
            .target_args()
            .unwrap()
        };
        let few = vec!["test_a.py::test_one".to_owned()];
        assert_eq!(run(&few), [OsString::from("test_a.py::test_one")]);
        let many: Vec<String> = (0..2000).map(|i| format!("test_a.py::test_{i}")).collect();
        let mut file = OsString::from("@");
        file.push(dir.join("targets.txt"));
        assert_eq!(run(&many), [file]);
        let listed = std::fs::read_to_string(dir.join("targets.txt")).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(listed.lines().collect::<Vec<_>>(), many);
    }
}
