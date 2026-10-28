//! Figure cases: Rust translations of the spec's matplotlib figure scripts,
//! compared with the goldens through structural facts and pixels.

mod cases;
mod facts;
mod pixels;
pub mod vector;

use std::error::Error;
use std::fmt;
use std::path::PathBuf;

use plt::figure::Figure;
use plt::look::FigureLook;
use plt::rc::RcParams;

use crate::json::Json;

pub(crate) use cases::projections::{RangeFrame, RangeFrameOptions, register_range_frame};
pub use facts::figure_facts;
pub use pixels::{
    PixelArtifacts, PixelReport, look_png, pixel_artifacts, pixel_report, render_pixels,
};

/// What a case's build function may fail with.
pub type CaseResult = Result<(), Box<dyn Error>>;

/// Changes a case makes to the rcParams in code.
pub type RcSetup = fn(&mut plt::rc::RcParams) -> CaseResult;

/// Where a case's image comes from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Capture {
    /// The figure saved as PNG after it is built (`savefig`).
    #[default]
    Savefig,
    /// The figure's canvas once the case is built: the case draws on the
    /// canvas and blits itself, and its image is
    /// [`FigureCanvas::buffer_rgba`](plt::figure::FigureCanvas::buffer_rgba)
    /// (a `canvas=True` case in the Python spec).
    Canvas,
}

/// A named figure recipe mirroring `@case("group/name")` in the Python spec.
#[derive(Clone, Copy)]
pub struct FigureCase {
    /// `group/name`, identical to the Python case.
    pub name: &'static str,
    /// Figure size in inches.
    pub figsize: (f64, f64),
    /// Figure resolution.
    pub dpi: f64,
    /// A bundled style sheet applied before `rc` (a case `rc` built from
    /// `matplotlib.style.library[name]`).
    pub style: Option<&'static str>,
    /// rcParams applied while building and drawing.
    pub rc: &'static [(&'static str, &'static str)],
    /// Changes to the rcParams that have no rc-file text (a case `rc`
    /// holding Python objects, such as `plt.xkcd()`'s path effects),
    /// applied after the style sheet and before `rc`.
    pub rc_setup: Option<RcSetup>,
    /// Populates the figure.
    pub build: fn(&Figure) -> CaseResult,
    /// Where the case's image comes from.
    pub capture: Capture,
    /// Whether the case typesets text with TeX (`text.usetex`): it is
    /// skipped where TeX is not installed (a `needs_tex=True` case in the
    /// Python spec).
    pub needs_tex: bool,
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
            style: None,
            rc: &[],
            rc_setup: None,
            build,
            capture: Capture::Savefig,
            needs_tex: false,
        }
    }

    /// The same case, typesetting with TeX: skipped where TeX is missing.
    #[must_use]
    pub const fn needing_tex(self) -> Self {
        Self {
            needs_tex: true,
            ..self
        }
    }

    /// Why the case cannot run here, if it cannot: it needs TeX and TeX
    /// is not installed (matplotlib's `_checkdep_usetex`, Ghostscript
    /// aside).
    pub fn skip_reason(&self) -> Option<String> {
        if !self.needs_tex {
            return None;
        }
        plt::text::check_usetex()
            .err()
            .map(|err| format!("needs TeX: {err}"))
    }

    /// The same case taken from the figure's canvas, on which it draws and
    /// blits itself.
    #[must_use]
    pub const fn from_canvas(self) -> Self {
        Self {
            capture: Capture::Canvas,
            ..self
        }
    }

    /// The same case with rcParams changed in code before its `rc`.
    #[must_use]
    pub const fn with_rc_setup(self, setup: RcSetup) -> Self {
        Self {
            rc_setup: Some(setup),
            ..self
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

    /// The same case with a bundled style sheet applied under its rc.
    #[must_use]
    pub const fn with_style(self, style: &'static str) -> Self {
        Self {
            style: Some(style),
            ..self
        }
    }

    /// Golden facts file for this case.
    pub fn facts_path(&self) -> PathBuf {
        crate::spec_root()
            .join("goldens/figures")
            .join(format!("{}.json", self.name))
    }

    /// Golden image for this case.
    pub fn png_path(&self) -> PathBuf {
        crate::spec_root()
            .join("goldens/figures")
            .join(format!("{}.png", self.name))
    }

    /// Builds the figure under the spec's reference configuration
    /// ([`reference_rc`](crate::reference_rc), then the case's style and
    /// rc overrides).
    pub fn build(&self) -> Result<Figure, Box<dyn Error>> {
        self.with_figure(|fig| Ok(fig.clone()))
    }

    /// Builds the figure and runs `f` on it, both under the reference
    /// configuration (so that drawing sees the case's rcParams too). The
    /// thread keeps the reference configuration afterwards, without the
    /// case's changes; setting it first keeps the thread from ever
    /// starting from the user's configuration.
    pub fn with_figure<T>(
        &self,
        f: impl FnOnce(&Figure) -> Result<T, Box<dyn Error>>,
    ) -> Result<T, Box<dyn Error>> {
        self.with_figure_from(Baseline::Reference, f)
    }

    /// Builds the figure and runs `f` on it, both under `baseline` with
    /// the case's style sheet and rcParams on top. The thread keeps the
    /// reference configuration afterwards, as [`with_figure`](Self::with_figure)
    /// leaves it.
    pub fn with_figure_from<T>(
        &self,
        baseline: Baseline,
        f: impl FnOnce(&Figure) -> Result<T, Box<dyn Error>>,
    ) -> Result<T, Box<dyn Error>> {
        crate::reference_rc();
        plt::rc::context(|| {
            let mut rc = baseline.params();
            if let Some(style) = self.style {
                plt::style::sheet(style)
                    .ok_or_else(|| plt::rc::RcError::UnknownStyle(style.to_owned()))?
                    .apply(&mut rc)?;
            }
            if let Some(setup) = self.rc_setup {
                setup(&mut rc)?;
            }
            plt::rc::replace(rc);
            for (key, value) in self.rc {
                plt::rc::set(key, value)?;
            }
            let fig = Figure::with_size(self.figsize.0, self.figsize.1, self.dpi);
            (self.build)(&fig)?;
            f(&fig)
        })
    }
}

/// The rcParams a case starts from, under its style sheet and rcParams.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Baseline {
    /// The spec's reference configuration, which the goldens show:
    /// matplotlib's defaults as `matplotlib.testing.setup` leaves them,
    /// with hinting off.
    #[default]
    Reference,
    /// A plt-rs figure look as users get it, to see how cases render in
    /// it (no goldens exist for it).
    Look(FigureLook),
}

