//! Probe operations for the `ticker` area.
//!
//! Each adapter builds the plt-rs locator or formatter described by the
//! probe's `params` (matplotlib constructor keywords), evaluates it against
//! a dummy axis like the reference module does, and returns the result in
//! the golden's JSON shape.

mod formatters;
mod locators;

pub(crate) use formatters::{
    eng, format_str_format_ticks, integer_label, log_formatter, logit, logit_format_ticks,
    offset_mode, percent, scalar, scalar_format_data, scalar_format_data_short,
};
pub(crate) use locators::{
    asinh_call, asinh_tick_values, index_tick_values, log_locator, log_tick_values,
    logit_nonsingular, logit_tick_values, maxn_tick_values, maxn_view_limits, multiple_tick_values,
    multiple_view_limits, symlog, symlog_tick_values, symlog_view_limits,
};

use plt::ticker::{
    AxisInfo, FormattedTicks, Formatter, Locator, MinorDivisions, TickContext, TickerError,
    TickerSettings,
};

use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("ticker.Locator.nonsingular", locators::locator_nonsingular),
    Op::new("ticker.MaxNLocator.tick_values", locators::maxn_tick_values),
    Op::new("ticker.MaxNLocator.view_limits", locators::maxn_view_limits),
    Op::new("ticker.AutoLocator.tick_values", locators::auto_tick_values),
    Op::new("ticker.AutoLocator.view_limits", locators::auto_view_limits),
    Op::new(
        "ticker.MultipleLocator.tick_values",
        locators::multiple_tick_values,
    ),
    Op::new(
        "ticker.MultipleLocator.view_limits",
        locators::multiple_view_limits,
    ),
    Op::new(
        "ticker.FixedLocator.tick_values",
        locators::fixed_tick_values,
    ),
    Op::new(
        "ticker.LinearLocator.tick_values",
        locators::linear_tick_values,
    ),
    Op::new(
        "ticker.LinearLocator.view_limits",
        locators::linear_view_limits,
    ),
    Op::new(
        "ticker.IndexLocator.tick_values",
        locators::index_tick_values,
    ),
    Op::new("ticker.NullLocator.tick_values", locators::null_tick_values),
    Op::new("ticker.LogLocator.tick_values", locators::log_tick_values),
    Op::new("ticker.LogLocator.view_limits", locators::log_view_limits),
    Op::new("ticker.LogLocator.nonsingular", locators::log_nonsingular),
    Op::new(
        "ticker.SymmetricalLogLocator.tick_values",
        locators::symlog_tick_values,
    ),
    Op::new(
        "ticker.SymmetricalLogLocator.view_limits",
        locators::symlog_view_limits,
    ),
    Op::new(
        "ticker.AsinhLocator.tick_values",
        locators::asinh_tick_values,
    ),
    Op::new("ticker.AsinhLocator.__call__", locators::asinh_call),
    Op::new(
        "ticker.LogitLocator.tick_values",
        locators::logit_tick_values,
    ),
    Op::new(
        "ticker.LogitLocator.nonsingular",
        locators::logit_nonsingular,
    ),
    Op::new(
        "ticker.AutoMinorLocator.__call__",
        locators::auto_minor_call,
    ),
    Op::new(
        "ticker.ScalarFormatter.format_ticks",
        formatters::scalar_format_ticks,
    ),
    Op::new(
        "ticker.ScalarFormatter.format_data",
        formatters::scalar_format_data,
    ),
    Op::new(
        "ticker.ScalarFormatter.format_data_short",
        formatters::scalar_format_data_short,
    ),
    Op::new(
        "ticker.EngFormatter.format_ticks",
        formatters::eng_format_ticks,
    ),
    Op::new(
        "ticker.EngFormatter.format_data",
        formatters::eng_format_data,
    ),
    Op::new(
        "ticker.LogFormatter.format_ticks",
        formatters::log_plain_format_ticks,
    ),
    Op::new(
        "ticker.LogFormatterExponent.format_ticks",
        formatters::log_exponent_format_ticks,
    ),
    Op::new(
        "ticker.LogFormatterMathtext.format_ticks",
        formatters::log_mathtext_format_ticks,
    ),
    Op::new(
        "ticker.LogFormatterSciNotation.format_ticks",
        formatters::log_sci_format_ticks,
    ),
    Op::new(
        "ticker.LogFormatter.format_data_short",
        formatters::log_format_data_short,
    ),
    Op::new(
        "ticker.LogitFormatter.format_ticks",
        formatters::logit_format_ticks,
    ),
    Op::new(
        "ticker.PercentFormatter.format_ticks",
        formatters::percent_format_ticks,
    ),
    Op::new(
        "ticker.StrMethodFormatter.format_ticks",
        formatters::str_method_format_ticks,
    ),
    Op::new(
        "ticker.FormatStrFormatter.format_ticks",
        formatters::format_str_format_ticks,
    ),
    Op::new(
        "ticker.FixedFormatter.format_ticks",
        formatters::fixed_format_ticks,
    ),
    Op::new(
        "ticker.NullFormatter.format_ticks",
        formatters::null_format_ticks,
    ),
];

