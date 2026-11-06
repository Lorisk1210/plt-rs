//! `cargo xtask examples [NAME...] [--check] [--require-pltrs]`: the gallery of `examples/`,
//! every graph drawn by a Rust binary (`examples/rust`, the
//! `pltrs-examples` crate) and by a Python script (`examples/python`,
//! written for pltrs in matplotlib's syntax).
//!
//! * The Rust examples are built and run, and write their images to
//!   `examples/images/`, the committed images of the gallery
//!   (`--check`: to `target/examples/rust/`).
//! * The Python examples run under real matplotlib in the reference venv
//!   (`.venv`), writing to `target/examples/python-mpl/`:
//!   `run_matplotlib.py` answers their `pltrs` imports with matplotlib
//!   and draws in the modern look, as pltrs (Python) does.
//! * Where pltrs can be imported ([`runners::Pltrs::find`]: by the pltrs
//!   venv's Python, `.venv-pltrs`, or `PLTRS_PYTHON`, with `python/` on
//!   `PYTHONPATH`), they also run under pltrs, writing to
//!   `target/examples/python-pltrs/`.
//! * A table gives the RMS difference between each Rust image and the
//!   Python ones. All draw in the modern look, so they nearly agree.
//!
//! * The tutorials (`examples/rust/tutorials`, matplotlib's tutorials as
//!   cargo examples of the same crate) are built and run, each in a
//!   directory of its own under `target/examples/tutorials/`, where they
//!   write their files.
//!
//! `--check` fails when an example or a tutorial does not run, a
//! committed image is not
//! the one the Rust example draws now, `examples/README.md` misses an
//! image or a source, `examples/images/` holds an image of no example, or
//! the Rust and matplotlib images of an example differ by more than
//! [`TOLERANCE`]. A failing tutorial fails the command without `--check`
//! too. Runs under pltrs are reported, and count as problems only with
//! `--require-pltrs`, until pltrs runs every example. Names restrict the
//! run to those examples and tutorials.

mod catalogue;
mod compare;
mod runners;

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use catalogue::Layout;

/// The largest RMS difference `--check` accepts between the Rust and the
/// matplotlib image of an example: the tolerance of the spec's pixel
/// comparisons.
const TOLERANCE: f64 = 1.0;

/// What `cargo xtask examples` was asked to do.
struct Request<'a> {
    /// Only these examples (all without any).
    names: Vec<&'a str>,
    /// Check the gallery instead of updating its images.
    check: bool,
    /// Count examples pltrs cannot run, or draws differently, as problems.
    require_pltrs: bool,
}

impl<'a> Request<'a> {
    fn parse(args: &'a [String]) -> Result<Self, String> {
        let mut request = Self {
            names: Vec::new(),
            check: false,
            require_pltrs: false,
        };
        for arg in args {
            match arg.as_str() {
                "--check" => request.check = true,
                "--require-pltrs" => request.require_pltrs = true,
                flag if flag.starts_with("--") => return Err(format!("unknown flag {flag}")),
                name => request.names.push(name),
            }
        }
        Ok(request)
    }

    /// The examples of `gallery` and the tutorials of `tutorials` the
    /// request names, or all of them.
    fn select(
        &self,
        gallery: &[String],
        tutorials: &[String],
    ) -> Result<(Vec<String>, Vec<String>), String> {
        let known = |name: &&str| gallery.iter().chain(tutorials).any(|n| n == *name);
        if let Some(unknown) = self.names.iter().find(|name| !known(name)) {
            return Err(format!("no example or tutorial is named {unknown}"));
        }
        let pick = |all: &[String]| -> Vec<String> {
            all.iter()
                .filter(|name| self.names.is_empty() || self.names.contains(&name.as_str()))
                .cloned()
                .collect()
        };
        Ok((pick(gallery), pick(tutorials)))
    }
}

/// How the Rust image of an example compares with a Python one.
#[derive(Debug)]
enum Difference {
    /// The Python version was not run, or an image is missing.
    Absent,
    /// The RMS of the colour differences.
    Rms(f64),
    /// The images cannot be compared.
    Incomparable(String),
}

