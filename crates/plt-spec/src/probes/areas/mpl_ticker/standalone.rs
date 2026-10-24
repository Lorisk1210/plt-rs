//! Locators and formatters used on their own, for the calls of
//! `test_ticker.py` the `ticker` area has no op for.
//!
//! plt-rs formatters label a whole set of ticks at once
//! ([`Formatter::format_ticks`]) instead of keeping matplotlib's
//! `set_locs` state. A formatter called on one value is therefore
//! evaluated as the labels of that value alone, and one called after
//! `set_locs(locs)` on a value of `locs` as its label among them.

use plt::ticker::{
    AutoLocator, FixedFormatter, Formatter, FuncFormatter, LinearLocator, Locator, LogFormatter,
    LogLabelStyle, LogitFormatter, MaxNLocator, Nbins, NullFormatter, ScalarFormatter, TickContext,
    TickLabels, TickerError, TickerSettings,
};

use super::ticker::{self, Params, floats, offset_mode, settings, text, usize_arg, view_axis};
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// The steps of `MaxNLocator(steps=steps)`.
pub(super) fn max_n_steps(args: Args<'_>) -> Outcome {
    let locator = MaxNLocator::new(Nbins::Fixed(10))
        .with_steps(&args.f64_vec("steps")?)
        .map_err(ProbeError::rejected)?;
    Ok(Json::from(locator.steps().to_vec()))
}

/// The steps of `AutoLocator()`.
pub(super) fn auto_steps(args: Args<'_>) -> Outcome {
    let locator = AutoLocator::from_settings(&settings(args)?);
    Ok(Json::from(locator.as_max_n().steps().to_vec()))
}

/// `LinearLocator(numticks, presets).tick_values(vmin, vmax)`; `presets`
/// is a list of `[[vmin, vmax], locs]`.
pub(super) fn linear(args: Args<'_>) -> Outcome {
    let params = Params::of(args, &["numticks", "presets"])?;
    let p = params.args();
    let mut locator = LinearLocator::new(usize_arg(p.opt_i64("numticks")?, "numticks")?);
    for preset in params
        .raw("presets")
        .and_then(Json::as_array)
        .unwrap_or(&[])
    {
        let (limits, locs) = match preset.as_array() {
            Some([limits, locs]) => (limits.as_f64_vec(), locs.as_f64_vec()),
            _ => (None, None),
        };
        match (limits.as_deref(), locs) {
            (Some(&[vmin, vmax]), Some(locs)) => locator = locator.with_preset(vmin, vmax, locs),
            _ => return Err(ProbeError::BadArgs(format!("bad preset {preset}"))),
        }
    }
    let settings = settings(args)?;
    floats(locator.tick_values(
        args.f64("vmin")?,
        args.f64("vmax")?,
        &TickContext::detached(&settings),
    ))
}

/// `SymmetricalLogLocator(...)()` on a dummy axis showing `view`.
pub(super) fn symlog_call(args: Args<'_>) -> Outcome {
    let locator = ticker::symlog(args)?;
    let axis = view_axis(args)?;
    let settings = settings(args)?;
    floats(locator.locate(&TickContext::new(&axis, &settings)))
}

/// `{useOffset, offset}` of a `ScalarFormatter`, after `set_useOffset`
/// when the probe gives `useOffset`.
pub(super) fn scalar_use_offset(args: Args<'_>) -> Outcome {
    let mut formatter = ScalarFormatter::from_settings(&settings(args)?);
    if let Some(use_offset) = args.get("useOffset") {
        formatter = formatter.with_use_offset(offset_mode(use_offset)?);
    }
    let mode = formatter.use_offset();
    Ok(encode::object([
        ("useOffset", Json::from(mode.is_auto())),
        ("offset", Json::from(mode.fixed_offset())),
    ]))
}

/// `ScalarFormatter().get_useMathText()`.
pub(super) fn scalar_use_mathtext(args: Args<'_>) -> Outcome {
    let formatter = ScalarFormatter::from_settings(&settings(args)?);
    Ok(Json::from(formatter.use_mathtext()))
}

/// The label of `value` alone.
fn alone(
    formatter: &dyn Formatter,
    value: f64,
    ctx: &TickContext<'_>,
) -> Result<String, TickerError> {
    let TickLabels { mut labels, .. } = formatter.format_ticks(&[value], ctx)?;
    Ok(labels.pop().unwrap_or_default())
}

/// The labels of `values`, each alone.
fn each_alone(formatter: &dyn Formatter, values: &[f64], ctx: &TickContext<'_>) -> Outcome {
    values
        .iter()
        .map(|&x| alone(formatter, x, ctx).map(Json::from))
        .collect::<Result<Vec<_>, _>>()
        .map(Json::Array)
        .map_err(ProbeError::rejected)
}

/// `[formatter(x, pos), ...]` for a log formatter whose `set_locs` was
/// never called: no coefficient is singled out for labels, which is what
/// infinite minor thresholds give (positions do not change log labels).
fn log_call(style: LogLabelStyle, args: Args<'_>) -> Outcome {
    let settings = settings(args)?;
    let formatter = ticker::log_formatter(style, args, &settings)?
        .with_minor_thresholds(f64::INFINITY, f64::INFINITY);
    let axis = args.get("view").map(|_| view_axis(args)).transpose()?;
    let ctx = axis.as_ref().map_or_else(
        || TickContext::detached(&settings),
        |axis| TickContext::new(axis, &settings),
    );
    each_alone(&formatter, &args.f64_vec("values")?, &ctx)
}

