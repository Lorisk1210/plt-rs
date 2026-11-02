//! The user's configuration: the configuration and cache directories, the
//! rc file and the style library in the configuration directory
//! (matplotlib's `get_configdir`, `get_cachedir`, `matplotlib_fname` and
//! `matplotlibrc`).
//!
//! | what                        | where pltrs looks, in order                                    | matplotlib's counterpart |
//! |-----------------------------|----------------------------------------------------------------|--------------------------|
//! | [`config_dir`]              | `$PLTRSCONFIGDIR`, `$XDG_CONFIG_HOME/pltrs`, `~/.config/pltrs` | `MPLCONFIGDIR`, `~/.config/matplotlib` |
//! | [`cache_dir`]               | `$PLTRSCONFIGDIR`, `$XDG_CACHE_HOME/pltrs`, `~/.cache/pltrs`   | `MPLCONFIGDIR`, `~/.cache/matplotlib` |
//! | [`rc_file_path`]            | `./pltrsrc`, `$PLTRSRC` (a file, or a directory holding `pltrsrc`), `pltrsrc` in the configuration directory | `matplotlibrc`, `MATPLOTLIBRC` |
//! | the user style library      | the `*.mplstyle` files of `stylelib` in the configuration directory | `stylelib`         |
//!
//! The XDG directories are Linux's (and the other Unix systems'): on
//! macOS and Windows both directories are `~/.pltrs` unless
//! `PLTRSCONFIGDIR` names one, as matplotlib's are `~/.matplotlib` there.
//! A process may also [honour](honour_matplotlib_config_dir) matplotlib's
//! `MPLCONFIGDIR`, read when `PLTRSCONFIGDIR` is unset: pltrs (Python)
//! does, so that it works in matplotlib's environment unchanged.
//!
//! Like matplotlib, pltrs makes the configuration and cache directories
//! when they are missing; when one cannot be made or written to, a new
//! private directory in the temporary directory takes its place for the
//! rest of the process, and [`temp_dir_notice`] says so in matplotlib's
//! words. Empty variables count as unset.
//!
//! The rc file has matplotlib's `matplotlibrc` syntax (`key: value  #
//! comment`, see [`RcFile`]). Threads start from the parameters of the
//! figure [look](crate::look) with the rc file's on top
//! ([`RcParams::startup`]), the way matplotlib's `matplotlibrc` applies
//! over its defaults; [`rc::restore_startup`](crate::rc::restore_startup)
//! goes back to them and [`rc::reset`](crate::rc::reset) to matplotlib's
//! defaults. The file is read once per process, when the first thread
//! needs its parameters; the lines it cannot use are skipped, and
//! [`user_rc_file`] lists them. The [style library](crate::style::library)
//! reads its user sheets the first time it is consulted.
//!
//! ```
//! use pltrs::config;
//! let dir = config::config_dir()?;
//! assert!(dir.is_dir());
//! if let Some(file) = config::user_rc_file()? {
//!     for skipped in &file.warnings {
//!         println!("{skipped}");
//!     }
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Tests
//!
//! Results must not depend on who runs them. [`ignore_user_config`] makes
//! the process ignore the rc file and the user style library, as
//! [`testing::setup`](crate::testing::setup) does; this crate's own unit
//! tests ignore them from the start. Within the pltrs workspace, cargo
//! sets `PLTRSCONFIGDIR` to an empty directory under `target/` unless it is
//! set already, so doctests and examples see no configuration from the
//! home directory either.

mod data;
#[cfg(test)]
mod fuzz_tests;
mod lookup;

use std::fmt;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

pub use data::{DataDirError, data_dir};
use lookup::{DirKind, Environment};

use crate::rc::{RcError, RcFile, RcParams};

/// The environment variable naming the configuration and cache directory
/// (matplotlib's `MPLCONFIGDIR`).
pub const CONFIG_DIR_VAR: &str = "PLTRSCONFIGDIR";

/// matplotlib's environment variable naming its configuration and cache
/// directory, which a process [honouring](honour_matplotlib_config_dir) it
/// reads when [`CONFIG_DIR_VAR`] is unset.
pub const MATPLOTLIB_CONFIG_DIR_VAR: &str = "MPLCONFIGDIR";

/// The environment variable naming the rc file, or a directory holding
/// one (matplotlib's `MATPLOTLIBRC`).
pub const RC_FILE_VAR: &str = "PLTRSRC";

/// The file name of the rc file (matplotlib's `matplotlibrc`).
pub const RC_FILE_NAME: &str = "pltrsrc";