impl Difference {
    /// The Rust image `rust` against the Python image of `name` in `dir`.
    fn of(rust: Option<&Path>, dir: Option<&Path>, name: &str) -> Self {
        let python = dir.and_then(|dir| catalogue::image(dir, name));
        match (rust, python) {
            (Some(rust), Some(python)) => match compare::rms(rust, &python) {
                Ok(rms) => Self::Rms(rms),
                Err(err) => Self::Incomparable(err),
            },
            _ => Self::Absent,
        }
    }
}

impl fmt::Display for Difference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Absent => f.pad("-"),
            Self::Rms(rms) => f.pad(&format!("{rms:.3}")),
            Self::Incomparable(_) => f.pad("incomparable"),
        }
    }
}

/// One example's images and how they compare.
#[derive(Debug)]
struct Row {
    name: String,
    /// The image of the Rust example, if it wrote one.
    image: Option<PathBuf>,
    /// Against the image matplotlib drew.
    matplotlib: Difference,
    /// Against the image pltrs drew.
    pltrs: Difference,
}

/// Prints the comparison table.
fn print_table(rows: &[Row]) {
    println!(
        "{:<20} {:>20} {:>16}",
        "example", "Rust vs matplotlib", "Rust vs pltrs"
    );
    for row in rows {
        println!("{:<20} {:>20} {:>16}", row.name, row.matplotlib, row.pltrs);
    }
    println!("(RMS of the colour differences, 0-255; animations by their first frames)");
}

/// Creates `dir` and removes the images of the examples `names` from it.
fn prepare(dir: &Path, names: &[String]) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|err| format!("cannot create {}: {err}", dir.display()))?;
    catalogue::clear_images(dir, names)
}

/// Runs the examples the request selects, prints the comparison table and
/// returns the problems found.
fn gallery(layout: &Layout, request: &Request<'_>) -> Result<Vec<String>, String> {
    let all = catalogue::discover(layout)?;
    let tutorials = catalogue::tutorials(layout)?;
    let (names, tutorials) = request.select(&all, &tutorials)?;
    let bin_dir = runners::build_rust()?;
    let mut problems = Vec::new();
    if !names.is_empty() {
        problems.extend(compare_gallery(layout, request, &all, &names, &bin_dir)?);
    }
    let failures = runners::run_tutorials(&bin_dir, &tutorials, &layout.output("tutorials"));
    println!(
        "tutorials: {} of {} ran (in {})",
        tutorials.len() - failures.len(),
        tutorials.len(),
        layout.output("tutorials").display()
    );
    problems.extend(failures);
    Ok(problems)
}

/// Runs the gallery examples `names` (of `all`) in Rust, under matplotlib
/// and under pltrs, prints the comparison table and returns the problems
/// found.
fn compare_gallery(
    layout: &Layout,
    request: &Request<'_>,
    all: &[String],
    names: &[String],
    bin_dir: &Path,
) -> Result<Vec<String>, String> {
    let rust_out = if request.check {
        layout.output("rust")
    } else {
        layout.images()
    };
    let (mpl_out, pltrs_out) = (layout.output("python-mpl"), layout.output("python-pltrs"));
    for dir in [&rust_out, &mpl_out, &pltrs_out] {
        prepare(dir, names)?;
    }

    let mut problems = runners::run_rust(bin_dir, names, &rust_out);
    problems.extend(runners::run_matplotlib(layout, names, &mpl_out));
    let pltrs_ran = match runners::Pltrs::find(layout) {
        Ok(pltrs) => {
            let failures = pltrs.run(layout, names, &pltrs_out);
            if request.require_pltrs {
                problems.extend(failures);
            } else {
                for failure in &failures {
                    println!("pltrs: {failure}");
                }
            }
            true
        }
        Err(why) => {
            println!("pltrs: not run, {why}");
            false
        }
    };

    let rows: Vec<Row> = names
        .iter()
        .map(|name| {
            let image = catalogue::image(&rust_out, name);
            Row {
                matplotlib: Difference::of(image.as_deref(), Some(&mpl_out), name),
                pltrs: Difference::of(image.as_deref(), pltrs_ran.then_some(&*pltrs_out), name),
                name: name.clone(),
                image,
            }
        })
        .collect();
    print_table(&rows);
    for row in &rows {
        let pltrs = request.require_pltrs.then_some(&row.pltrs);
        for difference in std::iter::once(&row.matplotlib).chain(pltrs) {
            if let Difference::Incomparable(why) = difference {
                problems.push(format!("{}: {why}", row.name));
            }
        }
    }
    if request.check {
        problems.extend(check(layout, all, &rows)?);
    }
    Ok(problems)
}

