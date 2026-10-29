//! Probe operations for the `mpl_rcparams` area: matplotlib's own tests of
//! rcParams (see `spec/reference/probes/test_mpl_rcparams.py`).

use std::path::PathBuf;

use pltrs::figure::Figure;
use pltrs::legend::LegendOptions;
use pltrs::lines::LineProps;
use pltrs::pyplot;
use pltrs::rc::{self, RcError, RcParams};

use super::rcsetup::{canonical, validate};
use super::text_metrics::{rc_text, with_rc};
use crate::json::Json;
use crate::probes::encode::resolved_rgba;
use crate::probes::{Args, Op, Outcome, ProbeError, encode};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("rc.validate", validate),
    Op::new("matplotlib.rcParamsDefault", default_value),
    Op::new("matplotlib.rc_context", rc_context),
    Op::new("matplotlib.rc_file", rc_file),
    Op::new("matplotlib.rc", rc_aliases),
    Op::new("legend.Legend.legendPatch.get_color", |args| {
        with_rc(args, || legend_color(args))
    }),
    Op::new("lines.Line2D.get_markerfacecolor", |args| {
        with_rc(args, || marker_color(args, true))
    }),
    Op::new("lines.Line2D.get_markeredgecolor", |args| {
        with_rc(args, || marker_color(args, false))
    }),
    Op::new("text.Text.get_color", |args| {
        with_rc(args, || title_color(args))
    }),
];

/// The canonical value of `key` in `params`.
fn canonical_value(params: &RcParams, key: &str) -> Outcome {
    canonical(params, key).ok_or_else(|| ProbeError::BadArgs(format!("{key} is not probed")))
}

fn default_value(args: Args<'_>) -> Outcome {
    canonical_value(&RcParams::matplotlib(), args.str("key")?)
}

/// The canonical current values of the parameters named by `params`.
fn current(args: Args<'_>) -> Outcome {
    let current = rc::params();
    args.json("params")?
        .as_array()
        .unwrap_or(&[])
        .iter()
        .map(|key| {
            let key = key
                .as_str()
                .ok_or_else(|| ProbeError::BadArgs(format!("{key} is not a name")))?;
            canonical_value(&current, key)
        })
        .collect::<Result<_, _>>()
        .map(Json::Array)
}

/// The `(key, rc text)` pairs of an object argument.
fn settings(args: Args<'_>, key: &str) -> Result<Vec<(String, String)>, ProbeError> {
    args.get(key)
        .and_then(Json::as_object)
        .unwrap_or(&[])
        .iter()
        .map(|(name, value)| Ok((name.clone(), rc_text(value)?)))
        .collect()
}

/// Borrowed views of owned settings.
fn borrowed(settings: &[(String, String)]) -> Vec<(&str, &str)> {
    settings
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect()
}

/// An rc file holding `text`, removed when dropped.
struct TempRcFile(PathBuf);

impl TempRcFile {
    fn new(text: &str) -> Result<Self, ProbeError> {
        let name = format!(
            "pltrs-spec-mpl-rcparams-{}-{:?}.rc",
            std::process::id(),
            std::thread::current().id()
        )
        .replace(
            |c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '.',
            "",
        );
        let path = std::env::temp_dir().join(name);
        std::fs::write(&path, text).map_err(ProbeError::rejected)?;
        Ok(Self(path))
    }
}

impl Drop for TempRcFile {
    fn drop(&mut self) {
        // A file left behind in the temporary directory is harmless.
        std::fs::remove_file(&self.0).ok();
    }
}

/// Runs `f` on matplotlib's defaults with the `setup` parameters set,
/// restoring the parameters afterwards.
fn from_setup(args: Args<'_>, f: impl FnOnce() -> Outcome) -> Outcome {
    rc::context(|| {
        rc::replace(RcParams::matplotlib());
        for (key, value) in settings(args, "setup")? {
            rc::set(&key, &value).map_err(ProbeError::rejected)?;
        }
        f()
    })
}

fn rc_context(args: Args<'_>) -> Outcome {
    from_setup(args, || {
        let changes = settings(args, "rc")?;
        let changes = borrowed(&changes);
        let inside = match args.opt_str("file")? {
            Some(text) => {
                let file = TempRcFile::new(text)?;
                pyplot::rc_file_context(&file.0, &changes, || Ok::<_, RcError>(current(args)))
            }
            None => pyplot::rc_context(&changes, || Ok::<_, RcError>(current(args))),
        }
        .map_err(ProbeError::rejected)??;
        Ok(encode::object([
            ("inside", inside),
            ("after", current(args)?),
        ]))
    })
}

fn rc_file(args: Args<'_>) -> Outcome {
    from_setup(args, || {
        let file = TempRcFile::new(args.str("file")?)?;
        pyplot::rc_file(&file.0).map_err(ProbeError::rejected)?;
        current(args)
    })
}

fn rc_aliases(args: Args<'_>) -> Outcome {
    rc::context(|| {
        rc::replace(RcParams::matplotlib());
        let kwargs = settings(args, "kwargs")?;
        pyplot::rc(args.str("group")?, &borrowed(&kwargs)).map_err(ProbeError::rejected)?;
        canonical_value(&rc::params(), args.str("key")?)
    })
}

fn legend_color(args: Args<'_>) -> Outcome {
    let ax = Figure::new().subplot();
    ax.plot_with(
        [0.0, 1.0, 2.0],
        [0.0, 1.0, 2.0],
        LineProps::default().label("test"),
    )
    .map_err(ProbeError::rejected)?;
    let legend = ax
        .legend(LegendOptions::default())
        .map_err(ProbeError::rejected)?;
    let colors = legend.frame().colors().map_err(ProbeError::rejected)?;
    match args.str("which")? {
        "face" => Ok(encode::rgba(colors.face)),
        "edge" => Ok(encode::rgba(colors.edge)),
        other => Err(ProbeError::BadArgs(format!("unknown colour {other:?}"))),
    }
}

fn marker_color(_: Args<'_>, face: bool) -> Outcome {
    let line = Figure::new()
        .subplot()
        .plot([1.0, 2.0], [1.0, 2.0])
        .map_err(ProbeError::rejected)?;
    if face {
        resolved_rgba(line.markerfacecolor().or(&line.color()))
    } else {
        resolved_rgba(&line.effective_markeredgecolor())
    }
}

fn title_color(_: Args<'_>) -> Outcome {
    resolved_rgba(&Figure::new().subplot().set_title("Title").color())
}
