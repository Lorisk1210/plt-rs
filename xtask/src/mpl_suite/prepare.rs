//! The directory a suite run works in: `target/mpl-suite/<mode>/site/`.
//!
//! The site lays out the package under test with matplotlib 3.11.2's test
//! packages inside it, as an editable install of matplotlib's source
//! tree does. `site/matplotlib/` holds a copy of the sdist's
//! `lib/matplotlib/tests` (with its `baseline_images` and data) and a
//! symbolic link to every other entry of the package: the reference
//! venv's installed matplotlib, or `python/pltrs`. Each toolkit in
//! `site/mpl_toolkits/` is laid out the same way. pytest runs in `site/`
//! with `site/` on `PYTHONPATH`, so test ids read
//! `matplotlib/tests/test_x.py::...` in both modes, and `import
//! matplotlib` loads the package under test under matplotlib's names.
//!
//! The test packages are real directories, not links, because some tests
//! compare paths of their data files after resolving links. They are
//! copied once (a stamp names the sdist); the links are made afresh for
//! each run, so that files added to the package since appear.
//!
//! The reference run thus imports real matplotlib with the sdist's test
//! data in place, without changing the reference venv (whose installed
//! package has no baseline images, so `test_getattr.py` could not even be
//! collected there). The pltrs site adds `pylab.py` and, as
//! `sitecustomize.py`, `python/mpl_suite/pltrs_alias.py`, which makes the
//! name `pltrs` itself resolve to the modules loaded as `matplotlib`.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use super::Mode;

/// The matplotlib toolkits whose tests are part of the suite.
pub(super) const TOOLKITS: [&str; 3] = ["axes_grid1", "axisartist", "mplot3d"];

/// The stamp naming the sdist a test package was copied from.
const STAMP: &str = ".copied-from";

/// The `__init__.py` of a package the package under test lacks.
const PLACEHOLDER: &str =
    "# The package under test has no such package yet (cargo xtask mpl-suite).\n";

/// pytest's configuration in each site: matplotlib's own
/// (`[tool.pytest.ini_options]` of its `pyproject.toml`).
const PYTEST_INI: &str = "[pytest]\nminversion = 7.0\naddopts = --import-mode=importlib\n";

/// The package directories of the site, relative to it, with the
/// directory of the sdist's `lib` holding their tests.
fn packages() -> Vec<String> {
    let mut packages = vec!["matplotlib".to_owned()];
    packages.extend(TOOLKITS.map(|kit| format!("mpl_toolkits/{kit}")));
    packages
}

/// Lays out `mode`'s site in `suite` and returns its directory.
pub(super) fn site(mode: Mode, suite: &Path, sdist: &Path) -> Result<PathBuf, String> {
    let root = crate::python::root();
    let site = suite.join(mode.name()).join("site");
    let lib = sdist.join("lib");
    if !lib.join("matplotlib/tests").is_dir() {
        return Err(format!(
            "{} is not matplotlib 3.11.2's sdist (set MPL_SDIST to the unpacked \
             matplotlib-3.11.2.tar.gz)",
            sdist.display()
        ));
    }
    let source = match mode {
        Mode::Reference => site_packages(&root.join(".venv"))?,
        Mode::Pltrs => root.join("python/pltrs"),
    };
    for package in packages() {
        let dest = site.join(&package);
        copy_tests(&lib.join(&package).join("tests"), &dest.join("tests"))?;
        let from = match (mode, package.strip_prefix("mpl_toolkits/")) {
            (Mode::Pltrs, Some(kit)) => source.join("mpl_toolkits").join(kit),
            (Mode::Pltrs, None) => source.clone(),
            (Mode::Reference, _) => source.join(&package),
        };
        relink(&from, &dest, &["tests", "mpl_toolkits"])?;
        mark_package(&dest)?;
    }
    relink(
        &source.join("mpl_toolkits"),
        &site.join("mpl_toolkits"),
        &TOOLKITS,
    )?;
    remove_links(&site)?;
    if mode == Mode::Pltrs {
        let pylab = source.join("pylab.py");
        if pylab.exists() {
            link(&pylab, &site.join("pylab.py"))?;
        }
        link(
            &root.join("python/mpl_suite/pltrs_alias.py"),
            &site.join("sitecustomize.py"),
        )?;
    }
    let ini = site.join("pytest.ini");
    fs::write(&ini, PYTEST_INI).map_err(|err| format!("{}: {err}", ini.display()))?;
    Ok(site)
}

