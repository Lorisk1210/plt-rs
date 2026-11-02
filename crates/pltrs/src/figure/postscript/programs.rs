//! The external programs the PostScript backend runs, run as matplotlib
//! runs them: `cbook._check_and_log_subprocess` (the program's output, or
//! an error quoting the command and everything it printed),
//! `mpl._get_executable_info` for the programs the distillers check
//! first, and a temporary directory for their files, removed afterwards.

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io::{self, Read as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use pltrs_core::os::Os;
use pltrs_text::tex::{TemporaryDirectory, format_command};

/// Why a program the PostScript backend runs did not do its work, with
/// matplotlib's messages.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProgramError {
    /// The program is missing, its version unreadable or too old
    /// (`mpl._get_executable_info`'s `ExecutableNotFoundError`).
    NotFound(String),
    /// The program could not be started (the `OSError` of
    /// `subprocess.run`).
    Start {
        /// The program.
        program: String,
        /// Python's message, such as `[Errno 2] No such file or directory:
        /// 'dvips'`.
        message: String,
    },
    /// The program exited with an error (`_check_and_log_subprocess`'s
    /// `RuntimeError`).
    Failed {
        /// The command line, quoted as a shell reads it.
        command: String,
        /// What the program wrote to its standard output.
        stdout: String,
        /// What it wrote to its standard error.
        stderr: String,
    },
    /// A page the size of the figure was asked for without the figure's
    /// bounding box.
    MissingBbox,
    /// A file in the temporary directory could not be written, read or
    /// moved.
    Io {
        /// The file or directory.
        path: String,
        /// What went wrong.
        message: String,
    },
}

impl ProgramError {
    /// An I/O error on `path`.
    pub(crate) fn io(path: &Path, err: &io::Error) -> Self {
        Self::Io {
            path: path.display().to_string(),
            message: err.to_string(),
        }
    }

    /// `program` failing to start with `err`.
    fn start(program: &str, err: &io::Error) -> Self {
        Self::Start {
            program: program.to_owned(),
            message: os_error(program, err),
        }
    }
}

/// Python's message of the `OSError` of `program` failing to start with
/// `err`: `[Errno <n>] <reason>: '<program>'`.
fn os_error(program: &str, err: &io::Error) -> String {
    let text = err.to_string();
    match err.raw_os_error() {
        Some(code) => {
            let reason = text
                .strip_suffix(&format!(" (os error {code})"))
                .unwrap_or(&text);
            format!("[Errno {code}] {reason}: '{program}'")
        }
        None => text,
    }
}

impl fmt::Display for ProgramError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(message) | Self::Start { message, .. } => f.write_str(message),
            Self::Failed {
                command,
                stdout,
                stderr,
            } => write!(
                f,
                "The command\n    {command}\nfailed and generated the following output:\n\
                 {stdout}\nand the following error:\n{stderr}"
            ),
            Self::MissingBbox => {
                f.write_str("a page the size of the figure needs the figure's bounding box")
            }
            Self::Io { path, message } => write!(f, "PostScript temporary file {path}: {message}"),
        }
    }
}

impl std::error::Error for ProgramError {}

/// A version as `packaging` reads the plain ones programs print
/// (`10.05.1` is 10.5.1).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Version(Vec<u64>);

impl Version {
    /// Dot-separated numbers, or `None`.
    fn parse(text: &str) -> Option<Self> {
        text.trim()
            .split('.')
            .map(|part| part.parse().ok())
            .collect::<Option<Vec<u64>>>()
            .map(Self)
    }

    /// The `index`-th number (0 when there are fewer).
    fn part(&self, index: usize) -> u64 {
        self.0.get(index).copied().unwrap_or(0)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = self.0.iter().map(u64::to_string).collect();
        f.write_str(&parts.join("."))
    }
}

/// A program checked before anything runs it (`mpl._get_executable_info`):
/// the distillers' and those of the image comparisons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Executable {
    /// Ghostscript, at least version 9.
    Ghostscript,
    /// poppler's (or xpdf's) `pdftops`, at least version 3.0 (or 0.9).
    Pdftops,
    /// Inkscape, which rasterises SVG files for comparisons.
    Inkscape,
}

