//! Probe operations for the `dates` area.
//!
//! Dates arrive as ISO 8601 strings (naive ones are UTC). Locators are
//! evaluated like the reference module does: attached to a dummy axis whose
//! view interval is `[date2num(vmin), date2num(vmax)]` and then called.

use plt::dates::{
    AutoDateFormatter, AutoDateLocator, AutoFrequency, ChosenLocator, ConciseDateFormatter,
    DateFormatter, DateLocator, Datetime64, Frequency, MicrosecondLocator, NthWeekday,
    RRuleLocator, RRuleSpec, TimeDelta, TimeZone, Weekday, YearLocator, ZonedDateTime, date2num,
    datetime64_to_num, drange, get_epoch, num2date, num2timedelta,
};
use plt::ticker::{AxisInfo, Formatter, Locator, TickContext, TickerSettings};

use super::ticker::Params;
use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Generates the `__call__` and `nonsingular` adapters of the fixed-frequency
/// locators.
macro_rules! fixed_locator_ops {
    ($($class:literal => $call:ident, $nonsingular:ident;)*) => {
        $(
            pub(crate) fn $call(args: Args<'_>) -> Outcome {
                let locator = fixed_locator($class, args)?;
                ticks(located(locator.as_ref(), args)?)
            }

            fn $nonsingular(args: Args<'_>) -> Outcome {
                let locator = fixed_locator($class, args)?;
                nonsingular(locator.as_ref(), args)
            }
        )*
    };
}

fixed_locator_ops! {
    "YearLocator" => year_call, year_nonsingular;
    "MonthLocator" => month_call, month_nonsingular;
    "WeekdayLocator" => weekday_call, weekday_nonsingular;
    "DayLocator" => day_call, day_nonsingular;
    "HourLocator" => hour_call, hour_nonsingular;
    "MinuteLocator" => minute_call, minute_nonsingular;
    "SecondLocator" => second_call, second_nonsingular;
    "MicrosecondLocator" => microsecond_call, microsecond_nonsingular;
}

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("dates.get_epoch", epoch),
    Op::new("dates.date2num", date2num_op),
    Op::new("dates.date2num.datetime64", date2num_datetime64),
    Op::new("dates.num2date", num2date_op),
    Op::new("dates.num2timedelta", num2timedelta_op),
    Op::new("dates.drange", drange_op),
    Op::new("dates.AutoDateLocator.__call__", auto_call),
    Op::new("dates.AutoDateLocator.nonsingular", auto_nonsingular),
    Op::new("dates.AutoDateFormatter.format_ticks", auto_format_ticks),
    Op::new(
        "dates.ConciseDateFormatter.format_ticks",
        concise_format_ticks,
    ),
    Op::new("dates.YearLocator.__call__", year_call),
    Op::new("dates.YearLocator.nonsingular", year_nonsingular),
    Op::new("dates.MonthLocator.__call__", month_call),
    Op::new("dates.MonthLocator.nonsingular", month_nonsingular),
    Op::new("dates.WeekdayLocator.__call__", weekday_call),
    Op::new("dates.WeekdayLocator.nonsingular", weekday_nonsingular),
    Op::new("dates.DayLocator.__call__", day_call),
    Op::new("dates.DayLocator.nonsingular", day_nonsingular),
    Op::new("dates.HourLocator.__call__", hour_call),
    Op::new("dates.HourLocator.nonsingular", hour_nonsingular),
    Op::new("dates.MinuteLocator.__call__", minute_call),
    Op::new("dates.MinuteLocator.nonsingular", minute_nonsingular),
    Op::new("dates.SecondLocator.__call__", second_call),
    Op::new("dates.SecondLocator.nonsingular", second_nonsingular),
    Op::new("dates.MicrosecondLocator.__call__", microsecond_call),
    Op::new(
        "dates.MicrosecondLocator.nonsingular",
        microsecond_nonsingular,
    ),
    Op::new("dates.RRuleLocator.__call__", rrule_call),
    Op::new("dates.DateFormatter.__call__", date_formatter_call),
];

