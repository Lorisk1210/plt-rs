//! Scratch directories for tests that write files, shared by the unit
//! tests and the fuzz tests: a fresh directory in the system's temporary
//! directory, removed with everything in it when the test ends (also
//! when it fails), so that tests leave nothing behind.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A fresh directory in the temporary directory, removed when dropped.
#[derive(Debug)]
pub(crate) struct Scratch(PathBuf);

impl Scratch {
    /// Creates the directory `pltrs-{label}-{process}-{n}`, `n` counting
    /// the scratch directories of the process.
    pub(crate) fn new(label: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "pltrs-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a scratch directory");
        Self(dir)
    }

    /// The path of the file `name` in the directory.
    pub(crate) fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        // A leftover scratch directory is harmless.
        let _ = fs::remove_dir_all(&self.0);
    }
}
