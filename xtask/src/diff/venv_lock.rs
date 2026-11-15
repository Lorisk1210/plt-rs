//! One `cargo xtask diff --corpus` at a time on the corpora's venvs.
//!
//! A run brings the venvs up to their pinned lists and then the driver
//! installs pltrs into the pltrs venv (`spec/diff/install.py`) and runs
//! programs from it for an hour: a second run's venv step or install under
//! those programs would change the files they import. The run holds an
//! exclusive lock on the venvs from its venv step to its end; a second one
//! says who holds it, then waits.

use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::Write;
use std::path::Path;

/// The lock file, beside the venvs.
const LOCK: &str = "corpus-run.lock";

/// Holds the corpora's venvs in `venvs` until the returned file is
/// dropped, waiting for a run that holds them already.
pub(crate) fn hold(venvs: &Path) -> Result<File, String> {
    fs::create_dir_all(venvs).map_err(|err| format!("{}: {err}", venvs.display()))?;
    let path = venvs.join(LOCK);
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&path)
        .map_err(|err| format!("{}: {err}", path.display()))?;
    match file.try_lock() {
        Ok(()) => {}
        Err(TryLockError::WouldBlock) => {
            let holder = fs::read_to_string(&path).unwrap_or_default();
            eprintln!(
                "cargo xtask diff: the corpora's venvs are held by {} ({}); waiting",
                holder.trim(),
                path.display()
            );
            file.lock()
                .map_err(|err| format!("{}: {err}", path.display()))?;
        }
        Err(TryLockError::Error(err)) => return Err(format!("{}: {err}", path.display())),
    }
    file.set_len(0)
        .and_then(|()| writeln!(file, "process {}", std::process::id()))
        .map_err(|err| format!("{}: {err}", path.display()))?;
    Ok(file)
}