// ---------------------------------------------------------------------------
// Argument helpers
// ---------------------------------------------------------------------------

fn bad(why: impl Into<String>) -> ProbeError {
    ProbeError::BadArgs(why.into())
}

/// `_dt(iso)`: naive strings are UTC.
pub(crate) fn zoned(text: &str) -> Result<ZonedDateTime, ProbeError> {
    ZonedDateTime::parse_iso(text).map_err(ProbeError::rejected)
}

pub(crate) fn date_num(text: &str) -> Result<f64, ProbeError> {
    date2num(&zoned(text)?).map_err(ProbeError::rejected)
}

/// `num2date(x).isoformat()`.
pub(crate) fn iso(x: f64) -> Result<Json, ProbeError> {
    num2date(x, &TimeZone::Utc)
        .map(|d| Json::from(d.isoformat()))
        .map_err(ProbeError::rejected)
}

/// An integer or a list of integers (`None` when absent), like dateutil's
/// `by*` arguments.
fn int_list(value: Option<&Json>) -> Result<Option<Vec<i64>>, ProbeError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let items = match value.as_array() {
        Some(items) => items.iter().map(Json::as_i64).collect::<Option<Vec<_>>>(),
        None => value.as_i64().map(|n| vec![n]),
    };
    items
        .map(Some)
        .ok_or_else(|| bad("expected an integer or a list of integers"))
}

fn convert_all<T: TryFrom<i64>>(values: Vec<i64>) -> Result<Vec<T>, ProbeError> {
    values
        .into_iter()
        .map(|v| T::try_from(v).map_err(|_| ProbeError::Rejected(format!("{v} is out of range"))))
        .collect()
}

/// An integer list argument converted to `T`; empty when absent.
fn by_list<T: TryFrom<i64>>(args: Args<'_>, key: &str) -> Result<Vec<T>, ProbeError> {
    convert_all(int_list(args.get(key))?.unwrap_or_default())
}

/// A positive integer argument; matplotlib rejects fractional and
/// non-positive intervals.
fn positive_int(args: Args<'_>, key: &str, default: u32) -> Result<u32, ProbeError> {
    let Some(value) = args.opt_f64(key)? else {
        return Ok(default);
    };
    if value.fract() != 0.0 || value < 1.0 || value > f64::from(u32::MAX) {
        return Err(ProbeError::Rejected(format!(
            "{key} must be an integer greater than 0"
        )));
    }
    Ok(value as u32)
}

fn weekdays(values: Vec<i64>) -> Vec<Weekday> {
    values.into_iter().map(Weekday::from_index).collect()
}

/// The dummy axis of `_attach`: view (and data) interval
/// `[date2num(vmin), date2num(vmax)]`.
fn view_axis(args: Args<'_>) -> Result<AxisInfo, ProbeError> {
    Ok(AxisInfo::with_view(
        date_num(args.str("vmin")?)?,
        date_num(args.str("vmax")?)?,
    ))
}

/// `locator()` on the dummy axis.
fn located(locator: &dyn Locator, args: Args<'_>) -> Result<Vec<f64>, ProbeError> {
    let axis = view_axis(args)?;
    let settings = TickerSettings::default();
    locator
        .locate(&TickContext::new(&axis, &settings))
        .map_err(ProbeError::rejected)
}

/// `{ticks, iso}`.
pub(crate) fn ticks(ticks: Vec<f64>) -> Outcome {
    Ok(tick_fields(ticks)?.into())
}

/// The `{ticks, iso}` fields as object entries.
fn tick_fields(ticks: Vec<f64>) -> Result<ObjectBuilder, ProbeError> {
    let iso = ticks
        .iter()
        .map(|&t| iso(t))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ObjectBuilder::default()
        .field("ticks", ticks)
        .field("iso", Json::Array(iso)))
}

/// Builds a JSON object field by field.
#[derive(Default)]
struct ObjectBuilder(Vec<(String, Json)>);

