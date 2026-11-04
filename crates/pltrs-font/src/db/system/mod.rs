//! Discovery of the fonts installed on the system, where matplotlib's
//! `findSystemFonts` looks on each system: the X11 and freedesktop.org
//! directories on Linux, those and macOS' own on macOS, and on Windows
//! the system's and the user's font directories and the fonts the
//! registry lists. (matplotlib also asks fontconfig's `fc-list` and
//! macOS' `system_profiler`; pltrs keeps to directories, which hold those
//! fonts too, so that starting up runs no program but, on Windows, the
//! registry query. On macOS these include the two places of fonts its
//! font list holds beyond its font directories: the reserved interface
//! fonts and the fonts it downloads on demand.)

mod registry;

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use pltrs_core::os::Os;

pub(crate) use registry::Registry;

/// matplotlib's `X11FontDirectories`, but the user's.
const X11_DIRECTORIES: [&str; 5] = [
    "/usr/X11R6/lib/X11/fonts/TTF/",
    "/usr/X11/lib/X11/fonts",
    "/usr/share/fonts/",
    "/usr/local/share/fonts/",
    "/usr/lib/openoffice/share/fonts/truetype/",
];

/// matplotlib's `OSXFontDirectories`, but the user's.
const MACOS_DIRECTORIES: [&str; 4] = [
    "/Library/Fonts/",
    "/Network/Library/Fonts/",
    "/System/Library/Fonts/",
    "/opt/local/share/fonts",
];

/// The directory of the fonts macOS reserves for its interface, which its
/// font list (and so matplotlib, through `system_profiler`) holds.
const MACOS_RESERVED_FONTS: &str =
    "/System/Library/PrivateFrameworks/FontServices.framework/Versions/A/Resources/Reserved";

/// Where macOS keeps what it downloads on demand.
const MACOS_ASSETS: &str = "/System/Library/AssetsV2";

/// The prefix of the asset directories holding downloaded fonts
/// (`com_apple_MobileAsset_Font8`, ...), which macOS' font list holds too.
const MACOS_FONT_ASSETS: &str = "com_apple_MobileAsset_Font";

/// matplotlib's `MSUserFontDirectories`, in the home directory.
const WINDOWS_USER_DIRECTORIES: [&str; 2] = [
    "AppData/Local/Microsoft/Windows/Fonts",
    "AppData/Roaming/Microsoft/Windows/Fonts",
];

/// Where a system keeps its fonts: directories to search, and font files
/// found elsewhere (those the Windows registry lists outside them).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SystemFonts {
    /// The directories, searched recursively.
    pub(crate) directories: Vec<PathBuf>,
    /// Single font files.
    pub(crate) files: Vec<PathBuf>,
}

impl SystemFonts {
    /// The fonts of this system, as the process's environment (and on
    /// Windows the registry) tells where they are.
    pub(crate) fn of_this_system() -> Self {
        let registry = if Os::CURRENT == Os::Windows {
            Registry::read()
        } else {
            Registry::default()
        };
        let mut fonts = Self::locate(
            Os::CURRENT,
            std::env::home_dir().as_deref(),
            |name| std::env::var_os(name),
            &registry,
        );
        if Os::CURRENT == Os::MacOs {
            fonts
                .directories
                .extend(font_asset_directories(Path::new(MACOS_ASSETS)));
        }
        fonts
    }

