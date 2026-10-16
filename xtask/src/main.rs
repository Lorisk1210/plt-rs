//! Developer automation for plt-rs.
//!
//! ```text
//! cargo xtask spec [AREA...] [--failures]   conformance summary vs matplotlib goldens
//! ```
#![forbid(unsafe_code)]
#![allow(clippy::print_stdout, clippy::print_stderr)] // a CLI reports on stdout

use std::process::ExitCode;

use plt_spec::probes::{self, AreaReport, ProbeFile};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.split_first() {
        Some((command, rest)) if command == "spec" => spec(rest),
        _ => {
            eprintln!("usage: cargo xtask spec [AREA...] [--failures]");
            ExitCode::FAILURE
        }
    }
}

/// Replays every probe golden against plt-rs and prints a summary table.
fn spec(args: &[String]) -> ExitCode {
    let show_failures = args.iter().any(|a| a == "--failures");
    let areas: Vec<&str> = args
        .iter()
        .filter(|a| !a.starts_with("--"))
        .map(String::as_str)
        .collect();

    let mut paths: Vec<_> = match std::fs::read_dir(plt_spec::probe_goldens_dir()) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "json"))
            .collect(),
        Err(err) => {
            eprintln!("cannot list probe goldens: {err}");
            return ExitCode::FAILURE;
        }
    };
    paths.sort();

    let mut reports = Vec::new();
    for path in paths {
        match ProbeFile::load(&path) {
            Ok(file) if areas.is_empty() || areas.contains(&file.area.as_str()) => {
                reports.push(probes::run(&file, probes::ops_for(&file.area)));
            }
            Ok(_) => {}
            Err(err) => {
                eprintln!("{}: {err}", path.display());
                return ExitCode::FAILURE;
            }
        }
    }

    print_summary(&reports, show_failures);
    if reports
        .iter()
        .all(|r| r.failures.is_empty() && r.unimplemented.is_empty())
    {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn print_summary(reports: &[AreaReport], show_failures: bool) {
    println!(
        "{:<20} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "area", "pass", "fail", "waived", "missing", "total"
    );
    let mut totals = [0_usize; 5];
    for report in reports {
        let missing: usize = report.unimplemented.values().sum();
        let row = [
            report.passed,
            report.failures.len(),
            report.waived.len(),
            missing,
            report.total(),
        ];
        println!(
            "{:<20} {:>8} {:>8} {:>8} {:>8} {:>8}",
            report.area, row[0], row[1], row[2], row[3], row[4]
        );
        for (total, value) in totals.iter_mut().zip(row) {
            *total += value;
        }
    }
    let [pass, fail, waived, missing, total] = totals;
    let percent = if total == 0 {
        0.0
    } else {
        100.0 * pass as f64 / total as f64
    };
    println!(
        "{:<20} {pass:>8} {fail:>8} {waived:>8} {missing:>8} {total:>8}  ({percent:.1}% passing)",
        "TOTAL"
    );

    if show_failures {
        for report in reports {
            for failure in &report.failures {
                println!("FAIL [{}] {failure}", report.area);
            }
            for waived in &report.waived {
                println!("WAIVED [{}] {waived}", report.area);
            }
            for (op, count) in &report.unimplemented {
                println!("MISSING [{}] {op} ({count} probes)", report.area);
            }
        }
    }
}
