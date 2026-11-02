//! The Windows registry's font entries (matplotlib's
//! `win32FontDirectory` and `_get_win32_installed_fonts`), read through
//! the system's `reg query` command, as no crate holding unsafe code may
//! call the registry's API.

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

use pltrs_core::os;

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
    /// Reads the keys, all queries running at once; a key that cannot be
    /// read holds nothing.
    pub(crate) fn read() -> Self {
        let machine = FONT_KEYS.map(|key| format!(r"HKLM\{key}"));
        let user = FONT_KEYS.map(|key| format!(r"HKCU\{key}"));
        let shell = query(SHELL_FOLDERS);
        let machine = machine.map(|key| query(&key));
        let user = user.map(|key| query(&key));
        Self {
            shell_fonts_folder: values(shell)
                .into_iter()
                .find(|(name, _)| name == "Fonts")
                .map(|(_, folder)| PathBuf::from(folder)),
            machine_fonts: machine.into_iter().flat_map(file_names).collect(),
            user_fonts: user.into_iter().flat_map(file_names).collect(),
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

/// The data of the string values a query printed: font file names.
fn file_names(query: Option<Child>) -> Vec<String> {
    values(query).into_iter().map(|(_, data)| data).collect()
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
        assert_eq!(file_names(None), Vec::<String>::new());
    }
}