impl ObjectBuilder {
    fn field(mut self, key: &str, value: impl Into<Json>) -> Self {
        self.0.push((key.to_owned(), value.into()));
        self
    }
}

impl From<ObjectBuilder> for Json {
    fn from(builder: ObjectBuilder) -> Self {
        Self::Object(builder.0)
    }
}

fn nonsingular(locator: &dyn Locator, args: Args<'_>) -> Outcome {
    let (lo, hi) = locator.nonsingular(
        args.f64("vmin")?,
        args.f64("vmax")?,
        &TickContext::default(),
    );
    Ok(Json::from(vec![lo, hi]))
}

// ---------------------------------------------------------------------------
// Conversions
// ---------------------------------------------------------------------------

fn epoch(_: Args<'_>) -> Outcome {
    Ok(Json::from(get_epoch().to_string()))
}

pub(crate) fn date2num_op(args: Args<'_>) -> Outcome {
    let d = args.json("d")?;
    match (d.as_str(), d.as_array()) {
        (Some(text), _) => Ok(Json::from(date_num(text)?)),
        (_, Some(items)) => items
            .iter()
            .map(|item| {
                let text = item.as_str().ok_or_else(|| bad("d must hold strings"))?;
                date_num(text)
            })
            .collect::<Result<Vec<f64>, _>>()
            .map(Json::from),
        _ => Err(bad("d must be a string or a list of strings")),
    }
}

fn date2num_datetime64(args: Args<'_>) -> Outcome {
    let items = args
        .json("d")?
        .as_array()
        .ok_or_else(|| bad("d must be a list"))?;
    items
        .iter()
        .map(|item| {
            let text = item.as_str().ok_or_else(|| bad("d must hold strings"))?;
            Datetime64::parse(text)
                .map(datetime64_to_num)
                .map_err(ProbeError::rejected)
        })
        .collect::<Result<Vec<f64>, _>>()
        .map(Json::from)
}

fn num2date_op(args: Args<'_>) -> Outcome {
    let x = args.json("x")?;
    match (x.as_f64(), x.as_f64_vec()) {
        (Some(x), _) => iso(x),
        (_, Some(values)) => values
            .into_iter()
            .map(iso)
            .collect::<Result<Vec<_>, _>>()
            .map(Json::Array),
        _ => Err(bad("x must be a number or a list of numbers")),
    }
}

pub(crate) fn num2timedelta_op(args: Args<'_>) -> Outcome {
    let parts = |x: f64| -> Result<Json, ProbeError> {
        let delta = num2timedelta(x).map_err(ProbeError::rejected)?;
        Ok(Json::from(vec![
            delta.days() as f64,
            delta.seconds() as f64,
            delta.microseconds() as f64,
        ]))
    };
    let x = args.json("x")?;
    match (x.as_f64(), x.as_f64_vec()) {
        (Some(x), _) => parts(x),
        (_, Some(values)) => values
            .into_iter()
            .map(parts)
            .collect::<Result<Vec<_>, _>>()
            .map(Json::Array),
        _ => Err(bad("x must be a number or a list of numbers")),
    }
}

pub(crate) fn drange_op(args: Args<'_>) -> Outcome {
    let delta = Args::new(args.json("delta")?);
    let part = |key: &str| delta.opt_i64(key).map(Option::unwrap_or_default);
    let seconds = part("hours")? * 3600 + part("minutes")? * 60 + part("seconds")?;
    let step = TimeDelta::new(part("days")?, seconds, part("microseconds")?)
        .map_err(ProbeError::rejected)?;
    drange(
        &zoned(args.str("dstart")?)?,
        &zoned(args.str("dend")?)?,
        step,
    )
    .map(Json::from)
    .map_err(ProbeError::rejected)
}

// ---------------------------------------------------------------------------
// AutoDateLocator and the automatic formatters
// ---------------------------------------------------------------------------