impl Executable {
    /// The names the program goes by on `os`, tried in order: Ghostscript
    /// is `gs`, and on Windows first `gswin32c`, `gswin64c` and `MiKTeX`'s
    /// `mgs`.
    pub(crate) fn names(self, os: Os) -> &'static [&'static str] {
        match (self, os) {
            (Self::Ghostscript, Os::Windows) => &["gswin32c", "gswin64c", "mgs", "gs"],
            (Self::Ghostscript, Os::Linux | Os::MacOs) => &["gs"],
            (Self::Pdftops, _) => &["pdftops"],
            (Self::Inkscape, _) => &["inkscape"],
        }
    }

    /// The program to run, once found to run here as `_get_executable_info`
    /// finds out: the first of its [names](Executable::names) that prints
    /// a version recent enough, when run with `--version` (Ghostscript),
    /// `-v` (`pdftops`) or `-V` (Inkscape).
    ///
    /// # Errors
    ///
    /// matplotlib's `ExecutableNotFoundError`: for Ghostscript always
    /// "Failed to find a Ghostscript installation"; for the others why.
    pub(crate) fn check(self) -> Result<&'static str, ProgramError> {
        self.found().map(|(program, _)| program)
    }

    /// The version of the program, as `packaging` writes it (`gs` 10.05.1
    /// is `10.5.1`), which matplotlib's comparisons hash with the files
    /// the program converts.
    ///
    /// # Errors
    ///
    /// As [`check`](Self::check).
    pub(crate) fn version(self) -> Result<String, ProgramError> {
        self.found().map(|(_, version)| version)
    }

    /// The program to run and its version.
    fn found(self) -> Result<(&'static str, String), ProgramError> {
        match self {
            Self::Ghostscript => self
                .names(Os::CURRENT)
                .iter()
                .copied()
                .find_map(|program| {
                    printed(program, &["--version"])
                        .ok()
                        .filter(|(success, _)| *success)
                        .and_then(|(_, output)| Version::parse(output.lines().next().unwrap_or("")))
                        .filter(|version| version.part(0) >= 9)
                        .map(|version| (program, version.to_string()))
                })
                .ok_or_else(|| {
                    ProgramError::NotFound("Failed to find a Ghostscript installation".to_owned())
                }),
            Self::Inkscape => {
                // `--without-gui` first, which Inkscape before 1.0 needs.
                let (_, output) = printed("inkscape", &["--without-gui", "-V"])
                    .ok()
                    .filter(|(success, _)| *success)
                    .map_or_else(|| printed("inkscape", &["-V"]), Ok)?;
                let token = output
                    .split("Inkscape ")
                    .nth(1)
                    .and_then(|rest| rest.split([' ', '\n']).next())
                    .ok_or_else(|| {
                        ProgramError::NotFound(format!(
                            "Failed to determine the version of inkscape from inkscape -V, \
                             which output {output}"
                        ))
                    })?;
                let version =
                    Version::parse(token).map_or_else(|| token.to_owned(), |v| v.to_string());
                Ok(("inkscape", version))
            }
            Self::Pdftops => {
                let (_, output) = printed("pdftops", &["-v"])?;
                let version = output
                    .strip_prefix("pdftops version ")
                    .and_then(|rest| Version::parse(rest.lines().next().unwrap_or("")))
                    .ok_or_else(|| {
                        ProgramError::NotFound(format!(
                            "Failed to determine the version of pdftops from pdftops -v, \
                             which output {output}"
                        ))
                    })?;
                let poppler = version.part(0) == 0 && version.part(1) >= 9;
                if version.part(0) >= 3 || poppler {
                    Ok(("pdftops", version.to_string()))
                } else {
                    Err(ProgramError::NotFound(format!(
                        "You have pdftops version {version} but the minimum version supported \
                         by Matplotlib is 3.0"
                    )))
                }
            }
        }
    }
}

/// How long a program may take to print its version (`timeout=30`).
const VERSION_TIMEOUT: Duration = Duration::from_secs(30);

