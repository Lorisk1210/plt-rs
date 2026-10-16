//! Probe operations for the `ticker` area.
//!
//! Each adapter builds the plt-rs locator or formatter described by the
//! probe's `params` (matplotlib constructor keywords), evaluates it against
//! a dummy axis like the reference module does, and returns the result in
//! the golden's JSON shape.

use plt::ticker::{
    AsinhLocator, AutoLocator, AutoMinorLocator, AutolimitMode, AxisInfo, AxisName, EngFormatter,
    FixedFormatter, FixedLocator, FormatStrFormatter, Formatter, IndexLocator, LinearLocator,
    Locator, LogFormatter, LogLabelStyle, LogLocator, LogSubs, LogitFormatter, LogitLocator,
    MaxNLocator, MinorDivisions, MultipleLocator, Nbins, NullFormatter, NullLocator, NumTicks,
    OffsetMode, PercentFormatter, Prune, ScalarFormatter, StrMethodFormatter,
    SymmetricalLogLocator, TickContext, TickLabels, TickerError, TickerSettings,
};

use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("ticker.Locator.nonsingular", locator_nonsingular),
    Op::new("ticker.MaxNLocator.tick_values", maxn_tick_values),
    Op::new("ticker.MaxNLocator.view_limits", maxn_view_limits),
    Op::new("ticker.AutoLocator.tick_values", auto_tick_values),
    Op::new("ticker.AutoLocator.view_limits", auto_view_limits),
    Op::new("ticker.MultipleLocator.tick_values", multiple_tick_values),
    Op::new("ticker.MultipleLocator.view_limits", multiple_view_limits),
    Op::new("ticker.FixedLocator.tick_values", fixed_tick_values),
    Op::new("ticker.LinearLocator.tick_values", linear_tick_values),
    Op::new("ticker.LinearLocator.view_limits", linear_view_limits),
    Op::new("ticker.IndexLocator.tick_values", index_tick_values),
    Op::new("ticker.NullLocator.tick_values", null_tick_values),
    Op::new("ticker.LogLocator.tick_values", log_tick_values),
    Op::new("ticker.LogLocator.view_limits", log_view_limits),
    Op::new("ticker.LogLocator.nonsingular", log_nonsingular),
    Op::new(
        "ticker.SymmetricalLogLocator.tick_values",
        symlog_tick_values,
    ),
    Op::new(
        "ticker.SymmetricalLogLocator.view_limits",
        symlog_view_limits,
    ),
    Op::new("ticker.AsinhLocator.tick_values", asinh_tick_values),
    Op::new("ticker.AsinhLocator.__call__", asinh_call),
    Op::new("ticker.LogitLocator.tick_values", logit_tick_values),
    Op::new("ticker.LogitLocator.nonsingular", logit_nonsingular),
    Op::new("ticker.AutoMinorLocator.__call__", auto_minor_call),
    Op::new("ticker.ScalarFormatter.format_ticks", scalar_format_ticks),
    Op::new("ticker.ScalarFormatter.format_data", scalar_format_data),
    Op::new(
        "ticker.ScalarFormatter.format_data_short",
        scalar_format_data_short,
    ),
    Op::new("ticker.EngFormatter.format_ticks", eng_format_ticks),
    Op::new("ticker.EngFormatter.format_data", eng_format_data),
    Op::new("ticker.LogFormatter.format_ticks", log_plain_format_ticks),
    Op::new(
        "ticker.LogFormatterExponent.format_ticks",
        log_exponent_format_ticks,
    ),
    Op::new(
        "ticker.LogFormatterMathtext.format_ticks",
        log_mathtext_format_ticks,
    ),
    Op::new(
        "ticker.LogFormatterSciNotation.format_ticks",
        log_sci_format_ticks,
    ),
    Op::new(
        "ticker.LogFormatter.format_data_short",
        log_format_data_short,
    ),
    Op::new("ticker.LogitFormatter.format_ticks", logit_format_ticks),
    Op::new("ticker.PercentFormatter.format_ticks", percent_format_ticks),
    Op::new(
        "ticker.StrMethodFormatter.format_ticks",
        str_method_format_ticks,
    ),
    Op::new(
        "ticker.FormatStrFormatter.format_ticks",
        format_str_format_ticks,
    ),
    Op::new("ticker.FixedFormatter.format_ticks", fixed_format_ticks),
    Op::new("ticker.NullFormatter.format_ticks", null_format_ticks),
];