pub(super) fn log_plain_call(args: Args<'_>) -> Outcome {
    log_call(LogLabelStyle::Plain, args)
}

pub(super) fn log_exponent_call(args: Args<'_>) -> Outcome {
    log_call(LogLabelStyle::Exponent, args)
}

pub(super) fn log_mathtext_call(args: Args<'_>) -> Outcome {
    log_call(LogLabelStyle::Mathtext, args)
}

pub(super) fn log_sci_call(args: Args<'_>) -> Outcome {
    log_call(LogLabelStyle::SciNotation, args)
}

/// `LogFormatter._pprint_val(value, domain)`.
pub(super) fn pprint_val(args: Args<'_>) -> Outcome {
    Ok(Json::from(LogFormatter::pprint_val(
        args.f64("value")?,
        args.f64("domain")?,
    )))
}

/// `[formatter(x), ...]` after `set_locs(locs)`: a value of `locs` takes
/// its label among them, any other value its label alone.
pub(super) fn logit_call(args: Args<'_>) -> Outcome {
    let settings = settings(args)?;
    let ctx = TickContext::detached(&settings);
    let formatter = ticker::logit(args)?;
    let locs = args.get("locs").map(|_| args.f64_vec("locs")).transpose()?;
    let among = locs
        .as_deref()
        .map(|locs| formatter.format_ticks(locs, &ctx))
        .transpose()
        .map_err(ProbeError::rejected)?;
    #[expect(
        clippy::float_cmp,
        reason = "a value of `locs` is the very number, as in matplotlib's lookup"
    )]
    let label = |x: f64| -> Result<String, TickerError> {
        let shown = locs
            .as_deref()
            .zip(among.as_ref())
            .and_then(|(locs, among)| {
                Some(among.labels[locs.iter().position(|&l| l == x)?].clone())
            });
        shown.map_or_else(|| alone(&formatter, x, &ctx), Ok)
    };
    args.f64_vec("values")?
        .into_iter()
        .map(|x| label(x).map(Json::from))
        .collect::<Result<Vec<_>, _>>()
        .map(Json::Array)
        .map_err(ProbeError::rejected)
}

/// `LogitFormatter().format_data_short(x)` for each value.
pub(super) fn logit_data_short(args: Args<'_>) -> Outcome {
    let settings = settings(args)?;
    let ctx = TickContext::detached(&settings);
    let formatter = LogitFormatter::new();
    args.f64_vec("values")?
        .into_iter()
        .map(|x| formatter.format_data_short(x, &ctx).map(Json::from))
        .collect::<Result<Vec<_>, _>>()
        .map(Json::Array)
        .map_err(ProbeError::rejected)
}

/// `EngFormatter(**params)(value)`.
pub(super) fn eng_call(args: Args<'_>) -> Outcome {
    let settings = settings(args)?;
    let formatter = ticker::eng(args, &settings)?;
    text(alone(
        &formatter,
        args.f64("value")?,
        &TickContext::detached(&settings),
    ))
}

/// `PercentFormatter(**params).format_pct(x, display_range)`.
pub(super) fn percent_format_pct(args: Args<'_>) -> Outcome {
    let formatter = ticker::percent(args)?;
    text(formatter.format_pct(args.f64("x")?, args.f64("display_range")?, &settings(args)?))
}

/// A `NullFormatter` evaluation in the probe's settings.
fn null(
    args: Args<'_>,
    evaluate: impl FnOnce(f64, &TickContext<'_>) -> Result<String, TickerError>,
) -> Outcome {
    let settings = settings(args)?;
    text(evaluate(
        args.f64("value")?,
        &TickContext::detached(&settings),
    ))
}

pub(super) fn null_call(args: Args<'_>) -> Outcome {
    null(args, |value, ctx| alone(&NullFormatter, value, ctx))
}

pub(super) fn null_data(args: Args<'_>) -> Outcome {
    null(args, |value, ctx| NullFormatter.format_data(value, ctx))
}

pub(super) fn null_data_short(args: Args<'_>) -> Outcome {
    null(args, |value, ctx| {
        NullFormatter.format_data_short(value, ctx)
    })
}

/// The offset text of `formatter`, after `set_offset_string` when the
/// probe gives `offset_string`.
fn offset_text(args: Args<'_>, formatter: &dyn Formatter) -> Outcome {
    let settings: TickerSettings = settings(args)?;
    text(
        formatter
            .format_ticks(&[], &TickContext::detached(&settings))
            .map(|labels| labels.offset),
    )
}

/// The `offset_string` argument, empty when absent.
fn offset_string(args: Args<'_>) -> Result<String, ProbeError> {
    Ok(args
        .opt_str("offset_string")?
        .unwrap_or_default()
        .to_owned())
}

/// `FixedFormatter(('foo', 'bar')).get_offset()`.
pub(super) fn fixed_offset(args: Args<'_>) -> Outcome {
    let formatter = FixedFormatter::new(vec!["foo".into(), "bar".into()])
        .with_offset_string(offset_string(args)?);
    offset_text(args, &formatter)
}

/// `FuncFormatter(lambda a: f'val: {a}').get_offset()`.
pub(super) fn func_offset(args: Args<'_>) -> Outcome {
    let formatter =
        FuncFormatter::new(|a, _| format!("val: {a}")).with_offset_string(offset_string(args)?);
    offset_text(args, &formatter)
}
