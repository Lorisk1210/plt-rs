//! `_native.ticker`: tick locating and formatting (matplotlib's
//! `ticker`).
//!
//! A Python locator or formatter keeps matplotlib's parameters as its
//! attributes and describes itself to the native side as a *spec*: a dict
//! with its `kind` and those parameters. pltrs (Rust) builds the matching
//! `pltrs::ticker` locator or formatter from the spec and computes the ticks
//! or labels against the axis the Python helper is attached to, described
//! by an *axis info* dict (view and data intervals, smallest positive
//! datum, tick space, major ticks), or a dummy axis.
//!
//! The specs of the helpers pltrs (Rust) installs on axes by itself go
//! the other way ([`describe`]), so that Python makes the matching
//! objects; installing helpers on an axis is `_native.axis`'s.

pub(super) mod describe;
mod locale;

use std::rc::Rc;

use pltrs::polar::ThetaLocator;
use pltrs::ticker::{
    AsinhLocator, AutoLocator, AutoMinorLocator, AxisInfo, EngFormatter, FixedFormatter,
    FixedLocator, FormatStrFormatter, Formatter, IndexLocator, LinearLocator, Locator, LogBase,
    LogFormatter, LogLabelStyle, LogLocator, LogSubs, LogitFormatter, LogitLocator, MaxNLocator,
    MinorDivisions, MultipleLocator, Nbins, NullFormatter, NullLocator, NumTicks, OffsetMode,
    PercentFormatter, Prune, ScalarFormatter, ScaleHint, StrMethodFormatter, SymmetricalLogLocator,
    TickContext, TickerError, TickerSettings,
};

use crate::binding::dict_value;
use crate::binding::prelude::*;

impl From<TickerError> for PyErr {
    /// matplotlib's `ValueError` for invalid parameters and data,
    /// Python's `OverflowError` for an infinity rounded to an integer,
    /// `NotImplementedError` for a logit radial axis and `TypeError` for a
    /// tick that cannot be made.
    fn from(err: TickerError) -> Self {
        let kind = match err {
            TickerError::NotAnInteger { infinite: true } => ExceptionKind::OverflowError,
            TickerError::PolarLogit => ExceptionKind::NotImplementedError,
            TickerError::TickKeywordTwice(_) => ExceptionKind::TypeError,
            _ => ExceptionKind::ValueError,
        };
        PyErr::new(kind, err.to_string())
    }
}

/// The rcParams in effect, as the ticker reads them.
fn settings() -> TickerSettings {
    TickerSettings::from(&*pltrs::rc::params())
}

/// The axis described by `info` (`None`: no axis).
fn axis_info(info: Option<&Bound<'_>>) -> PyResult<Option<AxisInfo>> {
    let Some(info) = info else {
        return Ok(None);
    };
    if let Ok(drawn) = info.extract::<AxisInfo>() {
        return Ok(Some(drawn));
    }
    let scale: String = dict_value(info, "scale")?;
    let linthresh: Option<f64> = dict_value(info, "linthresh")?;
    Ok(Some(AxisInfo {
        view_interval: dict_value(info, "view_interval")?,
        data_interval: dict_value(info, "data_interval")?,
        minpos: dict_value(info, "minpos")?,
        tick_space: dict_value(info, "tick_space")?,
        major_ticks: dict_value(info, "major_ticks")?,
        scale: match (scale.as_str(), linthresh) {
            ("linear", _) => ScaleHint::Linear,
            ("log", _) => ScaleHint::Log,
            ("symlog", Some(linthresh)) => ScaleHint::SymLog { linthresh },
            _ => ScaleHint::Other,
        },
        ..AxisInfo::default()
    }))
}

/// `"auto"` or a count.
fn auto_or<'py, T: FromPy<'py>>(spec: &Bound<'py>, key: &str) -> PyResult<Option<T>> {
    let value: Bound<'py> = dict_value(spec, key)?;
    if value.extract::<String>().is_ok_and(|text| text == "auto") || value.is_none() {
        Ok(None)
    } else {
        value.extract().map(Some)
    }
}