impl Baseline {
    /// The parameters of the baseline, once the thread is in the
    /// reference configuration ([`reference_rc`](crate::reference_rc)).
    fn params(self) -> RcParams {
        match self {
            Self::Reference => (*plt::rc::params()).clone(),
            Self::Look(look) => look.params(),
        }
    }
}

/// The outcome of one figure case.
#[derive(Clone, Debug, PartialEq)]
pub enum CaseVerdict {
    /// Facts match the golden.
    Pass,
    /// No Rust translation of this case exists yet.
    Untranslated,
    /// The case cannot run here (it needs TeX, which is missing).
    Skipped(String),
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
    if let Some(reason) = case.skip_reason() {
        return CaseVerdict::Skipped(reason);
    }
    let golden = std::fs::read_to_string(case.facts_path())
        .map_err(|e| e.to_string())
        .and_then(|text| Json::parse(&text).map_err(|e| e.to_string()));
    let golden = match golden {
        Ok(golden) => golden,
        Err(err) => return CaseVerdict::Fail(format!("cannot read golden: {err}")),
    };
    // The facts are taken under the case's rcParams, as the reference
    // takes them while drawing inside its `rc_context`.
    let facts = match case.with_figure(|fig| Ok(figure_facts(fig))) {
        Ok(facts) => facts,
        Err(err) => return CaseVerdict::Fail(format!("build failed: {err}")),
    };
    let problems = facts::diff_facts(&golden, &facts);
    if problems.is_empty() {
        CaseVerdict::Pass
    } else {
        CaseVerdict::Fail(problems.join("; "))
    }
}
