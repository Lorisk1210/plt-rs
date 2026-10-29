//! The date formatters: `AutoDateFormatter` and `ConciseDateFormatter`
//! labelling the ticks of the probe's `AutoDateLocator`, and
//! `DateFormatter` labelling a single date.

use pltrs::dates::{AutoDateFormatter, ConciseDateFormatter, DateFormatter};
use pltrs::ticker::{AxisInfo, Formatter, Locator, TickContext, TickerSettings};

use super::auto::auto_locator;
use super::support::{ObjectBuilder, date_num, view_axis};
use crate::json::Json;
use crate::probes::areas::ticker::Params;
use crate::probes::{Args, Outcome, ProbeError};

/// Locates with the probe's `AutoDateLocator` and formats the ticks with
/// `formatter`; the formatter sees the view in increasing order because the
/// reference module's dummy axis never reports itself as inverted.
fn locate_and_format(
    args: Args<'_>,
    formatter: &dyn Formatter,
) -> Result<(Vec<String>, String), ProbeError> {
    let locator = auto_locator(args.get("locator"))?;
    let axis = view_axis(args)?;
    let settings = TickerSettings::default();
    let found = locator
        .locate(&TickContext::new(&axis, &settings))
        .map_err(ProbeError::rejected)?;
    let (a, b) = axis.view_interval;
    let upright = AxisInfo::with_view(a.min(b), a.max(b));
    let labels = formatter
        .format_ticks(&found, &TickContext::new(&upright, &settings))
        .map_err(ProbeError::rejected)?;
    Ok((labels.labels, labels.offset))
}

pub(crate) fn auto_format_ticks(args: Args<'_>) -> Outcome {
    let params = Params::of(args, &["defaultfmt", "usetex"])?.args();
    let mut formatter = AutoDateFormatter::new(auto_locator(args.get("locator"))?);
    if let Some(format) = params.opt_str("defaultfmt")? {
        formatter = formatter.with_defaultfmt(format);
    }
    if let Some(usetex) = params.opt_bool("usetex")? {
        formatter = formatter.with_usetex(usetex);
    }
    let (labels, _) = locate_and_format(args, &formatter)?;
    Ok(Json::from(labels))
}

/// A list of format strings (`None` when absent or empty, as matplotlib
/// treats a falsy list).
fn formats(params: Args<'_>, key: &str) -> Result<Option<Vec<String>>, ProbeError> {
    let Some(value) = params.get(key) else {
        return Ok(None);
    };
    let items = value
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs(format!("{key} must be a list")))?
        .iter()
        .map(|item| item.as_str().map(str::to_owned))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| ProbeError::BadArgs(format!("{key} must hold strings")))?;
    Ok(Some(items).filter(|items| !items.is_empty()))
}

fn as_strs(list: &[String]) -> Vec<&str> {
    list.iter().map(String::as_str).collect()
}

pub(crate) fn concise_format_ticks(args: Args<'_>) -> Outcome {
    let keys = [
        "formats",
        "zero_formats",
        "offset_formats",
        "show_offset",
        "usetex",
    ];
    let formatter = concise_formatter(Params::of(args, &keys)?.args())?;
    let (labels, offset) = locate_and_format(args, &formatter)?;
    Ok(ObjectBuilder::default()
        .field("labels", labels)
        .field("offset", offset)
        .into())
}

/// `ConciseDateFormatter(locator, **params)` for the keyword arguments
/// `formats`, `zero_formats`, `offset_formats`, `show_offset` and `usetex`.
pub(crate) fn concise_formatter(params: Args<'_>) -> Result<ConciseDateFormatter, ProbeError> {
    let mut formatter = ConciseDateFormatter::new();
    if let Some(list) = formats(params, "formats")? {
        formatter = formatter
            .with_formats(&as_strs(&list))
            .map_err(ProbeError::rejected)?;
    }
    if let Some(list) = formats(params, "zero_formats")? {
        formatter = formatter
            .with_zero_formats(&as_strs(&list))
            .map_err(ProbeError::rejected)?;
    }
    if let Some(list) = formats(params, "offset_formats")? {
        formatter = formatter
            .with_offset_formats(&as_strs(&list))
            .map_err(ProbeError::rejected)?;
    }
    if let Some(show) = params.opt_bool("show_offset")? {
        formatter = formatter.with_show_offset(show);
    }
    if let Some(usetex) = params.opt_bool("usetex")? {
        formatter = formatter.with_usetex(usetex);
    }
    Ok(formatter)
}

pub(super) fn date_formatter_call(args: Args<'_>) -> Outcome {
    let mut formatter = DateFormatter::new(args.str("fmt")?);
    if let Some(usetex) = args.opt_bool("usetex")? {
        formatter = formatter.with_usetex(usetex);
    }
    let x = date_num(args.str("d")?)?;
    formatter
        .label(x, &TickContext::default())
        .map(Json::from)
        .map_err(ProbeError::rejected)
}
