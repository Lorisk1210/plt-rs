//! The two Python columns: matplotlib in the reference venv, and
//! pltrs (Python) in its own venv running the same scripts with only their import of
//! `matplotlib.pyplot` changed. Both go through the harness
//! `spec/reference/bench/bench.py`.

use std::error::Error;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::cold::ColdCommand;
use super::stats::{Samples, Sampling};
use super::workloads::{Output, Workload};

/// The import every script makes, the line pltrs' copies change.
const MATPLOTLIB_IMPORT: &str = "import matplotlib.pyplot as plt";

/// What pltrs' copies import instead.
const PLTRS_IMPORT: &str = "import pltrs.pyplot as plt";

/// Environment every Python process of the benchmark gets: the
/// non-interactive Agg backend (what matplotlib picks without a display),
/// and numpy's BLAS kept to one thread, as pltrs draws on one thread
/// (`OpenBLAS` would otherwise start a thread per core when numpy is
/// imported).
pub(super) const PYTHON_ENV: [(&str, &str); 4] = [
    ("MPLBACKEND", "agg"),
    ("OPENBLAS_NUM_THREADS", "1"),
    ("OMP_NUM_THREADS", "1"),
    ("MKL_NUM_THREADS", "1"),
];

/// Which Python implementation of matplotlib's API runs the scripts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Flavor {
    /// matplotlib itself.
    Matplotlib,
    /// pltrs (Python).
    Pltrs,
}

impl Flavor {
    /// The column's name.
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::Matplotlib => "matplotlib",
            Self::Pltrs => "pltrs-python",
        }
    }
}

/// What a script made of a request.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Outcome<T> {
    /// It ran.
    Done(T),
    /// It raised this exception: pltrs (Python) lacks something the script
    /// uses.
    Unsupported(String),
}

/// An interpreter with the scripts it runs and its environment.
#[derive(Clone, Debug)]
pub(super) struct PythonRunner {
    /// Which implementation it runs.
    pub(super) flavor: Flavor,
    /// The venv's interpreter.
    pub(super) python: PathBuf,
    /// The directory of the scripts it runs.
    scripts: PathBuf,
    /// Environment on top of [`PYTHON_ENV`].
    env: Vec<(String, String)>,
}

impl PythonRunner {
    /// matplotlib in the reference venv, running `benchmarks/python`.
    pub(super) fn matplotlib(root: &Path) -> Result<Self, Box<dyn Error>> {
        let python = root.join(".venv/bin/python");
        if !python.exists() {
            return Err(format!(
                "the reference venv {} is missing (see spec/mpl-reference)",
                python.display()
            )
            .into());
        }
        Ok(Self {
            flavor: Flavor::Matplotlib,
            python,
            scripts: root.join("benchmarks/python"),
            env: Vec::new(),
        })
    }

    /// pltrs in its venv, from the source tree (`python/` on the path),
    /// running copies of the scripts written to `target/bench/pltrs` with
    /// only their import changed.
    pub(super) fn pltrs(root: &Path) -> Result<Self, Box<dyn Error>> {
        let scripts = root.join("target/bench/pltrs");
        write_pltrs_copies(&root.join("benchmarks/python"), &scripts)?;
        Ok(Self {
            flavor: Flavor::Pltrs,
            python: crate::python::pltrs_python()?,
            scripts,
            env: vec![(
                "PYTHONPATH".to_owned(),
                path_text(&root.join("python"))?.to_owned(),
            )],
        })
    }

    /// The script `name` as this runner runs it.
    fn script(&self, name: &str) -> PathBuf {
        self.scripts.join(format!("{name}.py"))
    }

    /// A command running the harness with `args`.
    fn harness(&self, args: &[OsString]) -> Command {
        let mut command = Command::new(&self.python);
        command
            .arg(crate::python::root().join("spec/reference/bench/bench.py"))
            .args(args)
            .envs(PYTHON_ENV)
            .envs(self.env.iter().map(|(k, v)| (k, v)));
        command
    }

    /// Times the script `script` of `workload` in one process: its data
    /// made, a run to warm up, then the runs `sampling` asks for.
    pub(super) fn warm(
        &self,
        workload: &Workload,
        script: &str,
        sampling: Sampling,
    ) -> Result<Outcome<Samples>, Box<dyn Error>> {
        let mut args: Vec<OsString> = vec!["warm".into(), self.script(script).into()];
        match workload.output {
            Output::Save { format, tight } => {
                args.extend(["--format".into(), format.extension().into()]);
                if tight {
                    args.extend(["--option".into(), "bbox_inches=tight".into()]);
                }
            }
            Output::LayoutOnly => args.push("--layout".into()),
        }
        args.extend([
            "--min-runs".into(),
            sampling.min_runs.to_string().into(),
            "--min-seconds".into(),
            sampling.min_seconds.to_string().into(),
            "--max-runs".into(),
            sampling.max_runs.to_string().into(),
        ]);
        let reply = self.reply(&args)?;
        Ok(match reply {
            Outcome::Done(line) => {
                let times = line
                    .strip_prefix("times\t")
                    .ok_or_else(|| format!("unexpected reply from bench.py: {line}"))?
                    .split('\t')
                    .map(str::parse)
                    .collect::<Result<Vec<f64>, _>>()?;
                Outcome::Done(Samples::new(times).ok_or("bench.py timed no runs")?)
            }
            Outcome::Unsupported(why) => Outcome::Unsupported(why),
        })
    }