/// The directory of user style sheets in the configuration directory.
const STYLE_LIBRARY_DIR: &str = "stylelib";

/// Whether the process ignores the user configuration: from the start in
/// this crate's unit tests, which must not depend on the developer's.
static IGNORED: AtomicBool = AtomicBool::new(cfg!(test));

/// Whether the lookups read [`MATPLOTLIB_CONFIG_DIR_VAR`] too.
static MATPLOTLIB_VAR: AtomicBool = AtomicBool::new(false);

/// What the temporary directory stands in for, once it was made.
static TEMP_DIR_NOTICE: OnceLock<TempDirNotice> = OnceLock::new();

/// The configuration directory, once looked up.
static CONFIG_DIR: OnceLock<Result<PathBuf, ConfigError>> = OnceLock::new();

/// The cache directory, once looked up.
static CACHE_DIR: OnceLock<Result<PathBuf, ConfigError>> = OnceLock::new();

/// The temporary directory standing in for the configuration or cache
/// directory, once made.
static TEMP_DIR: OnceLock<Result<PathBuf, String>> = OnceLock::new();

/// The user rc file, once read.
static USER_RC: OnceLock<Result<Option<RcFile>, RcError>> = OnceLock::new();

/// The configuration directory (matplotlib's `get_configdir`): the one
/// `PLTRSCONFIGDIR` names, else `pltrs` in `$XDG_CONFIG_HOME` (by default
/// `~/.config`) on Linux and `~/.pltrs` on macOS and Windows. It is made if missing; when it cannot be made or written
/// to, a new private temporary directory takes its place. Looked up once
/// per process.
///
/// # Errors
///
/// When that temporary directory cannot be made either.
pub fn config_dir() -> Result<PathBuf, ConfigError> {
    CONFIG_DIR
        .get_or_init(|| lookup::find_dir(&Environment::process(), DirKind::Config, temp_dir))
        .clone()
}

/// Makes the process read matplotlib's `MPLCONFIGDIR` when
/// `PLTRSCONFIGDIR` is unset, in the lookups of [`config_dir`] and
/// [`cache_dir`] not made yet: what pltrs (Python) does, so that it works
/// in matplotlib's environment unchanged.
pub fn honour_matplotlib_config_dir() {
    MATPLOTLIB_VAR.store(true, Ordering::Relaxed);
}

/// Whether the process reads `MPLCONFIGDIR` (see
/// [`honour_matplotlib_config_dir`]).
fn matplotlib_config_dir_honoured() -> bool {
    MATPLOTLIB_VAR.load(Ordering::Relaxed)
}

/// Why a temporary directory stands in for the configuration or cache
/// directory, when one does: what matplotlib logs as a warning when it
/// makes one.
pub fn temp_dir_notice() -> Option<&'static TempDirNotice> {
    TEMP_DIR_NOTICE.get()
}

/// The cache directory (matplotlib's `get_cachedir`), looked up as
/// [`config_dir`] is, on Linux with `$XDG_CACHE_HOME` (by default
/// `~/.cache`) in place of `$XDG_CONFIG_HOME`. pltrs keeps there what `text.usetex`
/// makes, in `tex.cache` as matplotlib does (and the fonts kpathsea
/// generates in `vartexfonts`).
///
/// # Errors
///
/// As [`config_dir`].
pub fn cache_dir() -> Result<PathBuf, ConfigError> {
    CACHE_DIR
        .get_or_init(|| lookup::find_dir(&Environment::process(), DirKind::Cache, temp_dir))
        .clone()
}

/// The private temporary directory standing in for both directories,
/// made the first time one of them needs it because of `problem`
/// (matplotlib then points `MPLCONFIGDIR` at it); [`temp_dir_notice`]
/// tells why.
fn temp_dir(problem: &DirProblem) -> Result<PathBuf, String> {
    TEMP_DIR
        .get_or_init(|| {
            let dir =
                lookup::make_temp_dir(&Environment::process()).map_err(|err| err.to_string())?;
            TEMP_DIR_NOTICE.get_or_init(|| TempDirNotice {
                dir: dir.clone(),
                problem: problem.clone(),
            });
            Ok(dir)
        })
        .clone()
}

/// The rc file to start from (matplotlib's `matplotlib_fname`): the first
/// of `pltrsrc` in the working directory, the file `$PLTRSRC` names, `pltrsrc`
/// in the directory `$PLTRSRC` names and `pltrsrc` in the [`config_dir`]
/// that exists and is not a directory. `None` when there is none; pltrs'
/// defaults are built in rather than read from a file.
pub fn rc_file_path() -> Option<PathBuf> {
    lookup::find_rc_file(&Environment::process(), || config_dir().ok())
}

