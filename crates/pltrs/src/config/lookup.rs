//! Finding the configuration and cache directories and the rc file
//! (matplotlib's `_get_config_or_cache_dir` and `matplotlib_fname`) in an
//! [`Environment`] passed as a value, so that tests choose it.

use std::ffi::OsString;
use std::fs::{self, DirBuilder, OpenOptions};
use std::hash::{BuildHasher, RandomState};
use std::io;
use std::path::{Path, PathBuf};

use super::{CONFIG_DIR_VAR, ConfigError, DirProblem, RC_FILE_NAME, RC_FILE_VAR};

/// The name of pltrs' directories in the XDG base directories and of its
/// temporary directories.
const DIR_NAME: &str = "pltrs";

/// The environment variables the lookups read.
const VARIABLES: [&str; 4] = [
    CONFIG_DIR_VAR,
    RC_FILE_VAR,
    "XDG_CONFIG_HOME",
    "XDG_CACHE_HOME",
];

/// How many names [`make_temp_dir`] tries before giving up.
const TEMP_DIR_ATTEMPTS: u32 = 100;

/// What the lookups read from the process: environment variables and the
/// home, working and temporary directories.
#[derive(Clone, Debug, Default)]
pub(crate) struct Environment {
    /// Environment variables, by name.
    pub(crate) vars: Vec<(String, OsString)>,
    /// The home directory, if there is one.
    pub(crate) home: Option<PathBuf>,
    /// The directory relative paths start from; empty for the process's
    /// working directory, which then is never asked for (it may not
    /// exist), as in matplotlib.
    pub(crate) current_dir: PathBuf,
    /// Where temporary directories go.
    pub(crate) temp_dir: PathBuf,
}

impl Environment {
    /// The process's environment.
    pub(crate) fn process() -> Self {
        Self {
            vars: VARIABLES
                .iter()
                .filter_map(|&name| Some((name.to_owned(), std::env::var_os(name)?)))
                .collect(),
            home: std::env::home_dir(),
            current_dir: PathBuf::new(),
            temp_dir: std::env::temp_dir(),
        }
    }

    /// The path a variable holds, relative to the working directory, when
    /// it is set and not empty.
    fn path_var(&self, name: &str) -> Option<PathBuf> {
        self.vars
            .iter()
            .find(|(key, value)| key == name && !value.is_empty())
            .map(|(_, value)| self.current_dir.join(value))
    }
}

/// The two directories looked up alike.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DirKind {
    /// The configuration directory.
    Config,
    /// The cache directory.
    Cache,
}

impl DirKind {
    /// The XDG variable naming the base directory, and its default in the
    /// home directory.
    const fn xdg_base(self) -> (&'static str, &'static str) {
        match self {
            Self::Config => ("XDG_CONFIG_HOME", ".config"),
            Self::Cache => ("XDG_CACHE_HOME", ".cache"),
        }
    }
}

/// The directory to use: `$PLTRSCONFIGDIR`, else `pltrs` in the XDG base
/// directory; `None` without a home directory to find that in.
fn preferred_dir(env: &Environment, kind: DirKind) -> Option<PathBuf> {
    if let Some(dir) = env.path_var(CONFIG_DIR_VAR) {
        return Some(dir);
    }
    let (variable, default) = kind.xdg_base();
    let base = env
        .path_var(variable)
        .or_else(|| env.home.as_ref().map(|home| home.join(default)))?;
    Some(base.join(DIR_NAME))
}

/// The configuration or cache directory in `env` (matplotlib's
/// `_get_config_or_cache_dir`): the preferred directory, made if missing,
/// when it is a writable directory, else the directory `fallback` makes.
pub(crate) fn find_dir(
    env: &Environment,
    kind: DirKind,
    fallback: impl FnOnce() -> Result<PathBuf, String>,
) -> Result<PathBuf, ConfigError> {
    let problem = match preferred_dir(env, kind) {
        None => DirProblem::NoHome,
        Some(dir) => match usable_dir(&dir) {
            Ok(dir) => return Ok(dir),
            Err(err) => DirProblem::Unusable {
                dir,
                from_variable: env.path_var(CONFIG_DIR_VAR).is_some(),
                reason: err.to_string(),
            },
        },
    };
    fallback().map_err(|reason| ConfigError { problem, reason })
}

/// `dir`, made if missing, as an absolute path without symbolic links,
/// when it is a directory pltrs can write to.
fn usable_dir(dir: &Path) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let dir = fs::canonicalize(dir)?;
    if is_writable(&dir) {
        Ok(dir)
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "not a writable directory",
        ))
    }
}