    /// The fonts of `os` for the home directory `home`, the environment
    /// variables `var` gives (`XDG_DATA_HOME`, `WINDIR`) and, on Windows,
    /// what the registry holds.
    pub(crate) fn locate(
        os: Os,
        home: Option<&Path>,
        var: impl Fn(&str) -> Option<OsString>,
        registry: &Registry,
    ) -> Self {
        let in_home = |relative: &str| home.map(|home| home.join(relative));
        match os {
            Os::Linux | Os::MacOs => {
                let mut directories: Vec<PathBuf> =
                    X11_DIRECTORIES.iter().map(PathBuf::from).collect();
                let data_home = var("XDG_DATA_HOME")
                    .filter(|value| !value.is_empty())
                    .map(PathBuf::from)
                    .or_else(|| in_home(".local/share"));
                directories.extend(data_home.map(|dir| dir.join("fonts")));
                directories.extend(in_home(".fonts"));
                if os == Os::MacOs {
                    directories.extend(MACOS_DIRECTORIES.iter().map(PathBuf::from));
                    directories.extend(in_home("Library/Fonts"));
                    directories.push(PathBuf::from(MACOS_RESERVED_FONTS));
                }
                Self {
                    directories,
                    files: Vec::new(),
                }
            }
            Os::Windows => {
                // matplotlib's `win32FontDirectory`.
                let system = registry
                    .shell_fonts_folder
                    .clone()
                    .or_else(|| var("WINDIR").map(|windir| PathBuf::from(windir).join("Fonts")));
                let users: Vec<PathBuf> = WINDOWS_USER_DIRECTORIES
                    .iter()
                    .filter_map(|relative| in_home(relative))
                    .collect();
                let mut directories: Vec<PathBuf> = system.iter().cloned().collect();
                directories.extend(users.iter().cloned());
                let listed = system
                    .iter()
                    .flat_map(|dir| {
                        registry
                            .machine_fonts
                            .iter()
                            .map(move |file| dir.join(file))
                    })
                    .chain(users.iter().flat_map(|dir| {
                        registry.user_fonts.iter().map(move |file| dir.join(file))
                    }));
                let mut files: Vec<PathBuf> = Vec::new();
                for file in listed {
                    let searched = directories.iter().any(|dir| is_within(os, &file, dir));
                    if !searched && !files.iter().any(|known| same_path(os, known, &file)) {
                        files.push(file);
                    }
                }
                Self { directories, files }
            }
        }
    }
}

/// The directories of downloaded fonts among the assets in `assets` (see
/// [`MACOS_FONT_ASSETS`]), sorted.
fn font_asset_directories(assets: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(assets) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with(MACOS_FONT_ASSETS))
        })
        .map(|entry| entry.path())
        .collect();
    found.sort();
    found
}

/// `path` as `os` compares paths: Windows ignores case.
fn comparable(os: Os, path: &Path) -> PathBuf {
    match os {
        Os::Windows => PathBuf::from(path.to_string_lossy().to_lowercase()),
        Os::Linux | Os::MacOs => path.to_owned(),
    }
}

/// Whether `file` lies below the directory `dir`.
fn is_within(os: Os, file: &Path, dir: &Path) -> bool {
    comparable(os, file).starts_with(comparable(os, dir))
}

/// Whether `a` and `b` name the same file.
fn same_path(os: Os, a: &Path, b: &Path) -> bool {
    comparable(os, a) == comparable(os, b)
}

/// Whether `path` has a font extension matplotlib loads (`.ttf`, `.otf`,
/// `.ttc`, case-insensitive).
pub(crate) fn is_font_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ["ttf", "otf", "ttc"].contains(&ext.to_ascii_lowercase().as_str()))
}

