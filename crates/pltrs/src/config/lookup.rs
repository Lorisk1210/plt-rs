//! Finding the configuration and cache directories and the rc file
//! (matplotlib's `_get_config_or_cache_dir` and `matplotlib_fname`) in an
//! [`Environment`] passed as a value, so that tests choose it.

use std::ffi::OsString;
use std::fs::{self, DirBuilder, OpenOptions};
use std::hash::{BuildHasher, RandomState};
use std::io;
use std::path::{Path, PathBuf};

use pltrs_core::os::{self, Os};

use super::{
    CONFIG_DIR_VAR, ConfigError, DirProblem, MATPLOTLIB_CONFIG_DIR_VAR, RC_FILE_NAME, RC_FILE_VAR,
};

/// The name of pltrs' directories in the XDG base directories and of its
/// temporary directories.
const DIR_NAME: &str = "pltrs";

/// The name of pltrs' directory in the home directory on macOS and
/// Windows (matplotlib's `.matplotlib`).
const HOME_DIR_NAME: &str = ".pltrs";

/// The variable naming Windows' folder of local (not roaming)
/// application data, where pltrs keeps its directory on Windows.
const LOCAL_APP_DATA_VAR: &str = "LOCALAPPDATA";

/// The environment variables the lookups read.
const VARIABLES: [&str; 5] = [
    CONFIG_DIR_VAR,
    RC_FILE_VAR,
    "XDG_CONFIG_HOME",
    "XDG_CACHE_HOME",
    LOCAL_APP_DATA_VAR,
];

/// The variables naming the configuration and cache directory, in the
/// order they are read: matplotlib's counts only when the process
/// [honours](super::honour_matplotlib_config_dir) it, which
/// [`Environment::process`] decides.
const CONFIG_DIR_VARS: [&str; 2] = [CONFIG_DIR_VAR, MATPLOTLIB_CONFIG_DIR_VAR];

/// How many names [`make_temp_dir`] tries before giving up.
const TEMP_DIR_ATTEMPTS: u32 = 100;

