//! Scratch directories for files that probes and renderings write to disk
//! and read back.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

/// A fresh directory in the system's temporary directory, removed with
/// its contents when dropped.
#[derive(Debug)]
pub(crate) struct ScratchDir(PathBuf);

impl ScratchDir {
    /// Creates the directory `pltrs-spec-{label}-{process}-{n}`, `n` counting
    /// the scratch directories of the process.
    pub(crate) fn new(label: &str) -> io::Result<Self> {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "pltrs-spec-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir)?;
        Ok(Self(dir))
    }

    /// The directory.
    pub(crate) fn dir(&self) -> &Path {
        &self.0
    }

    /// The path of the file `name` in the directory.
    pub(crate) fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    /// Writes `contents` to the file `name` and returns its path.
    pub(crate) fn write(&self, name: &str, contents: impl AsRef<[u8]>) -> io::Result<PathBuf> {
        let path = self.path(name);
        std::fs::write(&path, contents)?;
        Ok(path)
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        // A leftover scratch directory is harmless.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directories_are_distinct_and_removed() {
        let first = ScratchDir::new("test").expect("a scratch directory");
        let second = ScratchDir::new("test").expect("a scratch directory");
        assert_ne!(first.dir(), second.dir());
        let file = first.write("a.txt", "text").expect("a written file");
        assert_eq!(std::fs::read_to_string(&file).expect("readable"), "text");
        let dir = first.dir().to_owned();
        drop(first);
        assert!(!dir.exists());
    }
}
