//! Probe goldens: `op(args) -> value` observations recorded from matplotlib.
//!
//! Each functional area (`colors`, `ticker`, ...) has a golden file in
//! `spec/goldens/probes/<area>.json` and, once implemented, a module here
//! exposing an [`Op`] table that evaluates the same operations on pltrs.

#[expect(
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

/// What a probe's evaluation produced on the pltrs side.
pub type Outcome = Result<Json, ProbeError>;

/// A pltrs implementation of one probe operation.
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
    /// pltrs rejected the input (expected for probes recording an error).
    Rejected(String),
    /// The golden's arguments could not be interpreted (a harness bug).
    BadArgs(String),
    /// The probe cannot run here (it needs TeX, which is missing).
    Skipped(String),
}

impl fmt::Display for ProbeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rejected(why) => write!(f, "rejected: {why}"),
            Self::BadArgs(why) => write!(f, "bad probe arguments: {why}"),
            Self::Skipped(why) => write!(f, "skipped: {why}"),
        }
    }
}

impl std::error::Error for ProbeError {}

impl ProbeError {
    /// Wraps any pltrs error as a rejection.
    pub fn rejected(err: impl fmt::Display) -> Self {
        Self::Rejected(err.to_string())
    }
}

/// A pltrs result as a probe result: its error becomes a rejection.
pub(crate) fn made<T>(result: Result<T, impl fmt::Display>) -> Result<T, ProbeError> {
    result.map_err(ProbeError::rejected)
}

/// The probe argument `what` could not be decoded (`"{what} is malformed"`).
pub(crate) fn malformed(what: &str) -> ProbeError {
    ProbeError::BadArgs(format!("{what} is malformed"))
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
    /// pltrs agrees with matplotlib.
    Pass,
    /// pltrs has no implementation of the operation yet.
    Unimplemented,
    /// pltrs disagrees.
    Fail(String),
    /// pltrs deliberately disagrees; see [`waivers`].
    Waived(&'static waivers::Waiver),
    /// The probe cannot run here (it needs TeX, which is missing).
    Skipped(String),
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
        if let Err(ProbeError::Skipped(why)) = outcome {
            return Verdict::Skipped(why);
        }
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
                "{} {}: matplotlib raised {kind}, pltrs returned {got}",
                self.op, self.args
            )),
            (Expected::Value(want), Err(err)) => Verdict::Fail(format!(
                "{} {}: expected {want}, got {err}",
                self.op, self.args
            )),
            (Expected::Error(_), Err(err @ (ProbeError::BadArgs(_) | ProbeError::Skipped(_)))) => {
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
    /// Probes that cannot run here, with the reason.
    pub skipped: Vec<String>,
}

impl AreaReport {
    /// Total number of probes in the area.
    pub fn total(&self) -> usize {
        self.passed
            + self.failures.len()
            + self.waived.len()
            + self.skipped.len()
            + self.unimplemented.values().sum::<usize>()
    }
}

/// Runs every probe of `file` against the operation table `ops`, in the
/// reference configuration ([`reference_rc`](crate::reference_rc)).
pub fn run(file: &ProbeFile, ops: &[Op]) -> AreaReport {
    crate::reference_rc();
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
            Verdict::Skipped(why) => report.skipped.push(format!("{}: {why}", probe.op)),
        }
    }
    report
}

