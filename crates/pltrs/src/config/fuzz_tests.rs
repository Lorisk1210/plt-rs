//! Randomised robustness tests of the configuration lookup: random
//! systems, values for `PLTRSCONFIGDIR`, `PLTRSRC`, `XDG_CONFIG_HOME` and
//! `XDG_CACHE_HOME`, home, working and temporary directories — relative
//! and absolute, empty, missing, files instead of directories, read-only
//! and unreadable directories and symbolic link loops (on Unix), `..`,
//! names too long for the file system and names holding NUL — all within
//! a scratch directory. The lookups must find a usable directory or fail with an
//! error, never leave the scratch directory, and the rc file they find
//! must read or fail cleanly.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use pltrs_core::os::{self, Os};

use super::lookup::{DirKind, Environment, find_dir, find_rc_file, make_temp_dir};
use super::{CONFIG_DIR_VAR, RC_FILE_NAME, RC_FILE_VAR};
use crate::fuzz::{Gen, fuzz};
use crate::rc::RcFile;

/// The variables the lookups read.
const VARIABLES: [&str; 4] = [
    CONFIG_DIR_VAR,
    RC_FILE_VAR,
    "XDG_CONFIG_HOME",
    "XDG_CACHE_HOME",
];

/// The names paths are made of: the scratch directory's fixtures, names
/// to be made, `..` (never more than the three levels the fixtures sit
/// below the scratch root), and odd names.
const NAMES: [&str; 16] = [
    "cwd",
    "home",
    "tmp",
    "file",
    "ro",
    "noaccess",
    "loop",
    "rcdir",
    "rc",
    "new",
    "pltrs",
    "..",
    ".",
    RC_FILE_NAME,
    "é ü",
    "x\0y",
];

/// The scratch directory: the fixtures three levels down, so that `..`
/// cannot leave it.
struct Scratch {
    root: PathBuf,
    base: PathBuf,
}

