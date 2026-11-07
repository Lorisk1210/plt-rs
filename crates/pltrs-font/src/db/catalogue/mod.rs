//! Catalogues of font files kept between processes, as matplotlib keeps
//! its font list (`fontlist-v390.json` in its cache directory): what the
//! faces of each file are matched by, so that a process starting up reads
//! one small file instead of every font installed.
//!
//! A catalogue is a file of pltrs' own format: an archive
//! ([`pltrs_core::archive`], which heads it with pltrs' version) of the
//! format's version and the font files in the order they are added, each
//! with its length and modification time when it was read and its
//! description (that of each of its faces, or of an Adobe Font Metrics
//! file), followed by a checksum of the archive. When fonts are added with a catalogue
//! ([`refresh`]):
//!
//! * a file that is new, or whose length or modification time changed,
//!   is read again, and so is one that could not be read before; the
//!   others are not opened;
//! * a catalogue that is missing, unreadable, corrupted (its checksum
//!   disagrees), truncated, too large, or of another format or pltrs
//!   version is rebuilt: every file is read;
//! * a catalogue that changed is written to a new file beside it, renamed
//!   over it, so that a process reading it meanwhile reads the old one or
//!   the new one whole. When it cannot be written (a read-only cache
//!   directory), the fonts are used all the same, and the
//!   [`CatalogueReport`] says why.
//!
//! Files whose path is not Unicode are read at every start, as the format
//! keeps paths as text.

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::UNIX_EPOCH;

use pltrs_core::archive::{self, Archive, NoProgram};

/// The version of the format, raised whenever what a catalogue holds
/// changes, or how faces are described: a catalogue of another version is
/// rebuilt.
const FORMAT: u32 = 2;

/// The largest catalogue read, far above any system's (a few hundred
/// kilobytes for a thousand fonts): a larger file is no catalogue, and
/// reading it could exhaust memory.
const MAX_SIZE: u64 = 16 << 20;

/// The name a catalogue's file has in a cache directory, which changes
/// with the format's version (as matplotlib's `fontlist-v390.json` does).
///
/// ```
/// assert_eq!(pltrs_font::catalogue_file_name(), "fontlist-v2.pltrs");
/// ```
pub fn catalogue_file_name() -> String {
    format!("fontlist-v{FORMAT}.pltrs")
}

/// What became of a catalogue when fonts were added with it.
#[derive(Debug)]
pub struct CatalogueReport {
    /// The catalogue's file.
    pub file: PathBuf,
    /// How much of the catalogue held.
    pub state: CatalogueState,
    /// Why the catalogue, which changed, could not be written; the fonts
    /// were added all the same.
    pub save_error: Option<io::Error>,
}

/// How much of a catalogue held.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CatalogueState {
    /// No font file changed: the catalogue was used as it was.
    Current,
    /// Font files were added, changed or removed since the catalogue was
    /// written: `read` files were read, the rest taken from it.
    Updated {
        /// The files read.
        read: usize,
    },
    /// The catalogue could not be used, for `reason` (missing, corrupt,
    /// of another version, ...): every font file was read.
    Rebuilt {
        /// Why the catalogue could not be used.
        reason: String,
    },
}

/// What tells whether a file changed since it was read: its length and
/// modification time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Stamp {
    len: u64,
    seconds: u64,
    nanos: u32,
}

pltrs_core::archive_struct!(Stamp {
    len,
    seconds,
    nanos
});

impl Stamp {
    /// The stamp of the file at `path` now; `None` when its length or a
    /// modification time after 1970 cannot be had.
    fn of(path: &Path) -> Option<Self> {
        let metadata = fs::metadata(path).ok()?;
        let since = metadata.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
        Some(Self {
            len: metadata.len(),
            seconds: since.as_secs(),
            nanos: since.subsec_nanos(),
        })
    }
}

/// A font file as catalogued, described by a `D`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CataloguedFile<D> {
    /// Where the file is.
    pub(crate) path: PathBuf,
    /// Its stamp when it was read; `None` when it had none (it is then
    /// read at every start).
    stamp: Option<Stamp>,
    /// Its description: `None` when it could not be read (it is then read
    /// at the next start).
    pub(crate) description: Option<D>,
}

pltrs_core::archive_struct!(CataloguedFile<D> {
    path,
    stamp,
    description
});

impl<D> CataloguedFile<D> {
    /// Whether the catalogue's file can keep this file: its path is text.
    fn is_kept(&self) -> bool {
        self.path.to_str().is_some()
    }
}

/// A catalogue as its file holds it.
#[derive(Debug)]
struct Kept<D> {
    format: u32,
    files: Vec<CataloguedFile<D>>,
}

pltrs_core::archive_struct!(Kept<D> { format, files });

