//! The two Python columns: matplotlib in the reference venv, and
//! pltrs (Python) in its own venv running the same scripts with only their
//! imports of matplotlib changed. Both go through the harness
//! `spec/reference/bench/bench.py`.

use std::error::Error;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::cold::ColdCommand;
use super::implementation::Implementation;
use super::workloads::{Output, Workload};

/// Environment every Python process of the benchmark gets: the
/// non-interactive Agg backend (what matplotlib picks without a display),
/// matplotlib's look for pltrs (Python), which otherwise draws in its
/// modern look, so that both columns draw the same figures, and numpy's
/// BLAS kept to one thread, as pltrs draws on one thread (`OpenBLAS`
/// would otherwise start a thread per core when numpy is imported).
pub(super) const PYTHON_ENV: [(&str, &str); 5] = [
    ("MPLBACKEND", "agg"),
    ("PLTRS_LOOK", "matplotlib"),
    ("OPENBLAS_NUM_THREADS", "1"),
    ("OMP_NUM_THREADS", "1"),
    ("MKL_NUM_THREADS", "1"),
];

/// An interpreter with the scripts it runs and its environment.
#[derive(Clone, Debug)]
pub(super) struct PythonRunner {
    /// Which implementation it runs.
    pub(super) implementation: Implementation,
    /// The venv's interpreter.
    pub(super) python: PathBuf,
    /// The directory of the scripts it runs.
    scripts: PathBuf,
    /// Environment on top of [`PYTHON_ENV`].
    env: Vec<(String, String)>,
}

impl PythonRunner {
    /// matplotlib in the reference venv, running `benchmarks/python`, its
    /// configuration directory an empty one of the benchmark's
    /// (`MPLCONFIGDIR`), so that no `matplotlibrc` of the user's changes its
    /// defaults.
    pub(super) fn matplotlib(root: &Path, config: &Path) -> Result<Self, Box<dyn Error>> {
        let python = root.join(".venv/bin/python");
        if !python.exists() {
            return Err(format!(
                "the reference venv {} is missing (see spec/mpl-reference)",
                python.display()
            )
            .into());
        }
        fs::create_dir_all(config)?;
        Ok(Self {
            implementation: Implementation::Matplotlib,
            python,
            scripts: root.join("benchmarks/python"),
            env: vec![("MPLCONFIGDIR".to_owned(), path_text(config)?.to_owned())],
        })
    }

    /// pltrs in its venv, from the source tree (`python/` on the path),
    /// running copies of the scripts written to `copies` with only their
    /// imports changed, its configuration directory an empty one of the
    /// benchmark's (`PLTRSCONFIGDIR`).
    pub(super) fn pltrs(root: &Path, copies: &Path, config: &Path) -> Result<Self, Box<dyn Error>> {
        write_pltrs_copies(&root.join("benchmarks/python"), copies)?;
        fs::create_dir_all(config)?;
        Ok(Self {
            implementation: Implementation::Python,
            python: crate::python::pltrs_python()?,
            scripts: copies.to_owned(),
            env: vec![
                (
                    "PYTHONPATH".to_owned(),
                    path_text(&root.join("python"))?.to_owned(),
                ),
                ("PLTRSCONFIGDIR".to_owned(), path_text(config)?.to_owned()),
            ],
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

    /// The harness serving `script`, the script of `workload`, for timed
    /// runs and saves.
    pub(super) fn serve_command(&self, workload: &Workload, script: &str) -> Command {
        let mut args: Vec<OsString> = vec!["serve".into(), self.script(script).into()];
        match workload.output {
            Output::Save { format, tight } => {
                args.extend(["--format".into(), format.extension().into()]);
                if tight {
                    args.extend(["--option".into(), "bbox_inches=tight".into()]);
                }
            }
            Output::LayoutOnly => args.push("--layout".into()),
            Output::Frames => args.extend(["--format".into(), "html".into()]),
        }
        self.harness(&args)
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
            Output::Frames => argv.push(out.into()),
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
            label: self.implementation.key().to_owned(),
            argv,
            env: self.env.clone(),
        }
    }

    /// What the venv runs on (see `bench.py versions`): `(name, version)`
    /// pairs.
    pub(super) fn versions(&self) -> Result<Vec<(String, String)>, Box<dyn Error>> {
        let output = self.harness(&["versions".into()]).output()?;
        if !output.status.success() {
            return Err(format!(
                "bench.py versions failed ({}): {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
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

/// Copies the scripts of `source` into `target` for pltrs: in each
/// script, the imports of matplotlib import the same from pltrs, and
/// nothing else changes. The shared modules (`_*.py`) are copied as they
/// are.
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

/// The import every script makes.
const PYPLOT_IMPORT: &str = "import matplotlib.pyplot as plt";

/// The text of a script with its imports of matplotlib changed to import
/// the same from pltrs: `import matplotlib.pyplot as plt` becomes `import
/// pltrs.pyplot as plt`, `from matplotlib.animation import FuncAnimation`
/// becomes `from pltrs.animation import FuncAnimation`. Refused unless
/// the script imports pyplot as `plt`.
fn pltrs_copy(script: &str) -> Result<String, String> {
    if !script.lines().any(|line| line == PYPLOT_IMPORT) {
        return Err(format!("a script imports pyplot as `{PYPLOT_IMPORT}`"));
    }
    let mut copy = String::with_capacity(script.len());
    for line in script.lines() {
        let changed = ["import matplotlib", "from matplotlib"]
            .iter()
            .find_map(|start| {
                let rest = line.strip_prefix(start)?;
                rest.starts_with(['.', ' ']).then(|| {
                    let keyword = &start[..start.len() - "matplotlib".len()];
                    format!("{keyword}pltrs{rest}")
                })
            });
        copy.push_str(changed.as_deref().unwrap_or(line));
        copy.push('\n');
    }
    Ok(copy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_imports_change() {
        let script = "\"\"\"Doc.\"\"\"\n\nimport numpy as np\nimport matplotlib.pyplot as plt\n\
                      from matplotlib.animation import FuncAnimation\n\nx = 1\n";
        let copy = pltrs_copy(script).expect("imports of pyplot and animation");
        assert_eq!(
            copy,
            script
                .replace("matplotlib.pyplot", "pltrs.pyplot")
                .replace("matplotlib.animation", "pltrs.animation")
        );
    }

    #[test]
    fn scripts_must_import_pyplot() {
        assert!(pltrs_copy("import numpy as np\n").is_err());
        let similar =
            "import matplotlib.pyplot as plt\nimport matplotlibx\n\"\"\"matplotlib\"\"\"\n";
        assert_eq!(
            pltrs_copy(similar).expect("pyplot imported"),
            "import pltrs.pyplot as plt\nimport matplotlibx\n\"\"\"matplotlib\"\"\"\n"
        );
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
