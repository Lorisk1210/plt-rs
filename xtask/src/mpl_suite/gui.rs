//! What matplotlib's GUI tests need beyond the venvs' packages, on Linux:
//! an X display, which the runner starts (an Xvfb of its own) when
//! `DISPLAY` is unset, the libraries, Python modules and variables of the
//! toolkits `python/tools/toolkits.sh` installs, and a fixed locale (see
//! [`LOCALE`]). Both runs get the same; the runner says what is missing,
//! and matplotlib's tests needing it skip. macOS and Windows need none of
//! it: their toolkits draw on the system's own windows.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use pltrs_core::os::Os;

use super::display::VirtualDisplay;
use super::pytest::Variable;

/// The locale both runs have. The toolkits set the process's locale
/// from the environment (Qt's `QCoreApplication`, GTK's initialisation),
/// which matplotlib's `testing.setup` leaves at C where `en_US.UTF-8` is
/// missing: with the host's locale, numbers would format with its digit
/// grouping in the rest of a pytest worker that ran a Qt or GTK test, so
/// that outcomes (`test_use_locale`) depended on the tests' order and the
/// host.
const LOCALE: &str = "C.UTF-8";

/// Where the Qt bindings keep their xcb platform plugin, in a venv's
/// `site-packages`.
const QT_XCB_PLUGINS: [&str; 3] = [
    "PyQt6/Qt6/plugins/platforms/libqxcb.so",
    "PySide6/Qt/plugins/platforms/libqxcb.so",
    "PyQt5/Qt5/plugins/platforms/libqxcb.so",
];

/// The GUI environment of a suite run. The Xvfb it started, if any, stops
/// when it is dropped.
#[derive(Debug, Default)]
pub(crate) struct Gui {
    /// The variables the run sets or removes.
    variables: Vec<Variable>,
    /// What the toolkits put on `PYTHONPATH` (their site).
    python_path: Vec<PathBuf>,
    /// The display the runner started.
    display: Option<VirtualDisplay>,
}

impl Gui {
    /// The GUI environment of a run with `python` whose files go to `dir`
    /// (Xvfb's log), saying what it has and what it lacks.
    pub(crate) fn prepare(python: &Path, dir: &Path) -> Self {
        let mut gui = Self::default();
        if Os::CURRENT != Os::Linux {
            return gui;
        }
        gui.variables
            .push(("LC_ALL".to_owned(), Some(LOCALE.into())));
        gui.add_toolkits(python);
        if gui.add_display(dir) {
            gui.check_qt(python);
        }
        gui
    }

    /// The variables the run sets or removes.
    pub(crate) fn variables(&self) -> &[Variable] {
        &self.variables
    }

    /// The directories the run needs on `PYTHONPATH`.
    pub(crate) fn python_path(&self) -> &[PathBuf] {
        &self.python_path
    }

    /// Adds the variables of `toolkits.sh env` for `python`, or says why
    /// there are none.
    fn add_toolkits(&mut self, python: &Path) {
        let script = crate::python::root().join("python/tools/toolkits.sh");
        // Without the inherited PYTHONPATH, the script's names the
        // toolkits' site only; the run keeps the inherited one itself.
        let output = Command::new("sh")
            .arg(&script)
            .arg("env")
            .env("TOOLKITS_PYTHON", python)
            .env_remove("PYTHONPATH")
            .output();
        match output {
            Ok(output) if output.status.success() => {
                let (variables, python_path) =
                    toolkit_variables(&String::from_utf8_lossy(&output.stdout));
                self.variables.extend(variables);
                self.python_path.extend(python_path);
                println!("GUI toolkits: GTK 3 and 4, wx, Cairo and Qt's X libraries");
            }
            Ok(output) => println!(
                "no GUI toolkits ({}): the GTK, wx and Cairo tests skip",
                String::from_utf8_lossy(&output.stderr).trim()
            ),
            Err(err) => println!(
                "no GUI toolkits (cannot run {}: {err}): the GTK, wx and Cairo tests skip",
                script.display()
            ),
        }
    }