/// All font files below `dir`, recursively, sorted for determinism.
/// Unreadable directories are skipped, like `os.walk` does.
pub(crate) fn font_files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => pending.push(path),
                Ok(_) if is_font_file(&path) => found.push(path),
                _ => {}
            }
        }
    }
    found.sort();
    found
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    #[test]
    fn recognises_font_extensions() {
        assert!(is_font_file(Path::new("/a/DejaVuSans.TTF")));
        assert!(is_font_file(Path::new("x.otf")));
        assert!(is_font_file(Path::new("x.ttc")));
        assert!(!is_font_file(Path::new("x.afm")));
        assert!(!is_font_file(Path::new("ttf")));
    }

    #[test]
    fn walks_directories() {
        let root = env::temp_dir().join(format!("pltrs-font-walk-{}", std::process::id()));
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::write(root.join("a.ttf"), b"").unwrap();
        fs::write(root.join("sub/b.otf"), b"").unwrap();
        fs::write(root.join("sub/readme.txt"), b"").unwrap();
        let files = font_files(&root);
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(font_files(&root), Vec::<PathBuf>::new());
    }

    /// The fonts of `os` for the home `/home/u`, with the variables
    /// `vars` set.
    fn locate(os: Os, vars: &[(&str, &str)], registry: &Registry) -> SystemFonts {
        let var = |name: &str| {
            vars.iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| OsString::from(value))
        };
        SystemFonts::locate(os, Some(Path::new("/home/u")), var, registry)
    }

    fn paths(list: &[&str]) -> Vec<PathBuf> {
        list.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn linux_searches_matplotlibs_x11_directories() {
        let fonts = locate(Os::Linux, &[], &Registry::default());
        assert_eq!(
            fonts.directories,
            paths(&[
                "/usr/X11R6/lib/X11/fonts/TTF/",
                "/usr/X11/lib/X11/fonts",
                "/usr/share/fonts/",
                "/usr/local/share/fonts/",
                "/usr/lib/openoffice/share/fonts/truetype/",
                "/home/u/.local/share/fonts",
                "/home/u/.fonts",
            ])
        );
        assert_eq!(fonts.files, Vec::<PathBuf>::new());
        let fonts = locate(
            Os::Linux,
            &[("XDG_DATA_HOME", "/data")],
            &Registry::default(),
        );
        assert!(fonts.directories.contains(&PathBuf::from("/data/fonts")));
        let fonts = locate(Os::Linux, &[("XDG_DATA_HOME", "")], &Registry::default());
        assert!(
            fonts
                .directories
                .contains(&PathBuf::from("/home/u/.local/share/fonts"))
        );
        let homeless =
            SystemFonts::locate(Os::Linux, None, |_| None, &Registry::default()).directories;
        assert_eq!(homeless, paths(&X11_DIRECTORIES));
    }

    #[test]
    fn macos_adds_its_own_directories() {
        let fonts = locate(Os::MacOs, &[], &Registry::default());
        let linux = locate(Os::Linux, &[], &Registry::default());
        assert_eq!(
            fonts.directories[..linux.directories.len()],
            linux.directories
        );
        assert_eq!(
            fonts.directories[linux.directories.len()..],
            paths(&[
                "/Library/Fonts/",
                "/Network/Library/Fonts/",
                "/System/Library/Fonts/",
                "/opt/local/share/fonts",
                "/home/u/Library/Fonts",
                MACOS_RESERVED_FONTS,
            ])
        );
    }

    #[test]
    fn macos_font_assets_are_found_by_name() {
        let root = env::temp_dir().join(format!("pltrs-font-assets-{}", std::process::id()));
        for dir in [
            "com_apple_MobileAsset_Font8",
            "com_apple_MobileAsset_Font7",
            "com_apple_MobileAsset_Dictionary",
        ] {
            fs::create_dir_all(root.join(dir)).unwrap();
        }
        let found = font_asset_directories(&root);
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(
            found,
            [
                root.join("com_apple_MobileAsset_Font7"),
                root.join("com_apple_MobileAsset_Font8")
            ]
        );
        assert_eq!(font_asset_directories(&root), Vec::<PathBuf>::new());
    }

    #[test]
    fn windows_searches_the_font_folders_and_the_registry() {
        let registry = Registry {
            shell_fonts_folder: None,
            machine_fonts: vec![
                "arial.ttf".to_owned(),
                "/Windows/FONTS/sub/Times.TTF".to_owned(),
                "/opt/app/fonts/app.ttf".to_owned(),
                "/opt/app/fonts/APP.ttf".to_owned(),
            ],
            user_fonts: vec!["/elsewhere/mine.otf".to_owned()],
        };
        let fonts = locate(Os::Windows, &[("WINDIR", "/Windows")], &registry);
        assert_eq!(
            fonts.directories,
            paths(&[
                "/Windows/Fonts",
                "/home/u/AppData/Local/Microsoft/Windows/Fonts",
                "/home/u/AppData/Roaming/Microsoft/Windows/Fonts",
            ])
        );
        assert_eq!(
            fonts.files,
            paths(&["/opt/app/fonts/app.ttf", "/elsewhere/mine.otf"]),
            "files in the folders searched are left to the search, case aside"
        );

        let registry = Registry {
            shell_fonts_folder: Some(PathBuf::from("/D/Fonts")),
            ..Registry::default()
        };
        let fonts = locate(Os::Windows, &[("WINDIR", "/Windows")], &registry);
        assert_eq!(
            fonts.directories[0],
            PathBuf::from("/D/Fonts"),
            "the shell's folder"
        );
        let fonts = SystemFonts::locate(Os::Windows, None, |_| None, &Registry::default());
        assert_eq!(fonts, SystemFonts::default());
    }

    #[test]
    fn this_system_has_font_directories() {
        let fonts = SystemFonts::of_this_system();
        assert_ne!(fonts.directories, Vec::<PathBuf>::new());
        if Os::CURRENT == Os::Linux {
            assert!(
                fonts
                    .directories
                    .iter()
                    .any(|dir| dir.ends_with("share/fonts"))
            );
        }
    }
}
