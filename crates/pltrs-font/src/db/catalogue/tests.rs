//! Catalogue tests: written once and read back, files read again only when
//! they change, and catalogues that cannot be used or written.

use std::cell::RefCell;
use std::fs::{self, File};
use std::time::{Duration, SystemTime};

use super::*;
use crate::db::FontStyle;

/// A scratch directory of the temporary directory, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "pltrs-font-catalogue-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    /// A file `name` of `bytes`, its path.
    fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, bytes).unwrap();
        path
    }

    fn catalogue(&self) -> PathBuf {
        self.0.join(catalogue_file_name())
    }

    /// The names of the files in the directory, sorted.
    fn listing(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(&self.0)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A description of a face named `name`.
fn described(name: &str) -> FaceStyle {
    FaceStyle {
        name: name.to_owned(),
        style: FontStyle::Italic,
        variant: crate::db::FontVariant::Normal,
        weight: 700,
        stretch: crate::db::Stretch::CONDENSED,
    }
}

/// A describer recording the files it reads: a file holding `font` has
/// one face named after the file, two for `fonts`, and others hold none;
/// files named `unreadable` cannot be read.
#[derive(Default)]
struct Describer {
    read: RefCell<Vec<PathBuf>>,
}

impl Describer {
    fn describe(&self, path: &Path) -> io::Result<Vec<Vec<FaceStyle>>> {
        self.read.borrow_mut().push(path.to_path_buf());
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        match fs::read(path)?.as_slice() {
            _ if name.starts_with("unreadable") => Err(io::ErrorKind::PermissionDenied.into()),
            b"font" => Ok(vec![vec![described(&name)]]),
            b"fonts" => Ok(vec![
                vec![described(&name), described("alternative")],
                vec![described("second")],
            ]),
            _ => Ok(Vec::new()),
        }
    }

    /// Refreshes the catalogue of `paths` in `catalogue`, returning the
    /// files and report, and the files read meanwhile.
    fn refresh(
        &self,
        catalogue: &Path,
        paths: &[PathBuf],
    ) -> (Vec<CataloguedFile>, CatalogueReport, Vec<PathBuf>) {
        self.read.borrow_mut().clear();
        let (files, report) = refresh(catalogue, paths, |path| self.describe(path));
        (files, report, self.read.take())
    }
}

/// Moves the modification time of the file at `path` by a second.
fn touch(path: &Path) {
    let modified = fs::metadata(path).unwrap().modified().unwrap();
    File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(modified + Duration::from_secs(1))
        .unwrap();
}

#[test]
fn catalogues_are_written_once_and_read_back() {
    let scratch = Scratch::new("written");
    let paths = vec![
        scratch.file("a.ttf", b"font"),
        scratch.file("b.ttc", b"fonts"),
        scratch.file("c.ttf", b"no font"),
    ];
    let describer = Describer::default();
    let (files, report, read) = describer.refresh(&scratch.catalogue(), &paths);
    assert!(matches!(report.state, CatalogueState::Rebuilt { .. }));
    assert!(report.save_error.is_none(), "{:?}", report.save_error);
    assert_eq!(read, paths);
    let faces: Vec<_> = files.iter().map(|file| file.faces.clone()).collect();
    assert_eq!(
        faces,
        [
            Some(vec![vec![described("a.ttf")]]),
            Some(vec![
                vec![described("b.ttc"), described("alternative")],
                vec![described("second")]
            ]),
            Some(Vec::new()),
        ]
    );
    // Written atomically: nothing but the catalogue is left beside it.
    assert_eq!(
        scratch.listing(),
        ["a.ttf", "b.ttc", "c.ttf", &catalogue_file_name()]
    );

    let (again, report, read) = describer.refresh(&scratch.catalogue(), &paths);
    assert_eq!(report.state, CatalogueState::Current);
    assert_eq!(read, Vec::<PathBuf>::new(), "no font file is opened");
    assert_eq!(again, files);
    assert_eq!(report.file, scratch.catalogue());
}

#[test]
fn files_are_read_again_when_they_change() {
    let scratch = Scratch::new("changes");
    let (a, b) = (
        scratch.file("a.ttf", b"font"),
        scratch.file("b.ttf", b"font"),
    );
    let catalogue = scratch.catalogue();
    let describer = Describer::default();
    describer.refresh(&catalogue, &[a.clone(), b.clone()]);

    touch(&b);
    let (_, report, read) = describer.refresh(&catalogue, &[a.clone(), b.clone()]);
    assert_eq!(report.state, CatalogueState::Updated { read: 1 });
    assert_eq!(read, [b.as_path()]);

    fs::write(&a, b"fonts").unwrap();
    let (files, report, read) = describer.refresh(&catalogue, &[a.clone(), b.clone()]);
    assert_eq!(report.state, CatalogueState::Updated { read: 1 });
    assert_eq!(read, [a.as_path()]);
    assert_eq!(files[0].faces.as_ref().map(Vec::len), Some(2));

    let c = scratch.file("c.otf", b"font");
    let (files, report, read) = describer.refresh(&catalogue, &[c.clone(), a.clone(), b.clone()]);
    assert_eq!(report.state, CatalogueState::Updated { read: 1 });
    assert_eq!(read, [c.as_path()]);
    let order: Vec<&Path> = files.iter().map(|file| file.path.as_path()).collect();
    assert_eq!(order, [c.as_path(), a.as_path(), b.as_path()]);

    let (files, report, read) = describer.refresh(&catalogue, std::slice::from_ref(&b));
    assert_eq!(report.state, CatalogueState::Updated { read: 0 });
    assert_eq!(read, Vec::<PathBuf>::new());
    assert_eq!(files.len(), 1);
    let (_, report, _) = describer.refresh(&catalogue, std::slice::from_ref(&b));
    assert_eq!(report.state, CatalogueState::Current);

    // A file listed twice (in two directories searched) is added twice.
    let (files, _, read) = describer.refresh(&catalogue, &[b.clone(), b.clone()]);
    assert_eq!(read, Vec::<PathBuf>::new());
    assert_eq!(files.len(), 2);
}

#[test]
fn unreadable_files_are_tried_at_every_start() {
    let scratch = Scratch::new("unreadable");
    let paths = vec![
        scratch.file("unreadable.ttf", b"font"),
        scratch.file("a.ttf", b"font"),
    ];
    let missing = scratch.0.join("missing.ttf");
    let catalogue = scratch.catalogue();
    let describer = Describer::default();
    let with_missing = [paths.clone(), vec![missing.clone()]].concat();
    let (files, _, _) = describer.refresh(&catalogue, &with_missing);
    assert_eq!(files[0].faces, None);
    assert_eq!(files[2].faces, None);
    let (_, report, read) = describer.refresh(&catalogue, &with_missing);
    assert_eq!(
        report.state,
        CatalogueState::Current,
        "unchanged, not rewritten"
    );
    assert_eq!(read, [paths[0].clone(), missing]);
}

/// A valid catalogue of one font file in `scratch`, its bytes.
fn valid_catalogue(scratch: &Scratch) -> (Vec<PathBuf>, Vec<u8>) {
    let paths = vec![scratch.file("a.ttf", b"font")];
    Describer::default().refresh(&scratch.catalogue(), &paths);
    (paths, fs::read(scratch.catalogue()).unwrap())
}

#[test]
fn broken_catalogues_are_rebuilt() {
    let scratch = Scratch::new("broken");
    let (paths, valid) = valid_catalogue(&scratch);
    let version = env!("CARGO_PKG_VERSION").as_bytes();
    let at = valid
        .windows(version.len())
        .position(|window| window == version)
        .expect("the archive names pltrs' version");
    // Valid catalogues of another pltrs and another format, checksums
    // included.
    let sealed = |mut archived: Vec<u8>| {
        let sum = checksum(&archived);
        archived.extend(sum.to_le_bytes());
        archived
    };
    let mut other_pltrs = valid[..valid.len() - 8].to_vec();
    other_pltrs[at] = if version[0] == b'9' { b'8' } else { b'9' };
    let other_pltrs = sealed(other_pltrs);
    let other_format = sealed(
        archive::to_bytes(
            &Kept {
                format: FORMAT + 1,
                files: Vec::new(),
            },
            &NoProgram,
        )
        .unwrap()
        .0,
    );
    let mut wrong_sum = valid.clone();
    let last = wrong_sum.len() - 1;
    wrong_sum[last] ^= 1;
    let mut weight = valid.clone();
    // A changed weight still reads as an archive; the checksum tells.
    let at_weight = weight.len() - 8 - 3;
    weight[at_weight] ^= 1;
    let mut flipped = valid.clone();
    flipped[valid.len() / 2] ^= 0xff;
    let cases: [(&str, Vec<u8>); 9] = [
        ("empty", Vec::new()),
        ("garbage", b"not a catalogue at all".repeat(20)),
        ("truncated", valid[..valid.len() / 2].to_vec()),
        ("trailing bytes", [valid.as_slice(), b"x"].concat()),
        ("flipped", flipped),
        ("wrong checksum", wrong_sum),
        ("changed description", weight),
        ("another pltrs", other_pltrs),
        ("another format", other_format),
    ];
    let describer = Describer::default();
    for (name, bytes) in cases {
        fs::write(scratch.catalogue(), &bytes).unwrap();
        let (files, report, read) = describer.refresh(&scratch.catalogue(), &paths);
        assert_eq!(read, paths, "{name}");
        assert_eq!(
            files[0].faces,
            Some(vec![vec![described("a.ttf")]]),
            "{name}"
        );
        assert!(report.save_error.is_none(), "{name}");
        assert_ne!(report.state, CatalogueState::Current, "{name}");
        assert_eq!(
            fs::read(scratch.catalogue()).unwrap(),
            valid,
            "{name} rewritten"
        );
    }

    // Too large to be a catalogue: refused unread.
    File::create(scratch.catalogue())
        .unwrap()
        .set_len(MAX_SIZE + 1)
        .unwrap();
    let (_, report, _) = describer.refresh(&scratch.catalogue(), &paths);
    assert!(
        matches!(&report.state, CatalogueState::Rebuilt { reason } if reason.contains("too large")),
        "{:?}",
        report.state
    );
    assert_eq!(fs::read(scratch.catalogue()).unwrap(), valid);
}

#[test]
fn catalogues_that_cannot_be_written_are_used_in_memory() {
    let scratch = Scratch::new("unwritable");
    let paths = vec![scratch.file("a.ttf", b"font")];
    let describer = Describer::default();
    // A directory where the catalogue should be: neither read nor replaced.
    fs::create_dir(scratch.catalogue()).unwrap();
    let (files, report, _) = describer.refresh(&scratch.catalogue(), &paths);
    assert!(matches!(report.state, CatalogueState::Rebuilt { .. }));
    assert!(report.save_error.is_some());
    assert_eq!(files[0].faces, Some(vec![vec![described("a.ttf")]]));
    assert_eq!(
        scratch.listing(),
        ["a.ttf", &catalogue_file_name()],
        "nothing left behind"
    );
    // In a directory that does not exist.
    let nowhere = scratch.0.join("missing/dir").join(catalogue_file_name());
    let (files, report, _) = describer.refresh(&nowhere, &paths);
    assert_eq!(
        report.save_error.map(|err| err.kind()),
        Some(io::ErrorKind::NotFound)
    );
    assert_eq!(files.len(), 1);
}

#[cfg(unix)]
#[test]
fn read_only_cache_directories_keep_their_catalogue() {
    use std::os::unix::fs::PermissionsExt;

    let scratch = Scratch::new("read-only");
    let (paths, valid) = valid_catalogue(&scratch);
    let set_mode = |mode| fs::set_permissions(&scratch.0, fs::Permissions::from_mode(mode));
    set_mode(0o555).unwrap();
    let describer = Describer::default();
    let (_, report, read) = describer.refresh(&scratch.catalogue(), &paths);
    assert_eq!(
        report.state,
        CatalogueState::Current,
        "read, though not writable"
    );
    assert_eq!(read, Vec::<PathBuf>::new());
    let other = scratch.0.join("b.ttf");
    let (files, report, _) = describer.refresh(&scratch.catalogue(), &[paths[0].clone(), other]);
    set_mode(0o755).unwrap();
    assert_eq!(report.state, CatalogueState::Updated { read: 1 });
    assert_eq!(
        report.save_error.map(|err| err.kind()),
        Some(io::ErrorKind::PermissionDenied)
    );
    assert_eq!(files.len(), 2);
    assert_eq!(fs::read(scratch.catalogue()).unwrap(), valid);
}

#[test]
fn files_without_a_stamp_are_read_at_every_start() {
    let scratch = Scratch::new("stamps");
    let path = scratch.file("a.ttf", b"font");
    File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(SystemTime::UNIX_EPOCH - Duration::from_secs(10))
        .unwrap();
    assert_eq!(Stamp::of(&path), None);
    let describer = Describer::default();
    describer.refresh(&scratch.catalogue(), std::slice::from_ref(&path));
    let (_, report, read) = describer.refresh(&scratch.catalogue(), std::slice::from_ref(&path));
    assert_eq!(read, [path]);
    assert_eq!(report.state, CatalogueState::Current);
}

#[cfg(unix)]
#[test]
fn paths_that_are_not_text_are_read_at_every_start() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let scratch = Scratch::new("bytes");
    let odd = scratch.0.join(OsStr::from_bytes(b"odd-\xff.ttf"));
    fs::write(&odd, b"font").unwrap();
    let paths = vec![odd, scratch.file("a.ttf", b"font")];
    let describer = Describer::default();
    let (files, report, _) = describer.refresh(&scratch.catalogue(), &paths);
    assert!(report.save_error.is_none());
    assert_eq!(files.len(), 2);
    let (files, report, read) = describer.refresh(&scratch.catalogue(), &paths);
    assert_eq!(report.state, CatalogueState::Current);
    assert_eq!(read, [paths[0].clone()]);
    assert_eq!(files.len(), 2);
}