    /// Gives the run a display: `DISPLAY`'s, else an Xvfb of its own,
    /// with `WAYLAND_DISPLAY` removed so that nothing reaches a
    /// compositor. Says which, or why there is none, and whether there is.
    fn add_display(&mut self, dir: &Path) -> bool {
        if let Some(display) = std::env::var_os("DISPLAY").filter(|d| !d.is_empty()) {
            println!("display: {}", display.to_string_lossy());
            return true;
        }
        let Some(xvfb) = pltrs_core::os::which("Xvfb") else {
            println!("no display: $DISPLAY is unset and Xvfb is not on PATH; the GUI tests skip");
            return false;
        };
        if let Err(err) = std::fs::create_dir_all(dir) {
            println!("no display: {}: {err}; the GUI tests skip", dir.display());
            return false;
        }
        match VirtualDisplay::start(&xvfb, &dir.join("xvfb.log")) {
            Ok(display) => {
                println!(
                    "display: {} (Xvfb, process {}, for this run)",
                    display.name(),
                    display.id()
                );
                self.variables
                    .push(("DISPLAY".to_owned(), Some(display.name().into())));
                self.variables.push(("WAYLAND_DISPLAY".to_owned(), None));
                self.display = Some(display);
                true
            }
            Err(err) => {
                println!("no display: {err}; the GUI tests skip");
                false
            }
        }
    }

    /// Lets Qt run offscreen when the xcb platform plugin of a binding in
    /// `python`'s venv lacks libraries, which would abort every Qt test,
    /// and says so.
    fn check_qt(&mut self, python: &Path) {
        let Some(site) = python
            .parent()
            .and_then(Path::parent)
            .and_then(|venv| crate::python::site_packages(venv).ok())
        else {
            return;
        };
        let library_path = self
            .variables
            .iter()
            .find_map(|(name, value)| (name == "LD_LIBRARY_PATH").then(|| value.clone()).flatten());
        let mut missing = Vec::new();
        for plugin in QT_XCB_PLUGINS.map(|plugin| site.join(plugin)) {
            if !plugin.exists() {
                continue;
            }
            let mut ldd = Command::new("ldd");
            ldd.arg(&plugin);
            if let Some(path) = &library_path {
                ldd.env("LD_LIBRARY_PATH", path);
            }
            if let Ok(output) = ldd.output() {
                for library in not_found(&String::from_utf8_lossy(&output.stdout)) {
                    if !missing.contains(&library) {
                        missing.push(library);
                    }
                }
            }
        }
        if !missing.is_empty() {
            println!(
                "Qt's xcb platform plugin lacks {} (python/tools/toolkits.sh install \
                 provides them): Qt runs offscreen",
                missing.join(", ")
            );
            self.variables
                .push(("QT_QPA_PLATFORM".to_owned(), Some("offscreen".into())));
        }
    }
}

/// The variables of `toolkits.sh env`'s `NAME=value` lines, and the
/// directories its `PYTHONPATH` names, which the run puts on its own.
fn toolkit_variables(lines: &str) -> (Vec<Variable>, Vec<PathBuf>) {
    let mut variables = Vec::new();
    let mut python_path = Vec::new();
    for (name, value) in lines.lines().filter_map(|line| line.split_once('=')) {
        if name == "PYTHONPATH" {
            python_path.extend(std::env::split_paths(value).filter(|d| !d.as_os_str().is_empty()));
        } else {
            variables.push((name.to_owned(), Some(OsString::from(value))));
        }
    }
    (variables, python_path)
}

/// The libraries `ldd`'s report `report` says are not found.
fn not_found(report: &str) -> Vec<String> {
    report
        .lines()
        .filter(|line| line.contains("not found"))
        .filter_map(|line| line.split_whitespace().next())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toolkit_lines_become_variables_and_python_path() {
        let (variables, python_path) = toolkit_variables(
            "LD_LIBRARY_PATH=/kit/lib:/kit/lib/pulseaudio\n\
             PYTHONPATH=/kit/site\n\
             GSETTINGS_BACKEND=memory\n\
             NO_AT_BRIDGE=1\n",
        );
        assert_eq!(
            variables,
            [
                ("LD_LIBRARY_PATH", "/kit/lib:/kit/lib/pulseaudio"),
                ("GSETTINGS_BACKEND", "memory"),
                ("NO_AT_BRIDGE", "1"),
            ]
            .map(|(name, value)| (name.to_owned(), Some(OsString::from(value))))
        );
        assert_eq!(python_path, [PathBuf::from("/kit/site")]);
        assert_eq!(toolkit_variables(""), (Vec::new(), Vec::new()));
    }

    #[test]
    fn missing_libraries_are_read_from_ldd() {
        let report = "\tlinux-vdso.so.1 (0x00007ffd)\n\
                      \tlibxcb-cursor.so.0 => not found\n\
                      \tlibX11.so.6 => /lib/x86_64-linux-gnu/libX11.so.6 (0x00007f)\n\
                      \tlibxkbcommon-x11.so.0 => not found\n";
        assert_eq!(
            not_found(report),
            ["libxcb-cursor.so.0", "libxkbcommon-x11.so.0"]
        );
        assert_eq!(not_found(""), Vec::<String>::new());
    }
}