/// Makes `dir` a package with an empty `__init__.py` when the package
/// under test has none there (pltrs without one of the toolkits yet), so
/// that pytest names its test modules `mpl_toolkits.<kit>.tests.test_x`
/// as in the reference run, rather than a bare `tests.test_x` that every
/// toolkit's tests would clash on.
fn mark_package(dir: &Path) -> Result<(), String> {
    let init = dir.join("__init__.py");
    if init.exists() {
        return Ok(());
    }
    fs::write(&init, PLACEHOLDER).map_err(|err| format!("{}: {err}", init.display()))
}

/// Every test file of a site, relative to it (`matplotlib/tests/
/// test_x.py`), sorted.
pub(super) fn test_files(site: &Path) -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    for package in packages() {
        let dir = PathBuf::from(package).join("tests");
        let entries =
            fs::read_dir(site.join(&dir)).map_err(|err| format!("{}: {err}", dir.display()))?;
        for entry in entries.filter_map(Result::ok) {
            let name = entry.file_name().to_string_lossy().into_owned();
            let python = Path::new(&name).extension().is_some_and(|ext| ext == "py");
            if name.starts_with("test_") && python {
                files.push(dir.join(name).display().to_string());
            }
        }
    }
    files.sort();
    Ok(files)
}

/// Copies a test package from the sdist to `dest` unless the copy is
/// already there.
fn copy_tests(from: &Path, dest: &Path) -> Result<(), String> {
    let stamp = dest.join(STAMP);
    let source = from.display().to_string();
    if fs::read_to_string(&stamp).is_ok_and(|copied| copied == source) {
        return Ok(());
    }
    remove(dest)?;
    copy_dir(from, dest)?;
    fs::write(&stamp, source).map_err(|err| format!("{}: {err}", stamp.display()))
}

/// The `site-packages` directory of a venv.
fn site_packages(venv: &Path) -> Result<PathBuf, String> {
    let lib = venv.join("lib");
    fs::read_dir(&lib)
        .map_err(|err| format!("{}: {err}", lib.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("site-packages"))
        .find(|path| path.is_dir())
        .ok_or_else(|| format!("no site-packages in {}", lib.display()))
}

/// Replaces the links in the directory `dest` by links to every entry of
/// `source` but those named in `skip` and byte-code caches (none if
/// `source` is missing or empty), leaving what is not a link alone.
fn relink(source: &Path, dest: &Path, skip: &[&str]) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|err| format!("{}: {err}", dest.display()))?;
    remove_links(dest)?;
    let Ok(entries) = fs::read_dir(source) else {
        return Ok(());
    };
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name();
        let name_text = name.to_string_lossy();
        if name_text == "__pycache__" || skip.contains(&name_text.as_ref()) {
            continue;
        }
        link(&entry.path(), &dest.join(&name))?;
    }
    Ok(())
}

/// Removes the symbolic links and placeholder `__init__.py` in the
/// directory `dir`.
fn remove_links(dir: &Path) -> Result<(), String> {
    for entry in fs::read_dir(dir)
        .map_err(|err| format!("{}: {err}", dir.display()))?
        .filter_map(Result::ok)
    {
        let placeholder = entry.file_name() == "__init__.py"
            && fs::read_to_string(entry.path()).is_ok_and(|text| text == PLACEHOLDER);
        if placeholder || entry.file_type().is_ok_and(|kind| kind.is_symlink()) {
            fs::remove_file(entry.path())
                .map_err(|err| format!("{}: {err}", entry.path().display()))?;
        }
    }
    Ok(())
}

/// Links `link_path` to `target`.
fn link(target: &Path, link_path: &Path) -> Result<(), String> {
    symlink(target, link_path).map_err(|err| format!("{}: {err}", link_path.display()))
}

/// Removes `path` and everything in it, if it exists.
fn remove(path: &Path) -> Result<(), String> {
    match fs::remove_dir_all(path) {
        Err(err) if err.kind() != std::io::ErrorKind::NotFound => {
            Err(format!("{}: {err}", path.display()))
        }
        _ => Ok(()),
    }
}

/// Copies the directory `source` into `dest`, recursively, leaving out
/// byte-code caches.
fn copy_dir(source: &Path, dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|err| format!("{}: {err}", dest.display()))?;
    let entries = fs::read_dir(source).map_err(|err| format!("{}: {err}", source.display()))?;
    for entry in entries.filter_map(Result::ok) {
        let (from, to) = (entry.path(), dest.join(entry.file_name()));
        if entry.file_name() == "__pycache__" {
            continue;
        }
        if from.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            fs::copy(&from, &to).map_err(|err| format!("{}: {err}", from.display()))?;
        }
    }
    Ok(())
}
