//! Discovery of font files in the usual Unix font directories.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Directories matplotlib searches on Linux (besides fontconfig's list):
/// `/usr/share/fonts`, `/usr/local/share/fonts`, `$XDG_DATA_HOME/fonts`
/// (default `~/.local/share/fonts`) and `~/.fonts`.
pub(crate) fn font_directories() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/usr/share/fonts"),
        PathBuf::from("/usr/local/share/fonts"),
    ];
    let home = env::var_os("HOME").map(PathBuf::from);
    let data_home = env::var_os("XDG_DATA_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| home.as_ref().map(|home| home.join(".local/share")));
    dirs.extend(data_home.map(|dir| dir.join("fonts")));
    dirs.extend(home.map(|home| home.join(".fonts")));
    dirs
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
        assert!(
            font_directories()
                .iter()
                .any(|dir| dir.ends_with("share/fonts"))
        );
    }
}
