//! `cargo xtask rust-coverage [--strict] [--no-doc]`: how much of
//! matplotlib's public API pltrs (Rust) offers as public Rust API.
//!
//! The callables are those `cargo xtask diff --coverage` measures
//! (`spec/rust-coverage/api.py`, run in the reference venv): matplotlib's
//! public functions, classes, methods and properties. The ledger
//! (`spec/rust-coverage/ledger/<module>.txt`, see [`ledger`]) maps each to
//! the pltrs (Rust) items doing what it does, or says why it does not
//! apply to Rust; deprecated callables do not count. Every path the
//! ledger names is checked against the public API rustdoc documents
//! (each crate's `all.html` and the pages of its items), which this
//! command builds first (`cargo doc --no-deps`; `--no-doc` reads what an
//! earlier build left).
//!
//! It prints the share covered per module, the unmapped callables by
//! class and the ledger's problems, and writes the same to
//! `target/rust-coverage/report.md`, with every callable's decision in
//! `target/rust-coverage/mapping.tsv`. `--strict` fails when a callable is
//! unmapped, or when a ledger line names a callable or an item that is
//! gone, or decides nothing.

mod api;
mod census;
mod ledger;
mod rustdoc;

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::process::{Command, ExitCode};

use census::Census;
use ledger::Ledger;
use rustdoc::{CRATES, Index};

use crate::python::{root, run_checked, target_dir};

/// Runs the `rust-coverage` subcommand.
pub(crate) fn run(args: &[String]) -> ExitCode {
    let mut strict = false;
    let mut build_docs = true;
    for arg in args {
        match arg.as_str() {
            "--strict" => strict = true,
            "--no-doc" => build_docs = false,
            other => {
                eprintln!(
                    "cargo xtask rust-coverage: unknown argument {other:?}; usage: cargo xtask rust-coverage [--strict] [--no-doc]"
                );
                return ExitCode::FAILURE;
            }
        }
    }
    match measure(build_docs) {
        Ok(clean) if clean || !strict => ExitCode::SUCCESS,
        Ok(_) => {
            eprintln!("cargo xtask rust-coverage --strict: the ledger is incomplete or stale");
            ExitCode::FAILURE
        }
        Err(err) => {
            eprintln!("cargo xtask rust-coverage: {err}");
            ExitCode::FAILURE
        }
    }
}

/// Takes the census, prints and writes it; whether it is clean (nothing
/// unmapped, nothing stale).
fn measure(build_docs: bool) -> Result<bool, String> {
    let callables = api::callables()?;
    if build_docs {
        let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let mut command = Command::new(cargo);
        command.args(["doc", "--no-deps", "--quiet"]);
        for package in CRATES {
            command.args(["-p", package]);
        }
        run_checked(&mut command)?;
    }
    let index = Index::read(&target_dir().join("doc"))?;
    let ledger = Ledger::read(&root().join("spec/rust-coverage/ledger"))?;
    let census = Census::take(&callables, &ledger, &index);
    let report = report(&census, index.len());
    print!("{report}");
    let out = root().join("target/rust-coverage");
    fs::create_dir_all(&out).map_err(|err| format!("cannot create {}: {err}", out.display()))?;
    for (name, text) in [
        ("report.md", report),
        ("mapping.tsv", census.mapping(&ledger)),
    ] {
        let path = out.join(name);
        fs::write(&path, text).map_err(|err| format!("cannot write {}: {err}", path.display()))?;
    }
    println!("\nwritten to {}", out.display());
    Ok(census.stale.is_empty() && census.total().unmapped == 0)
}

/// The report: the shares, the unmapped callables and the ledger's problems.
fn report(census: &Census<'_>, items: usize) -> String {
    let total = census.total();
    let mut out = format!(
        "# matplotlib's public API in pltrs (Rust)\n\n\
         Written by `cargo xtask rust-coverage` from the ledger in `spec/rust-coverage/ledger`, \
         checked against the {items} public items rustdoc documents.\n\n\
         * callables: {} ({} deprecated ones left out)\n\
         * mapped to pltrs (Rust): {}\n\
         * not applicable to Rust: {}\n\
         * unmapped: {}\n\
         * covered: {} of the applicable ones\n\n",
        total.callables,
        total.deprecated,
        total.mapped,
        total.not_applicable,
        total.unmapped,
        census::percent(total.covered()),
    );
    out.push_str(&census.table());
    let unmapped = census.unmapped();
    if !unmapped.is_empty() {
        out.push_str("\n## Unmapped\n\n");
        for (group, names) in &unmapped {
            let _ = writeln!(out, "* `{group}` ({}): {}", names.len(), names.join(", "));
        }
    }
    if !census.stale.is_empty() {
        out.push_str("\n## Ledger problems\n\n");
        for problem in &census.stale {
            let _ = writeln!(out, "* {problem}");
        }
    }
    out
}
