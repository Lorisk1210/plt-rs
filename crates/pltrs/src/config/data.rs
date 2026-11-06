//! The data directory (matplotlib's `get_data_path`, its `mpl-data`):
//! the files pltrs bundles in its binary, written out where code needs
//! them as files.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::{fmt, fs, io};

use pltrs_text::AfmDirectory;

use super::{ConfigError, RC_FILE_NAME, cache_dir};
use crate::rc::RcParams;

/// The data directory, once written.
static DATA_DIR: OnceLock<Result<PathBuf, DataDirError>> = OnceLock::new();

/// Why there is no data directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataDirError {
    /// There is no cache directory to hold it.
    Cache(ConfigError),
    /// Its files could not be written.
    Write {
        /// The data directory.
        dir: PathBuf,
        /// What went wrong.
        reason: String,
    },
}

impl fmt::Display for DataDirError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cache(err) => err.fmt(f),
            Self::Write { dir, reason } => {
                write!(
                    f,
                    "cannot write pltrs' data files to {}: {reason}",
                    dir.display()
                )
            }
        }
    }
}

impl std::error::Error for DataDirError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Cache(err) => Some(err),
            Self::Write { .. } => None,
        }
    }
}

/// The data directory (matplotlib's `get_data_path`): `mpl-data` in the
/// [cache directory](cache_dir), holding matplotlib's fonts as its
/// `mpl-data` does (`fonts/ttf`, and the AFM metrics of the PDF core
/// fonts in `fonts/pdfcorefonts` and of the standard PostScript fonts in
/// `fonts/afm`), the bundled style sheets (`stylelib`),
/// and the template of the rc file (`pltrsrc`, matplotlib's
/// `mpl-data/matplotlibrc`: every parameter at matplotlib's default,
/// commented out). The files are written from the binary the first time
/// the directory is asked for in a process, unless they are there already.
///
/// ```
/// let dir = pltrs::config::data_dir()?;
/// assert!(dir.join("fonts/ttf/DejaVuSans.ttf").is_file());
/// assert!(dir.join("fonts/pdfcorefonts/Helvetica.afm").is_file());
/// assert!(dir.join("fonts/afm/pagd8a.afm").is_file());
/// assert!(dir.join("stylelib/ggplot.mplstyle").is_file());
/// assert!(dir.join("pltrsrc").is_file());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// When the cache directory cannot be found or the files cannot be
/// written.
pub fn data_dir() -> Result<PathBuf, DataDirError> {
    DATA_DIR
        .get_or_init(|| {
            let dir = cache_dir().map_err(DataDirError::Cache)?.join("mpl-data");
            let fonts = dir.join("fonts");
            let template = rc_template();
            write_files(&fonts.join("ttf"), pltrs_text::bundled_font_files())
                .and_then(|()| {
                    AfmDirectory::ALL.into_iter().try_for_each(|directory| {
                        write_files(
                            &fonts.join(directory.name()),
                            pltrs_text::bundled_afm_files(directory),
                        )
                    })
                })
                .and_then(|()| write_files(&dir.join("stylelib"), crate::style::bundled_files()))
                .and_then(|()| {
                    write_files(&dir, std::iter::once((RC_FILE_NAME, template.as_bytes())))
                })
                .map_err(|err| DataDirError::Write {
                    dir: dir.clone(),
                    reason: err.to_string(),
                })?;
            Ok(dir)
        })
        .clone()
}

/// The template of the rc file: every parameter at matplotlib's default,
/// commented out, as matplotlib's `matplotlibrc` lists them.
fn rc_template() -> String {
    let defaults = RcParams::matplotlib();
    let mut text = String::from(
        "## pltrs' rc file template: every parameter at matplotlib's default.\n\
         ## Copy it to the configuration directory and uncomment what to change.\n",
    );
    for key in RcParams::KEYS {
        let value = defaults
            .get(key)
            .expect("every key of the parameter table has a value");
        // Writing into a `String` cannot fail.
        let _ = writeln!(text, "#{key}: {value}");
    }
    text
}

/// Writes each `(name, contents)` of `files` into `dir` unless a file of
/// that name and size is there. Each file is written under a name of its
/// own and renamed into place, so that processes writing the directory at
/// once never see a partial file.
fn write_files<'a>(dir: &Path, files: impl Iterator<Item = (&'a str, &'a [u8])>) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    for (name, contents) in files {
        let path = dir.join(name);
        let present = fs::metadata(&path).is_ok_and(|meta| meta.len() == contents.len() as u64);
        if !present {
            let partial = dir.join(format!(".{name}.{}", std::process::id()));
            fs::write(&partial, contents)?;
            fs::rename(&partial, &path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_are_written_once_and_kept() {
        let dir = std::env::temp_dir().join(format!("pltrs-data-test-{}", std::process::id()));
        let files = [
            ("a.txt", b"alpha".as_slice()),
            ("b.txt", b"beta".as_slice()),
        ];
        write_files(&dir, files.into_iter()).expect("writable");
        assert_eq!(fs::read(dir.join("a.txt")).expect("written"), b"alpha");
        // A file of the right size is kept as it is.
        fs::write(dir.join("b.txt"), b"BETA").expect("writable");
        write_files(&dir, files.into_iter()).expect("writable");
        assert_eq!(fs::read(dir.join("b.txt")).expect("kept"), b"BETA");
        fs::remove_dir_all(&dir).expect("removable");
    }

    #[test]
    fn the_rc_template_lists_every_parameter_commented_out() {
        let template = rc_template();
        let entries = crate::rc::parse_rc_text(&template).expect("a valid rc file");
        assert_eq!(entries.len(), 0);
        assert!(template.contains("\n#lines.linewidth: 1.5\n"));
        let lines = template.lines().filter(|line| !line.starts_with("##"));
        assert_eq!(lines.count(), RcParams::KEYS.len());
    }

    #[test]
    fn the_data_directory_holds_the_bundled_fonts() {
        let dir = data_dir().expect("a cache directory");
        for (name, contents) in pltrs_text::bundled_font_files() {
            let written = fs::read(dir.join("fonts/ttf").join(name)).expect("written");
            assert_eq!(written, contents, "{name}");
        }
    }
}