// ---------------------------------------------------------------------------
// Argument helpers
// ---------------------------------------------------------------------------

static EMPTY: Json = Json::Object(Vec::new());

fn bad(why: impl Into<String>) -> ProbeError {
    ProbeError::BadArgs(why.into())
}

fn rejected(err: TickerError) -> ProbeError {
    ProbeError::rejected(err)
}

/// The constructor keywords of a probe, restricted to `allowed` (other
/// keywords make matplotlib raise `TypeError`).
#[derive(Clone, Copy)]
pub(super) struct Params<'a> {
    raw: &'a Json,
}

impl<'a> Params<'a> {
    pub(super) fn of(args: Args<'a>, allowed: &[&str]) -> Result<Self, ProbeError> {
        let raw = args.get("params").unwrap_or(&EMPTY);
        let entries = raw
            .as_object()
            .ok_or_else(|| bad("params is not an object"))?;
        if let Some((key, _)) = entries.iter().find(|(k, _)| !allowed.contains(&k.as_str())) {
            return Err(ProbeError::Rejected(format!(
                "unexpected keyword argument {key:?}"
            )));
        }
        Ok(Self { raw })
    }

    /// Typed access (`null` counts as absent).
    pub(super) fn args(self) -> Args<'a> {
        Args::new(self.raw)
    }

    /// The raw value, including an explicit `null`.
    pub(super) fn raw(self, key: &str) -> Option<&'a Json> {
        self.raw.get(key)
    }
}