/// `AutoDateLocator(**params)`; `maxticks` is an int or `{freq name: int}`
/// and `tz` a zone name.
pub(crate) fn auto_locator(params: Option<&Json>) -> Result<AutoDateLocator, ProbeError> {
    static EMPTY: Json = Json::Object(Vec::new());
    let params = Args::new(params.unwrap_or(&EMPTY));
    let mut locator = AutoDateLocator::new();
    if let Some(tz) = params.opt_str("tz")? {
        locator = locator.with_tz(TimeZone::named(tz).map_err(ProbeError::rejected)?);
    }
    if let Some(multiples) = params.opt_bool("interval_multiples")? {
        locator = locator.with_interval_multiples(multiples);
    }
    if let Some(minticks) = params.opt_i64("minticks")? {
        locator = locator.with_minticks(u32::try_from(minticks).map_err(|_| bad("minticks"))?);
    }
    match params.get("maxticks") {
        None => {}
        Some(Json::Object(entries)) => {
            for (name, value) in entries {
                let frequency = AutoFrequency::from_name(name)
                    .ok_or_else(|| bad(format!("frequency {name}")))?;
                let count = value
                    .as_i64()
                    .and_then(|n| u32::try_from(n).ok())
                    .ok_or_else(|| bad("maxticks values must be integers"))?;
                locator = locator.with_maxticks_for(frequency, count);
            }
        }
        Some(value) => {
            let count = value
                .as_i64()
                .and_then(|n| u32::try_from(n).ok())
                .ok_or_else(|| bad("maxticks must be an integer or an object"))?;
            locator = locator.with_maxticks(count);
        }
    }
    Ok(locator)
}

/// A `by*` list as the reference records it: `null` when not given.
fn optional_list<T: Copy + Into<f64>>(values: &[T]) -> Json {
    if values.is_empty() {
        Json::Null
    } else {
        Json::from(values.iter().map(|&v| v.into()).collect::<Vec<f64>>())
    }
}

/// The reference module's `describe_locator`.
fn describe(locator: &ChosenLocator) -> Json {
    match locator {
        ChosenLocator::Year(year) => ObjectBuilder::default()
            .field("class", "YearLocator")
            .field("base", f64::from(year.base()))
            .field("month", f64::from(year.month()))
            .field("day", f64::from(year.day()))
            .into(),
        ChosenLocator::RRule(rrule) => {
            let rule = rrule.rule();
            ObjectBuilder::default()
                .field("class", "RRuleLocator")
                .field("freq", rule.freq.name())
                .field("interval", f64::from(rule.interval))
                .field("bymonth", optional_list(&rule.bymonth))
                .field("bymonthday", optional_list(&rule.bymonthday))
                .field("byhour", optional_list(&rule.byhour))
                .field("byminute", optional_list(&rule.byminute))
                .field("bysecond", optional_list(&rule.bysecond))
                .into()
        }
        ChosenLocator::Microsecond(micro) => ObjectBuilder::default()
            .field("class", "MicrosecondLocator")
            .field("interval", f64::from(micro.micros()))
            .into(),
    }
}

pub(crate) fn auto_call(args: Args<'_>) -> Outcome {
    let locator = auto_locator(args.get("params"))?;
    let axis = view_axis(args)?;
    let settings = TickerSettings::default();
    let ctx = TickContext::new(&axis, &settings);
    let found = locator.locate(&ctx).map_err(ProbeError::rejected)?;
    let (vmin, vmax) = axis.view_interval;
    let choice = locator.choose(vmin, vmax).map_err(ProbeError::rejected)?;
    Ok(tick_fields(found)?
        .field("unit", locator.unit(&ctx))
        .field("locator", describe(&choice.locator))
        .into())
}

fn auto_nonsingular(args: Args<'_>) -> Outcome {
    nonsingular(&AutoDateLocator::new(), args)
}

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
        .ok_or_else(|| bad(format!("{key} must be a list")))?
        .iter()
        .map(|item| item.as_str().map(str::to_owned))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| bad(format!("{key} must hold strings")))?;
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