impl Scratch {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("pltrs-fuzz-lookup-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let base = root.join("a/b/c");
        for dir in [
            "cwd",
            "home",
            "tmp",
            "ro",
            "noaccess/sub",
            "rcdir/pltrsrc",
            "rc",
        ] {
            fs::create_dir_all(base.join(dir)).expect("a fixture directory");
        }
        fs::write(base.join("file"), "not a directory").expect("a fixture file");
        fs::write(base.join("noaccess/pltrsrc"), "lines.linewidth: 2").expect("a fixture file");
        unix_fixtures::make(&base);
        let root = os::resolve(&root).expect("the scratch directory");
        let base = root.join("a/b/c");
        Self { root, base }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        unix_fixtures::unlock(&self.base);
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// The fixtures only Unix makes without privileges: a loop of symbolic
/// links, and directories without write or any access.
#[cfg(unix)]
mod unix_fixtures {
    use std::fs;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::path::Path;

    pub(super) fn make(base: &Path) {
        symlink(base.join("loop2"), base.join("loop")).expect("a link");
        symlink(base.join("loop"), base.join("loop2")).expect("a link");
        for (dir, mode) in [("ro", 0o555), ("noaccess", 0o000)] {
            fs::set_permissions(base.join(dir), fs::Permissions::from_mode(mode))
                .expect("permissions");
        }
    }

    /// Gives the directories their access back, so that they can go.
    pub(super) fn unlock(base: &Path) {
        for dir in ["ro", "noaccess"] {
            let _ = fs::set_permissions(base.join(dir), fs::Permissions::from_mode(0o755));
        }
    }
}

/// Elsewhere the fixtures are plain directories, and `loop` is missing.
#[cfg(not(unix))]
mod unix_fixtures {
    use std::path::Path;

    pub(super) fn make(_base: &Path) {}

    pub(super) fn unlock(_base: &Path) {}
}

/// A path of the scratch directory, as names below its base; absolute
/// (from the base) or relative.
#[derive(Clone, Debug)]
struct Place {
    absolute: bool,
    names: Vec<String>,
}

impl Place {
    fn generate(g: &mut Gen) -> Self {
        let mut ups = 0;
        let names = (0..g.len(3))
            .map(|_| match g.below(12) {
                0 => "n".repeat(300),
                _ => g.pick(&NAMES).to_owned(),
            })
            .filter(|name| {
                ups += usize::from(name == "..");
                ups <= 3
            })
            .collect();
        Self {
            absolute: g.chance(0.6),
            names,
        }
    }

    /// The path, from `base` when absolute.
    fn path(&self, base: &Path) -> PathBuf {
        let start = if self.absolute {
            base.to_owned()
        } else {
            PathBuf::new()
        };
        self.names.iter().fold(start, |path, name| path.join(name))
    }
}

/// An environment to look up in: variables, the home, working and
/// temporary directories, and the contents of an rc file.
#[derive(Debug)]
struct Lookup {
    os: Os,
    vars: Vec<(&'static str, Option<Place>)>,
    home: Option<Place>,
    /// The working directory; `None` for the process's, which only
    /// absolute variables may then be used with.
    current: Option<Place>,
    temp: Place,
    rc_text: String,
}

impl Lookup {
    fn generate(g: &mut Gen) -> Self {
        let current = g.chance(0.8).then(|| Place {
            absolute: true,
            names: vec![g.pick(&["cwd", "cwd", "rc", "file", "new"]).to_owned()],
        });
        let mut vars = Vec::new();
        for name in VARIABLES {
            if g.chance(0.5) {
                let mut place = g.chance(0.9).then(|| Place::generate(g));
                if let (None, Some(place)) = (&current, &mut place) {
                    place.absolute = true;
                }
                vars.push((name, place));
            }
        }
        Self {
            os: g.pick(&[Os::Linux, Os::MacOs, Os::Windows]),
            vars,
            home: g.chance(0.9).then(|| Place {
                absolute: true,
                ..Place::generate(g)
            }),
            current,
            temp: Place {
                absolute: true,
                names: vec![
                    g.pick(&["tmp", "tmp", "ro", "file", "new", "loop"])
                        .to_owned(),
                ],
            },
            rc_text: g.text(
                &[
                    "lines.linewidth: 2\n",
                    "axes.grid: True\nfoo: bar\n",
                    "a \"b\n",
                ],
                &[":", "\n", "#", "\"", "lines.color", "red", "\u{FEFF}"],
            ),
        }
    }

    /// The environment within the scratch directory.
    fn environment(&self, scratch: &Scratch) -> Environment {
        Environment {
            os: self.os,
            vars: self
                .vars
                .iter()
                .map(|(name, place)| {
                    let value = place
                        .as_ref()
                        .map_or_else(OsString::new, |p| p.path(&scratch.base).into_os_string());
                    ((*name).to_owned(), value)
                })
                .collect(),
            home: self.home.as_ref().map(|place| place.path(&scratch.base)),
            current_dir: self
                .current
                .as_ref()
                .map_or_else(PathBuf::new, |place| place.path(&scratch.base)),
            temp_dir: self.temp.path(&scratch.base),
        }
    }
}

#[test]
fn configuration_lookups() {
    let scratch = Scratch::new();
    fuzz("config lookup", 1_000, Lookup::generate, |lookup| {
        fs::write(scratch.base.join("rc").join(RC_FILE_NAME), &lookup.rc_text)
            .expect("the rc file is written");
        let env = lookup.environment(&scratch);
        let mut config = None;
        for kind in [DirKind::Config, DirKind::Cache] {
            match find_dir(&env, kind, |_| {
                make_temp_dir(&env).map_err(|e| e.to_string())
            }) {
                Ok(dir) => {
                    assert!(dir.is_dir(), "{dir:?} is not a directory");
                    assert!(
                        dir.starts_with(&scratch.root),
                        "{dir:?} left the scratch directory"
                    );
                    if kind == DirKind::Config {
                        config = Some(dir);
                    }
                }
                Err(err) => assert_ne!(err.to_string(), ""),
            }
        }
        if let Some(path) = find_rc_file(&env, || config) {
            assert!(path.is_file() || !path.is_dir(), "{path:?} is a directory");
            let _ = RcFile::read(&path);
        }
    });
}
