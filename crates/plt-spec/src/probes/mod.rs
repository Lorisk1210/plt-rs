//! Probe goldens: `op(args) -> value` observations recorded from matplotlib.
//!
//! Each functional area (`colors`, `ticker`, ...) has a golden file in
//! `spec/goldens/probes/<area>.json` and, once implemented, a module here
//! exposing an [`Op`] table that evaluates the same operations on plt-rs.

#[allow(
    clippy::unnecessary_wraps,
    reason = "every probe operation shares the fallible `Op` signature"
)]
mod areas;
mod args;
mod encode;
pub mod waivers;

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

pub use args::Args;

use crate::compare::{Mismatch, Tolerance, compare};
use crate::json::Json;

/// What a probe's evaluation produced on the plt-rs side.
pub type Outcome = Result<Json, ProbeError>;

/// A plt-rs implementation of one probe operation.
#[derive(Clone, Copy, Debug)]
pub struct Op {
    /// The operation name used in the golden file.
    pub name: &'static str,
    /// Evaluates the operation.
    pub eval: fn(Args<'_>) -> Outcome,
}

impl Op {
    /// Binds an operation name to its evaluator.
    pub const fn new(name: &'static str, eval: fn(Args<'_>) -> Outcome) -> Self {
        Self { name, eval }
    }
}

/// Why an operation produced no value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProbeError {
    /// plt-rs rejected the input (expected for probes recording an error).
    Rejected(String),
    /// The golden's arguments could not be interpreted (a harness bug).
    BadArgs(String),
}

impl fmt::Display for ProbeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rejected(why) => write!(f, "rejected: {why}"),
            Self::BadArgs(why) => write!(f, "bad probe arguments: {why}"),
        }
    }
}

impl std::error::Error for ProbeError {}

impl ProbeError {
    /// Wraps any plt-rs error as a rejection.
    pub fn rejected(err: impl fmt::Display) -> Self {
        Self::Rejected(err.to_string())
    }
}

/// What matplotlib produced for a probe.
#[derive(Clone, Debug, PartialEq)]
pub enum Expected {
    /// A value.
    Value(Json),
    /// An exception of the named Python type.
    Error(String),
}

/// One recorded observation.
#[derive(Clone, Debug, PartialEq)]
pub struct Probe {
    /// Operation name.
    pub op: String,
    /// Arguments as recorded.
    pub args: Json,
    /// matplotlib's result.
    pub expected: Expected,
    /// Relative tolerance override.
    pub tol: Option<f64>,
}

/// All probes of one area, as stored in a golden file.
#[derive(Clone, Debug, PartialEq)]
pub struct ProbeFile {
    /// Area name, e.g. `"colors"`.
    pub area: String,
    /// The recorded probes.
    pub probes: Vec<Probe>,
}

/// Failure to read a golden file.
#[derive(Debug)]
pub enum LoadError {
    /// The file could not be read.
    Io(std::io::Error),
    /// The file is not valid JSON.
    Json(crate::json::ParseError),
    /// The JSON does not have the golden layout.
    Layout(&'static str),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "cannot read golden: {err}"),
            Self::Json(err) => write!(f, "malformed golden: {err}"),
            Self::Layout(why) => write!(f, "unexpected golden layout: {why}"),
        }
    }
}

impl std::error::Error for LoadError {}

impl ProbeFile {
    /// Reads and parses a golden file.
    pub fn load(path: &Path) -> Result<Self, LoadError> {
        let text = std::fs::read_to_string(path).map_err(LoadError::Io)?;
        Self::parse(&Json::parse(&text).map_err(LoadError::Json)?)
    }

    /// Interprets a parsed golden document.
    pub fn parse(doc: &Json) -> Result<Self, LoadError> {
        let area = doc
            .get("area")
            .and_then(Json::as_str)
            .ok_or(LoadError::Layout("missing \"area\""))?
            .to_owned();
        let probes = doc
            .get("probes")
            .and_then(Json::as_array)
            .ok_or(LoadError::Layout("missing \"probes\" array"))?
            .iter()
            .map(parse_probe)
            .collect::<Result<_, _>>()?;
        Ok(Self { area, probes })
    }
}

fn parse_probe(entry: &Json) -> Result<Probe, LoadError> {
    let op = entry
        .get("op")
        .and_then(Json::as_str)
        .ok_or(LoadError::Layout("probe without \"op\""))?
        .to_owned();
    let args = entry
        .get("args")
        .cloned()
        .unwrap_or(Json::Object(Vec::new()));
    let expected = match (entry.get("value"), entry.get("error")) {
        (Some(value), None) => Expected::Value(value.clone()),
        (None, Some(Json::String(kind))) => Expected::Error(kind.clone()),
        _ => {
            return Err(LoadError::Layout(
                "probe needs exactly one of \"value\"/\"error\"",
            ));
        }
    };
    let tol = entry.get("tol").and_then(Json::as_f64);
    Ok(Probe {
        op,
        args,
        expected,
        tol,
    })
}

/// The verdict for a single probe.
#[derive(Clone, Debug, PartialEq)]
pub enum Verdict {
    /// plt-rs agrees with matplotlib.
    Pass,
    /// plt-rs has no implementation of the operation yet.
    Unimplemented,
    /// plt-rs disagrees.
    Fail(String),
    /// plt-rs deliberately disagrees; see [`waivers`].
    Waived(&'static waivers::Waiver),
}

impl Probe {
    /// Evaluates the probe with `op` and judges the outcome, honouring
    /// documented waivers for failures.
    pub fn judge(&self, op: Option<&Op>) -> Verdict {
        match self.evaluate(op) {
            Verdict::Fail(message) => {
                waivers::find(&self.op, &self.args).map_or(Verdict::Fail(message), Verdict::Waived)
            }
            verdict => verdict,
        }
    }