/// The rc file pltrs reads (matplotlib's `matplotlib_fname`): the
/// [`rc_file_path`], else the template of the defaults in the
/// [`data_dir`] (`pltrsrc`, every parameter commented out), as matplotlib
/// falls back on its `mpl-data/matplotlibrc`.
///
/// ```
/// let path = pltrs::config::rc_file_or_template()?;
/// assert!(path.is_file());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// When there is no rc file and the data directory cannot be written.
pub fn rc_file_or_template() -> Result<PathBuf, DataDirError> {
    rc_file_path().map_or_else(|| Ok(data_dir()?.join(RC_FILE_NAME)), Ok)
}

/// The user rc file threads start from, as read the first time it was
/// needed, with the lines skipped in [`RcFile::warnings`]. `None` when
/// there is none or the user configuration is
/// [ignored](ignore_user_config).
///
/// # Errors
///
/// When the file cannot be read or has a double quote that is not closed
/// (which matplotlib does not skip either); threads then start without
/// it.
pub fn user_rc_file() -> Result<Option<&'static RcFile>, RcError> {
    if user_config_ignored() {
        return Ok(None);
    }
    USER_RC
        .get_or_init(|| rc_file_path().map(RcFile::read).transpose())
        .as_ref()
        .map(Option::as_ref)
        .map_err(Clone::clone)
}

/// Makes the rest of the process ignore the user configuration: threads
/// started from now on, and [`rc::restore_startup`](crate::rc::restore_startup),
/// take the look's parameters without the rc file, and the style library
/// has no default user directory. Running threads keep their parameters,
/// and directories given to
/// [`style::set_user_library_paths`](crate::style::set_user_library_paths)
/// still count. [`testing::setup`](crate::testing::setup) calls this.
pub fn ignore_user_config() {
    IGNORED.store(true, Ordering::Relaxed);
}

/// Whether the process ignores the user configuration (see
/// [`ignore_user_config`]).
pub fn user_config_ignored() -> bool {
    IGNORED.load(Ordering::Relaxed)
}

/// `params` with the settings of the user rc file on top.
pub(crate) fn with_user_rc(mut params: RcParams) -> RcParams {
    if let Ok(Some(file)) = user_rc_file() {
        apply_rc_file(file, &mut params);
    }
    params
}

/// Applies the settings of `file` to `params`.
fn apply_rc_file(file: &RcFile, params: &mut RcParams) {
    for entry in &file.entries {
        params
            .set(&entry.key, &entry.value)
            .expect("rc file entries are validated when the file is read");
    }
}

/// The default user style library directories (matplotlib's
/// `USER_LIBRARY_PATHS`): `stylelib` in the [`config_dir`], or none while
/// the user configuration is ignored.
pub(crate) fn default_style_library_paths() -> Vec<PathBuf> {
    if user_config_ignored() {
        return Vec::new();
    }
    config_dir()
        .map(|dir| dir.join(STYLE_LIBRARY_DIR))
        .into_iter()
        .collect()
}

/// Why the configuration or cache directory was not used.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DirProblem {
    /// There is no home directory to find it in.
    NoHome,
    /// The directory cannot be made or written to.
    Unusable {
        /// The directory.
        dir: PathBuf,
        /// The variable that named it (`PLTRSCONFIGDIR` or
        /// `MPLCONFIGDIR`), if one did.
        variable: Option<&'static str>,
        /// Why it cannot be used.
        reason: String,
    },
}

impl DirProblem {
    /// The variable to set to a writable directory: the one that named
    /// the unusable directory, else [`CONFIG_DIR_VAR`].
    fn variable_to_set(&self) -> &'static str {
        match self {
            Self::Unusable {
                variable: Some(variable),
                ..
            } => variable,
            _ => CONFIG_DIR_VAR,
        }
    }
}

impl fmt::Display for DirProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoHome => f.write_str("resolving the home directory"),
            Self::Unusable {
                dir,
                variable,
                reason,
            } => {
                let source = variable.unwrap_or("the default path");
                write!(f, "{source} ({}): {reason}", dir.display())
            }
        }
    }
}

/// A temporary directory standing in for the configuration or cache
/// directory, and why ([`temp_dir_notice`]); it displays as the warning
/// matplotlib logs then.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TempDirNotice {
    /// The temporary directory.
    dir: PathBuf,
    /// Why the directory to use was not used.
    problem: DirProblem,
}

