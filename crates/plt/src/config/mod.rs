//! The user's configuration: the configuration and cache directories, the
//! rc file and the style library in the configuration directory
//! (matplotlib's `get_configdir`, `get_cachedir`, `matplotlib_fname` and
//! `matplotlibrc`).
//!
//! | what                        | where plt-rs looks, in order                                   | matplotlib's counterpart |
//! |-----------------------------|----------------------------------------------------------------|--------------------------|
//! | [`config_dir`]              | `$PLTCONFIGDIR`, `$XDG_CONFIG_HOME/plt-rs`, `~/.config/plt-rs` | `MPLCONFIGDIR`, `~/.config/matplotlib` |
//! | [`cache_dir`]               | `$PLTCONFIGDIR`, `$XDG_CACHE_HOME/plt-rs`, `~/.cache/plt-rs`   | `MPLCONFIGDIR`, `~/.cache/matplotlib` |
//! | [`rc_file_path`]            | `./pltrc`, `$PLTRC` (a file, or a directory holding `pltrc`), `pltrc` in the configuration directory | `matplotlibrc`, `MATPLOTLIBRC` |
//! | the user style library      | the `*.mplstyle` files of `stylelib` in the configuration directory | `stylelib`         |
//!
//! Like matplotlib, plt-rs makes the configuration and cache directories
//! when they are missing; when one cannot be made or written to, a new
//! private directory in the temporary directory takes its place for the
//! rest of the process. Empty variables count as unset.
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
//! use plt::config;
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
//! tests ignore them from the start. Within the plt-rs workspace, cargo
//! sets `PLTCONFIGDIR` to an empty directory under `target/` unless it is
//! set already, so doctests and examples see no configuration from the
//! home directory either.

mod lookup;

use std::fmt;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use lookup::{DirKind, Environment};

use crate::rc::{RcError, RcFile, RcParams};

/// The environment variable naming the configuration and cache directory
/// (matplotlib's `MPLCONFIGDIR`).
pub const CONFIG_DIR_VAR: &str = "PLTCONFIGDIR";

/// The environment variable naming the rc file, or a directory holding
/// one (matplotlib's `MATPLOTLIBRC`).
pub const RC_FILE_VAR: &str = "PLTRC";

/// The file name of the rc file (matplotlib's `matplotlibrc`).
pub const RC_FILE_NAME: &str = "pltrc";

/// The directory of user style sheets in the configuration directory.
const STYLE_LIBRARY_DIR: &str = "stylelib";

/// Whether the process ignores the user configuration: from the start in
/// this crate's unit tests, which must not depend on the developer's.
static IGNORED: AtomicBool = AtomicBool::new(cfg!(test));

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
/// `PLTCONFIGDIR` names, else `plt-rs` in `$XDG_CONFIG_HOME` (by default
/// `~/.config`). It is made if missing; when it cannot be made or written
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

/// The cache directory (matplotlib's `get_cachedir`), looked up as
/// [`config_dir`] is with `$XDG_CACHE_HOME` (by default `~/.cache`) in
/// place of `$XDG_CONFIG_HOME`. plt-rs caches nothing there itself.
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
/// made the first time one of them needs it (matplotlib then points
/// `MPLCONFIGDIR` at it).
fn temp_dir() -> Result<PathBuf, String> {
    TEMP_DIR
        .get_or_init(|| {
            lookup::make_temp_dir(&Environment::process()).map_err(|err| err.to_string())
        })
        .clone()
}

/// The rc file to start from (matplotlib's `matplotlib_fname`): the first
/// of `pltrc` in the working directory, the file `$PLTRC` names, `pltrc`
/// in the directory `$PLTRC` names and `pltrc` in the [`config_dir`]
/// that exists and is not a directory. `None` when there is none; plt-rs'
/// defaults are built in rather than read from a file.
pub fn rc_file_path() -> Option<PathBuf> {
    lookup::find_rc_file(&Environment::process(), || config_dir().ok())
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
        /// Whether `PLTCONFIGDIR` named it.
        from_variable: bool,
        /// Why it cannot be used.
        reason: String,
    },
}

impl fmt::Display for DirProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoHome => f.write_str("resolving the home directory"),
            Self::Unusable {
                dir,
                from_variable,
                reason,
            } => {
                let source = if *from_variable {
                    CONFIG_DIR_VAR
                } else {
                    "the default path"
                };
                write!(f, "{source} ({}): {reason}", dir.display())
            }
        }
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
            "plt-rs requires access to a writable cache directory, but there was an issue \
             with {}, and a temporary directory could not be created ({}); set the \
             {CONFIG_DIR_VAR} environment variable to a writable directory",
            self.problem, self.reason
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
                from_variable: false,
                reason: "read-only".to_owned(),
            },
            reason: "disk full".to_owned(),
        };
        assert_eq!(
            err.to_string(),
            "plt-rs requires access to a writable cache directory, but there was an issue \
             with the default path (/nowhere): read-only, and a temporary directory could \
             not be created (disk full); set the PLTCONFIGDIR environment variable to a \
             writable directory"
        );
    }
}
