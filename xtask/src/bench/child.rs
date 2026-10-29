//! What the report runs of plt in fresh processes, as a Rust program
//! would: `cargo xtask bench --child cold NAME OUT` makes the data of the
//! workload `NAME`, builds its figure and writes it to `OUT`, as its script
//! does in Python; `--child start` starts and exits, the floor of every
//! cold run.

use std::error::Error;
use std::ffi::OsString;
use std::path::Path;

use super::cold::ColdCommand;
use super::render::{render, user_configuration};
use super::workloads::{self, Output, Workload};

/// Runs the child mode `args` asks for.
pub(super) fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    match args {
        [mode] if mode == "start" => Ok(()),
        [mode, name, out] if mode == "cold" => {
            let workload =
                workloads::named(name).ok_or_else(|| format!("no workload named {name}"))?;
            cold(workload, Path::new(out))
        }
        _ => Err("usage: cargo xtask bench --child start | cold NAME OUT".into()),
    }
}

/// Makes the data of `workload`, builds its figure and writes what it
/// saves to `out`.
fn cold(workload: &Workload, out: &Path) -> Result<(), Box<dyn Error>> {
    user_configuration();
    let build = (workload.setup)();
    let document = render(workload, &build, workload.output)?;
    if workload.output != Output::LayoutOnly {
        std::fs::write(out, document)?;
    }
    Ok(())
}

/// The fresh process running `workload` and writing its document to
/// `out`, with this executable.
pub(super) fn cold_command(workload: &Workload, out: &Path) -> Result<ColdCommand, Box<dyn Error>> {
    command(vec![
        "cold".into(),
        workload.name.into(),
        out.as_os_str().to_owned(),
    ])
}

/// The fresh process that starts and exits.
pub(super) fn start_command() -> Result<ColdCommand, Box<dyn Error>> {
    command(vec!["start".into()])
}

/// This executable's child mode `args`.
fn command(args: Vec<OsString>) -> Result<ColdCommand, Box<dyn Error>> {
    let mut argv: Vec<OsString> = vec![
        std::env::current_exe()?.into(),
        "bench".into(),
        "--child".into(),
    ];
    argv.extend(args);
    Ok(ColdCommand {
        label: "plt".to_owned(),
        argv,
        env: Vec::new(),
    })
}
