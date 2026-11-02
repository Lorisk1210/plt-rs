//! Developer automation for pltrs.
//!
//! ```text
//! cargo xtask spec [AREA...] [--failures]        probe conformance vs matplotlib goldens
//! cargo xtask figures [PREFIX...] [--failures]   figure-case conformance
//! cargo xtask render PREFIX... [--vectors]       render cases to spec/out
//! cargo xtask render --look LOOK PREFIX...       render cases in a pltrs look to spec/out/look-LOOK
//! cargo xtask vectors [PREFIX...] [--formats F]  SVG/PDF/PS/PGF of every case vs matplotlib
//! cargo xtask bench [PREFIX...] [--reference]     timings of the benchmark workloads (vs matplotlib)
//! cargo xtask bench [PREFIX...] --report         matplotlib and pltrs (Python, Rust) timed: benchmarks/
//! cargo xtask bench --rewrite-report             benchmarks/README.md and charts again from results.json
//! cargo xtask mpl-tests [--strict]               matplotlib's own tests: ported, n/a or unaccounted
//! cargo xtask python [--release] | setup | test   build pltrs (Python)'s extension, make its venvs, smoke-test it
//! cargo xtask mpl-suite [--reference] [PATTERN...] [--failures] [--imports]
//!                                                 matplotlib's own test suite run against pltrs (or matplotlib)
//! cargo xtask ecosystem [setup] [PYTEST ARGS...]  pandas and seaborn drawing with pltrs as matplotlib
//! cargo xtask diff [--gallery | --generated] [PATTERN...] [--failures] [--reduce] [--baseline REPORT]
//!                                                 programs run under matplotlib and pltrs (Python), compared
//! ```
#![forbid(unsafe_code)]
#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a command-line tool reports on stdout and stderr"
)]

mod bench;
mod diff;
mod ecosystem;
mod examples;
mod mpl_suite;
mod mpl_tests;
mod python;

use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;

use pltrs::look::FigureLook;
use pltrs_spec::figures::vector::{VectorFormat, render_vector};
use pltrs_spec::figures::{self, CaseVerdict, FigureCase};
use pltrs_spec::probes::{self, AreaReport, ProbeFile};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.split_first() {
        Some((command, rest)) if command == "spec" => spec(rest),
        Some((command, rest)) if command == "figures" => figure_cases(rest),
        Some((command, rest))
            if command == "render" && rest.iter().any(|a| !a.starts_with("--")) =>
        {
            render(rest)
        }
        Some((command, rest)) if command == "vectors" => vectors(rest),
        Some((command, rest)) if command == "bench" => bench::run(rest),
        Some((command, rest)) if command == "mpl-tests" => mpl_tests::run(rest),
        Some((command, rest)) if command == "python" => python::run(rest),
        Some((command, rest)) if command == "mpl-suite" => mpl_suite::run(rest),
        Some((command, rest)) if command == "examples" => examples::run(rest),
        Some((command, rest)) if command == "ecosystem" => ecosystem::run(rest),
        Some((command, rest)) if command == "diff" => diff::run(rest),
        _ => {
            eprintln!("usage: cargo xtask spec [AREA...] [--failures]");
            eprintln!("       cargo xtask figures [PREFIX...] [--failures]");
            eprintln!("       cargo xtask render PREFIX... [--vectors]");
            eprintln!("       cargo xtask render --look modern|modern-dark PREFIX...");
            eprintln!("       cargo xtask vectors [PREFIX...] [--formats svg,pdf,ps,pgf]");
            eprintln!("       cargo xtask bench [PREFIX...] [--reference | --report] [--runs N]");
            eprintln!("       cargo xtask bench --rewrite-report [--to DIR]");
            eprintln!("       cargo xtask mpl-tests [--strict]");
            eprintln!("       cargo xtask python [--release] | setup | test");
            eprintln!(
                "       cargo xtask mpl-suite [--reference] [PATTERN...] [--failures] [--imports]"
            );
            eprintln!("       cargo xtask ecosystem [setup] [PYTEST ARGS...]");
            eprintln!(
                "       cargo xtask diff [--gallery | --generated] [PATTERN...] [--failures] [--reduce] [--baseline REPORT]"
            );
            ExitCode::FAILURE
        }
    }
}

