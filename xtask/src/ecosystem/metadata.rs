//! The core metadata of pltrs's `matplotlib` distribution for the pltrs
//! runs of `cargo xtask ecosystem suite`.
//!
//! The pltrs runs import matplotlib from `python/alias` on `PYTHONPATH`,
//! which holds the packages of the `matplotlib` distribution
//! (`pip install . -C distribution=matplotlib`) but not the metadata an
//! installation of it writes. A library asking for matplotlib's metadata
//! (scanpy's image comparisons read `importlib.metadata.version("matplotlib")`)
//! finds it in `target/ecosystem/metadata`: a `matplotlib-<version>.dist-info`
//! holding the `METADATA` the tree's build backend writes into the wheel
//! (`python/backend`), which the runs put on `PYTHONPATH` beside the alias.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::python;

/// The directory the pltrs runs put on `PYTHONPATH` for the metadata.
pub(super) fn dir() -> PathBuf {
    super::ecosystem_dir().join("metadata")
}

/// Writes the metadata (see the module) into [`dir`] with `python`, the
/// interpreter of a venv.
pub(super) fn write(python: &Path) -> Result<(), String> {
    let root = python::root();
    python::run_checked(
        Command::new(python)
            .arg("-I")
            .arg("-c")
            .arg(SCRIPT)
            .arg(root.join("python/backend"))
            .arg(&root)
            .arg(dir()),
    )
}

/// What [`write()`] runs: the backend's `Project` of the `matplotlib`
/// distribution writes its `METADATA` (and pip's `INSTALLER`) into a new
/// `<name>-<version>.dist-info` in the directory given, replacing what
/// was there.
const SCRIPT: &str = r#"
import pathlib, shutil, sys
backend, root, dest = (pathlib.Path(arg) for arg in sys.argv[1:])
sys.path.insert(0, str(backend))
from pltrs_backend._project import Project
project = Project(root, "matplotlib")
shutil.rmtree(dest, ignore_errors=True)
info = dest / f"matplotlib-{project.version}.dist-info"
info.mkdir(parents=True)
(info / "METADATA").write_text(project.metadata(), encoding="utf-8")
(info / "INSTALLER").write_text("pltrs-ecosystem\n", encoding="utf-8")
"#;