/// The catalogue of the font files `paths`, in their order, kept in the
/// file `catalogue` (see the module documentation): what it holds of the
/// files unchanged since it was written, and what `describe` reads of the
/// others (an error for one that cannot be read).
pub(crate) fn refresh<D: Archive + Clone + PartialEq>(
    catalogue: &Path,
    paths: &[PathBuf],
    describe: impl Fn(&Path) -> io::Result<D>,
) -> (Vec<CataloguedFile<D>>, CatalogueReport) {
    let kept = match read(catalogue) {
        Ok(files) if is_current(&files, paths) => {
            let report = CatalogueReport {
                file: catalogue.to_path_buf(),
                state: CatalogueState::Current,
                save_error: None,
            };
            return (files, report);
        }
        kept => kept,
    };
    let known: HashMap<&Path, &CataloguedFile<D>> = kept
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|file| (file.path.as_path(), file))
        .collect();
    let mut read_count = 0;
    let files: Vec<CataloguedFile<D>> = paths
        .iter()
        .map(|path| {
            let stamp = Stamp::of(path);
            let unchanged = known.get(path.as_path()).filter(|known| {
                stamp.is_some() && known.stamp == stamp && known.description.is_some()
            });
            if let Some(known) = unchanged {
                return (*known).clone();
            }
            read_count += 1;
            CataloguedFile {
                path: path.clone(),
                stamp,
                description: describe(path).ok(),
            }
        })
        .collect();
    let kept_now: Vec<CataloguedFile<D>> = files
        .iter()
        .filter(|file| file.is_kept())
        .cloned()
        .collect();
    let state = match kept {
        Err(reason) => CatalogueState::Rebuilt { reason },
        Ok(before) if before == kept_now => CatalogueState::Current,
        Ok(_) => CatalogueState::Updated { read: read_count },
    };
    let save_error = match state {
        CatalogueState::Current => None,
        _ => write(catalogue, kept_now).err(),
    };
    let report = CatalogueReport {
        file: catalogue.to_path_buf(),
        state,
        save_error,
    };
    (files, report)
}

/// Whether the catalogue of `files` holds the files `paths` as they are:
/// the same files in the same order, each read and unchanged since.
fn is_current<D>(files: &[CataloguedFile<D>], paths: &[PathBuf]) -> bool {
    files.len() == paths.len()
        && files.iter().zip(paths).all(|(file, path)| {
            file.path == *path
                && file.description.is_some()
                && file.stamp.is_some()
                && file.stamp == Stamp::of(path)
        })
}

/// The files of the catalogue in the file `catalogue`, or why it cannot
/// be used.
fn read<D: Archive>(catalogue: &Path) -> Result<Vec<CataloguedFile<D>>, String> {
    let size = fs::metadata(catalogue)
        .map_err(|err| err.to_string())?
        .len();
    if size > MAX_SIZE {
        return Err(format!("{size} bytes, too large for a font catalogue"));
    }
    let bytes = fs::read(catalogue).map_err(|err| err.to_string())?;
    let (archived, sum) = bytes
        .split_last_chunk::<8>()
        .ok_or("too short for a font catalogue")?;
    if u64::from_le_bytes(*sum) != checksum(archived) {
        return Err("a corrupted font catalogue".to_owned());
    }
    let kept: Kept<D> =
        archive::from_bytes(archived, &[], &NoProgram).map_err(|err| err.to_string())?;
    if kept.format != FORMAT {
        return Err(format!(
            "a catalogue of format {} (this is {FORMAT})",
            kept.format
        ));
    }
    Ok(kept.files)
}

/// Writes the catalogue of `files` to the file `catalogue`: to a new file
/// in the same directory, then renamed over it.
fn write<D: Archive>(catalogue: &Path, files: Vec<CataloguedFile<D>>) -> io::Result<()> {
    let kept = Kept {
        format: FORMAT,
        files,
    };
    let (mut bytes, _) = archive::to_bytes(&kept, &NoProgram)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?;
    let sum = checksum(&bytes);
    bytes.extend(sum.to_le_bytes());
    let temporary = temporary_path(catalogue);
    let written = fs::write(&temporary, bytes).and_then(|()| fs::rename(&temporary, catalogue));
    if written.is_err() {
        // Nothing is left behind; the file may not even have been made.
        let _ = fs::remove_file(&temporary);
    }
    written
}

/// The checksum ending a catalogue's file, which tells its bytes from
/// corrupted ones (a corrupted catalogue could otherwise still read, and
/// match fonts by the wrong descriptions): the 64-bit FNV-1a hash of the
/// archive before it.
fn checksum(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// A name beside `catalogue` no other writer uses at the same time: the
/// process's and a count of the writes it made.
fn temporary_path(catalogue: &Path) -> PathBuf {
    static WRITES: AtomicU64 = AtomicU64::new(0);
    let name = catalogue
        .file_name()
        .map_or_else(|| "fontlist".into(), |name| name.to_string_lossy());
    let count = WRITES.fetch_add(1, Ordering::Relaxed);
    catalogue.with_file_name(format!(".{name}.{}-{count}.tmp", std::process::id()))
}
