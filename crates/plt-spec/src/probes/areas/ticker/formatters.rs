//! Formatter adapters of the `ticker` area.

use plt::ticker::{
    EngFormatter, FixedFormatter, FormatStrFormatter, Formatter, LogFormatter, LogLabelStyle,
    LogitFormatter, NullFormatter, OffsetMode, PercentFormatter, ScalarFormatter,
    StrMethodFormatter, TickContext, TickerSettings,
};

use super::{Params, bad, format_ticks, int_pair, rejected, settings, text, usize_arg, view_axis};
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError};

fn offset_mode(value: &Json) -> Result<OffsetMode, ProbeError> {
    match (value.as_bool(), value.as_f64()) {
        (Some(true), _) => Ok(OffsetMode::Auto),
        (Some(false), _) => Ok(OffsetMode::Off),
        (_, Some(offset)) => Ok(OffsetMode::Fixed(offset)),
        _ => Err(bad("useOffset must be a bool or a number")),
    }
}

fn scalar(args: Args<'_>, settings: &TickerSettings) -> Result<ScalarFormatter, ProbeError> {
    let p = Params::of(
        args,
        &[
            "useOffset",
            "useMathText",
            "useLocale",
            "scientific",
            "powerlimits",
        ],
    )?
    .args();
    if p.opt_bool("useLocale")? == Some(true) {
        return Err(ProbeError::Rejected(
            "locale-aware formatting is not supported".into(),
        ));
    }
    let mut formatter = ScalarFormatter::from_settings(settings);
    if let Some(use_offset) = p.get("useOffset") {
        formatter = formatter.with_use_offset(offset_mode(use_offset)?);
    }
    if let Some(mathtext) = p.opt_bool("useMathText")? {
        formatter = formatter.with_use_mathtext(mathtext);
    }
    if let Some(scientific) = p.opt_bool("scientific")? {
        formatter = formatter.with_scientific(scientific);
    }
    if let Some(limits) = p.get("powerlimits") {
        formatter = formatter.with_powerlimits(int_pair(limits)?);
    }
    Ok(formatter)
}

pub(super) fn scalar_format_ticks(args: Args<'_>) -> Outcome {
    let settings = settings(args)?;
    format_ticks(&scalar(args, &settings)?, args, &settings)
}

pub(super) fn scalar_format_data(args: Args<'_>) -> Outcome {
    let settings = settings(args)?;
    let formatter = scalar(args, &settings)?;
    text(formatter.format_data(args.f64("value")?, &TickContext::detached(&settings)))
}

pub(super) fn scalar_format_data_short(args: Args<'_>) -> Outcome {
    let settings = settings(args)?;
    let formatter = scalar(args, &settings)?;
    let axis = view_axis(args)?;
    let ctx = TickContext::new(&axis, &settings);
    // matplotlib formats Python ints with "%d"; Rust data is always f64,
    // so the integer branch only exists in this adapter.
    if let Some(integer) = args
        .get("value")
        .filter(|v| v.is_integer())
        .and_then(Json::as_i64)
    {
        return Ok(Json::from(settings.fix_minus(&integer.to_string())));
    }
    text(formatter.format_data_short(args.f64("value")?, &ctx))
}

fn eng(args: Args<'_>, settings: &TickerSettings) -> Result<EngFormatter, ProbeError> {
    let p = Params::of(args, &["unit", "places", "sep", "useMathText", "useOffset"])?.args();
    let mut formatter = EngFormatter::from_settings(settings)
        .with_places(usize_arg(p.opt_i64("places")?, "places")?);
    if let Some(unit) = p.opt_str("unit")? {
        formatter = formatter.with_unit(unit);
    }
    if let Some(sep) = p.opt_str("sep")? {
        formatter = formatter.with_sep(sep);
    }
    if let Some(mathtext) = p.opt_bool("useMathText")? {
        formatter = formatter.with_use_mathtext(mathtext);
    }
    if let Some(use_offset) = p.get("useOffset") {
        formatter = formatter.with_use_offset(offset_mode(use_offset)?);
    }
    Ok(formatter)
}

pub(super) fn eng_format_ticks(args: Args<'_>) -> Outcome {
    let settings = settings(args)?;
    format_ticks(&eng(args, &settings)?, args, &settings)
}

pub(super) fn eng_format_data(args: Args<'_>) -> Outcome {
    let settings = settings(args)?;
    let formatter = eng(args, &settings)?;
    text(formatter.format_data(args.f64("value")?, &TickContext::detached(&settings)))
}