/// Whether a file can be made in the directory `dir` (what matplotlib
/// asks `os.access(dir, W_OK)`): the test file made is removed again.
fn is_writable(dir: &Path) -> bool {
    let probe = dir.join(format!(".{DIR_NAME}-write-test-{:016x}", unique()));
    let writable = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .is_ok();
    if writable {
        // Best effort: an empty test file left behind is harmless.
        let _ = fs::remove_file(&probe);
    }
    writable
}

/// A number unlikely to be drawn again, by this process or another (each
/// `RandomState` has keys of its own).
fn unique() -> u64 {
    RandomState::new().hash_one(std::process::id())
}

/// A new directory in the temporary directory that only the user can
/// enter (Python's `tempfile.mkdtemp(prefix="pltrs-")`).
pub(crate) fn make_temp_dir(env: &Environment) -> io::Result<PathBuf> {
    let mut builder = DirBuilder::new();
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    (0..TEMP_DIR_ATTEMPTS)
        .map(|_| env.temp_dir.join(format!("{DIR_NAME}-{:016x}", unique())))
        .find_map(|dir| match builder.create(&dir) {
            Ok(()) => Some(Ok(dir)),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => None,
            Err(err) => Some(Err(err)),
        })
        .unwrap_or_else(|| {
            Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "no unused temporary directory name",
            ))
        })
}