static EMPTY: Json = Json::Object(Vec::new());

/// The constructor keywords of a probe, restricted to `allowed` (other
/// keywords make matplotlib raise `TypeError`).
#[derive(Clone, Copy)]
pub(crate) struct Params<'a> {
    raw: &'a Json,
}

impl<'a> Params<'a> {
    pub(crate) fn of(args: Args<'a>, allowed: &[&str]) -> Result<Self, ProbeError> {
        let raw = args.get("params").unwrap_or(&EMPTY);
        let entries = raw
            .as_object()
            .ok_or_else(|| ProbeError::BadArgs("params is not an object".into()))?;
        if let Some((key, _)) = entries.iter().find(|(k, _)| !allowed.contains(&k.as_str())) {
            return Err(ProbeError::Rejected(format!(
                "unexpected keyword argument {key:?}"
            )));
        }
        Ok(Self { raw })
    }

    /// Typed access (`null` counts as absent).
    pub(crate) fn args(self) -> Args<'a> {
        Args::new(self.raw)
    }

    /// The raw value, including an explicit `null`.
    pub(crate) fn raw(self, key: &str) -> Option<&'a Json> {
        self.raw.get(key)
    }
}

/// The rcParams of the probe's optional `rc` argument.
pub(crate) fn settings(args: Args<'_>) -> Result<TickerSettings, ProbeError> {
    let mut s = TickerSettings::default();
    let Some(rc) = args.get("rc") else {
        return Ok(s);
    };
    let entries = rc
        .as_object()
        .ok_or_else(|| ProbeError::BadArgs("rc is not an object".into()))?;
    for (key, value) in entries {
        let flag = || {
            value
                .as_bool()
                .ok_or_else(|| ProbeError::BadArgs(format!("{key} is not a bool")))
        };
        let int = || {
            value
                .as_i64()
                .and_then(|v| i32::try_from(v).ok())
                .ok_or_else(|| ProbeError::BadArgs(format!("{key} is not an integer")))
        };
        match key.as_str() {
            "axes.autolimit_mode" => {
                s.axes_autolimit_mode = value
                    .as_str()
                    .and_then(|mode| mode.parse().ok())
                    .ok_or_else(|| ProbeError::BadArgs("unknown autolimit mode".into()))?;
            }
            "axes.unicode_minus" => s.axes_unicode_minus = flag()?,
            "axes.formatter.useoffset" => s.axes_formatter_useoffset = flag()?,
            "axes.formatter.use_mathtext" => s.axes_formatter_use_mathtext = flag()?,
            "axes.formatter.offset_threshold" => s.axes_formatter_offset_threshold = int()?,
            "axes.formatter.min_exponent" => s.axes_formatter_min_exponent = int()?,
            "axes.formatter.limits" => s.axes_formatter_limits = int_pair(value)?,
            "_internal.classic_mode" => s.classic_mode = flag()?,
            "text.usetex" => s.text_usetex = flag()?,
            "xtick.minor.ndivs" => s.xtick_minor_ndivs = minor_divisions(value)?,
            "ytick.minor.ndivs" => s.ytick_minor_ndivs = minor_divisions(value)?,
            "xtick.minor.visible" => s.xtick_minor_visible = flag()?,
            "ytick.minor.visible" => s.ytick_minor_visible = flag()?,
            other => return Err(ProbeError::BadArgs(format!("unsupported rcParam {other}"))),
        }
    }
    Ok(s)
}

