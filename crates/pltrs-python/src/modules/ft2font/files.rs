//! The bytes of the font files `FT2Font`s open, read once and shared.
//!
//! `FreeType` reads a font file as it needs it, so that matplotlib makes
//! an `FT2Font` of a large file without reading it. pltrs-font parses
//! faces from the file's bytes, which are read when the first face of the
//! file opens and shared by every face of it open meanwhile: making an
//! `FT2Font` of a file already open costs a look at the file's size and
//! modification time, which tell when it changed and must be read again.
//! The bytes go when the last face of them goes.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError, Weak};
use std::time::SystemTime;

/// A font file as it was read: its path, size and modification time.
type FileKey = (PathBuf, u64, Option<SystemTime>);

/// The bytes of the files read, while a face of them is open.
fn open_files() -> &'static Mutex<HashMap<FileKey, Weak<[u8]>>> {
    static FILES: OnceLock<Mutex<HashMap<FileKey, Weak<[u8]>>>> = OnceLock::new();
    FILES.get_or_init(Mutex::default)
}

/// The bytes of the font file at `path`: those a face of it open holds,
/// unless the file changed since, else read now.
///
/// # Errors
///
/// When the file cannot be read.
pub(super) fn read_shared(path: &Path) -> io::Result<Arc<[u8]>> {
    let metadata = std::fs::metadata(path)?;
    let key = (path.to_path_buf(), metadata.len(), metadata.modified().ok());
    let files = open_files();
    if let Some(data) = files
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&key)
        .and_then(Weak::upgrade)
    {
        return Ok(data);
    }
    let data: Arc<[u8]> = std::fs::read(path)?.into();
    let mut files = files.lock().unwrap_or_else(PoisonError::into_inner);
    files.retain(|_, data| data.strong_count() > 0);
    files.insert(key, Arc::downgrade(&data));
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faces_of_a_file_share_its_bytes_until_it_changes() {
        let dir = std::env::temp_dir().join(format!("pltrs-ft2font-files-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("font.bin");
        std::fs::write(&path, b"one").unwrap();
        let first = read_shared(&path).unwrap();
        let second = read_shared(&path).unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        std::fs::write(&path, b"three").unwrap();
        let changed = read_shared(&path).unwrap();
        assert_eq!(&*changed, b"three");
        drop((first, second, changed));
        assert!(read_shared(&dir.join("missing.bin")).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }
}
