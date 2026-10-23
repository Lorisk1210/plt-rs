//! Probe operations for the `mpl_style` area: style sheets ported from
//! matplotlib's `tests/test_style.py` (see
//! `spec/reference/probes/test_mpl_style.py`): `style.use` and
//! `style.context` with library names, files, URLs and parameter sets,
//! the user style library, the blacklist and `pyplot.xkcd`.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use plt::rc::{self, RcParams};
use plt::style::{self, STYLE_BLACKLIST, Style, XkcdSketch};

use super::rcsetup::canonical;
use super::text_metrics::{rc_text, with_rc};
use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("style.available", available),
    Op::new("style.reload_library", reload_library),
    Op::new("style.context", context),
    Op::new("style.context.equal", context_equal),
    Op::new("pyplot.xkcd", xkcd),
    Op::new("style.STYLE_BLACKLIST", blacklist),
];

/// A scratch directory, removed when dropped.
struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new() -> Result<Self, ProbeError> {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "plt-spec-style-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).map_err(ProbeError::rejected)?;
        Ok(Self(dir))
    }

    /// Writes `text` to the file `name` and returns its path.
    fn write(&self, name: &str, text: &str) -> Result<PathBuf, ProbeError> {
        let path = self.0.join(name);
        std::fs::write(&path, text).map_err(ProbeError::rejected)?;
        Ok(path)
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        // A leftover scratch directory is harmless.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Runs `f` with the `library` argument (`{name: text}`) as the user
/// style library, then empties the library again; returns what `f` and
/// the reload warnings.
fn with_user_library<T>(
    args: Args<'_>,
    f: impl FnOnce() -> Result<T, ProbeError>,
) -> Result<(T, Vec<plt::rc::RcWarning>), ProbeError> {
    let sheets = args.get("library").and_then(Json::as_object).unwrap_or(&[]);
    let scratch = ScratchDir::new()?;
    for (name, text) in sheets {
        let text = text
            .as_str()
            .ok_or_else(|| ProbeError::BadArgs(format!("style {name} is not text")))?;
        scratch.write(&format!("{name}.mplstyle"), text)?;
    }
    let paths = if sheets.is_empty() {
        Vec::new()
    } else {
        vec![scratch.0.clone()]
    };
    style::set_user_library_paths(paths);
    let warnings = style::reload_library();
    let result = f();
    style::set_user_library_paths(Vec::new());
    style::reload_library();
    Ok((result?, warnings))
}

fn available(args: Args<'_>) -> Outcome {
    let names = args
        .json("names")?
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs("names is not a list".into()))?;
    let (membership, _) = with_user_library(args, || {
        let public = style::available();
        names
            .iter()
            .map(|name| {
                let name = name
                    .as_str()
                    .ok_or_else(|| ProbeError::BadArgs(format!("bad name {name}")))?;
                Ok(Json::Array(vec![
                    public.iter().any(|p| p == name).into(),
                    style::sheet(name).is_some().into(),
                ]))
            })
            .collect::<Result<Vec<_>, ProbeError>>()
    })?;
    Ok(Json::Array(membership))
}

fn reload_library(args: Args<'_>) -> Outcome {
    let ((), warnings) = with_user_library(args, || Ok(()))?;
    Ok(Json::Array(
        warnings
            .iter()
            .map(|warning| {
                warning
                    .path
                    .as_deref()
                    .and_then(Path::file_name)
                    .map(|name| name.to_string_lossy().into_owned())
                    .into()
            })
            .collect(),
    ))
}

/// One style of the `style` argument; files are written to `scratch`.
fn style_spec(spec: &Json, scratch: &ScratchDir) -> Result<Style, ProbeError> {
    match spec {
        Json::String(name) => Ok(Style::Named(name.clone())),
        Json::Object(entries) => match entries.as_slice() {
            [(kind, Json::Object(params))] if kind == "dict" => Ok(Style::Params(
                params
                    .iter()
                    .map(|(key, value)| Ok((key.clone(), rc_text(value)?)))
                    .collect::<Result<_, ProbeError>>()?,
            )),
            [(kind, Json::String(text))] if kind == "path" || kind == "url" => {
                let path = scratch.write("style.mplstyle", text)?;
                Ok(if kind == "path" {
                    Style::File(path)
                } else {
                    Style::Named(format!("file:{}", path.display()))
                })
            }
            _ => Err(ProbeError::BadArgs(format!("bad style {spec}"))),
        },
        other => Err(ProbeError::BadArgs(format!("bad style {other}"))),
    }
}

/// The `style` argument: one style or a list of styles.
fn styles(spec: &Json, scratch: &ScratchDir) -> Result<Vec<Style>, ProbeError> {
    match spec {
        Json::Array(items) => items.iter().map(|item| style_spec(item, scratch)).collect(),
        single => Ok(vec![style_spec(single, scratch)?]),
    }
}

/// The canonical values of the `params` of the current rcParams.
fn read_params(params: &[Json]) -> Result<Json, ProbeError> {
    let rc = rc::params();
    params
        .iter()
        .map(|key| {
            let key = key
                .as_str()
                .ok_or_else(|| ProbeError::BadArgs(format!("bad param {key}")))?;
            canonical(&rc, key).ok_or_else(|| ProbeError::BadArgs(format!("{key} is not probed")))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Json::Array)
}

fn context(args: Args<'_>) -> Outcome {
    let params = args
        .json("params")?
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs("params is not a list".into()))?;
    with_rc(args, || {
        let (record, _) = with_user_library(args, || {
            let scratch = ScratchDir::new()?;
            let styles = styles(args.json("style")?, &scratch)?;
            let inside = style::context_with(&styles, || read_params(params))
                .ok()
                .transpose()?;
            Ok(Json::Object(vec![
                ("failed".into(), inside.is_none().into()),
                ("inside".into(), inside.unwrap_or(Json::Null)),
                ("after".into(), read_params(params)?),
            ]))
        })?;
        Ok(record)
    })
}

fn context_equal(args: Args<'_>) -> Outcome {
    let names = args
        .json("styles")?
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs("styles is not a list".into()))?;
    with_rc(args, || {
        let snapshots = names
            .iter()
            .map(|name| {
                let name = name
                    .as_str()
                    .ok_or_else(|| ProbeError::BadArgs(format!("bad style {name}")))?;
                style::context(name, || (*rc::params()).clone()).map_err(ProbeError::rejected)
            })
            .collect::<Result<Vec<RcParams>, _>>()?;
        Ok(snapshots.windows(2).all(|pair| pair[0] == pair[1]).into())
    })
}

fn xkcd(args: Args<'_>) -> Outcome {
    let scoped = args.bool("scoped")?;
    let sketch = [Json::from("path.sketch")];
    with_rc(args, || {
        let before = read_params(&sketch)?;
        let inside = if scoped {
            plt::pyplot::xkcd(XkcdSketch::default(), || read_params(&sketch))
                .map_err(ProbeError::rejected)??
        } else {
            let mut updated = (*rc::params()).clone();
            style::apply_xkcd(&mut updated, XkcdSketch::default()).map_err(ProbeError::rejected)?;
            rc::replace(updated);
            read_params(&sketch)?
        };
        Ok(Json::Array(vec![before, inside, read_params(&sketch)?]))
    })
}

fn blacklist(_: Args<'_>) -> Outcome {
    let mut keys = STYLE_BLACKLIST.to_vec();
    keys.sort_unstable();
    Ok(Json::from(keys))
}