/// The locator a spec describes.
pub(super) fn locator(spec: &Bound<'_>) -> PyResult<Box<dyn Locator>> {
    let kind: String = dict_value(spec, "kind")?;
    Ok(match kind.as_str() {
        "null" => Box::new(NullLocator),
        "theta" => {
            let base: Bound<'_> = dict_value(spec, "base")?;
            Box::new(ThetaLocator::wrapping(Rc::from(locator(&base)?)))
        }
        "auto" => Box::new(AutoLocator::from_settings(&settings())),
        "maxn" => {
            let mut locator =
                MaxNLocator::new(auto_or(spec, "nbins")?.map_or(Nbins::Auto, Nbins::Fixed))
                    .with_integer(dict_value(spec, "integer")?)
                    .with_symmetric(dict_value(spec, "symmetric")?)
                    .with_min_n_ticks(dict_value(spec, "min_n_ticks")?);
            let steps: Vec<f64> = dict_value(spec, "steps")?;
            locator = locator.with_steps(&steps)?;
            let prune: Option<String> = dict_value(spec, "prune")?;
            Box::new(locator.with_prune(match prune.as_deref() {
                None => None,
                Some("lower") => Some(Prune::Lower),
                Some("upper") => Some(Prune::Upper),
                Some(_) => Some(Prune::Both),
            }))
        }
        "linear" => {
            let presets: Vec<((f64, f64), Vec<f64>)> = dict_value(spec, "presets")?;
            let locator = presets.into_iter().fold(
                LinearLocator::new(dict_value(spec, "numticks")?),
                |locator, ((vmin, vmax), locs)| locator.with_preset(vmin, vmax, locs),
            );
            Box::new(locator)
        }
        "multiple" => Box::new(
            MultipleLocator::new(dict_value(spec, "base")?)?
                .with_offset(dict_value(spec, "offset")?),
        ),
        "fixed" => {
            let locator = FixedLocator::new(dict_value(spec, "locs")?);
            let nbins: Option<usize> = dict_value(spec, "nbins")?;
            Box::new(match nbins {
                Some(nbins) => locator.with_nbins(nbins),
                None => locator,
            })
        }
        "index" => Box::new(IndexLocator::new(
            dict_value(spec, "base")?,
            dict_value(spec, "offset")?,
        )),
        "log" => Box::new(log_locator(spec)?),
        "symlog" => {
            let mut locator = SymmetricalLogLocator::new(
                dict_value(spec, "base")?,
                dict_value(spec, "linthresh")?,
            )
            .with_numticks(dict_value(spec, "numticks")?);
            if let Some(subs) = dict_value::<Option<Vec<f64>>>(spec, "subs")? {
                locator = locator.with_subs(subs);
            }
            Box::new(locator)
        }
        "asinh" => {
            let mut locator = AsinhLocator::new(dict_value(spec, "linear_width")?)
                .with_numticks(dict_value(spec, "numticks")?)
                .with_symthresh(dict_value(spec, "symthresh")?)
                .with_base(dict_value(spec, "base")?);
            if let Some(subs) = dict_value::<Option<Vec<f64>>>(spec, "subs")? {
                locator = locator.with_subs(subs);
            }
            Box::new(locator)
        }
        "logit" => Box::new(
            LogitLocator::new(dict_value(spec, "minor")?)
                .with_nbins(auto_or(spec, "nbins")?.map_or(Nbins::Auto, Nbins::Fixed)),
        ),
        "autominor" => {
            let ndivs: Option<u32> = auto_or(spec, "n")?;
            let explicit: Bound<'_> = dict_value(spec, "n")?;
            Box::new(AutoMinorLocator::new(if explicit.is_none() {
                None
            } else {
                Some(ndivs.map_or(MinorDivisions::Auto, MinorDivisions::Count))
            }))
        }
        _ => {
            return super::dates::date_locator(&kind, spec)?
                .ok_or_else(|| PyErr::value_error(format!("no locator kind {kind:?}")));
        }
    })
}

/// A `LogLocator` from its spec (`base`, `subs`, `numticks`).
fn log_locator(spec: &Bound<'_>) -> PyResult<LogLocator> {
    let subs: Bound<'_> = dict_value(spec, "subs")?;
    let subs = match subs.extract::<String>().ok().as_deref() {
        Some("auto") => LogSubs::Auto,
        Some("all") => LogSubs::All,
        Some(other) => {
            return Err(PyErr::value_error(format!(
                "subs parameter ({other}) must be None, 'all', 'auto' or a sequence of floats"
            )));
        }
        None => LogSubs::Explicit(subs.extract()?),
    };
    let numticks = auto_or(spec, "numticks")?.map_or(NumTicks::Auto, NumTicks::Fixed);
    Ok(LogLocator::new(dict_value(spec, "base")?)?
        .with_subs(subs)
        .with_numticks(numticks))
}

