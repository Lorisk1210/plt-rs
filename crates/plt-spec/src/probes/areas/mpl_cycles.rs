//! Probe operations for the `mpl_cycles` area: property cycles ported from
//! matplotlib's `tests/test_cycles.py` (see
//! `spec/reference/probes/test_mpl_cycles.py`).
//!
//! `set_prop_cycle(*args, **kwargs)` is matplotlib's validating
//! `cycler(*args, **kwargs)`, so the arguments are written back as that
//! call and parsed as a cycler expression.

use plt::axes::Axes;
use plt::figure::Figure;
use plt::lines::{Line2D, LineProps};
use plt::patches::{Patch, PatchProps, Polygon};
use plt::rc::Cycler;
use plt::ticker::pyfmt;
use plt_core::Rgba;

use super::canon::Canon;
use super::text_metrics::with_rc;
use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[Op::new("axes.Axes.set_prop_cycle", set_prop_cycle)];

/// A JSON value written as the Python literal it was recorded from.
fn python_literal(value: &Json) -> Result<String, ProbeError> {
    Ok(match value {
        Json::Null => "None".to_owned(),
        Json::Bool(true) => "True".to_owned(),
        Json::Bool(false) => "False".to_owned(),
        Json::Integer(n) => n.to_string(),
        Json::Number(x) => pyfmt::repr(*x),
        Json::String(text) => format!("'{}'", text.replace('\\', "\\\\").replace('\'', "\\'")),
        Json::Array(items) => format!(
            "[{}]",
            items
                .iter()
                .map(python_literal)
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        ),
        Json::Object(_) => return Err(ProbeError::BadArgs(format!("no literal for {value}"))),
    })
}

/// The cycle the arguments of `set_prop_cycle` describe; `None` restores
/// `axes.prop_cycle`.
fn cycle(args: Args<'_>) -> Result<Option<Cycler>, ProbeError> {
    let expression = if let Some(cycle) = args.opt_str("cycle")? {
        cycle.to_owned()
    } else {
        let positional = args.get("args").and_then(Json::as_array).unwrap_or(&[]);
        if let [Json::Null] = positional {
            return Ok(None);
        }
        let keywords = args.get("kwargs").and_then(Json::as_object).unwrap_or(&[]);
        let mut parts = positional
            .iter()
            .map(python_literal)
            .collect::<Result<Vec<_>, _>>()?;
        for (key, value) in keywords {
            parts.push(format!("{key}={}", python_literal(value)?));
        }
        format!("cycler({})", parts.join(", "))
    };
    expression.parse().map(Some).map_err(ProbeError::rejected)
}

/// The line width of a call's `kw`.
fn call_linewidth(kw: &Json) -> Option<f64> {
    kw.get("lw").and_then(Json::as_f64)
}

/// `ax.plot(range(10), range(10), *fmt, **kw)`.
fn plot(ax: &Axes, call: &Json, x: &[f64]) -> Result<Line2D, ProbeError> {
    let kw = call.get("kw").unwrap_or(&Json::Null);
    let mut props = match call.get("fmt").and_then(Json::as_str) {
        Some(fmt) => LineProps::fmt(fmt).map_err(ProbeError::rejected)?,
        None => LineProps::default(),
    };
    if let Some(width) = call_linewidth(kw) {
        props = props.linewidth(width);
    }
    if let Some(label) = kw.get("label").and_then(Json::as_str) {
        props = props.label(label);
    }
    ax.plot_with(x.to_vec(), x.to_vec(), props)
        .map_err(ProbeError::rejected)
}

/// `ax.fill(range(10), range(10), *fmt, **kw)`; a format is a colour.
fn fill(ax: &Axes, call: &Json, x: &[f64]) -> Result<Patch<Polygon>, ProbeError> {
    let kw = call.get("kw").unwrap_or(&Json::Null);
    let mut props = PatchProps::default();
    if let Some(color) = call.get("fmt").and_then(Json::as_str) {
        props = props.color(color);
    }
    if let Some(width) = call_linewidth(kw) {
        props = props.linewidth(width);
    }
    if let Some(hatch) = kw.get("hatch").and_then(Json::as_str) {
        props = props.hatch(hatch);
    }
    ax.fill(x, x, props).map_err(ProbeError::rejected)
}

/// An RGBA colour as a list.
fn rgba(color: Rgba) -> Json {
    Json::from(vec![color.r, color.g, color.b, color.a])
}

/// A getter's values over `lines`.
fn line_values(lines: &[Line2D], getter: &str) -> Result<Vec<Json>, ProbeError> {
    lines
        .iter()
        .map(|line| {
            Ok(match getter {
                "get_color" => rgba(line.rgba().map_err(ProbeError::rejected)?),
                "get_marker" => line.marker().canon(),
                "get_linestyle" => line.linestyle().canon(),
                "get_linewidth" => line.linewidth().into(),
                "get_dash_pattern" => {
                    let dashes = line.linestyle().dash_pattern(&plt::rc::params());
                    Json::Array(vec![dashes.offset.into(), dashes.lengths().to_vec().into()])
                }
                other => return Err(ProbeError::BadArgs(format!("no line getter {other}"))),
            })
        })
        .collect()
}

/// A getter's values over `patches`.
fn patch_values(patches: &[Patch<Polygon>], getter: &str) -> Result<Vec<Json>, ProbeError> {
    let rc = plt::rc::params();
    patches
        .iter()
        .map(|patch| {
            let style = patch.style();
            Ok(match getter {
                "get_facecolor" => rgba(patch.colors().map_err(ProbeError::rejected)?.face),
                "get_hatch" => style.hatch.into(),
                "get_linestyle" => style.linestyle.canon(),
                "get_linewidth" => style.linewidth.unwrap_or(rc.patch_linewidth).into(),
                other => return Err(ProbeError::BadArgs(format!("no patch getter {other}"))),
            })
        })
        .collect()
}

fn set_prop_cycle(args: Args<'_>) -> Outcome {
    with_rc(args, || {
        let ax = Figure::new().subplot();
        ax.set_prop_cycle(cycle(args)?);
        let filling = match args.opt_str("artist")? {
            None | Some("plot") => false,
            Some("fill") => true,
            Some(other) => return Err(ProbeError::BadArgs(format!("unknown artist {other}"))),
        };
        let x: Vec<f64> = (0..10).map(f64::from).collect();
        let (mut lines, mut patches) = (Vec::new(), Vec::new());
        for call in args.get("calls").and_then(Json::as_array).unwrap_or(&[]) {
            if filling {
                patches.push(fill(&ax, call, &x)?);
            } else {
                lines.push(plot(&ax, call, &x)?);
            }
        }
        let getters = args.get("getters").and_then(Json::as_array).unwrap_or(&[]);
        let mut record = Vec::new();
        for getter in getters {
            let getter = getter
                .as_str()
                .ok_or_else(|| ProbeError::BadArgs(format!("bad getter {getter}")))?;
            let values = if filling {
                patch_values(&patches, getter)?
            } else {
                line_values(&lines, getter)?
            };
            record.push((getter.to_owned(), Json::Array(values)));
        }
        Ok(Json::Object(record))
    })
}
