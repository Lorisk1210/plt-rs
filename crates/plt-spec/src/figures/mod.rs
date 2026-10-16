//! Figure cases: Rust translations of the spec's matplotlib figure scripts,
//! compared with the goldens through structural facts (and pixels, once a
//! raster backend renders them).

mod cases;
mod facts;

use std::error::Error;
use std::fmt;
use std::path::PathBuf;

use plt::figure::Figure;

use crate::json::Json;

pub use facts::figure_facts;

/// What a case's build function may fail with.
pub type CaseResult = Result<(), Box<dyn Error>>;

/// A named figure recipe mirroring `@case("group/name")` in the Python spec.
#[derive(Clone, Copy)]
pub struct FigureCase {
    /// `group/name`, identical to the Python case.
    pub name: &'static str,
    /// Figure size in inches.
    pub figsize: (f64, f64),
    /// Figure resolution.
    pub dpi: f64,
    /// rcParams applied while building and drawing.
    pub rc: &'static [(&'static str, &'static str)],
    /// Populates the figure.
    pub build: fn(&Figure) -> CaseResult,
}

impl fmt::Debug for FigureCase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FigureCase")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl FigureCase {
    /// A case on the default 6.4 × 4.8 in, 100 dpi canvas.
    pub const fn new(name: &'static str, build: fn(&Figure) -> CaseResult) -> Self {
        Self {
            name,
            figsize: (6.4, 4.8),
            dpi: 100.0,
            rc: &[],
            build,
        }
    }

    /// The same case on a different canvas.
    #[must_use]
    pub const fn sized(self, width: f64, height: f64, dpi: f64) -> Self {
        Self {
            figsize: (width, height),
            dpi,
            ..self
        }
    }

    /// The same case with rcParams overrides.
    #[must_use]
    pub const fn with_rc(self, rc: &'static [(&'static str, &'static str)]) -> Self {
        Self { rc, ..self }
    }

    /// Golden facts file for this case.
    pub fn facts_path(&self) -> PathBuf {
        crate::spec_root()
            .join("goldens/figures")
            .join(format!("{}.json", self.name))
    }

    /// Builds the figure under the spec's reference configuration
    /// (matplotlib's defaults plus the case's rc overrides).
    pub fn build(&self) -> Result<Figure, Box<dyn Error>> {
        plt::rc::context(|| {
            plt::rc::replace(plt::rc::RcParams::matplotlib());
            for (key, value) in self.rc {
                plt::rc::set(key, value)?;
            }
            let fig = Figure::with_size(self.figsize.0, self.figsize.1, self.dpi);
            (self.build)(&fig)?;
            Ok(fig)
        })
    }
}

/// The outcome of one figure case.
#[derive(Clone, Debug, PartialEq)]
pub enum CaseVerdict {
    /// Facts match the golden.
    Pass,
    /// No Rust translation of this case exists yet.
    Untranslated,
    /// The case failed to build or its facts differ.
    Fail(String),
}

/// Every translated case.
pub fn translated_cases() -> Vec<FigureCase> {
    cases::all()
}

/// Names of every case in the golden set (`group/name`).
pub fn golden_case_names() -> Vec<String> {
    let root = crate::spec_root().join("goldens/figures");
    let mut names = Vec::new();
    for group in std::fs::read_dir(&root).into_iter().flatten().flatten() {
        let group_name = group.file_name().to_string_lossy().into_owned();
        for file in std::fs::read_dir(group.path())
            .into_iter()
            .flatten()
            .flatten()
        {
            let path = file.path();
            if path.extension().is_some_and(|e| e == "json") {
                let stem = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                names.push(format!("{group_name}/{stem}"));
            }
        }
    }
    names.sort();
    names
}

/// Builds a case and compares its facts with the golden.
pub fn judge(case: &FigureCase) -> CaseVerdict {
    let golden = std::fs::read_to_string(case.facts_path())
        .map_err(|e| e.to_string())
        .and_then(|text| Json::parse(&text).map_err(|e| e.to_string()));
    let golden = match golden {
        Ok(golden) => golden,
        Err(err) => return CaseVerdict::Fail(format!("cannot read golden: {err}")),
    };
    let fig = match case.build() {
        Ok(fig) => fig,
        Err(err) => return CaseVerdict::Fail(format!("build failed: {err}")),
    };
    let problems = facts::diff_facts(&golden, &figure_facts(&fig));
    if problems.is_empty() {
        CaseVerdict::Pass
    } else {
        CaseVerdict::Fail(problems.join("; "))
    }
}
