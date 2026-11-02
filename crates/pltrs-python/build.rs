//! Links the extension, and finds a Python to embed in the crate's unit
//! tests.
//!
//! The extension itself links against no Python: an abi3 extension takes
//! the interpreter's symbols from the process that loads it. On macOS the
//! linker must be told so (`-undefined dynamic_lookup`), and the library
//! gets the install name of the file it is installed as. (On Linux shared
//! libraries may leave symbols undefined; on Windows the declarations
//! name `python3.dll`, see `ffi`.)
//!
//! The unit tests are a program of their own, so they embed an
//! interpreter: this script finds a shared libpython (3.14 or later,
//! whose `PyInitConfig` API sets the interpreter's home) and lets the
//! tests link it. The Python is `PLTRS_PYTHON` if set, else the
//! workspace's pltrs or reference venv, else `python3` (`python` on
//! Windows) on `PATH`. With one found, the script puts the library (and
//! on Windows its DLLs) into `OUT_DIR/embed`, where `cargo test` also
//! finds them at run time, and sets the `pltrs_python_embed` cfg and the
//! `PLTRS_PYTHON_HOME` variable the tests read. Without one, or when
//! cross-compiling, the crate's test build is empty.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The file name the extension is installed as on macOS, which its
/// install name follows.
const MACOS_EXTENSION: &str = "_native.abi3.so";

/// What the tests need of a Python installation.
struct Embeddable {
    /// `sys.base_prefix`: the interpreter's home.
    home: String,
    /// The library to link (on Windows the import library).
    library: PathBuf,
    /// The libraries the test program loads at run time, by file name.
    runtime: Vec<PathBuf>,
}

/// Asks `python` where its home and libraries are, if it is 3.14 or
/// later with a shared library: the home, then the library to link, then
/// the libraries to load (or `-` when unusable).
const PROBE: &str = r#"import os, sys, sysconfig
v = sys.version_info
if sys.platform == "win32":
    name = f"python{v[0]}{v[1]}"
    files = [os.path.join(sys.base_prefix, "libs", name + ".lib"),
             os.path.join(sys.base_prefix, name + ".dll"),
             os.path.join(sys.base_prefix, "python3.dll")]
    shared = True
else:
    lib = os.path.join(sysconfig.get_config_var("LIBDIR") or "",
                       sysconfig.get_config_var("INSTSONAME") or "")
    files = [lib, lib]
    shared = bool(sysconfig.get_config_var("Py_ENABLE_SHARED"))
print(sys.base_prefix)
print("\n".join(files) if v >= (3, 14) and shared else "-")
"#;

/// The installation `python` describes, when its libraries exist.
fn embeddable(python: &Path) -> Option<Embeddable> {
    let output = Command::new(python).args(["-c", PROBE]).output().ok()?;
    let text = String::from_utf8(output.stdout).ok()?;
    let mut lines = text.lines().map(str::trim_end);
    let home = lines.next()?.to_owned();
    let library = PathBuf::from(lines.next()?);
    let runtime: Vec<PathBuf> = lines.map(PathBuf::from).collect();
    let present = library.is_file() && runtime.iter().all(|file| file.is_file());
    (present && !runtime.is_empty()).then_some(Embeddable {
        home,
        library,
        runtime,
    })
}

/// The Pythons to try, in order.
fn candidates() -> Vec<PathBuf> {
    if let Some(python) = std::env::var_os("PLTRS_PYTHON") {
        return vec![python.into()];
    }
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let (venv_python, system_python) = if cfg!(windows) {
        ("Scripts/python.exe", "python")
    } else {
        ("bin/python", "python3")
    };
    vec![
        workspace.join(".venv-pltrs").join(venv_python),
        workspace.join(".venv").join(venv_python),
        system_python.into(),
    ]
}

/// Makes `link` stand for `file`: a symbolic link on Unix, a copy on
/// Windows (where links need privileges).
fn stage(file: &Path, link: &Path) -> bool {
    let _ = std::fs::remove_file(link);
    #[cfg(unix)]
    let staged = std::os::unix::fs::symlink(file, link).is_ok();
    #[cfg(not(unix))]
    let staged = std::fs::copy(file, link).is_ok();
    staged
}

/// The name `found.library` is linked under on `target_os`
/// (`-l pltrs_python_embed`).
fn link_name(target_os: &str) -> &'static str {
    match target_os {
        "windows" => "pltrs_python_embed.lib",
        "macos" => "libpltrs_python_embed.dylib",
        _ => "libpltrs_python_embed.so",
    }
}

/// Tells the linker how the extension finds Python on macOS.
fn link_extension(target_os: &str) {
    if target_os == "macos" {
        for arg in [
            "-undefined",
            "dynamic_lookup",
            &format!("-Wl,-install_name,@rpath/{MACOS_EXTENSION}"),
        ] {
            println!("cargo::rustc-cdylib-link-arg={arg}");
        }
    }
}

fn main() {
    println!("cargo::rustc-check-cfg=cfg(pltrs_python_embed)");
    println!("cargo::rerun-if-env-changed=PLTRS_PYTHON");
    println!("cargo::rerun-if-changed=build.rs");
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    link_extension(&target_os);
    if std::env::var_os("HOST") != std::env::var_os("TARGET") {
        return;
    }
    let Some(found) = candidates().iter().find_map(|python| embeddable(python)) else {
        println!(
            "cargo::warning=no shared Python 3.14+ found: pltrs-python's unit tests are empty"
        );
        return;
    };
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    let embed = out.join("embed");
    let staged = std::fs::create_dir_all(&embed).is_ok()
        && stage(&found.library, &embed.join(link_name(&target_os)))
        && found.runtime.iter().all(|file| {
            file.file_name()
                .is_some_and(|name| stage(file, &embed.join(name)))
        });
    if !staged {
        println!(
            "cargo::warning=cannot link the Python library: pltrs-python's unit tests are empty"
        );
        return;
    }
    println!("cargo::rustc-link-search=native={}", embed.display());
    println!("cargo::rustc-cfg=pltrs_python_embed");
    println!("cargo::rustc-env=PLTRS_PYTHON_HOME={}", found.home);
}
