//! Where things are in a venv, and what the extension's files are called,
//! on each system: functions of the system, so that every system's
//! layout is tested on any.

use std::fs;
use std::path::{Path, PathBuf};

use pltrs_core::os::Os;

use super::root;

/// The interpreter of the venv `venv`: `bin/python`, on Windows
/// `Scripts\python.exe`.
pub(crate) fn interpreter(venv: &Path, os: Os) -> PathBuf {
    match os {
        Os::Windows => venv.join("Scripts").join("python.exe"),
        Os::Linux | Os::MacOs => venv.join("bin").join("python"),
    }
}

/// The reference venv's interpreter (`.venv`, real matplotlib).
pub(crate) fn reference_python() -> PathBuf {
    interpreter(&root().join(".venv"), Os::CURRENT)
}

/// The directories that may hold the `site-packages` of the venv `venv`
/// on `os`: `Lib\site-packages` on Windows, elsewhere
/// `lib/python3.X/site-packages` for each entry `lib_entries` names in
/// `lib`.
fn site_packages_candidates(venv: &Path, os: Os, lib_entries: &[String]) -> Vec<PathBuf> {
    match os {
        Os::Windows => vec![venv.join("Lib").join("site-packages")],
        Os::Linux | Os::MacOs => lib_entries
            .iter()
            .map(|entry| venv.join("lib").join(entry).join("site-packages"))
            .collect(),
    }
}

/// The `site-packages` directory of the venv `venv`.
pub(crate) fn site_packages(venv: &Path) -> Result<PathBuf, String> {
    let lib = venv.join("lib");
    let mut entries: Vec<String> = fs::read_dir(&lib)
        .map(|dir| {
            dir.filter_map(Result::ok)
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    entries.sort();
    site_packages_candidates(venv, Os::CURRENT, &entries)
        .into_iter()
        .find(|path| path.is_dir())
        .ok_or_else(|| format!("no site-packages in {}", venv.display()))
}

/// The file cargo builds `pltrs-python` into on `os`, and the name the
/// package imports it by (`_native`, an abi3 extension: `.abi3.so` on
/// Linux and macOS, `.pyd` on Windows, where abi3 has no tag of its own).
pub(crate) fn extension_files(os: Os) -> (&'static str, &'static str) {
    match os {
        Os::Linux => ("libpltrs_python.so", "_native.abi3.so"),
        Os::MacOs => ("libpltrs_python.dylib", "_native.abi3.so"),
        Os::Windows => ("pltrs_python.dll", "_native.pyd"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpreters_live_in_bin_or_scripts() {
        let venv = Path::new("v");
        assert_eq!(interpreter(venv, Os::Linux), Path::new("v/bin/python"));
        assert_eq!(interpreter(venv, Os::MacOs), Path::new("v/bin/python"));
        assert_eq!(
            interpreter(venv, Os::Windows),
            Path::new("v").join("Scripts").join("python.exe")
        );
    }

    #[test]
    fn site_packages_follow_the_system() {
        let venv = Path::new("v");
        let entries = ["python3.14".to_owned()];
        assert_eq!(
            site_packages_candidates(venv, Os::Linux, &entries),
            [Path::new("v/lib/python3.14/site-packages")]
        );
        assert_eq!(
            site_packages_candidates(venv, Os::Windows, &entries),
            [Path::new("v").join("Lib").join("site-packages")]
        );
        assert_eq!(
            site_packages_candidates(venv, Os::MacOs, &[]),
            Vec::<PathBuf>::new()
        );
        let reference = root().join(".venv");
        if reference.exists() {
            assert!(site_packages(&reference).is_ok_and(|dir| dir.ends_with("site-packages")));
        }
    }

    #[test]
    fn the_extension_is_named_as_python_imports_it() {
        assert_eq!(
            extension_files(Os::Linux),
            ("libpltrs_python.so", "_native.abi3.so")
        );
        assert_eq!(
            extension_files(Os::MacOs),
            ("libpltrs_python.dylib", "_native.abi3.so")
        );
        assert_eq!(
            extension_files(Os::Windows),
            ("pltrs_python.dll", "_native.pyd")
        );
    }
}