fn log_format_ticks(style: LogLabelStyle, args: Args<'_>) -> Outcome {
    let settings = settings(args)?;
    let p = Params::of(
        args,
        &["base", "labelOnlyBase", "minor_thresholds", "linthresh"],
    )?
    .args();
    let mut formatter =
        LogFormatter::from_settings(style, p.opt_f64("base")?.unwrap_or(10.0), &settings)
            .with_label_only_base(p.opt_bool("labelOnlyBase")?.unwrap_or(false))
            .with_linthresh(p.opt_f64("linthresh")?);
    if let Some(thresholds) = p.get("minor_thresholds") {
        match thresholds.as_f64_vec().as_deref() {
            Some(&[subset, all]) => formatter = formatter.with_minor_thresholds(subset, all),
            _ => return Err(bad("minor_thresholds must be a pair")),
        }
    }
    format_ticks(&formatter, args, &settings)
}

pub(super) fn log_plain_format_ticks(args: Args<'_>) -> Outcome {
    log_format_ticks(LogLabelStyle::Plain, args)
}

pub(super) fn log_exponent_format_ticks(args: Args<'_>) -> Outcome {
    log_format_ticks(LogLabelStyle::Exponent, args)
}

pub(super) fn log_mathtext_format_ticks(args: Args<'_>) -> Outcome {
    log_format_ticks(LogLabelStyle::Mathtext, args)
}

pub(super) fn log_sci_format_ticks(args: Args<'_>) -> Outcome {
    log_format_ticks(LogLabelStyle::SciNotation, args)
}

pub(super) fn log_format_data_short(args: Args<'_>) -> Outcome {
    let formatter = LogFormatter::new(LogLabelStyle::Plain, 10.0);
    text(formatter.format_data_short(args.f64("value")?, &TickContext::default()))
}

pub(super) fn logit_format_ticks(args: Args<'_>) -> Outcome {
    let p = Params::of(
        args,
        &[
            "use_overline",
            "one_half",
            "minor",
            "minor_threshold",
            "minor_number",
        ],
    )?
    .args();
    let mut formatter = LogitFormatter::new()
        .with_use_overline(p.opt_bool("use_overline")?.unwrap_or(false))
        .with_minor(p.opt_bool("minor")?.unwrap_or(false));
    if let Some(one_half) = p.opt_str("one_half")? {
        formatter = formatter.with_one_half(one_half);
    }
    if let Some(n) = usize_arg(p.opt_i64("minor_threshold")?, "minor_threshold")? {
        formatter = formatter.with_minor_threshold(n);
    }
    if let Some(n) = usize_arg(p.opt_i64("minor_number")?, "minor_number")? {
        formatter = formatter.with_minor_number(n);
    }
    format_ticks(&formatter, args, &settings(args)?)
}

pub(super) fn percent_format_ticks(args: Args<'_>) -> Outcome {
    let params = Params::of(args, &["xmax", "decimals", "symbol", "is_latex"])?;
    let p = params.args();
    let decimals = p
        .opt_i64("decimals")?
        .map(|d| i32::try_from(d).map_err(|_| bad("decimals out of range")))
        .transpose()?;
    let symbol = match params.raw("symbol") {
        None => Some("%".to_owned()),
        Some(symbol) => symbol.as_str().map(str::to_owned),
    };
    let formatter = PercentFormatter::new(p.opt_f64("xmax")?.unwrap_or(100.0))
        .with_decimals(decimals)
        .with_symbol(symbol)
        .with_is_latex(p.opt_bool("is_latex")?.unwrap_or(false));
    format_ticks(&formatter, args, &settings(args)?)
}

pub(super) fn str_method_format_ticks(args: Args<'_>) -> Outcome {
    let p = Params::of(args, &["fmt"])?.args();
    let formatter = StrMethodFormatter::new(p.str("fmt")?).map_err(rejected)?;
    format_ticks(&formatter, args, &settings(args)?)
}

pub(super) fn format_str_format_ticks(args: Args<'_>) -> Outcome {
    let p = Params::of(args, &["fmt"])?.args();
    let formatter = FormatStrFormatter::new(p.str("fmt")?).map_err(rejected)?;
    format_ticks(&formatter, args, &settings(args)?)
}

pub(super) fn fixed_format_ticks(args: Args<'_>) -> Outcome {
    let p = Params::of(args, &["seq"])?.args();
    let seq = p
        .json("seq")?
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .map(|s| s.as_str().map(str::to_owned))
                .collect()
        })
        .ok_or_else(|| bad("seq must be an array of strings"))?;
    format_ticks(&FixedFormatter::new(seq), args, &settings(args)?)
}

pub(super) fn null_format_ticks(args: Args<'_>) -> Outcome {
    format_ticks(&NullFormatter, args, &settings(args)?)
}
