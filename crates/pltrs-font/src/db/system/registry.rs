//! The Windows registry's font entries (matplotlib's
//! `win32FontDirectory` and `_get_win32_installed_fonts`). This crate may
//! not call the registry's API (it holds no unsafe code), so it reads
//! them with the reader a program installs ([`read_registry_with`]):
//! pltrs (Rust) installs pltrs-platform's, which reads them in the
//! process, as Python's `winreg` does. Without one, it runs the system's
//! `reg query` command for each key, a new process each (some 170 ms
//! with the system's virus scanner, on every start).

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;

use pltrs_core::os;

/// Reads the string values (`REG_SZ`, and `REG_EXPAND_SZ` unexpanded) of
/// the registry key it is given, named as `reg query` names it
/// (`HKLM\SOFTWARE\...`, `HKCU\Software\...`): `(name, data)` pairs,
/// `None` when the key cannot be read.
pub type RegistryReader = fn(&str) -> Option<Vec<(String, String)>>;

/// The reader installed with [`read_registry_with`].
static READER: OnceLock<RegistryReader> = OnceLock::new();

/// Makes the font database read the Windows registry with `reader`
/// rather than with `reg query`; the first reader installed stays. pltrs
/// (Rust) installs pltrs-platform's in-process reader.
pub fn read_registry_with(reader: RegistryReader) {
    let _ = READER.set(reader);
}

/// The key naming the user's shell folders (matplotlib's `MSFolders`),
/// below `HKEY_CURRENT_USER`.
const SHELL_FOLDERS: &str =
    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Shell Folders";

/// The keys listing installed fonts (matplotlib's `MSFontDirectories`),
/// below `HKEY_LOCAL_MACHINE` and `HKEY_CURRENT_USER`.
const FONT_KEYS: [&str; 2] = [
    r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts",
    r"SOFTWARE\Microsoft\Windows\CurrentVersion\Fonts",
];

/// What the registry says of the fonts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Registry {
    /// The user's `Fonts` shell folder, the system's font directory.
    pub(crate) shell_fonts_folder: Option<PathBuf>,
    /// The font files installed for every user: names in the system's
    /// font directory, or absolute paths.
    pub(crate) machine_fonts: Vec<String>,
    /// The font files installed for the user: names in the user's font
    /// directories, or absolute paths.
    pub(crate) user_fonts: Vec<String>,
}

impl Registry {
    /// Reads the keys with the installed reader, else with `reg query`
    /// (all queries running at once); a key that cannot be read holds
    /// nothing.
    pub(crate) fn read() -> Self {
        let machine = FONT_KEYS.map(|key| format!(r"HKLM\{key}"));
        let user = FONT_KEYS.map(|key| format!(r"HKCU\{key}"));
        if let Some(reader) = READER.get() {
            let read = |key: &str| reader(key).unwrap_or_default();
            return Self::of(
                read(SHELL_FOLDERS),
                machine.iter().flat_map(|key| read(key)),
                user.iter().flat_map(|key| read(key)),
            );
        }
        let shell = query(SHELL_FOLDERS);
        let machine = machine.map(|key| query(&key));
        let user = user.map(|key| query(&key));
        Self::of(
            values(shell),
            machine.into_iter().flat_map(values),
            user.into_iter().flat_map(values),
        )
    }

    /// What the values of the shell folders and of the machine's and the
    /// user's font keys say.
    fn of(
        shell: Vec<(String, String)>,
        machine: impl Iterator<Item = (String, String)>,
        user: impl Iterator<Item = (String, String)>,
    ) -> Self {
        Self {
            shell_fonts_folder: shell
                .into_iter()
                .find(|(name, _)| name == "Fonts")
                .map(|(_, folder)| PathBuf::from(folder)),
            machine_fonts: machine.map(|(_, file)| file).collect(),
            user_fonts: user.map(|(_, file)| file).collect(),
        }
    }
}

/// Starts `reg query key`, its output piped.
fn query(key: &str) -> Option<Child> {
    let mut command = Command::new("reg");
    command
        .args(["query", key])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    os::hide_console(&mut command).spawn().ok()
}

/// The string values a query printed, once it has finished.
fn values(query: Option<Child>) -> Vec<(String, String)> {
    query
        .and_then(|child| child.wait_with_output().ok())
        .filter(|output| output.status.success())
        .map(|output| parse_values(&String::from_utf8_lossy(&output.stdout)))
        .unwrap_or_default()
}

/// The string values (`REG_SZ`, `REG_EXPAND_SZ`, unexpanded, as Python's
/// `winreg` gives them) in what `reg query` prints: a line per value,
/// indented by four spaces, with the name, the type and the data apart
/// by four spaces.
pub(crate) fn parse_values(output: &str) -> Vec<(String, String)> {
    const GAP: &str = "    ";
    output
        .lines()
        .filter_map(|line| {
            let line = line.strip_prefix(GAP)?;
            let (name, rest) = line.split_once(&format!("{GAP}REG_"))?;
            let (kind, data) = rest.split_once(GAP).unwrap_or((rest, ""));
            matches!(kind, "SZ" | "EXPAND_SZ").then(|| (name.to_owned(), data.to_owned()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_what_reg_query_prints() {
        let output = "\r\nHKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Fonts\r\n    \
                      Arial (TrueType)    REG_SZ    arial.ttf\r\n    \
                      Segoe UI Variable (TrueType)    REG_SZ    C:\\Fonts\\SegUIVar.ttf\r\n    \
                      Some Size    REG_DWORD    0x1\r\n    \
                      Empty    REG_SZ    \r\n    \
                      Expanded    REG_EXPAND_SZ    %WINDIR%\\Fonts\\x.ttf\r\n\r\n";
        assert_eq!(
            parse_values(output),
            [
                ("Arial (TrueType)", "arial.ttf"),
                ("Segoe UI Variable (TrueType)", r"C:\Fonts\SegUIVar.ttf"),
                ("Empty", ""),
                ("Expanded", r"%WINDIR%\Fonts\x.ttf"),
            ]
            .map(|(name, data)| (name.to_owned(), data.to_owned()))
        );
        assert_eq!(parse_values(""), []);
        assert_eq!(
            parse_values("ERROR: The system was unable to find the key"),
            []
        );
    }

    #[test]
    fn missing_queries_hold_nothing() {
        assert_eq!(values(None), []);
    }

    #[test]
    fn values_name_the_folder_and_the_font_files() {
        let pair = |name: &str, data: &str| (name.to_owned(), data.to_owned());
        let registry = Registry::of(
            vec![
                pair("Desktop", r"C:\Desktop"),
                pair("Fonts", r"C:\Windows\Fonts"),
            ],
            [pair("Arial (TrueType)", "arial.ttf")].into_iter(),
            [pair("Mine", r"C:\Users\u\mine.otf")].into_iter(),
        );
        assert_eq!(
            registry,
            Registry {
                shell_fonts_folder: Some(PathBuf::from(r"C:\Windows\Fonts")),
                machine_fonts: vec!["arial.ttf".to_owned()],
                user_fonts: vec![r"C:\Users\u\mine.otf".to_owned()],
            }
        );
        assert_eq!(
            Registry::of(Vec::new(), [].into_iter(), [].into_iter()),
            Registry::default()
        );
    }
}
