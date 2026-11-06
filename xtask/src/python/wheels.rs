//! `cargo xtask python wheels [TARGET...]`: pltrs (Python)'s release
//! archives, built on this host into `target/wheels/` by the tree's own
//! build backend (`python/backend`) and checked as an index would check
//! them (`python/tools/check_dist.py`):
//!
//! * the sdist, and the pure wheel of the `matplotlib` distribution;
//! * pltrs' wheels for the Rust targets given, by default this system's
//!   release set: on Linux, `x86_64` and `aarch64` linked by Zig against
//!   glibc 2.17 (`manylinux2014`) and against musl (`musllinux_1_2`); on
//!   macOS, `arm64`, `x86_64` and `universal2`; on Windows, the host's.
//!   A Linux glibc target given is linked against glibc 2.17 too.
//!
//! Zig is `ZIG`, else the one `python/tools/zig.sh install` installed.
//! The backend runs in the reference venv's Python.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use pltrs_core::os::Os;

use super::{reference_python, root, run_checked};

/// The glibc the Linux wheels are linked against: manylinux2014's.
const GLIBC: &str = "2.17";

/// The release set of each system.
const LINUX: [&str; 4] = [
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-unknown-linux-musl",
    "aarch64-unknown-linux-musl",
];
const MACOS: [&str; 3] = ["aarch64-apple-darwin", "x86_64-apple-darwin", "universal2"];
const WINDOWS: [&str; 1] = ["x86_64-pc-windows-msvc"];

/// The two macOS targets of a universal2 wheel.
const UNIVERSAL2: [&str; 2] = ["x86_64-apple-darwin", "aarch64-apple-darwin"];

/// Runs one of the backend's hooks (`sdist` or `wheel`) into a
/// directory, with `key=value` config settings, and prints the archive's
/// name.
const BUILD: &str = r#"import sys
sys.path.insert(0, "python/backend")
import pltrs_backend
hook, directory, *settings = sys.argv[1:]
build = pltrs_backend.build_sdist if hook == "sdist" else pltrs_backend.build_wheel
print(build(directory, dict(setting.split("=", 1) for setting in settings)))
"#;

/// Runs the `wheels` subcommand.
pub(crate) fn run(args: &[String]) -> Result<(), String> {
    let targets: Vec<&str> = if args.is_empty() {
        match Os::CURRENT {
            Os::Linux => LINUX.to_vec(),
            Os::MacOs => MACOS.to_vec(),
            Os::Windows => WINDOWS.to_vec(),
        }
    } else {
        args.iter().map(String::as_str).collect()
    };
    for target in &targets {
        check_installed(target)?;
    }
    let out = root().join("target/wheels");
    fs::create_dir_all(&out).map_err(|err| format!("cannot create {}: {err}", out.display()))?;
    let mut built = vec![
        build(&out, "sdist", &[], None)?,
        build(&out, "wheel", &["distribution=matplotlib".into()], None)?,
    ];
    for target in targets {
        let mut settings = vec![format!("target={target}")];
        let zig = if target.contains("-linux-") {
            if target.ends_with("-gnu") {
                settings.push(format!("glibc={GLIBC}"));
            }
            Some(zig()?)
        } else {
            None
        };
        built.push(build(&out, "wheel", &settings, zig.as_deref())?);
    }
    let mut check = Command::new(reference_python());
    check
        .arg(root().join("python/tools/check_dist.py"))
        .args(&built);
    run_checked(&mut check)
}

/// Builds one archive into `out` and returns its path.
fn build(
    out: &Path,
    hook: &str,
    settings: &[String],
    zig: Option<&Path>,
) -> Result<PathBuf, String> {
    let mut command = Command::new(reference_python());
    command
        .args(["-c", BUILD, hook])
        .arg(out)
        .args(settings)
        .current_dir(root());
    if let Some(zig) = zig {
        command.env("ZIG", zig);
    }
    let output = command
        .stderr(std::process::Stdio::inherit())
        .output()
        .map_err(|err| format!("cannot run the reference venv's python: {err}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    match stdout.lines().last() {
        Some(name) if output.status.success() => {
            println!("built {name}");
            Ok(out.join(name.trim()))
        }
        _ => Err(format!(
            "building the {hook} ({}) failed",
            settings.join(" ")
        )),
    }
}

/// Zig: `ZIG`, else `python/tools/zig.sh`'s.
fn zig() -> Result<PathBuf, String> {
    if let Some(zig) = env::var_os("ZIG") {
        return Ok(zig.into());
    }
    let output = Command::new("sh")
        .arg(root().join("python/tools/zig.sh"))
        .arg("path")
        .output()
        .map_err(|err| format!("cannot run python/tools/zig.sh: {err}"))?;
    let path = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    if path.is_file() {
        Ok(path)
    } else {
        Err("Linux wheels are linked by Zig: python/tools/zig.sh install".into())
    }
}

/// Fails unless the standard library of `target` (both halves of
/// `universal2`) is installed.
fn check_installed(target: &str) -> Result<(), String> {
    let output = Command::new(env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .args(["--print", "sysroot"])
        .current_dir(root())
        .output()
        .map_err(|err| format!("cannot run rustc: {err}"))?;
    let sysroot = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    let halves: Vec<&str> = if target == "universal2" {
        UNIVERSAL2.to_vec()
    } else {
        vec![target]
    };
    for half in halves {
        if !sysroot.join("lib/rustlib").join(half).join("lib").is_dir() {
            return Err(format!(
                "the Rust target {half} is not installed: rustup target add {half}"
            ));
        }
    }
    Ok(())
}