impl TempDirNotice {
    /// The temporary directory.
    pub fn dir(&self) -> &std::path::Path {
        &self.dir
    }
}

impl fmt::Display for TempDirNotice {
    /// matplotlib's message.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "pltrs created a temporary cache directory at {} because there was an issue \
             with {}; it is highly recommended to set the {} environment variable to a \
             writable directory, in particular to speed up the import of pltrs and to \
             better support multiprocessing.",
            self.dir.display(),
            self.problem,
            self.problem.variable_to_set()
        )
    }
}

/// No configuration or cache directory: the one to use is unusable and no
/// temporary directory can be made in its place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigError {
    /// Why the directory to use was not used.
    problem: DirProblem,
    /// Why no temporary directory could be made.
    reason: String,
}

impl fmt::Display for ConfigError {
    /// matplotlib's message.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "pltrs requires access to a writable cache directory, but there was an issue \
             with {}, and a temporary directory could not be created ({}); set the {} \
             environment variable to a writable directory",
            self.problem,
            self.reason,
            self.problem.variable_to_set()
        )
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::look::FigureLook;
    use crate::rc::RcWarningKind;

    #[test]
    fn unit_tests_ignore_the_user_configuration() {
        assert!(user_config_ignored());
        assert_eq!(user_rc_file(), Ok(None));
        assert_eq!(default_style_library_paths(), Vec::<PathBuf>::new());
        assert_eq!(with_user_rc(RcParams::matplotlib()), RcParams::matplotlib());
    }

    #[test]
    fn rc_files_apply_over_the_look_skipping_invalid_lines() {
        let file = RcFile::parse(
            "lines.linewidth: 5\n\
             no colon here\n\
             lines.color: notacolor\n\
             lines.nonsense: 1\n\
             axes.grid: True\n",
        )
        .unwrap();
        let kinds: Vec<_> = file.warnings.iter().map(|w| w.kind.clone()).collect();
        assert_eq!(kinds.len(), 3);
        assert_eq!(kinds[0], RcWarningKind::MissingColon);
        assert!(matches!(kinds[1], RcWarningKind::InvalidValue(_)));
        assert_eq!(
            kinds[2],
            RcWarningKind::UnknownKey("lines.nonsense".to_owned())
        );

        let mut params = FigureLook::Modern.params();
        apply_rc_file(&file, &mut params);
        let look = FigureLook::Modern.params();
        assert_eq!(params.lines_linewidth, 5.0);
        assert!(params.axes_grid);
        assert_eq!(params.lines_color, look.lines_color, "skipped");
        assert_eq!(params.font_sans_serif, look.font_sans_serif, "the look's");
    }

    #[test]
    fn errors_say_what_to_do() {
        let err = ConfigError {
            problem: DirProblem::Unusable {
                dir: PathBuf::from("/nowhere"),
                variable: None,
                reason: "read-only".to_owned(),
            },
            reason: "disk full".to_owned(),
        };
        assert_eq!(
            err.to_string(),
            "pltrs requires access to a writable cache directory, but there was an issue \
             with the default path (/nowhere): read-only, and a temporary directory could \
             not be created (disk full); set the PLTRSCONFIGDIR environment variable to a \
             writable directory"
        );
    }

    #[test]
    fn notices_name_the_variable_that_named_the_directory() {
        let notice = TempDirNotice {
            dir: PathBuf::from("/tmp/pltrs-1"),
            problem: DirProblem::Unusable {
                dir: PathBuf::from("/locked"),
                variable: Some(MATPLOTLIB_CONFIG_DIR_VAR),
                reason: "not a writable directory".to_owned(),
            },
        };
        assert_eq!(notice.dir(), std::path::Path::new("/tmp/pltrs-1"));
        assert_eq!(
            notice.to_string(),
            "pltrs created a temporary cache directory at /tmp/pltrs-1 because there was an \
             issue with MPLCONFIGDIR (/locked): not a writable directory; it is highly \
             recommended to set the MPLCONFIGDIR environment variable to a writable \
             directory, in particular to speed up the import of pltrs and to better support \
             multiprocessing."
        );
        let homeless = TempDirNotice {
            dir: PathBuf::from("/tmp/pltrs-2"),
            problem: DirProblem::NoHome,
        };
        assert!(
            homeless
                .to_string()
                .contains("set the PLTRSCONFIGDIR environment")
        );
    }
}