// ---------------------------------------------------------------------------
// Fixed-frequency locators
// ---------------------------------------------------------------------------

/// `LOCATORS[class](**params)`.
fn fixed_locator(class: &str, args: Args<'_>) -> Result<Box<dyn DateLocator>, ProbeError> {
    let interval = |params: Args<'_>| positive_int(params, "interval", 1);
    Ok(match class {
        "YearLocator" => {
            let params = Params::of(args, &["base", "month", "day"])?.args();
            let get = |key: &str| positive_int(params, key, 1);
            Box::new(
                YearLocator::new(get("base")?, get("month")?, get("day")?)
                    .map_err(ProbeError::rejected)?,
            )
        }
        "MonthLocator" => {
            let params = Params::of(args, &["bymonth", "bymonthday", "interval"])?.args();
            let monthday = int_list(params.get("bymonthday"))?.unwrap_or_else(|| vec![1]);
            Box::new(RRuleLocator::months(
                &by_list(params, "bymonth")?,
                &convert_all(monthday)?,
                interval(params)?,
            ))
        }
        "WeekdayLocator" => {
            let params = Params::of(args, &["byweekday", "interval"])?.args();
            let days = int_list(params.get("byweekday"))?.unwrap_or_else(|| vec![1]);
            Box::new(RRuleLocator::weekdays(&weekdays(days), interval(params)?))
        }
        "DayLocator" => {
            let params = Params::of(args, &["bymonthday", "interval"])?.args();
            Box::new(
                RRuleLocator::days(&by_list(params, "bymonthday")?, interval(params)?)
                    .map_err(ProbeError::rejected)?,
            )
        }
        "HourLocator" => {
            let params = Params::of(args, &["byhour", "interval"])?.args();
            Box::new(RRuleLocator::hours(
                &by_list(params, "byhour")?,
                interval(params)?,
            ))
        }
        "MinuteLocator" => {
            let params = Params::of(args, &["byminute", "interval"])?.args();
            Box::new(RRuleLocator::minutes(
                &by_list(params, "byminute")?,
                interval(params)?,
            ))
        }
        "SecondLocator" => {
            let params = Params::of(args, &["bysecond", "interval"])?.args();
            Box::new(RRuleLocator::seconds(
                &by_list(params, "bysecond")?,
                interval(params)?,
            ))
        }
        "MicrosecondLocator" => {
            let params = Params::of(args, &["interval"])?.args();
            Box::new(MicrosecondLocator::new(interval(params)?).map_err(ProbeError::rejected)?)
        }
        other => return Err(bad(format!("unknown locator {other}"))),
    })
}

fn rrule_call(args: Args<'_>) -> Outcome {
    let spec = rrule_spec(Args::new(args.json("rule")?))?;
    ticks(located(&RRuleLocator::new(spec), args)?)
}

/// The rule `rrulewrapper(freq, **rule)` describes.
pub(crate) fn rrule_spec(rule: Args<'_>) -> Result<RRuleSpec, ProbeError> {
    let freq = Frequency::from_name(rule.str("freq")?).ok_or_else(|| bad("unknown frequency"))?;
    Ok(RRuleSpec {
        interval: positive_int(rule, "interval", 1)?,
        bysetpos: by_list(rule, "bysetpos")?,
        bymonth: by_list(rule, "bymonth")?,
        bymonthday: by_list(rule, "bymonthday")?,
        byyearday: by_list(rule, "byyearday")?,
        byweekday: weekdays(int_list(rule.get("byweekday"))?.unwrap_or_default())
            .into_iter()
            .map(NthWeekday::from)
            .collect(),
        byhour: by_list(rule, "byhour")?,
        byminute: by_list(rule, "byminute")?,
        bysecond: by_list(rule, "bysecond")?,
        ..RRuleSpec::new(freq)
    })
}

// ---------------------------------------------------------------------------
// DateFormatter
// ---------------------------------------------------------------------------

fn date_formatter_call(args: Args<'_>) -> Outcome {
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
