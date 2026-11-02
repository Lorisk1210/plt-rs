//! The operating systems pltrs runs on, and the conventions that differ
//! between them: which files on `PATH` run a program, how programs are
//! started, how a resolved path is spelled, and where `~` leads.
//!
//! Each rule is a method of [`Os`], taking what it reads from the process
//! as arguments, so that every system's rules are tested on any of them;
//! [`Os::CURRENT`] is the system the program was built for, and
//! [`which`], [`resolve`] and [`expand_user`] apply its rules to the
//! running process.

use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A family of operating systems, as matplotlib tells them apart
/// (`sys.platform`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Os {
    /// Linux, and the other Unix-like systems but macOS: the
    /// freedesktop.org conventions (XDG base directories, X11 font
    /// directories).
    Linux,
    /// macOS.
    MacOs,
    /// Windows.
    Windows,
}

/// The extensions Windows runs a program by when `PATHEXT` is unset
/// (Python's `shutil._WIN_DEFAULT_PATHEXT`).
const DEFAULT_PATHEXT: &str = ".COM;.EXE;.BAT;.CMD;.VBS;.JS;.WS;.MSC";

impl Os {
    /// The system pltrs was built for.
    pub const CURRENT: Self = if cfg!(windows) {
        Self::Windows
    } else if cfg!(target_os = "macos") {
        Self::MacOs
    } else {
        Self::Linux
    };

    /// The file names `shutil.which` looks for in each directory of `PATH`
    /// to run `program`: the name itself, or on Windows the name with each
    /// extension of `pathext` (the `PATHEXT` variable, `;`-separated, by
    /// default `.COM;.EXE;.BAT;.CMD;...`) appended, preceded by the name
    /// itself when it already ends with one of them.
    pub fn executable_names(self, program: &str, pathext: Option<&str>) -> Vec<String> {
        if self != Self::Windows {
            return vec![program.to_owned()];
        }
        let extensions: Vec<&str> = pathext
            .filter(|value| !value.is_empty())
            .unwrap_or(DEFAULT_PATHEXT)
            .split(';')
            .map(|ext| ext.trim_end_matches('.'))
            .filter(|ext| !ext.is_empty())
            .collect();
        let upper = program.to_uppercase();
        let has_extension = extensions
            .iter()
            .any(|ext| upper.ends_with(&ext.to_uppercase()));
        has_extension
            .then(|| program.to_owned())
            .into_iter()
            .chain(extensions.iter().map(|ext| format!("{program}{ext}")))
            .collect()
    }

    /// The first file that runs `program` (`shutil.which`): the first of
    /// its [`Os::executable_names`] for which `is_executable` holds, in the
    /// directory `program` names when it has one, else in the directories
    /// `path` lists. (Unlike Python on Windows, the working directory is
    /// not searched first.)
    pub fn find_executable(
        self,
        program: &str,
        path: impl IntoIterator<Item = PathBuf>,
        pathext: Option<&str>,
        is_executable: impl Fn(&Path) -> bool,
    ) -> Option<PathBuf> {
        let given = Path::new(program);
        let (dirs, name): (Vec<PathBuf>, &str) =
            match given.parent().filter(|dir| !dir.as_os_str().is_empty()) {
                Some(dir) => (vec![dir.to_owned()], given.file_name()?.to_str()?),
                None => (path.into_iter().collect(), program),
            };
        let names = self.executable_names(name, pathext);
        dirs.iter()
            .flat_map(|dir| names.iter().map(move |name| dir.join(name)))
            .find(|candidate| is_executable(candidate))
    }

    /// `path` as Python's `Path.resolve()` spells it: on Windows without
    /// the verbatim prefix that [`std::fs::canonicalize`] adds
    /// (`\\?\C:\x` is `C:\x`, `\\?\UNC\host\share` is `\\host\share`),
    /// unchanged elsewhere and when the prefix is needed (other verbatim
    /// paths).
    pub fn without_verbatim_prefix(self, path: PathBuf) -> PathBuf {
        if self != Self::Windows {
            return path;
        }
        let Some(text) = path.to_str() else {
            return path;
        };
        if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{rest}"));
        }
        match text.strip_prefix(r"\\?\") {
            Some(rest) if is_drive_path(rest) => PathBuf::from(rest),
            _ => path,
        }
    }

    /// `path` with forward slashes (Python's `PurePath.as_posix()`), as
    /// programs such as TeX take paths on any system: on Windows its
    /// backslashes become slashes.
    pub fn as_posix(self, path: &Path) -> String {
        let text = path.to_string_lossy();
        match self {
            Self::Windows => text.replace('\\', "/"),
            Self::Linux | Self::MacOs => text.into_owned(),
        }
    }
}

