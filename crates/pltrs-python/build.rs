//! Links the extension, and finds a Python to embed in the crate's unit
//! tests.
//!
//! The extension itself links against no Python: an abi3 extension takes
//! the interpreter's symbols from the process that loads it. On macOS the
//! linker must be told so (`-undefined dynamic_lookup`), and the library
//! gets the install name of the file it is installed as. (On Linux shared
//! libraries may leave symbols undefined; on Windows the declarations
//! name `python3.dll`, see `ffi`.) On Windows with a GNU toolchain the
//! script links `python3.dll` itself, from the first of the Pythons below
//! (see [`link_python3`]).
//!
//! On x86-64 Linux the extension's functions are laid out in the order of
//! `symbol-order.txt`, which `tools/symbol_order.py` writes from what the
//! benchmark scripts run: the code every program runs first, then the
//! code fewer programs run. A process maps the library's code a page at a
//! time and the pages around it with it, so that this halves the code a
//! process holds (a small figure's 11.4 MiB became 5.0 MiB). The order
//! goes to rustc's own linker for the target, LLD (which the wheels' Zig
//! links with too); a linker configured for the target or in `RUSTFLAGS`
//! gets none.
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
//! cross-compiling, the crate's test build is empty (which a build of the
//! tests, not of the optimised extension, warns of).

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

/// What `python` prints running `code`, when it runs.
fn run_python(python: &Path, code: &str) -> Option<String> {
    let output = Command::new(python).args(["-c", code]).output().ok()?;
    String::from_utf8(output.stdout).ok()
}

/// The installation `python` describes, when its libraries exist.
fn embeddable(python: &Path) -> Option<Embeddable> {
    let text = run_python(python, PROBE)?;
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

/// The folder `libs` of `python`'s installation, when it holds the
/// import library of the stable ABI's `python3.dll`, `python3.lib` (the
/// GNU linkers read MSVC's import libraries; an embeddable Python has
/// none).
fn python3_link_dir(python: &Path) -> Option<PathBuf> {
    let text = run_python(python, "import sys; print(sys.base_prefix)")?;
    let libs = Path::new(text.trim_end()).join("libs");
    libs.join("python3.lib").is_file().then_some(libs)
}

/// Links the extension against `python3.dll` on Windows with a GNU
/// toolchain, where rustc's `raw-dylib` runs binutils' `dlltool` and
/// assembler, which rustup's GNU toolchains do not ship: with the import
/// library of the first of the [`candidates`] that has one, setting
/// `pltrs_python_link_python3`. Without one the declarations keep
/// `raw-dylib`, which then needs MinGW-w64's binutils on `PATH`.
fn link_python3() {
    match candidates()
        .iter()
        .find_map(|python| python3_link_dir(python))
    {
        Some(dir) => {
            println!("cargo::rustc-link-search=native={}", dir.display());
            println!("cargo::rustc-cfg=pltrs_python_link_python3");
        }
        None => println!(
            "cargo::warning=no Python with libs/python3.lib found (set PLTRS_PYTHON): \
             raw-dylib needs MinGW-w64's dlltool and as on PATH"
        ),
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

/// The order of the extension's functions (see the module's
/// documentation), relative to the crate.
const SYMBOL_ORDER: &str = "symbol-order.txt";

/// Lays the extension's functions out in [`SYMBOL_ORDER`]'s order where
/// rustc links with LLD through the C compiler: on x86-64 Linux with GNU's
/// C library, unless a linker is configured for the target or the flags
/// in `RUSTFLAGS` change how it links. A function the order does not name
/// (the code has changed since it was written) is laid out after those it
/// does.
fn order_functions(target_os: &str) {
    println!("cargo::rerun-if-changed={SYMBOL_ORDER}");
    let var = |name| std::env::var(name).unwrap_or_default();
    let linked_by_lld = target_os == "linux"
        && var("CARGO_CFG_TARGET_ARCH") == "x86_64"
        && var("CARGO_CFG_TARGET_ENV") == "gnu"
        && std::env::var_os("RUSTC_LINKER").is_none()
        && !var("CARGO_ENCODED_RUSTFLAGS").contains("link");
    if linked_by_lld {
        let crate_dir = PathBuf::from(var("CARGO_MANIFEST_DIR"));
        let order = crate_dir.join(SYMBOL_ORDER);
        println!(
            "cargo::rustc-cdylib-link-arg=-Wl,--symbol-ordering-file={}",
            order.display()
        );
        println!("cargo::rustc-cdylib-link-arg=-Wl,--no-warn-symbol-ordering");
    }
}

/// Tells the crate the cargo profile it is built with, as
/// `PLTRS_BUILD_PROFILE`: the directory cargo builds that profile in
/// (`OUT_DIR` is `<target>/[<triple>/]<profile>/build/<package>-<hash>/out`),
/// so `extension` for `cargo xtask python --release` and the wheels, whose
/// functions [`SYMBOL_ORDER`] names, and `debug` for the `dev` profile.
fn tell_profile() {
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap_or_default());
    let profile = out
        .ancestors()
        .nth(3)
        .and_then(|dir| dir.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("unknown");
    println!("cargo::rustc-env=PLTRS_BUILD_PROFILE={profile}");
}

fn main() {
    println!("cargo::rustc-check-cfg=cfg(pltrs_python_embed)");
    tell_profile();
    println!("cargo::rustc-check-cfg=cfg(pltrs_python_link_python3)");
    println!("cargo::rerun-if-env-changed=PLTRS_PYTHON");
    println!("cargo::rerun-if-changed=build.rs");
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    link_extension(&target_os);
    order_functions(&target_os);
    if std::env::var_os("HOST") != std::env::var_os("TARGET") {
        return;
    }
    if target_os == "windows" && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
        link_python3();
    }
    let Some(found) = candidates().iter().find_map(|python| embeddable(python)) else {
        // Said where tests are built from: not in the optimised builds of
        // the extension and the wheels (`PROFILE` is `release` for every
        // profile inheriting it), whose Python may well be older.
        if std::env::var("PROFILE").as_deref() != Ok("release") {
            println!(
                "cargo::warning=no shared Python 3.14+ found: pltrs-python's unit tests are empty"
            );
        }
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