/// The wide comparison of the cases whose names start with a prefix (all
/// without one): matplotlib writes their documents in the `--formats`
/// asked for (default all) into `spec/out/vector-reference`
/// (`render_vectors.py` through the reference venv), then pltrs's are
/// compared with them item by item.
fn vectors(args: &[String]) -> ExitCode {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let reference = root.join("spec/out/vector-reference");
    let mut python = std::process::Command::new(python::reference_python());
    python
        .arg(root.join("spec/reference/figures/render_vectors.py"))
        .arg(&reference);
    let mut prefixes = Vec::new();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        if arg == "--formats" {
            python.arg("--formats").args(rest.next());
        } else {
            prefixes.push(arg.as_str());
        }
    }
    python.args(&prefixes);
    match python.status() {
        Ok(status) if status.success() => {}
        Ok(status) => {
            eprintln!("render_vectors.py failed: {status}");
            return ExitCode::FAILURE;
        }
        Err(err) => {
            eprintln!("cannot run the reference venv's python: {err}");
            return ExitCode::FAILURE;
        }
    }
    let cases: Vec<_> = figures::translated_cases()
        .into_iter()
        .filter(|case| prefixes.is_empty() || prefixes.iter().any(|p| case.name.starts_with(p)))
        .collect();
    let results = pltrs_spec::figures::vector::wide_comparison(&cases, &reference);
    print!("{}", pltrs_spec::figures::vector::wide_summary(&results));
    ExitCode::SUCCESS
}

/// Renders the translated cases whose names start with a prefix into
/// `spec/out/<case>.png`, with `<case>.diff.png` marking the pixels that
/// differ from the golden image; `--vectors` also writes the case as SVG,
/// PDF and PostScript the way the vector goldens are saved. `--look LOOK`
/// renders in one of pltrs' figure looks instead (`modern`,
/// `modern-dark`) into `spec/out/look-LOOK/<case>.png` (PNG only, no diff:
/// the goldens show matplotlib's look).
fn render(args: &[String]) -> ExitCode {
    let mut vectors = false;
    let mut look = None;
    let mut prefixes = Vec::new();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--vectors" => vectors = true,
            "--look" => match rest.next().map(|name| name.parse::<FigureLook>()) {
                Some(Ok(chosen)) => look = Some(chosen),
                Some(Err(err)) => {
                    eprintln!("--look: {err}");
                    return ExitCode::FAILURE;
                }
                None => {
                    eprintln!("--look needs a look: modern or modern-dark");
                    return ExitCode::FAILURE;
                }
            },
            prefix => prefixes.push(prefix),
        }
    }
    let cases: Vec<_> = figures::translated_cases()
        .into_iter()
        .filter(|case| prefixes.iter().any(|p| case.name.starts_with(p)))
        .collect();
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("../spec/out");
    match look {
        Some(look) if look != FigureLook::Matplotlib => {
            render_in_look(&cases, look, &out.join(format!("look-{look}")))
        }
        _ => render_against_goldens(&cases, vectors, &out),
    }
}

/// Writes each case rendered in `look` to `out/<case>.png`.
fn render_in_look(cases: &[FigureCase], look: FigureLook, out: &Path) -> ExitCode {
    let mut status = ExitCode::SUCCESS;
    for case in cases {
        let path = out.join(format!("{}.png", case.name));
        let written = figures::look_png(case, look).and_then(|png| {
            std::fs::create_dir_all(path.parent().unwrap_or(out))
                .and_then(|()| std::fs::write(&path, png))
                .map_err(|e| e.to_string())
        });
        match written {
            Ok(()) => println!("{:<44} {}", case.name, path.display()),
            Err(err) => {
                eprintln!("{}: {err}", case.name);
                status = ExitCode::FAILURE;
            }
        }
    }
    status
}

/// Writes each case's rendering and its diff against the golden image to
/// `out`, and its vector documents with `vectors`.
fn render_against_goldens(cases: &[FigureCase], vectors: bool, out: &Path) -> ExitCode {
    let mut status = ExitCode::SUCCESS;
    for case in cases {
        let written = figures::pixel_artifacts(case).and_then(|artifacts| {
            let base = out.join(case.name);
            let dir = base.parent().unwrap_or(out);
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            std::fs::write(base.with_extension("png"), &artifacts.rendered)
                .and_then(|()| std::fs::write(base.with_extension("diff.png"), &artifacts.diff))
                .map_err(|e| e.to_string())?;
            if vectors {
                for format in VectorFormat::ALL {
                    let document = render_vector(case, format).map_err(|e| e.to_string())?;
                    std::fs::write(base.with_extension(format.extension()), document)
                        .map_err(|e| e.to_string())?;
                }
            }
            Ok(artifacts.report)
        });
        match written {
            Ok(report) => println!("{:<44} {report}", case.name),
            Err(err) => {
                eprintln!("{}: {err}", case.name);
                status = ExitCode::FAILURE;
            }
        }
    }
    status
}