/// Operation tables of every area pltrs implements, keyed by area name.
const AREAS: &[(&str, &[Op])] = &[
    ("colors", areas::colors::OPS),
    ("colormaps", areas::colormaps::OPS),
    ("colormap_luts", areas::colormap_luts::OPS),
    ("norms", areas::norms::OPS),
    ("lightsource", areas::lightsource::OPS),
    ("animation", areas::animation::OPS),
    ("api_audit", areas::api_audit::OPS),
    ("api_audit_3d", areas::api_audit_3d::OPS),
    ("imsave", areas::imsave::OPS),
    ("axes3d", areas::axes3d::OPS),
    ("axisartist", areas::axisartist::OPS),
    ("pick", areas::pick::OPS),
    ("cursor", areas::cursor::OPS),
    ("multivar", areas::multivar::OPS),
    ("imread", areas::imread::OPS),
    ("imread_webp", areas::imread::OPS),
    ("ticker", areas::ticker::OPS),
    ("scales", areas::scales::OPS),
    ("dates", areas::dates::OPS),
    ("transforms", areas::transforms::OPS),
    ("path", areas::path::OPS),
    ("markers", areas::markers::OPS),
    ("text_metrics", areas::text_metrics::OPS),
    ("text_shaping", areas::text_shaping::OPS),
    ("afm", areas::afm::OPS),
    ("type1font", areas::type1font::OPS),
    ("texmanager", areas::texmanager::OPS),
    ("usetex", areas::usetex::OPS),
    ("mathtext", areas::mathtext::OPS),
    ("stats", areas::stats::OPS),
    ("argsort", areas::argsort::OPS),
    ("contour", areas::contour::OPS),
    ("tri", areas::tri::OPS),
    ("rcsetup", areas::rcsetup::OPS),
    ("fancy", areas::fancy::OPS),
    ("navigation", areas::navigation::OPS),
    ("widgets", areas::widgets::OPS),
    ("mosaic", areas::mosaic::OPS),
    ("axes_behaviour", areas::axes_behaviour::OPS),
    ("legend_behaviour", areas::legend_behaviour::OPS),
    ("colorbar_behaviour", areas::colorbar_behaviour::OPS),
    ("text_behaviour", areas::text_behaviour::OPS),
    ("line_behaviour", areas::line_behaviour::OPS),
    ("patch_behaviour", areas::patch_behaviour::OPS),
    ("collection_behaviour", areas::collection_behaviour::OPS),
    ("artist_behaviour", areas::artist_behaviour::OPS),
    ("image_behaviour", areas::image_behaviour::OPS),
    ("contour_behaviour", areas::contour_behaviour::OPS),
    ("contour_generator", areas::contour_generator::OPS),
    ("quiver_behaviour", areas::quiver_behaviour::OPS),
    ("polar_behaviour", areas::polar_behaviour::OPS),
    ("projections", areas::projections::OPS),
    ("tri_behaviour", areas::tri_behaviour::OPS),
    ("axes3d_behaviour", areas::axes3d_behaviour::OPS),
    ("offsetbox_behaviour", areas::offsetbox_behaviour::OPS),
    ("sankey_behaviour", areas::sankey_behaviour::OPS),
    ("streamplot_behaviour", areas::streamplot_behaviour::OPS),
    ("mpl_font_manager", areas::mpl_font_manager::OPS),
    ("mpl_fontconfig_pattern", areas::mpl_fontconfig_pattern::OPS),
    ("mpl_style", areas::mpl_style::OPS),
    ("mpl_cycles", areas::mpl_cycles::OPS),
    ("mpl_cbook", areas::mpl_cbook::OPS),
    ("mpl_mlab", areas::mpl_mlab::OPS),
    ("mpl_bezier", areas::mpl_bezier::OPS),
    ("mpl_textpath", areas::mpl_textpath::OPS),
    ("mpl_determinism", areas::mpl_determinism::OPS),
    ("mpl_backend_bases", areas::mpl_backend_bases::OPS),
    ("mpl_container", areas::mpl_container::OPS),
    ("mpl_gridspec", areas::mpl_gridspec::OPS),
    ("mpl_pyplot", areas::mpl_pyplot::OPS),
    ("mpl_colors", areas::mpl_colors::OPS),
    ("mpl_dates", areas::mpl_dates::OPS),
    ("mpl_ticker", areas::mpl_ticker::OPS),
    ("mpl_transforms", areas::mpl_transforms::OPS),
    ("mpl_axes", areas::mpl_axes::OPS),
    ("mpl_path", areas::mpl_path::OPS),
    ("mpl_scale", areas::mpl_scale::OPS),
    ("mpl_polar", areas::mpl_polar::OPS),
    ("mpl_category", areas::mpl_category::OPS),
    ("mpl_rcparams", areas::mpl_rcparams::OPS),
    ("mpl_ft2font", areas::mpl_ft2font::OPS),
    ("mpl_image", areas::mpl_image::OPS),
    ("mpl_figure", areas::mpl_figure::OPS),
    ("mpl_legend", areas::mpl_legend::OPS),
    ("mpl_colorbar", areas::mpl_colorbar::OPS),
    ("mpl_constrainedlayout", areas::mpl_constrainedlayout::OPS),
    ("mpl_tightlayout", areas::mpl_tightlayout::OPS),
    ("mpl_widgets", areas::mpl_widgets::OPS),
    ("mpl_animation", areas::mpl_animation::OPS),
    ("mpl_units", areas::mpl_units::OPS),
    ("mpl_agg", areas::mpl_agg::OPS),
    ("mpl_afm", areas::mpl_afm::OPS),
    ("mpl_backend_pdf", areas::mpl_backend_pdf::OPS),
    ("mpl_backend_svg", areas::mpl_backend_svg::OPS),
    ("mpl_backend_ps", areas::mpl_backend_ps::OPS),
    ("mpl_patches", areas::mpl_patches::OPS),
    ("mpl_triangulation", areas::mpl_triangulation::OPS),
    ("mpl_simplification", areas::mpl_simplification::OPS),
    ("mpl_bbox_tight", areas::mpl_bbox_tight::OPS),
    ("mpl_text", areas::mpl_text::OPS),
    ("mpl_spines", areas::mpl_spines::OPS),
    ("mpl_lines", areas::mpl_lines::OPS),
    ("mpl_table", areas::mpl_table::OPS),
    ("mpl_offsetbox", areas::mpl_offsetbox::OPS),
    ("mpl_marker", areas::mpl_marker::OPS),
    ("mpl_arrow_patches", areas::mpl_arrow_patches::OPS),
    ("mpl_artist", areas::mpl_artist::OPS),
    ("mpl_contour", areas::mpl_contour::OPS),
    ("mpl_collections", areas::mpl_collections::OPS),
    (
        "mpl_multivariate_colormaps",
        areas::mpl_multivariate_colormaps::OPS,
    ),
    ("mpl_backend_pgf", areas::mpl_backend_pgf::OPS),
    ("mpl_backend_tools", areas::mpl_backend_tools::OPS),
    ("units", areas::units::OPS),
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