/// What the lookups read from the process: the system, environment
/// variables and the home, working and temporary directories.
#[derive(Clone, Debug)]
pub(crate) struct Environment {
    /// The system, whose conventions place the directories.
    pub(crate) os: Os,
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
    /// The process's environment: `MPLCONFIGDIR` among its variables only
    /// when the process [honours](super::honour_matplotlib_config_dir) it.
    pub(crate) fn process() -> Self {
        let matplotlib =
            super::matplotlib_config_dir_honoured().then_some(MATPLOTLIB_CONFIG_DIR_VAR);
        Self {
            os: Os::CURRENT,
            vars: VARIABLES
                .iter()
                .copied()
                .chain(matplotlib)
                .filter_map(|name| Some((name.to_owned(), std::env::var_os(name)?)))
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

/// The directory to use, with the variable that named it: the first of
/// `$PLTRSCONFIGDIR` and `$MPLCONFIGDIR` set, else (named by no variable)
/// `pltrs` in the XDG base directory on Linux, `~/.pltrs` on macOS, and
/// on Windows [`windows_dir`]; `None` without a home directory (or on
/// Windows `LOCALAPPDATA`) to find that in.
fn preferred_dir(env: &Environment, kind: DirKind) -> Option<(PathBuf, Option<&'static str>)> {
    if let Some(named) = CONFIG_DIR_VARS
        .iter()
        .find_map(|&name| Some((env.path_var(name)?, Some(name))))
    {
        return Some(named);
    }
    let dir = match env.os {
        Os::Linux => {
            let (variable, default) = kind.xdg_base();
            env.path_var(variable)
                .or_else(|| env.home.as_ref().map(|home| home.join(default)))?
                .join(DIR_NAME)
        }
        Os::MacOs => env.home.as_ref()?.join(HOME_DIR_NAME),
        Os::Windows => windows_dir(env)?,
    };
    Some((dir, None))
}

/// The directory on Windows (matplotlib 3.11's rule): `~/.pltrs` when it
/// already is a directory, so that an existing configuration stays in
/// use, else `pltrs` in `%LOCALAPPDATA%` (the local application data,
/// which does not roam), else `~/.pltrs`.
fn windows_dir(env: &Environment) -> Option<PathBuf> {
    let in_home = env.home.as_ref().map(|home| home.join(HOME_DIR_NAME));
    in_home
        .clone()
        .filter(|dir| dir.is_dir())
        .or_else(|| Some(env.path_var(LOCAL_APP_DATA_VAR)?.join(DIR_NAME)))
        .or(in_home)
}

/// The configuration or cache directory in `env` (matplotlib's
/// `_get_config_or_cache_dir`): the preferred directory, made if missing,
/// when it is a writable directory, else the directory `fallback` makes,
/// told why the preferred one is not used.
pub(crate) fn find_dir(
    env: &Environment,
    kind: DirKind,
    fallback: impl FnOnce(&DirProblem) -> Result<PathBuf, String>,
) -> Result<PathBuf, ConfigError> {
    let problem = match preferred_dir(env, kind) {
        None => DirProblem::NoHome,
        Some((dir, variable)) => match usable_dir(&dir) {
            Ok(dir) => return Ok(dir),
            Err(err) => DirProblem::Unusable {
                // As matplotlib resolves it, as far as it exists.
                dir: os::resolve(&dir).unwrap_or(dir),
                variable,
                reason: err.to_string(),
            },
        },
    };
    fallback(&problem).map_err(|reason| ConfigError { problem, reason })
}

/// `dir`, made if missing, as an absolute path without symbolic links,
/// when it is a directory pltrs can write to.
fn usable_dir(dir: &Path) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let dir = os::resolve(dir)?;
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
    let builder = private_dir_builder();
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

/// A builder of directories only the user can enter: mode 0700 on Unix.
#[cfg(unix)]
fn private_dir_builder() -> DirBuilder {
    use std::os::unix::fs::DirBuilderExt as _;
    let mut builder = DirBuilder::new();
    builder.mode(0o700);
    builder
}

/// A builder of directories only the user can enter: on Windows a new
/// directory inherits the permissions of the user's temporary directory,
/// which only the user can enter.
#[cfg(not(unix))]
fn private_dir_builder() -> DirBuilder {
    DirBuilder::new()
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
            Self(os::resolve(&dir).unwrap())
        }

        fn path(&self, relative: &str) -> PathBuf {
            self.0.join(relative)
        }

        /// An environment of Linux with only `vars` set, the home
        /// directory `home` and the working and temporary directories
        /// `cwd` and `tmp`, all inside the scratch directory.
        fn env(&self, vars: &[(&str, &str)]) -> Environment {
            for dir in ["cwd", "tmp"] {
                fs::create_dir_all(self.path(dir)).unwrap();
            }
            Environment {
                os: Os::Linux,
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

    fn no_fallback(_: &DirProblem) -> Result<PathBuf, String> {
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
    fn windows_keeps_both_directories_in_the_local_application_data() {
        let scratch = Scratch::new("local-app-data");
        let mut env = scratch.env(&[("LOCALAPPDATA", "AppData/Local")]);
        env.os = Os::Windows;
        let local = scratch.path("AppData/Local/pltrs");
        assert_eq!(lookup(&env, DirKind::Config), local);
        assert_eq!(lookup(&env, DirKind::Cache), local);
        assert!(!scratch.path("home/.pltrs").exists(), "not made");

        env.home = None;
        assert_eq!(lookup(&env, DirKind::Config), local, "without a home");

        env.home = Some(scratch.path("home"));
        fs::create_dir_all(scratch.path("home/.pltrs")).unwrap();
        assert_eq!(
            lookup(&env, DirKind::Config),
            scratch.path("home/.pltrs"),
            "an existing directory in the home directory stays in use"
        );
    }

    #[test]
    fn macos_and_windows_without_local_app_data_use_the_home_directory() {
        let scratch = Scratch::new("home-dir");
        for os in [Os::MacOs, Os::Windows] {
            let mut env = scratch.env(&[("XDG_CONFIG_HOME", "xdg"), ("XDG_CACHE_HOME", "xdg")]);
            env.os = os;
            assert_eq!(lookup(&env, DirKind::Config), scratch.path("home/.pltrs"));
            assert_eq!(lookup(&env, DirKind::Cache), scratch.path("home/.pltrs"));
            env.vars
                .push(("PLTRSCONFIGDIR".to_owned(), scratch.path("mine").into()));
            assert_eq!(lookup(&env, DirKind::Config), scratch.path("mine"));
            env.home = None;
            env.vars.clear();
            let err = find_dir(&env, DirKind::Cache, no_fallback).unwrap_err();
            assert_eq!(err.problem, DirProblem::NoHome);
        }
    }

    #[test]
    fn matplotlibs_variable_comes_after_pltrs_own() {
        let scratch = Scratch::new("matplotlib");
        let env = scratch.env(&[("MPLCONFIGDIR", "theirs")]);
        assert_eq!(lookup(&env, DirKind::Config), scratch.path("theirs"));
        assert_eq!(lookup(&env, DirKind::Cache), scratch.path("theirs"));
        let env = scratch.env(&[("MPLCONFIGDIR", "theirs"), ("PLTRSCONFIGDIR", "mine")]);
        assert_eq!(lookup(&env, DirKind::Config), scratch.path("mine"));

        fs::write(scratch.path("file"), "not a directory").unwrap();
        let env = scratch.env(&[("MPLCONFIGDIR", "file")]);
        let message = find_dir(&env, DirKind::Config, no_fallback)
            .unwrap_err()
            .to_string();
        let named = format!("MPLCONFIGDIR ({})", scratch.path("file").display());
        assert!(message.contains(&named), "{message}");
        assert!(
            message.ends_with("set the MPLCONFIGDIR environment variable to a writable directory"),
            "{message}"
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
        let dir = find_dir(&env, DirKind::Config, |problem| {
            assert!(
                matches!(
                    problem,
                    DirProblem::Unusable { dir, variable: Some("PLTRSCONFIGDIR"), .. }
                        if *dir == scratch.path("file")
                ),
                "{problem:?}"
            );
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
