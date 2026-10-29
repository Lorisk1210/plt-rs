//! Python's `TemporaryDirectory`: a fresh directory, removed with
//! everything in it when dropped. TeX writes its files into one, as do the
//! programs the PostScript backend runs.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// A fresh directory, removed with its contents when dropped.
#[derive(Debug)]
pub struct TemporaryDirectory(PathBuf);

impl TemporaryDirectory {
    /// Makes a fresh directory in `parent`, named after the process, a
    /// counter and the time so that no other one has its name.
    ///
    /// # Errors
    ///
    /// When the directory cannot be made.
    pub fn new(parent: &Path) -> io::Result<Self> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        loop {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.subsec_nanos());
            let name = format!(
                "tmp{}_{}_{nanos}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            );
            let path = parent.join(name);
            match std::fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {}
                Err(err) => return Err(err),
            }
        }
    }

    /// The directory.
    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directories_are_fresh_and_removed() {
        let parent = std::env::temp_dir();
        let first = TemporaryDirectory::new(&parent).unwrap();
        let second = TemporaryDirectory::new(&parent).unwrap();
        assert_ne!(first.path(), second.path());
        std::fs::write(first.path().join("file"), b"x").unwrap();
        let path = first.path().to_path_buf();
        drop(first);
        assert!(!path.exists());
        assert!(TemporaryDirectory::new(&parent.join("plt-rs-no-such-parent")).is_err());
    }
}