    /// Saves the figure of `script` as the PNG file `out`, cropped as
    /// `workload` crops it.
    pub(super) fn png(
        &self,
        workload: &Workload,
        script: &str,
        out: &Path,
    ) -> Result<Outcome<()>, Box<dyn Error>> {
        let mut args: Vec<OsString> = vec!["png".into(), self.script(script).into(), out.into()];
        if matches!(workload.png_output(), Output::Save { tight: true, .. }) {
            args.extend(["--option".into(), "bbox_inches=tight".into()]);
        }
        Ok(match self.reply(&args)? {
            Outcome::Done(_) => Outcome::Done(()),
            Outcome::Unsupported(why) => Outcome::Unsupported(why),
        })
    }

    /// Runs the harness with `args` and reads its one-line reply.
    fn reply(&self, args: &[OsString]) -> Result<Outcome<String>, Box<dyn Error>> {
        let output = self.harness(args).output()?;
        if !output.status.success() {
            return Err(format!(
                "bench.py ({}) failed ({}): {}",
                self.flavor.name(),
                output.status,
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        let stdout = String::from_utf8(output.stdout)?;
        let line = stdout.trim_end().to_owned();
        Ok(match line.strip_prefix("unsupported\t") {
            Some(why) => Outcome::Unsupported(why.to_owned()),
            None => Outcome::Done(line),
        })
    }

    /// The fresh process a user runs: the script `script` saving the
    /// figure of `workload` to `out`.
    pub(super) fn script_command(
        &self,
        workload: &Workload,
        script: &str,
        out: &Path,
    ) -> ColdCommand {
        let mut argv: Vec<OsString> = vec![self.python.clone().into(), self.script(script).into()];
        match workload.output {
            Output::Save { tight, .. } => {
                argv.push(out.into());
                if tight {
                    argv.push("bbox_inches=tight".into());
                }
            }
            Output::LayoutOnly => argv.push("--layout".into()),
        }
        self.cold_command(argv)
    }

    /// A fresh interpreter running `code` (`python -c`).
    pub(super) fn code_command(&self, code: &str) -> ColdCommand {
        self.cold_command(vec![self.python.clone().into(), "-c".into(), code.into()])
    }

    /// `argv` run with this runner's environment.
    fn cold_command(&self, argv: Vec<OsString>) -> ColdCommand {
        ColdCommand {
            label: self.flavor.name().to_owned(),
            argv,
            env: self.env.clone(),
        }
    }

    /// The versions of the venv's Python and numpy, and of matplotlib
    /// where installed, and today's date: `(name, version)` pairs.
    pub(super) fn versions(&self) -> Result<Vec<(String, String)>, Box<dyn Error>> {
        let output = self.harness(&["versions".into()]).output()?;
        if !output.status.success() {
            return Err(format!("bench.py versions failed ({})", output.status).into());
        }
        Ok(String::from_utf8(output.stdout)?
            .lines()
            .filter_map(|line| line.split_once('\t'))
            .map(|(name, version)| (name.to_owned(), version.to_owned()))
            .collect())
    }
}

/// `path` as UTF-8 text.
pub(super) fn path_text(path: &Path) -> Result<&str, Box<dyn Error>> {
    path.to_str()
        .ok_or_else(|| format!("{} is not UTF-8", path.display()).into())
}

/// Copies the scripts of `source` into `target` for pltrs: each script's
/// one import of `matplotlib.pyplot` becomes an import of `pltrs.pyplot`,
/// and nothing else changes. The shared modules (`_*.py`) are copied as
/// they are.
fn write_pltrs_copies(source: &Path, target: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let path = entry?.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if path.extension().is_none_or(|e| e != "py") {
            continue;
        }
        let text = fs::read_to_string(&path)?;
        let copy = if name.starts_with('_') {
            text
        } else {
            pltrs_copy(&text).map_err(|why| format!("{}: {why}", path.display()))?
        };
        fs::write(target.join(name), copy)?;
    }
    Ok(())
}

/// The text of a script with its import of `matplotlib.pyplot` changed to
/// pltrs', refused unless that is its only import from matplotlib.
fn pltrs_copy(script: &str) -> Result<String, String> {
    let imports = script
        .lines()
        .filter(|line| line.starts_with("import matplotlib") || line.starts_with("from matplotlib"))
        .collect::<Vec<_>>();
    if imports != [MATPLOTLIB_IMPORT] {
        return Err(format!(
            "a script imports matplotlib once, as `{MATPLOTLIB_IMPORT}`, not {imports:?}"
        ));
    }
    Ok(script
        .lines()
        .map(|line| {
            if line == MATPLOTLIB_IMPORT {
                PLTRS_IMPORT
            } else {
                line
            }
        })
        .flat_map(|line| [line, "\n"])
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_import_changes() {
        let script =
            "\"\"\"Doc.\"\"\"\n\nimport numpy as np\nimport matplotlib.pyplot as plt\n\nx = 1\n";
        let copy = pltrs_copy(script).expect("one import");
        assert_eq!(copy, script.replace("matplotlib.pyplot", "pltrs.pyplot"));
    }

    #[test]
    fn other_matplotlib_imports_are_refused() {
        let script = "import matplotlib.pyplot as plt\nfrom matplotlib import colors\n";
        assert!(pltrs_copy(script).is_err());
        assert!(pltrs_copy("import numpy as np\n").is_err());
    }

    #[test]
    fn every_script_has_a_pltrs_copy() {
        let scripts = crate::python::root().join("benchmarks/python");
        for entry in fs::read_dir(&scripts).expect("benchmarks/python") {
            let path = entry.expect("an entry").path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if path.extension().is_some_and(|e| e == "py") && !name.starts_with('_') {
                let text = fs::read_to_string(&path).expect("a script");
                assert!(pltrs_copy(&text).is_ok(), "{name}");
            }
        }
    }
}