/// What `--check` finds wrong with the gallery, given the fresh images
/// of `rows` and every example's name in `all`.
fn check(layout: &Layout, all: &[String], rows: &[Row]) -> Result<Vec<String>, String> {
    let readme_path = layout.readme();
    let readme = fs::read_to_string(&readme_path)
        .map_err(|err| format!("cannot read {}: {err}", readme_path.display()))?;
    let mut problems = Vec::new();
    for row in rows {
        let Some(fresh) = &row.image else { continue };
        match catalogue::image(&layout.images(), &row.name) {
            Some(committed) if compare::same_bytes(fresh, &committed)? => {}
            Some(committed) => problems.push(format!(
                "{} is out of date: run `cargo xtask examples`",
                committed.display()
            )),
            None => problems.push(format!(
                "examples/images has no image of {}: run `cargo xtask examples`",
                row.name
            )),
        }
        problems.extend(catalogue::readme_gaps(&readme, &row.name, fresh));
        if let Difference::Rms(rms) = row.matplotlib
            && rms > TOLERANCE
        {
            problems.push(format!(
                "{}: the Rust and matplotlib images differ by an RMS of {rms:.3} (at most {TOLERANCE})",
                row.name
            ));
        }
    }
    for stray in catalogue::strays(&layout.images(), all)? {
        problems.push(format!("{} belongs to no example", stray.display()));
    }
    Ok(problems)
}

/// The workspace root.
fn root() -> PathBuf {
    let xtask = Path::new(env!("CARGO_MANIFEST_DIR"));
    xtask.parent().unwrap_or(xtask).to_owned()
}

/// Runs `cargo xtask examples`.
pub(crate) fn run(args: &[String]) -> ExitCode {
    let request = match Request::parse(args) {
        Ok(request) => request,
        Err(err) => {
            eprintln!("{err}");
            eprintln!("usage: cargo xtask examples [NAME...] [--check] [--require-pltrs]");
            return ExitCode::FAILURE;
        }
    };
    match gallery(&Layout::new(root()), &request) {
        Ok(problems) if problems.is_empty() => ExitCode::SUCCESS,
        Ok(problems) => {
            for problem in &problems {
                eprintln!("{problem}");
            }
            ExitCode::FAILURE
        }
        Err(err) => {
            eprintln!("cargo xtask examples: {err}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(names: &[&str]) -> Vec<String> {
        names.iter().map(|&name| name.to_owned()).collect()
    }

    #[test]
    fn requests_select_examples_by_name() {
        let args = names(&["polar", "layouts", "--check"]);
        let request = Request::parse(&args).unwrap();
        assert!(request.check);
        let all = names(&["line_plot", "polar"]);
        let tutorials = names(&["layouts", "saving"]);
        let (gallery, chosen) = request.select(&all, &tutorials).unwrap();
        assert_eq!((gallery, chosen), (names(&["polar"]), names(&["layouts"])));
        let everything = Request::parse(&[]).unwrap();
        assert_eq!(
            everything.select(&all, &tutorials).unwrap(),
            (all, tutorials)
        );
    }

    #[test]
    fn unknown_names_and_flags_are_refused() {
        let args = names(&["nothing"]);
        let request = Request::parse(&args).unwrap();
        assert!(
            request
                .select(&names(&["polar"]), &names(&["layouts"]))
                .is_err()
        );
        assert!(Request::parse(&names(&["--fast"])).is_err());
    }

    #[test]
    fn differences_print_in_a_column() {
        assert_eq!(format!("{:>6}", Difference::Rms(0.0512)), " 0.051");
        assert_eq!(format!("{:>3}", Difference::Absent), "  -");
    }
}