/// The rcParams of the probe's optional `rc` argument.
pub(super) fn settings(args: Args<'_>) -> Result<TickerSettings, ProbeError> {
    let mut s = TickerSettings::default();
    let Some(rc) = args.get("rc") else {
        return Ok(s);
    };
    let entries = rc.as_object().ok_or_else(|| bad("rc is not an object"))?;
    for (key, value) in entries {
        let flag = || {
            value
                .as_bool()
                .ok_or_else(|| bad(format!("{key} is not a bool")))
        };
        let int = || {
            value
                .as_i64()
                .and_then(|v| i32::try_from(v).ok())
                .ok_or_else(|| bad(format!("{key} is not an integer")))
        };
        match key.as_str() {
            "axes.autolimit_mode" => {
                s.axes_autolimit_mode = match value.as_str() {
                    Some("data") => AutolimitMode::Data,
                    Some("round_numbers") => AutolimitMode::RoundNumbers,
                    _ => return Err(bad("unknown autolimit mode")),
                }
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
            other => return Err(bad(format!("unsupported rcParam {other}"))),
        }
    }
    Ok(s)
}

fn int_pair(value: &Json) -> Result<(i32, i32), ProbeError> {
    match value.as_f64_vec().as_deref() {
        Some(&[a, b]) if a.fract() == 0.0 && b.fract() == 0.0 => Ok((a as i32, b as i32)),
        Some(_) => Err(ProbeError::Rejected("expected a pair of integers".into())),
        None => Err(bad("expected an array of numbers")),
    }
}

fn minor_divisions(value: &Json) -> Result<MinorDivisions, ProbeError> {
    match (value.as_str(), value.as_i64()) {
        (Some("auto"), _) => Ok(MinorDivisions::Auto),
        (_, Some(n)) => u32::try_from(n)
            .map(MinorDivisions::Count)
            .map_err(|_| ProbeError::Rejected("negative ndivs".into())),
        _ => Err(bad("ndivs must be an integer or 'auto'")),
    }
}

fn usize_arg(value: Option<i64>, name: &str) -> Result<Option<usize>, ProbeError> {
    value
        .map(|v| usize::try_from(v).map_err(|_| ProbeError::Rejected(format!("negative {name}"))))
        .transpose()
}

fn pair_arg(args: Args<'_>, key: &str) -> Result<(f64, f64), ProbeError> {
    match args.f64_vec(key)?.as_slice() {
        &[a, b] => Ok((a, b)),
        _ => Err(bad(format!("{key} is not a pair"))),
    }
}

/// A dummy axis whose view and data intervals are the probe's `view`.
fn view_axis(args: Args<'_>) -> Result<AxisInfo, ProbeError> {
    let (vmin, vmax) = pair_arg(args, "view")?;
    Ok(AxisInfo::with_view(vmin, vmax))
}

fn floats(values: Result<Vec<f64>, TickerError>) -> Outcome {
    values.map(Json::from).map_err(rejected)
}

fn pair(values: Result<(f64, f64), TickerError>) -> Outcome {
    values
        .map(|(a, b)| Json::from(vec![a, b]))
        .map_err(rejected)
}

fn labels(result: Result<TickLabels, TickerError>) -> Outcome {
    let labels = result.map_err(rejected)?;
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

fn text(result: Result<String, TickerError>) -> Outcome {
    result.map(Json::from).map_err(rejected)
}

// ---------------------------------------------------------------------------
// Locators
// ---------------------------------------------------------------------------

fn locator_nonsingular(args: Args<'_>) -> Outcome {
    let (lo, hi) =
        NullLocator.nonsingular(args.f64("v0")?, args.f64("v1")?, &TickContext::default());
    Ok(Json::from(vec![lo, hi]))
}

fn nbins(value: Option<&Json>) -> Result<Option<Nbins>, ProbeError> {
    value
        .filter(|v| !v.is_null())
        .map(|v| match (v.as_str(), v.as_i64()) {
            (Some("auto"), _) => Ok(Nbins::Auto),
            (_, Some(n)) => usize::try_from(n)
                .map(Nbins::Fixed)
                .map_err(|_| ProbeError::Rejected("negative nbins".into())),
            _ => Err(ProbeError::Rejected(format!("invalid nbins {v}"))),
        })
        .transpose()
}

fn max_n(args: Args<'_>) -> Result<MaxNLocator, ProbeError> {
    let params = Params::of(
        args,
        &[
            "nbins",
            "steps",
            "integer",
            "symmetric",
            "prune",
            "min_n_ticks",
        ],
    )?;
    let p = params.args();
    let mut locator = MaxNLocator::new(nbins(params.raw("nbins"))?.unwrap_or(Nbins::Fixed(10)));
    if let Some(steps) = p.get("steps") {
        let steps = steps
            .as_f64_vec()
            .ok_or_else(|| ProbeError::Rejected("steps must be a sequence".into()))?;
        locator = locator.with_steps(&steps).map_err(rejected)?;
    }
    let prune = match p.opt_str("prune")? {
        None => None,
        Some("lower") => Some(Prune::Lower),
        Some("upper") => Some(Prune::Upper),
        Some("both") => Some(Prune::Both),
        Some(other) => return Err(ProbeError::Rejected(format!("invalid prune {other:?}"))),
    };
    let mut locator = locator
        .with_integer(p.opt_bool("integer")?.unwrap_or(false))
        .with_symmetric(p.opt_bool("symmetric")?.unwrap_or(false))
        .with_prune(prune);
    if let Some(n) = p.opt_i64("min_n_ticks")? {
        locator = locator.with_min_n_ticks(usize::try_from(n).unwrap_or(0));
    }
    Ok(locator)
}

fn maxn_tick_values(args: Args<'_>) -> Outcome {
    tick_values(&max_n(args)?, args)
}

fn maxn_view_limits(args: Args<'_>) -> Outcome {
    view_limits(&max_n(args)?, args, "dmin", "dmax")
}

fn auto_tick_values(args: Args<'_>) -> Outcome {
    tick_values(&AutoLocator::from_settings(&settings(args)?), args)
}

fn auto_view_limits(args: Args<'_>) -> Outcome {
    view_limits(
        &AutoLocator::from_settings(&settings(args)?),
        args,
        "dmin",
        "dmax",
    )
}

fn multiple(args: Args<'_>) -> Result<MultipleLocator, ProbeError> {
    let p = Params::of(args, &["base", "offset"])?.args();
    Ok(MultipleLocator::new(p.opt_f64("base")?.unwrap_or(1.0))
        .map_err(rejected)?
        .with_offset(p.opt_f64("offset")?.unwrap_or(0.0)))
}

fn multiple_tick_values(args: Args<'_>) -> Outcome {
    tick_values(&multiple(args)?, args)
}

fn multiple_view_limits(args: Args<'_>) -> Outcome {
    view_limits(&multiple(args)?, args, "dmin", "dmax")
}

fn fixed_tick_values(args: Args<'_>) -> Outcome {
    let p = Params::of(args, &["locs", "nbins"])?.args();
    let locs = p
        .json("locs")?
        .as_f64_vec()
        .ok_or_else(|| ProbeError::Rejected("locs must be one-dimensional".into()))?;
    let mut locator = FixedLocator::new(locs);
    if let Some(n) = usize_arg(p.opt_i64("nbins")?, "nbins")? {
        locator = locator.with_nbins(n);
    }
    floats(locator.locate(&TickContext::default()))
}

fn linear(args: Args<'_>) -> Result<LinearLocator, ProbeError> {
    let p = Params::of(args, &["numticks"])?.args();
    Ok(LinearLocator::new(usize_arg(
        p.opt_i64("numticks")?,
        "numticks",
    )?))
}

fn linear_tick_values(args: Args<'_>) -> Outcome {
    tick_values(&linear(args)?, args)
}

fn linear_view_limits(args: Args<'_>) -> Outcome {
    view_limits(&linear(args)?, args, "vmin", "vmax")
}

fn index_tick_values(args: Args<'_>) -> Outcome {
    let p = Params::of(args, &["base", "offset"])?.args();
    tick_values(&IndexLocator::new(p.f64("base")?, p.f64("offset")?), args)
}

fn null_tick_values(args: Args<'_>) -> Outcome {
    tick_values(&NullLocator, args)
}

fn log_locator(args: Args<'_>) -> Result<LogLocator, ProbeError> {
    let params = Params::of(args, &["base", "subs", "numticks"])?;
    let p = params.args();
    let mut locator = LogLocator::new(p.opt_f64("base")?.unwrap_or(10.0)).map_err(rejected)?;
    if let Some(subs) = params.raw("subs") {
        let subs = match (subs, subs.as_str()) {
            (Json::Null, _) | (_, Some("auto")) => LogSubs::Auto,
            (_, Some("all")) => LogSubs::All,
            _ => LogSubs::Explicit(
                subs.as_f64_vec()
                    .ok_or_else(|| ProbeError::Rejected(format!("invalid subs {subs}")))?,
            ),
        };
        locator = locator.with_subs(subs);
    }
    let numticks = match nbins(params.raw("numticks"))? {
        None | Some(Nbins::Auto) => NumTicks::Auto,
        Some(Nbins::Fixed(n)) => NumTicks::Fixed(n),
    };
    Ok(locator.with_numticks(numticks))
}

/// A dummy axis with the probe's `minpos`.
fn minpos_axis(args: Args<'_>) -> Result<AxisInfo, ProbeError> {
    Ok(AxisInfo {
        minpos: args.f64("minpos")?,
        ..AxisInfo::DUMMY
    })
}

fn log_tick_values(args: Args<'_>) -> Outcome {
    let locator = log_locator(args)?;
    if args.get("minpos").is_none() {
        return tick_values(&locator, args);
    }
    let axis = minpos_axis(args)?;
    let settings = settings(args)?;
    let ctx = TickContext::new(&axis, &settings);
    floats(locator.tick_values(args.f64("vmin")?, args.f64("vmax")?, &ctx))
}

fn log_view_limits(args: Args<'_>) -> Outcome {
    let locator = log_locator(args)?;
    let axis = minpos_axis(args)?;
    let settings = settings(args)?;
    let ctx = TickContext::new(&axis, &settings);
    pair(locator.view_limits(args.f64("vmin")?, args.f64("vmax")?, &ctx))
}

fn log_nonsingular(args: Args<'_>) -> Outcome {
    let locator = log_locator(args)?;
    let axis = minpos_axis(args)?;
    let settings = TickerSettings::default();
    let ctx = TickContext::new(&axis, &settings);
    pair(Ok(locator.nonsingular(
        args.f64("vmin")?,
        args.f64("vmax")?,
        &ctx,
    )))
}

fn symlog(args: Args<'_>) -> Result<SymmetricalLogLocator, ProbeError> {
    let p = Params::of(args, &["base", "linthresh", "subs", "numticks"])?.args();
    let (Some(base), Some(linthresh)) = (p.opt_f64("base")?, p.opt_f64("linthresh")?) else {
        return Err(ProbeError::Rejected(
            "either transform, or both linthresh and base, must be provided".into(),
        ));
    };
    let mut locator = SymmetricalLogLocator::new(base, linthresh);
    if let Some(subs) = p.get("subs") {
        locator = locator.with_subs(subs.as_f64_vec().ok_or_else(|| bad("subs"))?);
    }
    if let Some(n) = usize_arg(p.opt_i64("numticks")?, "numticks")? {
        locator = locator.with_numticks(n);
    }
    Ok(locator)
}

fn symlog_tick_values(args: Args<'_>) -> Outcome {
    tick_values(&symlog(args)?, args)
}

fn symlog_view_limits(args: Args<'_>) -> Outcome {
    view_limits(&symlog(args)?, args, "vmin", "vmax")
}

fn asinh(args: Args<'_>) -> Result<AsinhLocator, ProbeError> {
    let p = Params::of(
        args,
        &["linear_width", "numticks", "symthresh", "base", "subs"],
    )?
    .args();
    let mut locator = AsinhLocator::new(p.f64("linear_width")?);
    if let Some(n) = usize_arg(p.opt_i64("numticks")?, "numticks")? {
        locator = locator.with_numticks(n);
    }
    if let Some(symthresh) = p.opt_f64("symthresh")? {
        locator = locator.with_symthresh(symthresh);
    }
    if let Some(base) = p.opt_f64("base")? {
        locator = locator.with_base(base);
    }
    let subs = p
        .get("subs")
        .map(|s| s.as_f64_vec().ok_or_else(|| bad("subs")))
        .transpose()?;
    Ok(locator.with_subs(subs))
}

fn asinh_tick_values(args: Args<'_>) -> Outcome {
    tick_values(&asinh(args)?, args)
}

fn asinh_call(args: Args<'_>) -> Outcome {
    let axis = view_axis(args)?;
    let settings = TickerSettings::default();
    floats(asinh(args)?.locate(&TickContext::new(&axis, &settings)))
}

fn logit_locator(args: Args<'_>) -> Result<LogitLocator, ProbeError> {
    let params = Params::of(args, &["minor", "nbins"])?;
    let mut locator = LogitLocator::new(params.args().opt_bool("minor")?.unwrap_or(false));
    if let Some(n) = nbins(params.raw("nbins"))? {
        locator = locator.with_nbins(n);
    }
    Ok(locator)
}

fn logit_tick_values(args: Args<'_>) -> Outcome {
    tick_values(&logit_locator(args)?, args)
}

fn logit_nonsingular(args: Args<'_>) -> Outcome {
    let locator = logit_locator(args)?;
    let ctx = TickContext::default();
    pair(Ok(locator.nonsingular(
        args.f64("vmin")?,
        args.f64("vmax")?,
        &ctx,
    )))
}

fn auto_minor_call(args: Args<'_>) -> Outcome {
    let params = Params::of(args, &["n"])?;
    let ndivs = params
        .raw("n")
        .filter(|n| !n.is_null())
        .map(minor_divisions)
        .transpose()?;
    let (vmin, vmax) = pair_arg(args, "view")?;
    let axis = AxisInfo {
        name: match args.str("axis_name")? {
            "y" => AxisName::Y,
            _ => AxisName::X,
        },
        view_interval: (vmin, vmax),
        major_ticks: args.f64_vec("major_locs")?,
        ..AxisInfo::DUMMY
    };
    let settings = settings(args)?;
    floats(AutoMinorLocator::new(ndivs).locate(&TickContext::new(&axis, &settings)))
}

// ---------------------------------------------------------------------------
// Formatters
// ---------------------------------------------------------------------------

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

fn scalar_format_ticks(args: Args<'_>) -> Outcome {
    let settings = settings(args)?;
    format_ticks(&scalar(args, &settings)?, args, &settings)
}

fn scalar_format_data(args: Args<'_>) -> Outcome {
    let settings = settings(args)?;
    let formatter = scalar(args, &settings)?;
    text(formatter.format_data(args.f64("value")?, &TickContext::detached(&settings)))
}

fn scalar_format_data_short(args: Args<'_>) -> Outcome {
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

fn eng_format_ticks(args: Args<'_>) -> Outcome {
    let settings = settings(args)?;
    format_ticks(&eng(args, &settings)?, args, &settings)
}

fn eng_format_data(args: Args<'_>) -> Outcome {
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

fn log_plain_format_ticks(args: Args<'_>) -> Outcome {
    log_format_ticks(LogLabelStyle::Plain, args)
}

fn log_exponent_format_ticks(args: Args<'_>) -> Outcome {
    log_format_ticks(LogLabelStyle::Exponent, args)
}

fn log_mathtext_format_ticks(args: Args<'_>) -> Outcome {
    log_format_ticks(LogLabelStyle::Mathtext, args)
}

fn log_sci_format_ticks(args: Args<'_>) -> Outcome {
    log_format_ticks(LogLabelStyle::SciNotation, args)
}

fn log_format_data_short(args: Args<'_>) -> Outcome {
    let formatter = LogFormatter::new(LogLabelStyle::Plain, 10.0);
    text(formatter.format_data_short(args.f64("value")?, &TickContext::default()))
}

fn logit_format_ticks(args: Args<'_>) -> Outcome {
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

fn percent_format_ticks(args: Args<'_>) -> Outcome {
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

fn str_method_format_ticks(args: Args<'_>) -> Outcome {
    let p = Params::of(args, &["fmt"])?.args();
    let formatter = StrMethodFormatter::new(p.str("fmt")?).map_err(rejected)?;
    format_ticks(&formatter, args, &settings(args)?)
}

fn format_str_format_ticks(args: Args<'_>) -> Outcome {
    let p = Params::of(args, &["fmt"])?.args();
    let formatter = FormatStrFormatter::new(p.str("fmt")?).map_err(rejected)?;
    format_ticks(&formatter, args, &settings(args)?)
}

fn fixed_format_ticks(args: Args<'_>) -> Outcome {
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

fn null_format_ticks(args: Args<'_>) -> Outcome {
    format_ticks(&NullFormatter, args, &settings(args)?)
}