/// Runs `f` with the context of the axis `info` describes.
pub(super) fn with_context<R>(
    info: Option<Owned>,
    f: impl FnOnce(&TickContext<'_>) -> PyResult<R>,
) -> PyResult<R> {
    let info = Python::with_gil(|py| axis_info(info.map(|info| info.bind(py)).as_ref()))?;
    let settings = settings();
    let ctx = match &info {
        Some(info) => TickContext::new(info, &settings),
        None => TickContext::detached(&settings),
    };
    f(&ctx)
}

/// `tick_values(vmin, vmax)` of the locator `spec`, on the axis `info`.
fn tick_values(spec: Owned, vmin: f64, vmax: f64, info: Option<Owned>) -> PyResult<NdArray> {
    let locator = Python::with_gil(|py| locator(&spec.bind(py)))?;
    with_context(info, |ctx| {
        Ok(NdArray::from_vec(locator.tick_values(vmin, vmax, ctx)?))
    })
}

/// The ticks of the locator `spec` on the axis `info` (`__call__`).
fn locate(spec: Owned, info: Option<Owned>) -> PyResult<NdArray> {
    let locator = Python::with_gil(|py| locator(&spec.bind(py)))?;
    with_context(info, |ctx| Ok(NdArray::from_vec(locator.locate(ctx)?)))
}

/// `view_limits(dmin, dmax)` of the locator `spec`.
fn view_limits(spec: Owned, dmin: f64, dmax: f64, info: Option<Owned>) -> PyResult<(f64, f64)> {
    let locator = Python::with_gil(|py| locator(&spec.bind(py)))?;
    with_context(info, |ctx| Ok(locator.view_limits(dmin, dmax, ctx)?))
}

/// `nonsingular(v0, v1)` of the locator `spec`.
fn nonsingular(spec: Owned, v0: f64, v1: f64, info: Option<Owned>) -> PyResult<(f64, f64)> {
    let locator = Python::with_gil(|py| locator(&spec.bind(py)))?;
    with_context(info, |ctx| Ok(locator.nonsingular(v0, v1, ctx)))
}

/// The label style of a log formatter kind.
fn log_style(kind: &str) -> Option<LogLabelStyle> {
    Some(match kind {
        "log" => LogLabelStyle::Plain,
        "log_exponent" => LogLabelStyle::Exponent,
        "log_mathtext" => LogLabelStyle::Mathtext,
        "log_scinotation" => LogLabelStyle::SciNotation,
        _ => return None,
    })
}

/// The log formatter a spec of a log kind describes.
fn log_formatter(spec: &Bound<'_>) -> PyResult<LogFormatter> {
    let kind: String = dict_value(spec, "kind")?;
    let style =
        log_style(&kind).ok_or_else(|| PyErr::value_error(format!("no log formatter {kind:?}")))?;
    let base: f64 = dict_value(spec, "base")?;
    let (subset, all): (f64, f64) = dict_value(spec, "minor_thresholds")?;
    Ok(LogFormatter::from_settings(style, base, &settings())
        .with_label_only_base(dict_value(spec, "label_only_base")?)
        .with_minor_thresholds(subset, all)
        .with_linthresh(dict_value(spec, "linthresh")?))
}

/// The percent formatter a `"percent"` spec describes.
fn percent_formatter(spec: &Bound<'_>) -> PyResult<PercentFormatter> {
    Ok(PercentFormatter::new(dict_value(spec, "xmax")?)
        .with_decimals(dict_value(spec, "decimals")?)
        .with_symbol(dict_value(spec, "symbol")?)
        .with_is_latex(dict_value(spec, "is_latex")?))
}

/// The formatter a spec describes.
pub(super) fn formatter(spec: &Bound<'_>) -> PyResult<Box<dyn Formatter>> {
    let kind: String = dict_value(spec, "kind")?;
    if log_style(&kind).is_some() {
        return Ok(Box::new(log_formatter(spec)?));
    }
    Ok(match kind.as_str() {
        "null" => Box::new(NullFormatter),
        "fixed" => Box::new(FixedFormatter::new(dict_value(spec, "seq")?)),
        "format_str" => Box::new(FormatStrFormatter::new(&dict_value::<String>(
            spec, "fmt",
        )?)?),
        "str_method" => Box::new(StrMethodFormatter::new(&dict_value::<String>(
            spec, "fmt",
        )?)?),
        "scalar" => Box::new(scalar_formatter(spec)?),
        "logit" => Box::new(
            LogitFormatter::new()
                .with_use_overline(dict_value(spec, "use_overline")?)
                .with_one_half(dict_value::<String>(spec, "one_half")?)
                .with_minor(dict_value(spec, "minor")?)
                .with_minor_threshold(dict_value(spec, "minor_threshold")?)
                .with_minor_number(dict_value(spec, "minor_number")?),
        ),
        "eng" => Box::new(eng_formatter(spec)?),
        "percent" => Box::new(percent_formatter(spec)?),
        _ => return Err(PyErr::value_error(format!("no formatter kind {kind:?}"))),
    })
}

/// The scalar formatter a `"scalar"` spec describes.
fn scalar_formatter(spec: &Bound<'_>) -> PyResult<ScalarFormatter> {
    let use_offset: Bound<'_> = dict_value(spec, "use_offset")?;
    let use_offset = match use_offset.extract::<bool>() {
        Ok(true) if use_offset.type_name() == "bool" => OffsetMode::Auto,
        Ok(false) if use_offset.type_name() == "bool" => OffsetMode::Off,
        _ => OffsetMode::Fixed(use_offset.extract()?),
    };
    Ok(ScalarFormatter::from_settings(&settings())
        .with_use_offset(use_offset)
        .with_use_mathtext(dict_value(spec, "use_mathtext")?)
        .with_usetex(dict_value(spec, "usetex")?)
        .with_scientific(dict_value(spec, "scientific")?)
        .with_powerlimits(dict_value(spec, "powerlimits")?)
        .with_locale(dict_value::<Option<locale::PyLocale>>(spec, "locale")?.map(|conv| conv.0)))
}

/// The engineering formatter an `"eng"` spec describes.
fn eng_formatter(spec: &Bound<'_>) -> PyResult<EngFormatter> {
    let use_offset: bool = dict_value(spec, "use_offset")?;
    Ok(EngFormatter::from_settings(&settings())
        .with_unit(dict_value::<String>(spec, "unit")?)
        .with_places(dict_value(spec, "places")?)
        .with_sep(dict_value::<String>(spec, "sep")?)
        .with_usetex(dict_value(spec, "usetex")?)
        .with_use_mathtext(dict_value(spec, "use_mathtext")?)
        .with_prefixes(
            dict_value::<Vec<(i64, String)>>(spec, "prefixes")?
                .into_iter()
                .collect(),
        )
        .with_use_offset(if use_offset {
            OffsetMode::Auto
        } else {
            OffsetMode::Off
        }))
}

/// A formatter writing ticks with an offset and an order of magnitude.
enum OffsetFormatter {
    /// matplotlib's `ScalarFormatter`.
    Scalar(ScalarFormatter),
    /// matplotlib's `EngFormatter`, whose orders are multiples of three.
    Eng(EngFormatter),
}

/// The offset and order of magnitude the scalar (or engineering)
/// formatter `spec` gives the ticks `values` on the axis `info`
/// (`ScalarFormatter.set_locs`).
fn scalar_offset(spec: Owned, values: NdArray, info: Option<Owned>) -> PyResult<(f64, i32)> {
    let formatter = Python::with_gil(|py| -> PyResult<_> {
        let spec = spec.bind(py);
        Ok(if dict_value::<String>(&spec, "kind")? == "eng" {
            OffsetFormatter::Eng(eng_formatter(&spec)?)
        } else {
            OffsetFormatter::Scalar(scalar_formatter(&spec)?)
        })
    })?;
    with_context(info, |ctx| {
        Ok(match &formatter {
            OffsetFormatter::Scalar(scalar) => scalar.offset_and_order(&values.data, ctx)?,
            OffsetFormatter::Eng(eng) => eng.offset_and_order(&values.data, ctx)?,
        })
    })
}

/// The printf format of the labels the scalar formatter `spec` gives the
/// ticks at `values` on the axis `info` (matplotlib's `format`).
fn scalar_format(spec: Owned, values: NdArray, info: Option<Owned>) -> PyResult<String> {
    let formatter = Python::with_gil(|py| scalar_formatter(&spec.bind(py)))?;
    with_context(info, |ctx| Ok(formatter.label_format(&values.data, ctx)?))
}

/// `format_ticks(values)` of the formatter `spec` on the axis `info`: the
/// labels and the offset text.
fn format_ticks(
    spec: Owned,
    values: NdArray,
    info: Option<Owned>,
) -> PyResult<(Vec<String>, String)> {
    let formatter = Python::with_gil(|py| formatter(&spec.bind(py)))?;
    with_context(info, |ctx| {
        let formatted = formatter.format_ticks(&values.data, ctx)?;
        Ok((formatted.labels, formatted.offset))
    })
}

/// `format_data(value)` (or, `short`, `format_data_short(value)`) of the
/// formatter `spec` on the axis `info`.
fn format_data(spec: Owned, value: f64, short: bool, info: Option<Owned>) -> PyResult<String> {
    let formatter = Python::with_gil(|py| formatter(&spec.bind(py)))?;
    with_context(info, |ctx| {
        Ok(if short {
            formatter.format_data_short(value, ctx)?
        } else {
            formatter.format_data(value, ctx)?
        })
    })
}

/// The coefficients of the ticks the log formatter `spec` labels on the
/// axis `info`, `None` for all (`LogFormatter.set_locs`, kept as
/// `_sublabels`).
fn log_sublabels(spec: Owned, info: Option<Owned>) -> PyResult<Option<Vec<f64>>> {
    let formatter = Python::with_gil(|py| log_formatter(&spec.bind(py)))?;
    with_context(info, |ctx| Ok(formatter.sublabels(ctx)?))
}

/// The label the log formatter `spec` gives `x` when the ticks with the
/// coefficients `sublabels` get labels (`LogFormatter.__call__`).
fn log_label(
    spec: Owned,
    x: f64,
    sublabels: Option<Vec<f64>>,
    info: Option<Owned>,
) -> PyResult<String> {
    let formatter = Python::with_gil(|py| log_formatter(&spec.bind(py)))?;
    with_context(info, |ctx| {
        Ok(formatter.call(x, sublabels.as_deref(), ctx)?)
    })
}

/// `LogFormatter._pprint_val(x, d)`: `x` written for a view spanning `d`.
fn pprint_val(x: f64, d: f64) -> PyResult<String> {
    Ok(LogFormatter::pprint_val(x, d))
}

/// `PercentFormatter.format_pct(x, display_range)` of the formatter
/// `spec`.
fn format_pct(spec: Owned, x: f64, display_range: f64) -> PyResult<String> {
    let formatter = Python::with_gil(|py| percent_formatter(&spec.bind(py)))?;
    Ok(formatter.format_pct(x, display_range, &settings())?)
}

/// matplotlib's `scale_range(vmin, vmax, n, threshold)`.
fn scale_range(vmin: f64, vmax: f64, n: f64, threshold: f64) -> PyResult<(f64, f64)> {
    Ok(pltrs::ticker::scale_range(vmin, vmax, n, threshold))
}

/// Whether `base` is a valid log base (finite, positive, not 1).
fn is_log_base(base: f64) -> PyResult<bool> {
    Ok(LogBase::new(base).is_some())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("tick_values", tick_values)?;
    module.add("locate", locate)?;
    module.add("view_limits", view_limits)?;
    module.add("nonsingular", nonsingular)?;
    module.add("format_ticks", format_ticks)?;
    module.add("scalar_offset", scalar_offset)?;
    module.add("scalar_format", scalar_format)?;
    module.add("format_data", format_data)?;
    module.add("log_sublabels", log_sublabels)?;
    module.add("log_label", log_label)?;
    module.add("pprint_val", pprint_val)?;
    module.add("format_pct", format_pct)?;
    module.add("scale_range", scale_range)?;
    module.add("is_log_base", is_log_base)
}
