//! pltrs installed into the pltrs venvs of `cargo xtask ecosystem suite`
//! as its users have it.
//!
//! The pltrs runs used to find pltrs on `PYTHONPATH` (`python/alias`
//! beside the sources), which a Python a test starts itself does not
//! inherit when the test gives it an environment of its own: pyfolio's
//! test of its optional import runs `python -c "import pyfolio"` in a
//! subprocess that found no matplotlib. Each venv a run uses gets the two
//! distributions as `pip install .` and `pip install . -C
//! distribution=matplotlib` lay them out, with the corpora's installer
//! (`spec/diff/install.py`): the `pltrs` package with the extension the
//! run built, and the `matplotlib` distribution's aliases with their
//! `.dist-info` (which `importlib.metadata.version("matplotlib")` reads).

use std::path::Path;
use std::process::Command;

use crate::python;

/// Installs the tree's pltrs and its `matplotlib` distribution into the
/// venv of the interpreter `python`, over what an earlier run installed.
pub(super) fn install(python: &Path) -> Result<(), String> {
    let root = python::root();
    python::run_checked(
        Command::new(python)
            .arg("-I")
            .arg("-c")
            .arg(SCRIPT)
            .arg(root.join("spec/diff"))
            .arg(&root)
            .arg(python),
    )
}

/// What [`install()`] runs: the corpora's installer, from the tree.
const SCRIPT: &str = r"
import pathlib, sys
installer, root, python = sys.argv[1:]
sys.path.insert(0, installer)
import install
install.install(pathlib.Path(root), python)
";