/// Runs every figure case (or those whose name starts with a prefix) and
/// prints a per-group summary.
fn figure_cases(args: &[String]) -> ExitCode {
    let show_failures = args.iter().any(|a| a == "--failures");
    let prefixes: Vec<&str> = args
        .iter()
        .filter(|a| !a.starts_with("--"))
        .map(String::as_str)
        .collect();
    let selected = |name: &str| prefixes.is_empty() || prefixes.iter().any(|p| name.starts_with(p));
    let translated = figures::translated_cases();

    let mut groups: BTreeMap<String, [usize; 4]> = BTreeMap::new();
    let mut failures = Vec::new();
    let mut skipped = Vec::new();
    for name in figures::golden_case_names()
        .into_iter()
        .filter(|n| selected(n))
    {
        let group = name.split('/').next().unwrap_or_default().to_owned();
        let verdict = translated
            .iter()
            .find(|case| case.name == name)
            .map_or(CaseVerdict::Untranslated, figures::judge);
        let counts = groups.entry(group).or_default();
        match verdict {
            CaseVerdict::Pass => counts[0] += 1,
            CaseVerdict::Fail(why) => {
                counts[1] += 1;
                failures.push(format!("FAIL {name}: {why}"));
            }
            CaseVerdict::Skipped(why) => {
                counts[2] += 1;
                skipped.push(format!("SKIP {name}: {why}"));
            }
            CaseVerdict::Untranslated => counts[3] += 1,
        }
    }

    println!(
        "{:<20} {:>8} {:>8} {:>8} {:>12}",
        "group", "pass", "fail", "skip", "untranslated"
    );
    let mut totals = [0_usize; 4];
    for (group, counts) in &groups {
        println!(
            "{group:<20} {:>8} {:>8} {:>8} {:>12}",
            counts[0], counts[1], counts[2], counts[3]
        );
        for (total, n) in totals.iter_mut().zip(counts) {
            *total += n;
        }
    }
    let [pass, fail, skip, untranslated] = totals;
    println!(
        "{:<20} {pass:>8} {fail:>8} {skip:>8} {untranslated:>12}",
        "TOTAL"
    );
    if let Some(first) = skipped.first() {
        println!("{} case(s) skipped, e.g. {first}", skipped.len());
    }
    if show_failures {
        for failure in &failures {
            println!("{failure}");
        }
    }
    if fail == 0 && untranslated == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Replays every probe golden against pltrs and prints a summary table.
fn spec(args: &[String]) -> ExitCode {
    let show_failures = args.iter().any(|a| a == "--failures");
    let areas: Vec<&str> = args
        .iter()
        .filter(|a| !a.starts_with("--"))
        .map(String::as_str)
        .collect();

    let mut paths: Vec<_> = match std::fs::read_dir(pltrs_spec::probe_goldens_dir()) {
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
        "{:<20} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "area", "pass", "fail", "waived", "skipped", "missing", "total"
    );
    let mut totals = [0_usize; 6];
    for report in reports {
        let missing: usize = report.unimplemented.values().sum();
        let row = [
            report.passed,
            report.failures.len(),
            report.waived.len(),
            report.skipped.len(),
            missing,
            report.total(),
        ];
        println!(
            "{:<20} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
            report.area, row[0], row[1], row[2], row[3], row[4], row[5]
        );
        for (total, value) in totals.iter_mut().zip(row) {
            *total += value;
        }
    }
    let [pass, fail, waived, skipped, missing, total] = totals;
    let percent = if total == skipped {
        0.0
    } else {
        100.0 * pass as f64 / (total - skipped) as f64
    };
    println!(
        "{:<20} {pass:>8} {fail:>8} {waived:>8} {skipped:>8} {missing:>8} {total:>8}  \
         ({percent:.1}% of those run passing)",
        "TOTAL"
    );
    if let Some(reason) = reports.iter().find_map(|r| r.skipped.first()) {
        println!("{skipped} probe(s) skipped, e.g. {reason}");
    }

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