/// How often a program printing its version is looked in on.
const VERSION_POLL: Duration = Duration::from_millis(5);

/// What `program` prints when run with `args`, its errors after its
/// output (the programs checked write one or the other), and whether it
/// succeeded.
///
/// # Errors
///
/// `ExecutableNotFoundError` with Python's message when it cannot be
/// started, or when it runs longer than [`VERSION_TIMEOUT`] (killed).
fn printed(program: &str, args: &[&str]) -> Result<(bool, String), ProgramError> {
    let not_found = |err: &io::Error| ProgramError::NotFound(os_error(program, err));
    let mut child = crate::process::spawn(
        Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()),
    )
    .map_err(|err| not_found(&err))?;
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|err| not_found(&err))? {
            break status;
        }
        if started.elapsed() > VERSION_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ProgramError::NotFound(format!(
                "Timed out running {program} {}",
                args.join(" ")
            )));
        }
        std::thread::sleep(VERSION_POLL);
    };
    let mut output = Vec::new();
    if let Some(mut stdout) = child.stdout.take() {
        stdout
            .read_to_end(&mut output)
            .map_err(|err| not_found(&err))?;
    }
    if let Some(mut stderr) = child.stderr.take() {
        stderr
            .read_to_end(&mut output)
            .map_err(|err| not_found(&err))?;
    }
    Ok((
        status.success(),
        String::from_utf8_lossy(&output).into_owned(),
    ))
}

/// A temporary directory for the files of a PostScript document, in the
/// system's temporary directory (Python's `TemporaryDirectory()`), where
/// the programs run with the environment the document asks for.
pub(crate) struct Workspace {
    directory: TemporaryDirectory,
    /// `SOURCE_DATE_EPOCH` for the programs.
    source_date: i64,
}

impl Workspace {
    /// A fresh directory; the programs run in it date what they write
    /// `source_date` (seconds since the epoch, as `SOURCE_DATE_EPOCH`).
    pub(crate) fn new(source_date: i64) -> Result<Self, ProgramError> {
        let parent = std::env::temp_dir();
        let directory =
            TemporaryDirectory::new(&parent).map_err(|err| ProgramError::io(&parent, &err))?;
        Ok(Self {
            directory,
            source_date,
        })
    }

    /// The file `name` in the directory.
    pub(crate) fn path(&self, name: &str) -> PathBuf {
        self.directory.path().join(name)
    }

    /// A fresh directory inside this one (another `TemporaryDirectory()`
    /// of matplotlib's), removed when dropped.
    pub(crate) fn subdirectory(&self) -> Result<TemporaryDirectory, ProgramError> {
        TemporaryDirectory::new(self.directory.path())
            .map_err(|err| ProgramError::io(self.directory.path(), &err))
    }