    fn evaluate(&self, op: Option<&Op>) -> Verdict {
        let Some(op) = op else {
            return Verdict::Unimplemented;
        };
        let outcome = (op.eval)(Args::new(&self.args));
        let tol = self
            .tol
            .map_or_else(Tolerance::default, Tolerance::relative);
        match (&self.expected, outcome) {
            (Expected::Value(want), Ok(got)) => match compare(want, &got, tol) {
                Ok(()) => Verdict::Pass,
                Err(Mismatch {
                    path,
                    expected,
                    actual,
                }) => Verdict::Fail(format!(
                    "{} {}: at {path}: expected {expected}, got {actual}",
                    self.op, self.args
                )),
            },
            (Expected::Error(_), Err(ProbeError::Rejected(_))) => Verdict::Pass,
            (Expected::Error(kind), Ok(got)) => Verdict::Fail(format!(
                "{} {}: matplotlib raised {kind}, plt-rs returned {got}",
                self.op, self.args
            )),
            (Expected::Value(want), Err(err)) => Verdict::Fail(format!(
                "{} {}: expected {want}, got {err}",
                self.op, self.args
            )),
            (Expected::Error(_), Err(err @ ProbeError::BadArgs(_))) => {
                Verdict::Fail(format!("{} {}: {err}", self.op, self.args))
            }
        }
    }
}

/// Aggregated verdicts for one area.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AreaReport {
    /// Area name.
    pub area: String,
    /// Number of passing probes.
    pub passed: usize,
    /// Failure messages.
    pub failures: Vec<String>,
    /// Probes that fail by design, with the documented reason.
    pub waived: Vec<String>,
    /// Unimplemented operations and how many probes use each.
    pub unimplemented: BTreeMap<String, usize>,
}

impl AreaReport {
    /// Total number of probes in the area.
    pub fn total(&self) -> usize {
        self.passed
            + self.failures.len()
            + self.waived.len()
            + self.unimplemented.values().sum::<usize>()
    }
}

/// Runs every probe of `file` against the operation table `ops`.
pub fn run(file: &ProbeFile, ops: &[Op]) -> AreaReport {
    let mut report = AreaReport {
        area: file.area.clone(),
        ..AreaReport::default()
    };
    for probe in &file.probes {
        match probe.judge(ops.iter().find(|op| op.name == probe.op)) {
            Verdict::Pass => report.passed += 1,
            Verdict::Fail(message) => report.failures.push(message),
            Verdict::Waived(waiver) => report
                .waived
                .push(format!("{} {}: {}", probe.op, probe.args, waiver.reason)),
            Verdict::Unimplemented => {
                *report.unimplemented.entry(probe.op.clone()).or_default() += 1;
            }
        }
    }
    report
}

/// Operation tables of every area plt-rs implements, keyed by area name.
const AREAS: &[(&str, &[Op])] = &[
    ("colors", areas::colors::OPS),
    ("colormaps", areas::colormaps::OPS),
    ("colormap_luts", areas::colormap_luts::OPS),
    ("norms", areas::norms::OPS),
    ("lightsource", areas::lightsource::OPS),
    ("animation", areas::animation::OPS),
    ("imsave", areas::imsave::OPS),
    ("axes3d", areas::axes3d::OPS),
    ("axisartist", areas::axisartist::OPS),
    ("pick", areas::pick::OPS),
    ("cursor", areas::cursor::OPS),
    ("multivar", areas::multivar::OPS),
    ("imread", areas::imread::OPS),
    ("ticker", areas::ticker::OPS),
    ("scales", areas::scales::OPS),
    ("dates", areas::dates::OPS),
    ("transforms", areas::transforms::OPS),
    ("path", areas::path::OPS),
    ("markers", areas::markers::OPS),
    ("text_metrics", areas::text_metrics::OPS),
    ("afm", areas::afm::OPS),
    ("mathtext", areas::mathtext::OPS),
    ("stats", areas::stats::OPS),
    ("contour", areas::contour::OPS),
    ("tri", areas::tri::OPS),
    ("rcsetup", areas::rcsetup::OPS),
    ("fancy", areas::fancy::OPS),
    ("navigation", areas::navigation::OPS),
    ("widgets", areas::widgets::OPS),
    ("mosaic", areas::mosaic::OPS),
];

/// The operation table for an area, or an empty slice if none exists yet.
pub fn ops_for(area: &str) -> &'static [Op] {
    AREAS
        .iter()
        .find(|(name, _)| *name == area)
        .map_or(&[], |(_, ops)| ops)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn double(args: Args<'_>) -> Outcome {
        let x = args.f64("x")?;
        if x < 0.0 {
            return Err(ProbeError::rejected("negative"));
        }
        Ok(Json::from(2.0 * x))
    }

    const OPS: &[Op] = &[Op::new("double", double)];

    fn golden() -> ProbeFile {
        let doc = Json::parse(
            r#"{"area": "demo", "probes": [
                {"op": "double", "args": {"x": 2}, "value": 4.0},
                {"op": "double", "args": {"x": 3}, "value": 7.0},
                {"op": "double", "args": {"x": -1}, "error": "ValueError"},
                {"op": "triple", "args": {"x": 1}, "value": 3.0}
            ]}"#,
        )
        .unwrap();
        ProbeFile::parse(&doc).unwrap()
    }

    #[test]
    fn run_reports_each_verdict() {
        let report = run(&golden(), OPS);
        assert_eq!(report.passed, 2);
        assert_eq!(report.failures.len(), 1);
        assert!(report.failures[0].contains("expected 7.0, got 6.0"));
        assert_eq!(report.unimplemented.get("triple"), Some(&1));
        assert_eq!(report.total(), 4);
    }
}
