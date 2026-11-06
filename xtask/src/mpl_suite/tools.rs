//! The programs, fonts and TeX packages matplotlib's suite looks for
//! beyond Python. A test needing one that is missing skips, in both modes
//! alike, so a run works without them; it names what it lacks.
//!
//! `python/tools/suite-tools.sh install` installs those that only the
//! suite needs: Inkscape (found on `PATH`, like Ghostscript and TeX), and
//! fonts and TeX packages in directories of their own, which the suite's
//! runs get from the variables of `<prefix>/environment` (`XDG_DATA_HOME`,
//! `TEXMFHOME`, ...): nothing else sees them.

use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The variable naming where `suite-tools.sh` installs (as it reads it).
const PREFIX_VARIABLE: &str = "SUITE_TOOLS_PREFIX";

/// The programs the suite runs, and what skips without each.
const PROGRAMS: [(&str, &str); 4] = [
    ("gs", "PDF and EPS image comparisons"),
    ("inkscape", "SVG image comparisons"),
    ("latex", "the usetex tests"),
    ("xelatex", "the PGF backend's tests"),
];

/// The suite tools' installation directory: `SUITE_TOOLS_PREFIX`, else
/// `~/.local/mpl-suite`.
fn prefix() -> Option<PathBuf> {
    env::var_os(PREFIX_VARIABLE)
        .map(PathBuf::from)
        .or_else(|| env::home_dir().map(|home| home.join(".local/mpl-suite")))
}

/// The variables giving the suite's runs their fonts and TeX packages,
/// read from `<prefix>/environment` (one `NAME=VALUE` a line); none when
/// the tools are not installed.
pub(crate) fn environment() -> Result<Vec<(String, OsString)>, String> {
    let Some(file) = prefix().map(|prefix| prefix.join("environment")) else {
        return Ok(Vec::new());
    };
    match std::fs::read_to_string(&file) {
        Ok(text) => parse_environment(&text).map_err(|err| format!("{}: {err}", file.display())),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(err) => Err(format!("{}: {err}", file.display())),
    }
}

/// The `NAME=VALUE` lines of `text`, blank lines left out.
fn parse_environment(text: &str) -> Result<Vec<(String, OsString)>, String> {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| match line.split_once('=') {
            Some((name, value)) if !name.is_empty() => Ok((name.to_owned(), OsString::from(value))),
            _ => Err(format!("not a NAME=VALUE line: {line:?}")),
        })
        .collect()
}

/// Prints what a run with `environment` lacks, and what skips for it.
pub(crate) fn report_missing(environment: &[(String, OsString)]) {
    let path = env::var_os("PATH").unwrap_or_default();
    let dirs: Vec<PathBuf> = env::split_paths(&path).collect();
    for (program, needs) in PROGRAMS {
        if !on_path(&dirs, program) {
            println!("no {program} on PATH: {needs} skip");
        }
    }
    if environment.is_empty() {
        println!(
            "no suite tools (python/tools/suite-tools.sh install): the tests needing \
             their fonts and TeX packages skip"
        );
    }
}

/// Whether one of `dirs` holds the program `name`.
fn on_path(dirs: &[PathBuf], name: &str) -> bool {
    let file = format!("{name}{}", env::consts::EXE_SUFFIX);
    dirs.iter().any(|dir| is_file(&dir.join(&file)))
}

/// Whether `path` is a file (or a link to one).
fn is_file(path: &Path) -> bool {
    path.metadata().is_ok_and(|metadata| metadata.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_files_hold_one_variable_a_line() {
        let parsed =
            parse_environment("XDG_DATA_HOME=/s/share\n\nTEXMFHOME=/s/texmf=x\n").expect("valid");
        assert_eq!(
            parsed,
            [
                ("XDG_DATA_HOME".to_owned(), OsString::from("/s/share")),
                ("TEXMFHOME".to_owned(), OsString::from("/s/texmf=x")),
            ]
        );
    }

    #[test]
    fn environment_files_reject_other_lines() {
        assert!(parse_environment("XDG_DATA_HOME\n").is_err());
        assert!(parse_environment("=/s\n").is_err());
        assert_eq!(parse_environment(""), Ok(Vec::new()));
    }

    #[test]
    fn programs_are_found_in_the_path_directories() {
        let exe = env::current_exe().expect("the test binary");
        let dir = exe.parent().expect("its directory").to_path_buf();
        let name = exe.file_stem().expect("its name").to_string_lossy();
        let dirs = [PathBuf::from("/nonexistent"), dir];
        assert!(on_path(&dirs, &name));
        assert!(!on_path(&dirs, "no-such-program"));
    }
}