    /// `_check_and_log_subprocess`: runs `program` with `args` in the
    /// directory and returns what it wrote to its standard output.
    ///
    /// # Errors
    ///
    /// When it cannot be started, or exits with an error (quoting the
    /// command and everything it printed).
    pub(crate) fn run(&self, program: &str, args: &[OsString]) -> Result<Vec<u8>, ProgramError> {
        let output = crate::process::output(
            Command::new(program)
                .args(args)
                .current_dir(self.directory.path())
                .env("SOURCE_DATE_EPOCH", self.source_date.to_string())
                .stdin(Stdio::null()),
        )
        .map_err(|err| ProgramError::start(program, &err))?;
        if output.status.success() {
            return Ok(output.stdout);
        }
        let args: Vec<&OsStr> = args.iter().map(OsString::as_os_str).collect();
        Err(ProgramError::Failed {
            command: format_command(program, &args),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

/// `prefix` followed by `path`, as one argument.
pub(crate) fn path_argument(prefix: &str, path: &Path) -> OsString {
    let mut argument = OsString::from(prefix);
    argument.push(path);
    argument
}

/// Reads `path`.
pub(crate) fn read(path: &Path) -> Result<Vec<u8>, ProgramError> {
    std::fs::read(path).map_err(|err| ProgramError::io(path, &err))
}

/// Writes `data` to `path`.
pub(crate) fn write(path: &Path, data: &[u8]) -> Result<(), ProgramError> {
    std::fs::write(path, data).map_err(|err| ProgramError::io(path, &err))
}

/// Moves `from` to `to` (`shutil.move`), replacing it.
pub(crate) fn replace(from: &Path, to: &Path) -> Result<(), ProgramError> {
    std::fs::rename(from, to).map_err(|err| ProgramError::io(from, &err))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_read_like_packaging() {
        let version = Version::parse("10.05.1\n").unwrap();
        assert_eq!(version.to_string(), "10.5.1");
        assert!(version.part(0) >= 9 && version.part(3) == 0);
        assert!(Version::parse("0.9").unwrap() < Version::parse("0.10").unwrap());
        assert_eq!(Version::parse("10.05.1 (2025)"), None);
        assert_eq!(Version::parse(""), None);
    }

    #[test]
    fn errors_carry_matplotlibs_messages() {
        let missing = io::Error::from_raw_os_error(2);
        assert_eq!(
            ProgramError::start("dvips", &missing).to_string(),
            "[Errno 2] No such file or directory: 'dvips'"
        );
        let failed = ProgramError::Failed {
            command: "gs -dBATCH 'a b'".to_owned(),
            stdout: "out".to_owned(),
            stderr: "err".to_owned(),
        };
        assert_eq!(
            failed.to_string(),
            "The command\n    gs -dBATCH 'a b'\nfailed and generated the following output:\n\
             out\nand the following error:\nerr"
        );
        assert_eq!(
            ProgramError::NotFound("Failed to find a Ghostscript installation".to_owned())
                .to_string(),
            "Failed to find a Ghostscript installation"
        );
    }

    #[test]
    fn workspaces_run_programs_in_their_directory() {
        let workspace = Workspace::new(0).unwrap();
        let directory = workspace.path("");
        assert!(directory.is_dir());
        let missing = workspace.run("pltrs-no-such-program", &[]);
        assert!(
            matches!(&missing, Err(ProgramError::Start { message, .. })
                if message == "[Errno 2] No such file or directory: 'pltrs-no-such-program'"),
            "{missing:?}"
        );
        if pltrs_core::os::which("sh").is_some() {
            let script = |text: &str| [OsString::from("-c"), OsString::from(text)];
            let printed = workspace.run("sh", &script("pwd; echo $SOURCE_DATE_EPOCH"));
            let expected = format!("{}\n0\n", directory.canonicalize().unwrap().display());
            assert_eq!(printed, Ok(expected.into_bytes()));
            let failed = workspace.run("sh", &script("echo out; echo err >&2; exit 3"));
            assert_eq!(
                failed,
                Err(ProgramError::Failed {
                    command: "sh -c 'echo out; echo err >&2; exit 3'".to_owned(),
                    stdout: "out\n".to_owned(),
                    stderr: "err\n".to_owned(),
                })
            );
        }
        let inner = workspace.subdirectory().unwrap();
        assert!(inner.path().starts_with(&directory));
        drop(workspace);
        assert!(!directory.exists(), "the directory is removed");
    }

    #[test]
    fn ghostscript_has_more_names_on_windows() {
        assert_eq!(
            Executable::Ghostscript.names(Os::Windows),
            ["gswin32c", "gswin64c", "mgs", "gs"]
        );
        for os in [Os::Linux, Os::MacOs] {
            assert_eq!(Executable::Ghostscript.names(os), ["gs"]);
        }
        assert_eq!(Executable::Pdftops.names(Os::Windows), ["pdftops"]);
    }

    #[test]
    fn executables_are_found_or_named() {
        match Executable::Ghostscript.check() {
            Ok(program) => assert!(pltrs_core::os::which(program).is_some()),
            Err(err) => assert_eq!(err.to_string(), "Failed to find a Ghostscript installation"),
        }
        if let Err(err) = Executable::Pdftops.check() {
            let message = err.to_string();
            assert!(
                message == "[Errno 2] No such file or directory: 'pdftops'"
                    || message.starts_with("Failed to determine the version of pdftops")
                    || message.starts_with("You have pdftops version"),
                "{message}"
            );
        }
    }
}
