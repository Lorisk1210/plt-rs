//! What the report runs of pltrs (Rust) in fresh processes, as a Rust
//! program would: `cargo xtask bench --child cold NAME OUT` makes the data of the
//! workload `NAME`, builds its figure and writes it to `OUT`, as its script
//! does in Python; `--child start` starts and exits, the floor of every
//! cold run. `--child serve NAME` serves timed runs of the workload as the
//! Python harness serves a script, so that the builds `--against`
//! compares run alike.

use std::error::Error;
use std::ffi::OsString;
use std::io::BufRead as _;
use std::path::Path;
use std::process::Command;

use super::cold::ColdCommand;
use super::implementation::Implementation;
use super::pack::Tree;
use super::render::{Documents, render, timed_run, user_configuration};
use super::session::{READY, Run, time_reply};
use super::workloads::{self, Output, Workload};

/// How to call the child modes.
const USAGE: &str = "usage: cargo xtask bench --child start | cold NAME OUT | serve NAME";

/// Runs the child mode `args` asks for.
pub(super) fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let workload =
        |name: &str| workloads::named(name).ok_or_else(|| format!("no workload named {name}"));
    match args {
        [mode] if mode == "start" => Ok(()),
        [mode, name, out] if mode == "cold" => cold(workload(name)?, Path::new(out)),
        [mode, name] if mode == "serve" => serve(workload(name)?),
        _ => Err(USAGE.into()),
    }
}

/// Makes the data of `workload`, builds its figure and writes what it
/// saves to `out`.
fn cold(workload: &Workload, out: &Path) -> Result<(), Box<dyn Error>> {
    user_configuration();
    let built = workload.build();
    let timed = render(workload, &built, workload.output, Documents::AsUsers)?;
    if workload.output != Output::LayoutOnly {
        std::fs::write(out, timed.document)?;
    }
    Ok(())
}

/// Makes the data of `workload`, then answers requests read from standard
/// input as `bench.py serve` does: `run` makes a timed run and replies
/// with its time and the size of its document, `quit` (or the end of the
/// input) ends the process.
fn serve(workload: &'static Workload) -> Result<(), Box<dyn Error>> {
    user_configuration();
    let built = workload.build();
    println!("{READY}");
    for request in std::io::stdin().lock().lines() {
        match request?.as_str() {
            "run" => {
                let timed = timed_run(workload, &built)?;
                println!(
                    "{}",
                    time_reply(Run {
                        seconds: timed.seconds,
                        bytes: timed.document.len() as u64,
                    })
                );
            }
            "quit" => break,
            other => println!("error\tunknown request {other:?}"),
        }
    }
    Ok(())
}

/// The fresh process running `workload` and writing its document to
/// `out`, with the executable of `tree`.
pub(super) fn cold_command(
    tree: &Tree,
    workload: &Workload,
    out: &Path,
) -> Result<ColdCommand, Box<dyn Error>> {
    Ok(ColdCommand {
        label: Implementation::Rust.key().to_owned(),
        argv: argv(
            tree,
            [
                "cold".into(),
                workload.name.into(),
                out.as_os_str().to_owned(),
            ],
        )?,
        env: Vec::new(),
    })
}

/// The fresh process of the executable of `tree` that starts and exits.
pub(super) fn start_command(tree: &Tree) -> Result<ColdCommand, Box<dyn Error>> {
    Ok(ColdCommand {
        label: Implementation::Rust.key().to_owned(),
        argv: argv(tree, ["start".into()])?,
        env: Vec::new(),
    })
}

/// The executable of `tree` serving `workload` (`--child serve`).
pub(super) fn serve_command(tree: &Tree, workload: &Workload) -> Result<Command, Box<dyn Error>> {
    let argv = argv(tree, ["serve".into(), workload.name.into()])?;
    let mut command = Command::new(&argv[0]);
    command.args(&argv[1..]);
    Ok(command)
}

/// The arguments running the child mode `args` of the executable of
/// `tree`, the executable first.
fn argv<const N: usize>(tree: &Tree, args: [OsString; N]) -> Result<Vec<OsString>, Box<dyn Error>> {
    let mut argv: Vec<OsString> = vec![tree.executable()?.into(), "bench".into(), "--child".into()];
    argv.extend(args);
    Ok(argv)
}