/// The rc file in `env` (matplotlib's `matplotlib_fname`): the first of
/// `./pltrsrc`, `$PLTRSRC`, `$PLTRSRC/pltrsrc` and `pltrsrc` in the directory
/// `config_dir` returns that exists and is not a directory. `config_dir`
/// is only asked when the others are missing.
pub(crate) fn find_rc_file(
    env: &Environment,
    config_dir: impl FnOnce() -> Option<PathBuf>,
) -> Option<PathBuf> {
    let is_file = |path: &PathBuf| path.exists() && !path.is_dir();
    let from_variable = env.path_var(RC_FILE_VAR);
    let early = [
        Some(env.current_dir.join(RC_FILE_NAME)),
        from_variable.clone(),
        from_variable.map(|path| path.join(RC_FILE_NAME)),
    ];
    early
        .into_iter()
        .flatten()
        .find(is_file)
        .or_else(|| Some(config_dir()?.join(RC_FILE_NAME)).filter(is_file))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh, empty temporary directory for one test.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("pltrs-config-lookup-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(fs::canonicalize(dir).unwrap())
        }

        fn path(&self, relative: &str) -> PathBuf {
            self.0.join(relative)
        }

        /// An environment with only `vars` set, the home directory `home`
        /// and the working and temporary directories `cwd` and `tmp`, all
        /// inside the scratch directory.
        fn env(&self, vars: &[(&str, &str)]) -> Environment {
            for dir in ["cwd", "tmp"] {
                fs::create_dir_all(self.path(dir)).unwrap();
            }
            Environment {
                vars: vars
                    .iter()
                    .map(|&(name, value)| (name.to_owned(), self.path(value).into()))
                    .collect(),
                home: Some(self.path("home")),
                current_dir: self.path("cwd"),
                temp_dir: self.path("tmp"),
            }
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn no_fallback() -> Result<PathBuf, String> {
        Err("no temporary directory".to_owned())
    }

    fn lookup(env: &Environment, kind: DirKind) -> PathBuf {
        find_dir(env, kind, no_fallback).unwrap()
    }

    #[test]
    fn directories_follow_matplotlibs_order() {
        let scratch = Scratch::new("order");
        let env = scratch.env(&[]);
        assert_eq!(
            lookup(&env, DirKind::Config),
            scratch.path("home/.config/pltrs")
        );
        assert_eq!(
            lookup(&env, DirKind::Cache),
            scratch.path("home/.cache/pltrs")
        );
        assert!(scratch.path("home/.config/pltrs").is_dir(), "made");

        let env = scratch.env(&[("XDG_CONFIG_HOME", "xdg"), ("XDG_CACHE_HOME", "xdg-cache")]);
        assert_eq!(lookup(&env, DirKind::Config), scratch.path("xdg/pltrs"));
        assert_eq!(
            lookup(&env, DirKind::Cache),
            scratch.path("xdg-cache/pltrs")
        );

        let env = scratch.env(&[("XDG_CONFIG_HOME", "xdg"), ("PLTRSCONFIGDIR", "mine")]);
        assert_eq!(lookup(&env, DirKind::Config), scratch.path("mine"));
        assert_eq!(
            lookup(&env, DirKind::Cache),
            scratch.path("mine"),
            "PLTRSCONFIGDIR holds the cache too"
        );
    }

    #[test]
    fn empty_variables_count_as_unset() {
        let scratch = Scratch::new("empty");
        let mut env = scratch.env(&[]);
        env.vars = vec![
            ("PLTRSCONFIGDIR".to_owned(), OsString::new()),
            ("XDG_CONFIG_HOME".to_owned(), OsString::new()),
        ];
        assert_eq!(
            lookup(&env, DirKind::Config),
            scratch.path("home/.config/pltrs")
        );
    }

    #[test]
    fn relative_directories_start_from_the_working_directory() {
        let scratch = Scratch::new("relative");
        let mut env = scratch.env(&[]);
        env.vars = vec![("PLTRSCONFIGDIR".to_owned(), "conf".into())];
        assert_eq!(lookup(&env, DirKind::Config), scratch.path("cwd/conf"));
    }

    #[test]
    fn unusable_directories_fall_back_to_a_temporary_one() {
        let scratch = Scratch::new("fallback");
        fs::write(scratch.path("file"), "not a directory").unwrap();
        let env = scratch.env(&[("PLTRSCONFIGDIR", "file")]);
        let dir = find_dir(&env, DirKind::Config, || {
            make_temp_dir(&env).map_err(|err| err.to_string())
        })
        .unwrap();
        assert_eq!(dir.parent(), Some(scratch.path("tmp").as_path()));
        assert!(dir.is_dir() && is_writable(&dir));
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();
        assert!(name.starts_with("pltrs-"), "{name}");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&dir).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o700, "private");
        }
        assert_ne!(make_temp_dir(&env).unwrap(), dir, "a new one each time");

        let err = find_dir(&env, DirKind::Cache, no_fallback).unwrap_err();
        let message = err.to_string();
        assert!(message.contains("PLTRSCONFIGDIR ("), "{message}");
        assert!(message.contains("no temporary directory"), "{message}");
    }

    #[test]
    fn no_home_directory_falls_back_too() {
        let scratch = Scratch::new("homeless");
        let mut env = scratch.env(&[]);
        env.home = None;
        let err = find_dir(&env, DirKind::Config, no_fallback).unwrap_err();
        assert_eq!(err.problem, DirProblem::NoHome);
        assert!(err.to_string().contains("resolving the home directory"));
        env.vars = vec![("XDG_CONFIG_HOME".to_owned(), scratch.path("xdg").into())];
        assert_eq!(lookup(&env, DirKind::Config), scratch.path("xdg/pltrs"));
    }

    #[test]
    fn the_rc_file_follows_matplotlibs_order() {
        let scratch = Scratch::new("rc");
        let config = scratch.path("config");
        fs::create_dir_all(&config).unwrap();
        let env = scratch.env(&[("PLTRSRC", "custom")]);
        let find = |env: &Environment| find_rc_file(env, || Some(config.clone()));
        assert_eq!(find(&env), None);

        fs::write(config.join("pltrsrc"), "").unwrap();
        assert_eq!(find(&env), Some(config.join("pltrsrc")));

        fs::create_dir_all(scratch.path("custom")).unwrap();
        fs::write(scratch.path("custom/pltrsrc"), "").unwrap();
        assert_eq!(
            find(&env),
            Some(scratch.path("custom/pltrsrc")),
            "a directory named by PLTRSRC holds the file"
        );

        let env = scratch.env(&[("PLTRSRC", "custom/pltrsrc")]);
        assert_eq!(find(&env), Some(scratch.path("custom/pltrsrc")), "a file");

        fs::create_dir_all(scratch.path("cwd/pltrsrc")).unwrap();
        assert_eq!(
            find(&env),
            Some(scratch.path("custom/pltrsrc")),
            "directories are skipped"
        );
        fs::remove_dir(scratch.path("cwd/pltrsrc")).unwrap();
        fs::write(scratch.path("cwd/pltrsrc"), "").unwrap();
        assert_eq!(
            find(&env),
            Some(scratch.path("cwd/pltrsrc")),
            "the working directory comes first"
        );
    }

    #[test]
    fn the_configuration_directory_is_asked_last() {
        let scratch = Scratch::new("lazy");
        let env = scratch.env(&[]);
        fs::write(scratch.path("cwd/pltrsrc"), "").unwrap();
        let found = find_rc_file(&env, || panic!("not needed"));
        assert_eq!(found, Some(scratch.path("cwd/pltrsrc")));
        let env = scratch.env(&[("PLTRSRC", "missing")]);
        fs::remove_file(scratch.path("cwd/pltrsrc")).unwrap();
        assert_eq!(find_rc_file(&env, || None), None);
    }
}