pub(crate) fn int_pair(value: &Json) -> Result<(i32, i32), ProbeError> {
    match value.as_f64_vec().as_deref() {
        Some(&[a, b]) if a.fract() == 0.0 && b.fract() == 0.0 => Ok((a as i32, b as i32)),
        Some(_) => Err(ProbeError::Rejected("expected a pair of integers".into())),
        None => Err(ProbeError::BadArgs("expected an array of numbers".into())),
    }
}

pub(crate) fn minor_divisions(value: &Json) -> Result<MinorDivisions, ProbeError> {
    match (value.as_str(), value.as_i64()) {
        (Some("auto"), _) => Ok(MinorDivisions::Auto),
        (_, Some(n)) => u32::try_from(n)
            .map(MinorDivisions::Count)
            .map_err(|_| ProbeError::Rejected("negative ndivs".into())),
        _ => Err(ProbeError::BadArgs(
            "ndivs must be an integer or 'auto'".into(),
        )),
    }
}

pub(crate) fn usize_arg(value: Option<i64>, name: &str) -> Result<Option<usize>, ProbeError> {
    value
        .map(|v| usize::try_from(v).map_err(|_| ProbeError::Rejected(format!("negative {name}"))))
        .transpose()
}

fn pair_arg(args: Args<'_>, key: &str) -> Result<(f64, f64), ProbeError> {
    match args.f64_vec(key)?.as_slice() {
        &[a, b] => Ok((a, b)),
        _ => Err(ProbeError::BadArgs(format!("{key} is not a pair"))),
    }
}

/// A dummy axis whose view and data intervals are the probe's `view`.
pub(crate) fn view_axis(args: Args<'_>) -> Result<AxisInfo, ProbeError> {
    let (vmin, vmax) = pair_arg(args, "view")?;
    Ok(AxisInfo::with_view(vmin, vmax))
}

pub(crate) fn floats(values: Result<Vec<f64>, TickerError>) -> Outcome {
    values.map(Json::from).map_err(ProbeError::rejected)
}

pub(crate) fn pair(values: Result<(f64, f64), TickerError>) -> Outcome {
    values
        .map(|(a, b)| Json::from(vec![a, b]))
        .map_err(ProbeError::rejected)
}

pub(crate) fn labels(result: Result<FormattedTicks, TickerError>) -> Outcome {
    let labels = result.map_err(ProbeError::rejected)?;
    Ok(Json::Object(vec![
        ("labels".to_owned(), Json::from(labels.labels)),
        ("offset".to_owned(), Json::from(labels.offset)),
    ]))
}

/// Evaluates `locator.tick_values(vmin, vmax)` without an axis.
fn tick_values(locator: &dyn Locator, args: Args<'_>) -> Outcome {
    let settings = settings(args)?;
    let ctx = TickContext::detached(&settings);
    floats(locator.tick_values(args.f64("vmin")?, args.f64("vmax")?, &ctx))
}

/// Evaluates `locator.view_limits(lo, hi)` without an axis.
fn view_limits(locator: &dyn Locator, args: Args<'_>, lo: &str, hi: &str) -> Outcome {
    let settings = settings(args)?;
    let ctx = TickContext::detached(&settings);
    pair(locator.view_limits(args.f64(lo)?, args.f64(hi)?, &ctx))
}

/// `formatter.format_ticks(locs)` on a dummy axis showing `view`.
fn format_ticks(formatter: &dyn Formatter, args: Args<'_>, settings: &TickerSettings) -> Outcome {
    let axis = view_axis(args)?;
    let ctx = TickContext::new(&axis, settings);
    labels(formatter.format_ticks(&args.f64_vec("locs")?, &ctx))
}

pub(crate) fn text(result: Result<String, TickerError>) -> Outcome {
    result.map(Json::from).map_err(ProbeError::rejected)
}
