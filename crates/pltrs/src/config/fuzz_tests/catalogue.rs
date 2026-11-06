//! The font catalogue kept in the cache directory, fuzzed: in the cache
//! directory of a random environment, a catalogue that is missing,
//! written and then corrupted or cut short, of another pltrs version, a
//! directory, or in a read-only directory (on Unix); font files added,
//! removed, touched or replaced since it was written. The fonts added with
//! it must be exactly those added without it, and a catalogue that could
//! be written must serve the next start as it is.

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::Duration;

use pltrs_text::{CatalogueState, FontDatabase, bundled_font_data, catalogue_file_name};

use super::super::lookup::{DirKind, find_dir, make_temp_dir};
use super::{Lookup, Scratch};
use crate::fuzz::{Corruption, Gen, fuzz};

/// The font files a case catalogues, by name: fonts, a file that is no
/// font, and one that does not exist.
const FONTS: [&str; 5] = [
    "cmr10.ttf",
    "cmtt10.ttf",
    "cmb10.ttf",
    "broken.ttf",
    "missing.ttf",
];

/// The font a file is replaced with.
const REPLACEMENT: &str = "cmss10.ttf";

/// What is where the catalogue goes before the fonts are added with it.
#[derive(Debug)]
enum Kept {
    /// Nothing.
    Nothing,
    /// The catalogue of the case's fonts, written by an earlier start and
    /// then corrupted, if the corruption is given.
    Written(Option<Corruption>),
    /// That catalogue with another pltrs version in its header (which its
    /// checksum, or else its version, rejects).
    OtherVersion,
    /// A directory.
    Directory,
}

/// What became of a font file since the catalogue was written.
#[derive(Debug)]
enum Change {
    /// Nothing.
    None,
    /// Its modification time moved.
    Touched(usize),
    /// Another font replaced it.
    Replaced(usize),
    /// It is no longer listed.
    Removed(usize),
}

/// A case: an environment, what its cache directory holds, the font files
/// listed (by index into [`FONTS`], repeated as directories searched
/// twice repeat them), what changed, and whether the cache directory is
/// read-only.
#[derive(Debug)]
struct Case {
    lookup: Lookup,
    kept: Kept,
    fonts: Vec<usize>,
    change: Change,
    read_only: bool,
}

impl Case {
    fn generate(g: &mut Gen) -> Self {
        let fonts: Vec<usize> = (0..=g.len(5)).map(|_| g.below(FONTS.len())).collect();
        let kept = match g.below(6) {
            0 => Kept::Nothing,
            1 | 2 => Kept::Written(None),
            3 => {
                // A valid catalogue's bytes stand in for the corruption's
                // reach: they are a few hundred bytes.
                let corruption = g.corrupt(&[0; 512]);
                Kept::Written(Some(corruption))
            }
            4 => Kept::OtherVersion,
            _ => Kept::Directory,
        };
        let change = match g.below(5) {
            0 => Change::Touched(g.below(fonts.len())),
            1 => Change::Replaced(g.below(fonts.len())),
            2 => Change::Removed(g.below(fonts.len())),
            _ => Change::None,
        };
        Self {
            lookup: Lookup::generate(g),
            kept,
            fonts,
            change,
            read_only: cfg!(unix) && g.chance(0.2),
        }
    }
}

/// Writes the case's font files afresh in `dir`.
fn write_fonts(dir: &Path) {
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).expect("the fonts' directory");
    for name in &FONTS[..3] {
        let bytes = bundled_font_data(name).expect("a bundled font");
        fs::write(dir.join(name), bytes).expect("a font file");
    }
    fs::write(dir.join(FONTS[3]), b"not a font").expect("a file");
}

/// Makes `dir` read-only, or writable again.
fn set_read_only(dir: &Path, read_only: bool) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = if read_only { 0o555 } else { 0o755 };
        let _ = fs::set_permissions(dir, fs::Permissions::from_mode(mode));
    }
    #[cfg(not(unix))]
    let _ = (dir, read_only);
}

/// Puts what the case keeps where the catalogue `file` of the font files
/// `paths` goes.
fn keep(kept: &Kept, file: &Path, paths: &[PathBuf]) {
    let written = || {
        FontDatabase::new().add_files_catalogued(paths, file);
        fs::read(file).unwrap_or_default()
    };
    match kept {
        Kept::Nothing => {}
        Kept::Written(None) => drop(written()),
        Kept::Written(Some(corruption)) => {
            let _ = fs::write(file, corruption.apply(&written()));
        }
        Kept::OtherVersion => {
            let mut bytes = written();
            let version = env!("CARGO_PKG_VERSION").as_bytes();
            if let Some(at) = bytes.windows(version.len()).position(|w| w == version) {
                bytes[at] = if version[0] == b'9' { b'8' } else { b'9' };
            }
            let _ = fs::write(file, bytes);
        }
        Kept::Directory => {
            let _ = fs::create_dir(file);
        }
    }
}

/// Makes the case's change to the listed font files `paths`.
fn change(change: &Change, paths: &mut Vec<PathBuf>) {
    match *change {
        Change::None => {}
        Change::Touched(index) => {
            if let Ok(file) = File::options().write(true).open(&paths[index])
                && let Ok(modified) = file.metadata().and_then(|meta| meta.modified())
            {
                let _ = file.set_modified(modified + Duration::from_secs(1));
            }
        }
        Change::Replaced(index) => {
            let bytes = bundled_font_data(REPLACEMENT).expect("a bundled font");
            let _ = fs::write(&paths[index], bytes);
        }
        Change::Removed(index) => drop(paths.remove(index)),
    }
}

/// Every catalogue entry of `db` with its face's file and index.
fn described(db: &FontDatabase) -> Vec<String> {
    db.entries()
        .iter()
        .map(|entry| {
            format!(
                "{entry:?} {:?} {:?}",
                db.source(entry.font),
                db.face_index(entry.font)
            )
        })
        .collect()
}

#[test]
fn font_catalogues_in_cache_directories() {
    let scratch = Scratch::new("catalogue");
    let fonts = scratch.root.join("fonts");
    fuzz("font catalogue", 300, Case::generate, |case| {
        let env = case.lookup.environment(&scratch);
        let Ok(dir) = find_dir(&env, DirKind::Cache, |_| {
            make_temp_dir(&env).map_err(|err| err.to_string())
        }) else {
            return;
        };
        write_fonts(&fonts);
        let file = dir.join(catalogue_file_name());
        let _ = fs::remove_file(&file);
        let _ = fs::remove_dir_all(&file);
        let mut paths: Vec<PathBuf> = case.fonts.iter().map(|&i| fonts.join(FONTS[i])).collect();
        keep(&case.kept, &file, &paths);
        change(&case.change, &mut paths);
        let read_only = case.read_only && !fonts.starts_with(&dir);
        set_read_only(&dir, read_only);

        let catalogued = FontDatabase::new();
        let report = catalogued.add_files_catalogued(&paths, &file);
        let plain = FontDatabase::new();
        for path in &paths {
            let _ = plain.add_file(path);
        }
        assert_eq!(described(&catalogued), described(&plain));
        let writable = !read_only && !matches!(case.kept, Kept::Directory);
        if writable {
            assert!(report.save_error.is_none(), "{:?}", report.save_error);
            let again = FontDatabase::new().add_files_catalogued(&paths, &file);
            let unreadable = paths.iter().any(|path| !path.exists());
            assert!(
                again.state == CatalogueState::Current || unreadable,
                "{:?}",
                again.state
            );
        }
        set_read_only(&dir, false);
    });
}