/// Whether `text` starts with a drive and a root, as `C:\` does.
fn is_drive_path(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'\\'
}

/// `shutil.which(program)` in this process: the file on `PATH` that runs
/// `program` (see [`Os::find_executable`]), `None` when there is none.
pub fn which(program: impl AsRef<OsStr>) -> Option<PathBuf> {
    let program = program.as_ref().to_str()?;
    let path = std::env::var_os("PATH").unwrap_or_default();
    let pathext = std::env::var_os("PATHEXT").map(OsString::into_string);
    let pathext = pathext.as_ref().and_then(|value| value.as_deref().ok());
    Os::CURRENT.find_executable(
        program,
        std::env::split_paths(&path).filter(|dir| !dir.as_os_str().is_empty()),
        pathext,
        is_executable,
    )
}

/// Whether `path` is a file the system runs: on Unix with an execute
/// permission bit set.
fn is_executable(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|meta| meta.is_file() && has_execute_permission(&meta))
}

#[cfg(unix)]
fn has_execute_permission(meta: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    meta.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn has_execute_permission(_meta: &std::fs::Metadata) -> bool {
    true
}

/// `command` set to open no console window on Windows (Python's
/// `subprocess.CREATE_NO_WINDOW`), which a program without one (a GUI
/// program) would otherwise show for each console program it starts;
/// unchanged elsewhere.
pub fn hide_console(command: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        /// `CREATE_NO_WINDOW`.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// `path` with a leading `~` replaced by the user's home directory
/// (Python's `os.path.expanduser`: `HOME` on Unix, `USERPROFILE` on
/// Windows, as [`std::env::home_dir`] finds it); unchanged without a home
/// directory.
pub fn expand_user(path: &Path) -> PathBuf {
    match (path.strip_prefix("~"), std::env::home_dir()) {
        (Ok(rest), Some(home)) if rest.as_os_str().is_empty() => home,
        (Ok(rest), Some(home)) => home.join(rest),
        _ => path.to_owned(),
    }
}

/// `path` made absolute with its links resolved, as Python's
/// `Path.resolve(strict=True)` gives it ([`std::fs::canonicalize`],
/// spelled without Windows' verbatim prefix).
///
/// # Errors
///
/// When `path` does not exist or cannot be read.
pub fn resolve(path: &Path) -> io::Result<PathBuf> {
    std::fs::canonicalize(path).map(|resolved| Os::CURRENT.without_verbatim_prefix(resolved))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_current_system_is_the_target() {
        assert_eq!(Os::CURRENT == Os::Windows, cfg!(windows));
        assert_eq!(Os::CURRENT == Os::MacOs, cfg!(target_os = "macos"));
    }

    #[test]
    fn programs_are_their_names_off_windows() {
        for os in [Os::Linux, Os::MacOs] {
            assert_eq!(os.executable_names("gs", Some(".EXE")), ["gs"]);
        }
    }

    #[test]
    fn windows_programs_take_the_extensions_of_pathext() {
        assert_eq!(
            Os::Windows.executable_names("gswin64c", None),
            [
                "gswin64c.COM",
                "gswin64c.EXE",
                "gswin64c.BAT",
                "gswin64c.CMD",
                "gswin64c.VBS",
                "gswin64c.JS",
                "gswin64c.WS",
                "gswin64c.MSC"
            ]
        );
        assert_eq!(
            Os::Windows.executable_names("ffmpeg", Some(".exe;;.cmd.")),
            ["ffmpeg.exe", "ffmpeg.cmd"]
        );
        assert_eq!(
            Os::Windows.executable_names("ffmpeg.EXE", Some(".exe;.bat")),
            ["ffmpeg.EXE", "ffmpeg.EXE.exe", "ffmpeg.EXE.bat"],
            "a name with an extension of PATHEXT comes first"
        );
        assert_eq!(
            Os::Windows.executable_names("x", Some("")),
            Os::Windows.executable_names("x", None),
            "an empty PATHEXT is the default"
        );
    }

    #[test]
    fn programs_are_found_in_the_first_directory_holding_one() {
        let files = [PathBuf::from("/b/tex"), PathBuf::from("/c/tex")];
        let exists = |path: &Path| files.iter().any(|file| file == path);
        let dirs = || ["/a", "/b", "/c"].map(PathBuf::from);
        assert_eq!(
            Os::Linux.find_executable("tex", dirs(), None, exists),
            Some(PathBuf::from("/b/tex"))
        );
        assert_eq!(
            Os::Linux.find_executable("latex", dirs(), None, exists),
            None
        );
        assert_eq!(
            Os::Linux.find_executable("/c/tex", dirs(), None, exists),
            Some(PathBuf::from("/c/tex")),
            "a path is looked up where it says"
        );
        assert_eq!(
            Os::Linux.find_executable("/a/tex", dirs(), None, exists),
            None
        );
        assert_eq!(Os::Linux.find_executable("tex", [], None, exists), None);

        let files = [PathBuf::from("/bin/gswin64c.exe")];
        let exists = |path: &Path| files.iter().any(|file| file == path);
        assert_eq!(
            Os::Windows.find_executable(
                "gswin64c",
                dirs().map(|_| "/bin".into()),
                Some(".exe"),
                exists
            ),
            Some(PathBuf::from("/bin/gswin64c.exe"))
        );
        assert_eq!(
            Os::Linux.find_executable("gswin64c", ["/bin".into()], Some(".exe"), exists),
            None
        );
    }

    #[test]
    fn windows_paths_lose_the_verbatim_prefix() {
        let strip = |text: &str| Os::Windows.without_verbatim_prefix(PathBuf::from(text));
        assert_eq!(strip(r"\\?\C:\Users\me"), PathBuf::from(r"C:\Users\me"));
        assert_eq!(
            strip(r"\\?\UNC\host\share\x"),
            PathBuf::from(r"\\host\share\x")
        );
        assert_eq!(strip(r"\\?\Volume{1}\x"), PathBuf::from(r"\\?\Volume{1}\x"));
        assert_eq!(strip(r"C:\x"), PathBuf::from(r"C:\x"));
        assert_eq!(strip(r"\\?\C:"), PathBuf::from(r"\\?\C:"));
        assert_eq!(
            Os::Linux.without_verbatim_prefix(PathBuf::from(r"\\?\C:\x")),
            PathBuf::from(r"\\?\C:\x"),
            "only Windows has verbatim paths"
        );
    }

    #[test]
    fn posix_spellings_use_slashes() {
        let path = Path::new(r"C:\Users\me\tmp.eps");
        assert_eq!(Os::Windows.as_posix(path), "C:/Users/me/tmp.eps");
        assert_eq!(Os::Linux.as_posix(path), r"C:\Users\me\tmp.eps");
        assert_eq!(Os::MacOs.as_posix(Path::new("/tmp/a b")), "/tmp/a b");
    }

    #[test]
    fn the_process_finds_its_programs() {
        assert_eq!(which("pltrs-no-such-program"), None);
        if cfg!(unix) {
            assert!(which("sh").is_some_and(|sh| sh.ends_with("sh")));
        }
        let dir = std::env::temp_dir();
        assert_eq!(
            resolve(&dir).unwrap(),
            resolve(&resolve(&dir).unwrap()).unwrap()
        );
        assert!(resolve(&dir.join("pltrs-no-such-file")).is_err());
    }

    #[test]
    fn home_directories_expand() {
        let Some(home) = std::env::home_dir() else {
            return;
        };
        assert_eq!(expand_user(Path::new("~")), home);
        assert_eq!(expand_user(Path::new("~/a/b")), home.join("a/b"));
        assert_eq!(expand_user(Path::new("a/~")), PathBuf::from("a/~"));
        assert_eq!(expand_user(Path::new("~x")), PathBuf::from("~x"));
    }
}
