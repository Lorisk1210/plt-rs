//! Finds a Python to embed in the crate's unit tests.
//!
//! The extension itself links against no Python: an abi3 extension takes
//! the interpreter's symbols from the process that loads it. The unit
//! tests are a program of their own, so they embed an interpreter: this
//! script finds a shared libpython (3.14 or later, whose `PyInitConfig`
//! API sets the interpreter's home) and lets the tests link it.
//!
//! The Python is `PLTRS_PYTHON` if set, else the workspace's pltrs or
//! reference venv, else `python3` on `PATH`. With one found, the script
//! links the library into `OUT_DIR/embed` (where `cargo test` also finds
//! it at run time), and sets the `pltrs_python_embed` cfg and the
//! `PLTRS_PYTHON_HOME` variable the tests read. Without one, the crate's
//! test build is empty.

use std::path::{Path, PathBuf};
use std::process::Command;

/// What the tests need of a Python installation.
struct Embeddable {
    /// `sys.base_prefix`: the interpreter's home.
    home: String,
    /// The shared library.
    library: PathBuf,
}

/// Asks `python` where its shared library and home are, if it has a
/// shared library and is 3.14 or later.
fn embeddable(python: &Path) -> Option<Embeddable> {
    let script = "import sys, sysconfig\n\
                  print(sys.base_prefix)\n\
                  print(sysconfig.get_config_var('LIBDIR') or '')\n\
                  print(sysconfig.get_config_var('INSTSONAME') or '')\n\
                  print(int(sys.version_info >= (3, 14) and bool(sysconfig.get_config_var('Py_ENABLE_SHARED'))))";
    let output = Command::new(python).args(["-c", script]).output().ok()?;
    let text = String::from_utf8(output.stdout).ok()?;
    let mut lines = text.lines();
    let (home, libdir, soname, usable) =
        (lines.next()?, lines.next()?, lines.next()?, lines.next()?);
    let library = Path::new(libdir).join(soname);
    (usable == "1" && library.is_file()).then(|| Embeddable {
        home: home.to_owned(),
        library,
    })
}

/// The Pythons to try, in order.
fn candidates() -> Vec<PathBuf> {
    if let Some(python) = std::env::var_os("PLTRS_PYTHON") {
        return vec![python.into()];
    }
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    vec![
        workspace.join(".venv-pltrs/bin/python"),
        workspace.join(".venv/bin/python"),
        "python3".into(),
    ]
}

fn main() {
    println!("cargo::rustc-check-cfg=cfg(pltrs_python_embed)");
    println!("cargo::rerun-if-env-changed=PLTRS_PYTHON");
    println!("cargo::rerun-if-changed=build.rs");
    let Some(found) = candidates().iter().find_map(|python| embeddable(python)) else {
        println!(
            "cargo::warning=no shared Python 3.14+ found: pltrs-python's unit tests are empty"
        );
        return;
    };
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    let embed = out.join("embed");
    let soname = found
        .library
        .file_name()
        .expect("the library path names a file");
    let links = [embed.join("libpltrs_python_embed.so"), embed.join(soname)];
    let linked = std::fs::create_dir_all(&embed).is_ok()
        && links.iter().all(|link| {
            let _ = std::fs::remove_file(link);
            std::os::unix::fs::symlink(&found.library, link).is_ok()
        });
    if !linked {
        println!(
            "cargo::warning=cannot link the Python library: pltrs-python's unit tests are empty"
        );
        return;
    }
    println!("cargo::rustc-link-search=native={}", embed.display());
    println!("cargo::rustc-cfg=pltrs_python_embed");
    println!("cargo::rustc-env=PLTRS_PYTHON_HOME={}", found.home);
}
